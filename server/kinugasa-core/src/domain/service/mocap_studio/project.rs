use crate::domain::model::mocap_studio::{event, state};

pub fn project(
    prev: state::MocapStudio,
    transition: event::MocapStudioEventLatest,
) -> state::MocapStudio {
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
) -> state::MocapStudio {
    prev.cameras.insert(
        transition.id,
        state::Camera {
            name: transition.name,
            rist_url: transition.rist_url,
            status: state::CameraStatus::Idle,
        },
    );
    prev
}

pub fn project_camera_deleted(
    mut prev: state::MocapStudio,
    transition: event::CameraDeletedEventLatest,
) -> state::MocapStudio {
    if let Some(camera) = prev.cameras.get_mut(&transition.id) {
        camera.status = state::CameraStatus::Deleted;
    }
    prev
}

pub fn project_take_started(
    mut prev: state::MocapStudio,
    transition: event::TakeStartedEventLatest,
) -> state::MocapStudio {
    let videos = transition
        .video_keys
        .into_iter()
        .map(|video| {
            set_active_camera_status(&mut prev, video.camera_id, state::CameraStatus::Capturing);
            (
                video.id,
                state::Video {
                    camera_id: video.camera_id,
                    video_key: video.video_key,
                },
            )
        })
        .collect();

    prev.ongoing_take = Some((transition.id, state::Take { videos }));
    prev
}

pub fn project_take_completed(
    mut prev: state::MocapStudio,
    transition: event::TakeCompletedEventLatest,
) -> state::MocapStudio {
    if let Some((take_id, take)) = prev.ongoing_take.take() {
        if take_id == transition.id {
            for video in take.videos.values() {
                set_active_camera_status(&mut prev, video.camera_id, state::CameraStatus::Idle);
            }
            prev.completed_takes.insert(take_id, take);
        } else {
            prev.ongoing_take = Some((take_id, take));
        }
    }

    prev
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
        );

        assert_eq!(
            state.cameras.get(&camera_id),
            Some(&state::Camera {
                name: "main".to_string(),
                rist_url: "rist://main".to_string(),
                status: state::CameraStatus::Idle,
            })
        );

        let state =
            project_camera_deleted(state, event::CameraDeletedEventLatest { id: camera_id });

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
        let state = project_camera_created(
            state::MocapStudio::default(),
            event::CameraCreatedEventLatest {
                id: camera_id,
                name: "main".to_string(),
                rist_url: "rist://main".to_string(),
            },
        );

        let state = project_take_started(
            state,
            event::TakeStartedEventLatest {
                id: take_id,
                video_keys: vec![event::take_started_v0::TakeStartedEventV0Video {
                    id: video_id,
                    camera_id,
                    video_key: "videos/take-1-main.mp4".to_string(),
                }],
            },
        );

        let (_, ongoing_take) = state
            .ongoing_take
            .as_ref()
            .expect("started take should be ongoing");
        assert_eq!(
            ongoing_take.videos.get(&video_id),
            Some(&state::Video {
                camera_id,
                video_key: "videos/take-1-main.mp4".to_string(),
            })
        );
        assert_eq!(
            state.cameras.get(&camera_id).map(|camera| &camera.status),
            Some(&state::CameraStatus::Capturing)
        );

        let state = project_take_completed(state, event::TakeCompletedEventLatest { id: take_id });

        assert!(state.ongoing_take.is_none());
        assert!(state.completed_takes.contains_key(&take_id));
        assert_eq!(
            state.cameras.get(&camera_id).map(|camera| &camera.status),
            Some(&state::CameraStatus::Idle)
        );
    }

    #[test]
    fn keeps_deleted_camera_deleted_when_take_completes() {
        let camera_id = id::CameraId(uuid::Uuid::from_u128(1));
        let take_id = id::TakeId(uuid::Uuid::from_u128(2));
        let video_id = id::VideoId(uuid::Uuid::from_u128(3));
        let state = project_camera_created(
            state::MocapStudio::default(),
            event::CameraCreatedEventLatest {
                id: camera_id,
                name: "main".to_string(),
                rist_url: "rist://main".to_string(),
            },
        );
        let state = project_take_started(
            state,
            event::TakeStartedEventLatest {
                id: take_id,
                video_keys: vec![event::take_started_v0::TakeStartedEventV0Video {
                    id: video_id,
                    camera_id,
                    video_key: "videos/take-1-main.mp4".to_string(),
                }],
            },
        );
        let state =
            project_camera_deleted(state, event::CameraDeletedEventLatest { id: camera_id });

        let state = project_take_completed(state, event::TakeCompletedEventLatest { id: take_id });

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
                videos: std::collections::HashMap::new(),
            },
        ));

        let state =
            project_take_completed(state, event::TakeCompletedEventLatest { id: other_take_id });

        assert_eq!(state.ongoing_take.map(|(id, _)| id), Some(take_id));
        assert!(!state.completed_takes.contains_key(&other_take_id));
    }
}
