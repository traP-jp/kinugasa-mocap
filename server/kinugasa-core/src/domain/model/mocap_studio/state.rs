use crate::domain::model::id;

#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct MocapStudio {
    pub cameras: HashMap<id::CameraId, Camera>,
    pub completed_takes: HashMap<id::TakeId, Take>,
    pub ongoing_take: Option<(id::TakeId, Take)>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Camera {
    pub name: String,
    pub rist_url: String,
    pub status: CameraStatus,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum CameraStatus {
    Capturing,
    Idle,
    Deleted,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Take {
    pub videos: HashMap<id::VideoId, Video>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Video {
    pub camera_id: id::CameraId,
    pub video_key: String,
}

type HashMap<K, V> = std::collections::HashMap<K, V>;
