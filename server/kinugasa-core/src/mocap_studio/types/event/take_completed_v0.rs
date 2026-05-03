#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub struct TakeCompletedEventV0 {
    pub mocap_studio_id: crate::id::MocapStudioId,
    pub id: crate::id::TakeId,
}
