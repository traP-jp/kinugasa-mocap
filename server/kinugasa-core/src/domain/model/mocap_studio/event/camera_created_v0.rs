use crate::domain::model::id;

#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub struct CameraCreatedEventV0 {
    pub id: id::CameraId,
    pub name: String,
    pub rist_url: String,
}
