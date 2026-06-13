use crate::domain::model::id;

#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub struct CameraDeletedEventV0 {
    pub id: id::CameraId,
}
