#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub struct CameraDeletedEventV0 {
    pub id: crate::id::CameraId,
}
