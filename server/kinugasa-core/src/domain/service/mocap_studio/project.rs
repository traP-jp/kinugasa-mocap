use crate::domain::model::mocap_studio::{event, state};

pub type ProjectionResult<T> = Result<T, ProjectionError>;

#[derive(Debug, Clone, PartialEq, Eq, thiserror::Error)]
pub enum ProjectionError {
    #[error("active camera cannot be deleted")]
    ActiveCameraCannotBeDeleted,

    #[error("take already started")]
    TakeAlreadyStarted,

    #[error("take is not started")]
    TakeNotStarted,
}

pub fn project(
    prev: state::MocapStudio,
    transition: event::MocapStudioEventLatest,
) -> ProjectionResult<state::MocapStudio> {
    match transition {
        event::MocapStudioEventLatest::CameraCreated(transition) => {
            project_camera_created(prev, transition)
        }
        event::MocapStudioEventLatest::CameraDeleted(transition) => {
            project_camera_deleted(prev, transition)
        }
        event::MocapStudioEventLatest::TakeStarted(transition) => {
            project_take_started(prev, transition)
        }
        event::MocapStudioEventLatest::TakeCompleted(transition) => {
            project_take_completed(prev, transition)
        }
    }
}

pub fn project_camera_created(
    mut prev: state::MocapStudio,
    transition: event::CameraCreatedEventLatest,
) -> ProjectionResult<state::MocapStudio> {
    prev.cameras.insert(
        transition.id,
        state::Camera {
            name: transition.name,
            rist_configuration: state::RistConfiguration {
                url: transition.rist_url,
            },
            status: state::CameraStatus::Idle,
        },
    );
    Ok(prev)
}

pub fn project_camera_deleted(
    mut prev: state::MocapStudio,
    transition: event::CameraDeletedEventLatest,
) -> ProjectionResult<state::MocapStudio> {
    if let Some(camera) = prev.cameras.get_mut(&transition.id) {
        if camera.status == state::CameraStatus::Capturing {
            return Err(ProjectionError::ActiveCameraCannotBeDeleted);
        }
        camera.status = state::CameraStatus::Deleted;
    }
    Ok(prev)
}

pub fn project_take_started(
    mut prev: state::MocapStudio,
    transition: event::TakeStartedEventLatest,
) -> ProjectionResult<state::MocapStudio> {
    if prev.ongoing_take.is_some() {
        return Err(ProjectionError::TakeAlreadyStarted);
    }

    let videos = transition
        .video_keys
        .into_iter()
        .map(|video| {
            set_active_camera_status(&mut prev, video.camera_id, state::CameraStatus::Capturing);
            (
                video.id,
                state::Video {
                    take_id: transition.id,
                    camera_id: video.camera_id,
                    video_key: video.video_key,
                },
            )
        })
        .collect();

    prev.ongoing_take = Some((
        transition.id,
        state::Take {
            take_number: transition.take_number,
            started_at: transition.started_at,
            completed_at: None,
            videos,
        },
    ));
    Ok(prev)
}

pub fn project_take_completed(
    mut prev: state::MocapStudio,
    transition: event::TakeCompletedEventLatest,
) -> ProjectionResult<state::MocapStudio> {
    let Some((take_id, take)) = prev.ongoing_take.take() else {
        return Err(ProjectionError::TakeNotStarted);
    };

    if take_id == transition.id {
        let mut take = take;
        for video in take.videos.values() {
            set_active_camera_status(&mut prev, video.camera_id, state::CameraStatus::Idle);
        }
        take.completed_at = Some(transition.completed_at);
        prev.completed_takes.insert(take_id, take);
    } else {
        prev.ongoing_take = Some((take_id, take));
    }

    Ok(prev)
}

