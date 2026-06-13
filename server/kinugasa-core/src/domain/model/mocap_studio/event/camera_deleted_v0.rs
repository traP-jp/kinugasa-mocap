use crate::domain::model::id;

#[derive(Debug, Clone, PartialEq, Eq, serde::Serialize, serde::Deserialize)]
pub struct CameraDeletedEventV0 {
    pub id: id::CameraId,
}
