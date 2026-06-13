use crate::domain::model::mocap_studio::event;

pub fn upcast(event: event::MocapStudioEvent) -> event::MocapStudioEventLatest {
    match event {
        event::MocapStudioEvent::CameraCreated(event) => {
            event::MocapStudioEventLatest::CameraCreated(upcast_camera_created_event(event))
        }
        event::MocapStudioEvent::CameraDeleted(event) => {
            event::MocapStudioEventLatest::CameraDeleted(upcast_camera_deleted_event(event))
        }
        event::MocapStudioEvent::TakeStarted(event) => {
            event::MocapStudioEventLatest::TakeStarted(upcast_take_started_event(event))
        }
        event::MocapStudioEvent::TakeCompleted(event) => {
            event::MocapStudioEventLatest::TakeCompleted(upcast_take_completed_event(event))
        }
    }
}

pub fn upcast_camera_created_event(
    event: event::CameraCreatedEvent,
) -> event::CameraCreatedEventLatest {
    match event {
        event::CameraCreatedEvent::V0(event_v0) => event_v0,
    }
}

pub fn upcast_camera_deleted_event(
    event: event::CameraDeletedEvent,
) -> event::CameraDeletedEventLatest {
    match event {
        event::CameraDeletedEvent::V0(event_v0) => event_v0,
    }
}

pub fn upcast_take_started_event(event: event::TakeStartedEvent) -> event::TakeStartedEventLatest {
    match event {
        event::TakeStartedEvent::V0(event_v0) => event_v0,
    }
}

pub fn upcast_take_completed_event(
    event: event::TakeCompletedEvent,
) -> event::TakeCompletedEventLatest {
    match event {
        event::TakeCompletedEvent::V0(event_v0) => event_v0,
    }
}
