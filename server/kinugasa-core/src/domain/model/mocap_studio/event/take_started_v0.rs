use crate::domain::model::id;

#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub struct TakeStartedEventV0 {
    pub id: id::TakeId,
    pub video_keys: Vec<TakeStartedEventV0Video>,
}

#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub struct TakeStartedEventV0Video {
    pub id: id::VideoId,
    pub camera_id: id::CameraId,
    pub video_key: String,
}
