use crate::domain::model::{id, time, unit_of_work};

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct MocapTeam {
    pub id: id::MocapTeamId,
    pub external_usergroup_key: id::ExternalGroupKey,
    pub created_at: time::Timestamp,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct MocapStudio {
    pub id: id::MocapStudioId,
    pub team_id: id::MocapTeamId,
    pub name: String,
    pub status: MocapStudioStatus,
    pub last_event_sequence_number: StudioEventSequenceNumber,
    pub created_at: time::Timestamp,
    pub updated_at: time::Timestamp,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct StudioEventSequenceNumber(pub u64);

#[derive(Debug, Clone, Default)]
pub struct UpdateMocapStudioParams {
    pub status: Option<MocapStudioStatus>,
    pub last_event_sequence_number: Option<StudioEventSequenceNumber>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum MocapStudioStatus {
    Capturing,
    Idle,
    Closed,
}

#[async_trait::async_trait]
pub trait MocapTeamRepository: Clone + Send + Sync + 'static {
    type UoW: unit_of_work::UnitOfWork;

    async fn get_mocap_team(&self, id: id::MocapTeamId) -> anyhow::Result<Option<MocapTeam>>;

    async fn find_mocap_team_by_external_usergroup_key(
        &self,
        external_usergroup_key: id::ExternalGroupKey,
    ) -> anyhow::Result<Option<MocapTeam>>;

    async fn list_mocap_teams(&self) -> anyhow::Result<Vec<MocapTeam>>;

    async fn add_mocap_team(
        &self,
        uow: &mut Self::UoW,
        external_usergroup_key: id::ExternalGroupKey,
    ) -> anyhow::Result<id::MocapTeamId>;

    async fn add_mocap_studio(
        &self,
        uow: &mut Self::UoW,
        team_id: id::MocapTeamId,
        name: String,
    ) -> anyhow::Result<id::MocapStudioId>;

    async fn get_mocap_studio(&self, id: id::MocapStudioId) -> anyhow::Result<Option<MocapStudio>>;

    async fn list_mocap_studios_by_team(
        &self,
        team_id: id::MocapTeamId,
    ) -> anyhow::Result<Vec<MocapStudio>>;

    async fn update_mocap_studio(
        &self,
        uow: &mut Self::UoW,
        id: id::MocapStudioId,
        params: UpdateMocapStudioParams,
    ) -> anyhow::Result<()>;
}
