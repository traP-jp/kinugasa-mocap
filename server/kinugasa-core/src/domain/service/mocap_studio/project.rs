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
    _prev: state::MocapStudio,
    _transition: event::CameraCreatedEventLatest,
) -> state::MocapStudio {
    unimplemented!()
}

pub fn project_camera_deleted(
    _prev: state::MocapStudio,
    _transition: event::CameraDeletedEventLatest,
) -> state::MocapStudio {
    unimplemented!()
}

pub fn project_take_started(
    _prev: state::MocapStudio,
    _transition: event::TakeStartedEventLatest,
) -> state::MocapStudio {
    unimplemented!()
}

pub fn project_take_completed(
    _prev: state::MocapStudio,
    _transition: event::TakeCompletedEventLatest,
) -> state::MocapStudio {
    unimplemented!()
}
