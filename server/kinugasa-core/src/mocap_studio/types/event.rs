pub mod camera_created_v0;
pub mod camera_deleted_v0;
pub mod take_completed_v0;
pub mod take_started_v0;

pub use camera_created_v0::*;
pub use camera_deleted_v0::*;
pub use take_completed_v0::*;
pub use take_started_v0::*;

pub type CameraCreatedEventLatest = CameraCreatedEventV0;
pub type CameraDeletedEventLatest = CameraDeletedEventV0;
pub type TakeStartedEventLatest = TakeStartedEventV0;
pub type TakeCompletedEventLatest = TakeCompletedEventV0;

#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub enum MocapStudioEvent {
    CameraCreated(CameraCreatedEvent),
    CameraDeleted(CameraDeletedEvent),
    TakeStarted(TakeStartedEvent),
    TakeCompleted(TakeCompletedEvent),
}

/// Do NOT serialize or deserialize!!
#[derive(Debug, Clone)]
pub enum MocapStudioEventLatest {
    CameraCreated(CameraCreatedEventLatest),
    CameraDeleted(CameraDeletedEventLatest),
    TakeStarted(TakeStartedEventLatest),
    TakeCompleted(TakeCompletedEventLatest),
}

#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub enum CameraCreatedEvent {
    V0(CameraCreatedEventV0),
}

#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub enum CameraDeletedEvent {
    V0(CameraDeletedEventV0),
}

#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub enum TakeStartedEvent {
    V0(TakeStartedEventV0),
}
#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub enum TakeCompletedEvent {
    V0(TakeCompletedEventV0),
}
