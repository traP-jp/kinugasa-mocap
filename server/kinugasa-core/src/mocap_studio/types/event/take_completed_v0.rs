#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub struct TakeCompletedEventV0 {
    pub id: crate::id::TakeId,
}
