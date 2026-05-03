#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub struct CameraCreatedEventV0 {
    pub id: crate::id::CameraId,
    pub name: String,
    pub rist_url: String,
}
