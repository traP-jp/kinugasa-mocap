pub mod camera_created_v0;
pub mod camera_deleted_v0;
pub mod take_completed_v0;
pub mod take_started_v0;

use crate::domain::model::{id, mocap_team, time, unit_of_work};

pub type CameraCreatedEventLatest = camera_created_v0::CameraCreatedEventV0;
pub type CameraDeletedEventLatest = camera_deleted_v0::CameraDeletedEventV0;
pub type TakeStartedEventLatest = take_started_v0::TakeStartedEventV0;
pub type TakeCompletedEventLatest = take_completed_v0::TakeCompletedEventV0;

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct StudioEventRecord {
    pub id: id::StudioEventId,
    pub studio_id: id::MocapStudioId,
    pub sequence_number: mocap_team::StudioEventSequenceNumber,
    pub event: MocapStudioEvent,
    pub created_at: time::Timestamp,
}

#[async_trait::async_trait]
pub trait MocapStudioEventRepository {
    type UoW: unit_of_work::UnitOfWork;

    async fn append_studio_event(
        &self,
        uow: &mut Self::UoW,
        studio_id: id::MocapStudioId,
        sequence_number: mocap_team::StudioEventSequenceNumber,
        event: MocapStudioEvent,
    ) -> anyhow::Result<StudioEventRecord>;

    async fn get_studio_event(
        &self,
        uow: &mut Self::UoW,
        studio_id: id::MocapStudioId,
        sequence_number: mocap_team::StudioEventSequenceNumber,
    ) -> anyhow::Result<Option<StudioEventRecord>>;

    async fn list_studio_events_from(
        &self,
        uow: &mut Self::UoW,
        studio_id: id::MocapStudioId,
        from_sequence_number: mocap_team::StudioEventSequenceNumber,
    ) -> anyhow::Result<Vec<StudioEventRecord>>;
}

#[derive(Debug, Clone, PartialEq, Eq, serde::Serialize, serde::Deserialize)]
pub enum MocapStudioEvent {
    CameraCreated(CameraCreatedEvent),
    CameraDeleted(CameraDeletedEvent),
    TakeStarted(TakeStartedEvent),
    TakeCompleted(TakeCompletedEvent),
}

/// Do NOT serialize or deserialize!!
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum MocapStudioEventLatest {
    CameraCreated(CameraCreatedEventLatest),
    CameraDeleted(CameraDeletedEventLatest),
    TakeStarted(TakeStartedEventLatest),
    TakeCompleted(TakeCompletedEventLatest),
}

#[derive(Debug, Clone, PartialEq, Eq, serde::Serialize, serde::Deserialize)]
pub enum CameraCreatedEvent {
    V0(camera_created_v0::CameraCreatedEventV0),
}

#[derive(Debug, Clone, PartialEq, Eq, serde::Serialize, serde::Deserialize)]
pub enum CameraDeletedEvent {
    V0(camera_deleted_v0::CameraDeletedEventV0),
}

#[derive(Debug, Clone, PartialEq, Eq, serde::Serialize, serde::Deserialize)]
pub enum TakeStartedEvent {
    V0(take_started_v0::TakeStartedEventV0),
}
#[derive(Debug, Clone, PartialEq, Eq, serde::Serialize, serde::Deserialize)]
pub enum TakeCompletedEvent {
    V0(take_completed_v0::TakeCompletedEventV0),
}
