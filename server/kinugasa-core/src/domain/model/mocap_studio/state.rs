use crate::domain::model::{id, time, unit_of_work};

#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct MocapStudio {
    pub cameras: HashMap<id::CameraId, Camera>,
    pub completed_takes: HashMap<id::TakeId, Take>,
    pub ongoing_take: Option<(id::TakeId, Take)>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Camera {
    pub name: String,
    pub rist_configuration: RistConfiguration,
    pub status: CameraStatus,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct RistConfiguration {
    pub url: String,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum CameraStatus {
    Capturing,
    Idle,
    Deleted,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Take {
    pub take_number: TakeNumber,
    pub started_at: time::Timestamp,
    pub completed_at: Option<time::Timestamp>,
    pub videos: HashMap<id::VideoId, Video>,
}

#[derive(
    Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash, serde::Serialize, serde::Deserialize,
)]
pub struct TakeNumber(pub u32);

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Video {
    pub take_id: id::TakeId,
    pub camera_id: id::CameraId,
    pub video_key: String,
}

type HashMap<K, V> = std::collections::HashMap<K, V>;

#[async_trait::async_trait]
pub trait MocapStudioCameraRepository {
    type UoW: unit_of_work::UnitOfWork;

    async fn get_camera(
        &self,
        uow: &mut Self::UoW,
        studio_id: id::MocapStudioId,
        id: id::CameraId,
    ) -> anyhow::Result<Option<Camera>>;

    async fn list_cameras(
        &self,
        uow: &mut Self::UoW,
        studio_id: id::MocapStudioId,
    ) -> anyhow::Result<Vec<(id::CameraId, Camera)>>;

    async fn upsert_camera(
        &self,
        uow: &mut Self::UoW,
        studio_id: id::MocapStudioId,
        id: id::CameraId,
        camera: Camera,
    ) -> anyhow::Result<()>;
}

#[async_trait::async_trait]
pub trait MocapStudioTakeRepository {
    type UoW: unit_of_work::UnitOfWork;

    async fn get_take(
        &self,
        uow: &mut Self::UoW,
        studio_id: id::MocapStudioId,
        id: id::TakeId,
    ) -> anyhow::Result<Option<Take>>;

    async fn get_ongoing_take(
        &self,
        uow: &mut Self::UoW,
        studio_id: id::MocapStudioId,
    ) -> anyhow::Result<Option<(id::TakeId, Take)>>;

    async fn list_takes(
        &self,
        uow: &mut Self::UoW,
        studio_id: id::MocapStudioId,
    ) -> anyhow::Result<Vec<(id::TakeId, Take)>>;

    async fn upsert_take(
        &self,
        uow: &mut Self::UoW,
        studio_id: id::MocapStudioId,
        id: id::TakeId,
        take: Take,
    ) -> anyhow::Result<()>;
}

#[async_trait::async_trait]
pub trait MocapStudioVideoRepository {
    type UoW: unit_of_work::UnitOfWork;

    async fn get_video(
        &self,
        uow: &mut Self::UoW,
        studio_id: id::MocapStudioId,
        id: id::VideoId,
    ) -> anyhow::Result<Option<Video>>;

    async fn list_videos_by_take(
        &self,
        uow: &mut Self::UoW,
        studio_id: id::MocapStudioId,
        take_id: id::TakeId,
    ) -> anyhow::Result<Vec<(id::VideoId, Video)>>;

    async fn upsert_video(
        &self,
        uow: &mut Self::UoW,
        studio_id: id::MocapStudioId,
        id: id::VideoId,
        video: Video,
    ) -> anyhow::Result<()>;
}

#[async_trait::async_trait]
pub trait MocapStudioStateRepository {
    type UoW: unit_of_work::UnitOfWork;

    async fn get_mocap_studio_state(
        &self,
        uow: &mut Self::UoW,
        studio_id: id::MocapStudioId,
    ) -> anyhow::Result<MocapStudio>;
}
