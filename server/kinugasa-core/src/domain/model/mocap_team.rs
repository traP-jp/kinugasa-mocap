use crate::domain::model::{id, unit_of_work};

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

#[async_trait::async_trait]
pub trait MocapTeamRepository {
    type UoW: unit_of_work::UnitOfWork;

    async fn add_mocap_team(
        &self,
        uow: &mut Self::UoW,
        external_usergroup_key: id::ExternalGroupKey,
    ) -> anyhow::Result<id::MocapTeamId>;

    async fn update_mocap_team(
        &self,
        uow: &mut Self::UoW,
        id: id::MocapTeamId,
        params: UpdateMocapTeamParams,
    ) -> anyhow::Result<()>;

    async fn add_mocap_studio(
        &self,
        uow: &mut Self::UoW,
        team_id: id::MocapTeamId,
        name: String,
    ) -> anyhow::Result<id::MocapStudioId>;

    async fn update_mocap_studio(
        &self,
        uow: &mut Self::UoW,
        id: id::MocapStudioId,
        params: UpdateMocapStudioParams,
    ) -> anyhow::Result<()>;
}
