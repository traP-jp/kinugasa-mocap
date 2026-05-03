#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub struct CameraCreatedEventV0 {
    pub mocap_studio_id: crate::id::MocapStudioId,
    pub id: crate::id::CameraId,
    pub name: String,
    pub rist_url: String,
}
