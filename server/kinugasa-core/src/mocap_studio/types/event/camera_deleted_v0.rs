#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub struct CameraDeletedEventV0 {
    pub mocap_studio_id: crate::id::MocapStudioId,
    pub id: crate::id::CameraId,
}
