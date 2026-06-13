pub mod handlers;

use crate::domain::model::id;

pub use crate::domain::model::mocap_team::{
    MocapStudioStatus, UpdateMocapStudioParams, UpdateMocapTeamParams,
};

#[async_trait::async_trait]
pub trait MocapTeamRepository {
    async fn add_mocap_team(
        &self,
        external_usergroup_key: id::ExternalGroupKey,
    ) -> anyhow::Result<id::MocapTeamId>;

    async fn update_mocap_team(
        &self,
        id: id::MocapTeamId,
        params: UpdateMocapTeamParams,
    ) -> anyhow::Result<()>;

    async fn add_mocap_studio(
        &self,
        team_id: id::MocapTeamId,
        name: String,
    ) -> anyhow::Result<id::MocapStudioId>;

    async fn update_mocap_studio(
        &self,
        id: id::MocapStudioId,
        params: UpdateMocapStudioParams,
    ) -> anyhow::Result<()>;
}
