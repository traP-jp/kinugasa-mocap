#[async_trait::async_trait]
pub trait MocapTeamRepository {
    async fn add_mocap_team(
        &self,
        external_usergroup_key: crate::id::ExternalGroupKey,
    ) -> anyhow::Result<crate::id::MocapTeamId>;

    async fn update_mocap_team(
        &self,
        id: crate::id::MocapTeamId,
        params: UpdateMocapTeamParams,
    ) -> anyhow::Result<()>;

    async fn add_mocap_studio(
        &self,
        team_id: crate::id::MocapTeamId,
        name: String,
    ) -> anyhow::Result<crate::id::MocapStudioId>;

    async fn update_mocap_studio(
        &self,
        id: crate::id::MocapStudioId,
        params: UpdateMocapStudioParams,
    ) -> anyhow::Result<()>;
}

#[derive(Debug, Clone, Default)]
pub struct UpdateMocapTeamParams {
    pub external_usergroup_key: Option<crate::id::ExternalGroupKey>,
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
