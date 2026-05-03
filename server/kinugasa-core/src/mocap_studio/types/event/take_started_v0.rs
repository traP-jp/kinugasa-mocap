#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub struct TakeStartedEventV0 {
    pub id: crate::id::TakeId,
    pub video_keys: Vec<TakeStartedEventV0Video>,
}

#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub struct TakeStartedEventV0Video {
    pub id: crate::id::VideoId,
    pub camera_id: crate::id::CameraId,
    pub video_key: String,
}
