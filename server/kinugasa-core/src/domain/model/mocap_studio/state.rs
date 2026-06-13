use crate::domain::model::{id, time};

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
    pub take_number: Option<TakeNumber>,
    pub started_at: Option<time::Timestamp>,
    pub completed_at: Option<time::Timestamp>,
    pub videos: HashMap<id::VideoId, Video>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct TakeNumber(pub u32);

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Video {
    pub take_id: id::TakeId,
    pub camera_id: id::CameraId,
    pub video_key: String,
}

type HashMap<K, V> = std::collections::HashMap<K, V>;
