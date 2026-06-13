use crate::domain::model::{id, time};

#[derive(Debug, Clone, PartialEq, Eq, serde::Serialize, serde::Deserialize)]
pub struct TakeCompletedEventV0 {
    pub id: id::TakeId,
    pub completed_at: time::Timestamp,
}