fn set_active_camera_status(
    studio: &mut state::MocapStudio,
    camera_id: crate::domain::model::id::CameraId,
    status: state::CameraStatus,
) {
    if let Some(camera) = studio.cameras.get_mut(&camera_id) {
        if camera.status != state::CameraStatus::Deleted {
            camera.status = status;
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::domain::model::id;

    fn timestamp(seconds: i64) -> chrono::DateTime<chrono::Utc> {
        chrono::DateTime::from_timestamp(seconds, 0).expect("test timestamp should be valid")
    }

    #[test]
    fn projects_camera_lifecycle() {
        let camera_id = id::CameraId(uuid::Uuid::from_u128(1));
        let state = state::MocapStudio::default();

        let state = project_camera_created(
            state,
            event::CameraCreatedEventLatest {
                id: camera_id,
                name: "main".to_string(),
                rist_url: "rist://main".to_string(),
            },
        )
        .expect("camera creation should be projected");

        assert_eq!(
            state.cameras.get(&camera_id),
            Some(&state::Camera {
                name: "main".to_string(),
                rist_configuration: state::RistConfiguration {
                    url: "rist://main".to_string(),
                },
                status: state::CameraStatus::Idle,
            })
        );

        let state =
            project_camera_deleted(state, event::CameraDeletedEventLatest { id: camera_id })
                .expect("idle camera deletion should be projected");

        assert_eq!(
            state.cameras.get(&camera_id).map(|camera| &camera.status),
            Some(&state::CameraStatus::Deleted)
        );
    }

    #[test]
    fn projects_take_lifecycle() {
        let camera_id = id::CameraId(uuid::Uuid::from_u128(1));
        let take_id = id::TakeId(uuid::Uuid::from_u128(2));
        let video_id = id::VideoId(uuid::Uuid::from_u128(3));
        let started_at = timestamp(100);
        let completed_at = timestamp(200);
        let state = project_camera_created(
            state::MocapStudio::default(),
            event::CameraCreatedEventLatest {
                id: camera_id,
                name: "main".to_string(),
                rist_url: "rist://main".to_string(),
            },
        )
        .expect("camera creation should be projected");

        let state = project_take_started(
            state,
            event::TakeStartedEventLatest {
                id: take_id,
                take_number: state::TakeNumber(1),
                started_at,
                video_keys: vec![event::take_started_v0::TakeStartedEventV0Video {
                    id: video_id,
                    camera_id,
                    video_key: "videos/take-1-main.mp4".to_string(),
                }],
            },
        )
        .expect("take start should be projected");

        let (_, ongoing_take) = state
            .ongoing_take
            .as_ref()
            .expect("started take should be ongoing");
        assert_eq!(ongoing_take.take_number, state::TakeNumber(1));
        assert_eq!(ongoing_take.started_at, started_at);
        assert_eq!(ongoing_take.completed_at, None);
        assert_eq!(
            ongoing_take.videos.get(&video_id),
            Some(&state::Video {
                take_id,
                camera_id,
                video_key: "videos/take-1-main.mp4".to_string(),
            })
        );
        assert_eq!(
            state.cameras.get(&camera_id).map(|camera| &camera.status),
            Some(&state::CameraStatus::Capturing)
        );

        let state = project_take_completed(
            state,
            event::TakeCompletedEventLatest {
                id: take_id,
                completed_at,
            },
        )
        .expect("take completion should be projected");

        assert!(state.ongoing_take.is_none());
        assert_eq!(
            state
                .completed_takes
                .get(&take_id)
                .map(|take| take.completed_at),
            Some(Some(completed_at))
        );
        assert_eq!(
            state.cameras.get(&camera_id).map(|camera| &camera.status),
            Some(&state::CameraStatus::Idle)
        );
    }

    #[test]
    fn deleted_camera_is_not_reactivated_by_take_lifecycle() {
        let camera_id = id::CameraId(uuid::Uuid::from_u128(1));
        let take_id = id::TakeId(uuid::Uuid::from_u128(2));
        let video_id = id::VideoId(uuid::Uuid::from_u128(3));
        let started_at = timestamp(100);
        let completed_at = timestamp(200);
        let state = project_camera_created(
            state::MocapStudio::default(),
            event::CameraCreatedEventLatest {
                id: camera_id,
                name: "main".to_string(),
                rist_url: "rist://main".to_string(),
            },
        )
        .expect("camera creation should be projected");
        let state =
            project_camera_deleted(state, event::CameraDeletedEventLatest { id: camera_id })
                .expect("idle camera deletion should be projected");
        let state = project_take_started(
            state,
            event::TakeStartedEventLatest {
                id: take_id,
                take_number: state::TakeNumber(1),
                started_at,
                video_keys: vec![event::take_started_v0::TakeStartedEventV0Video {
                    id: video_id,
                    camera_id,
                    video_key: "videos/take-1-main.mp4".to_string(),
                }],
            },
        )
        .expect("take start should be projected");

        let state = project_take_completed(
            state,
            event::TakeCompletedEventLatest {
                id: take_id,
                completed_at,
            },
        )
        .expect("take completion should be projected");

        assert_eq!(
            state.cameras.get(&camera_id).map(|camera| &camera.status),
            Some(&state::CameraStatus::Deleted)
        );
    }

    #[test]
    fn ignores_take_completion_for_a_different_ongoing_take() {
        let take_id = id::TakeId(uuid::Uuid::from_u128(1));
        let other_take_id = id::TakeId(uuid::Uuid::from_u128(2));
        let mut state = state::MocapStudio::default();
        state.ongoing_take = Some((
            take_id,
            state::Take {
                take_number: state::TakeNumber(1),
                started_at: timestamp(100),
                completed_at: None,
                videos: std::collections::HashMap::new(),
            },
        ));

        let state = project_take_completed(
            state,
            event::TakeCompletedEventLatest {
                id: other_take_id,
                completed_at: timestamp(200),
            },
        )
        .expect("completion with another take id should preserve ongoing take");

        assert_eq!(state.ongoing_take.map(|(id, _)| id), Some(take_id));
        assert!(!state.completed_takes.contains_key(&other_take_id));
    }

    #[test]
    fn rejects_deleting_active_camera() {
        let camera_id = id::CameraId(uuid::Uuid::from_u128(1));
        let take_id = id::TakeId(uuid::Uuid::from_u128(2));
        let video_id = id::VideoId(uuid::Uuid::from_u128(3));
        let started_at = timestamp(100);
        let state = project_camera_created(
            state::MocapStudio::default(),
            event::CameraCreatedEventLatest {
                id: camera_id,
                name: "main".to_string(),
                rist_url: "rist://main".to_string(),
            },
        )
        .expect("camera creation should be projected");
        let state = project_take_started(
            state,
            event::TakeStartedEventLatest {
                id: take_id,
                take_number: state::TakeNumber(1),
                started_at,
                video_keys: vec![event::take_started_v0::TakeStartedEventV0Video {
                    id: video_id,
                    camera_id,
                    video_key: "videos/take-1-main.mp4".to_string(),
                }],
            },
        )
        .expect("take start should be projected");

        let error =
            project_camera_deleted(state, event::CameraDeletedEventLatest { id: camera_id })
                .expect_err("active camera deletion should be rejected");

        assert_eq!(error, ProjectionError::ActiveCameraCannotBeDeleted);
    }

    #[test]
    fn rejects_starting_take_when_one_is_ongoing() {
        let first_take_id = id::TakeId(uuid::Uuid::from_u128(1));
        let second_take_id = id::TakeId(uuid::Uuid::from_u128(2));
        let state = project_take_started(
            state::MocapStudio::default(),
            event::TakeStartedEventLatest {
                id: first_take_id,
                take_number: state::TakeNumber(1),
                started_at: timestamp(100),
                video_keys: vec![],
            },
        )
        .expect("first take start should be projected");

        let error = project_take_started(
            state,
            event::TakeStartedEventLatest {
                id: second_take_id,
                take_number: state::TakeNumber(2),
                started_at: timestamp(200),
                video_keys: vec![],
            },
        )
        .expect_err("second simultaneous take should be rejected");

        assert_eq!(error, ProjectionError::TakeAlreadyStarted);
    }

    #[test]
    fn rejects_completing_take_when_none_is_ongoing() {
        let take_id = id::TakeId(uuid::Uuid::from_u128(1));

        let error = project_take_completed(
            state::MocapStudio::default(),
            event::TakeCompletedEventLatest {
                id: take_id,
                completed_at: timestamp(200),
            },
        )
        .expect_err("take completion without ongoing take should be rejected");

        assert_eq!(error, ProjectionError::TakeNotStarted);
    }
}
