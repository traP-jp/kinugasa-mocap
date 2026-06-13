use crate::domain::model::{id, mocap_studio::state, time};

#[derive(Debug, Clone, PartialEq, Eq, serde::Serialize, serde::Deserialize)]
pub struct TakeStartedEventV0 {
    pub id: id::TakeId,
    pub take_number: state::TakeNumber,
    pub started_at: time::Timestamp,
    pub video_keys: Vec<TakeStartedEventV0Video>,
}

#[derive(Debug, Clone, PartialEq, Eq, serde::Serialize, serde::Deserialize)]
pub struct TakeStartedEventV0Video {
    pub id: id::VideoId,
    pub camera_id: id::CameraId,
    pub video_key: String,
}
