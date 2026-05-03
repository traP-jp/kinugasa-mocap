#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub struct TakeStartedEventV0 {
    pub mocap_studio_id: crate::id::MocapStudioId,
    pub id: crate::id::TakeId,
    pub video_keys: std::collections::HashMap<crate::id::CameraId, String>,
}
