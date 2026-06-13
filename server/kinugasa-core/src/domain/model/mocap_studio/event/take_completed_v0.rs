use crate::domain::model::id;

#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub struct TakeCompletedEventV0 {
    pub id: id::TakeId,
}
