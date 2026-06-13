use crate::domain::model::id;

#[derive(Debug, Clone, Default)]
pub struct UpdateMocapTeamParams {
    pub external_usergroup_key: Option<id::ExternalGroupKey>,
}

#[derive(Debug, Clone, Default)]
pub struct UpdateMocapStudioParams {
    pub status: Option<MocapStudioStatus>,
    pub last_event_sequence_number: Option<u64>,
}

#[derive(Debug, Clone)]
pub enum MocapStudioStatus {
    Capturing,
    Idle,
    Closed,
}
