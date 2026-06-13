use crate::domain::model::{mocap_team, time};
use serde::Serialize;
use uuid::Uuid;

#[derive(Debug, Serialize)]
pub struct ErrorResponse {
    code: &'static str,
    message: String,
}

impl ErrorResponse {
    pub fn new(code: &'static str, message: impl Into<String>) -> Self {
        Self {
            code,
            message: message.into(),
        }
    }
}

#[derive(Debug, Serialize)]
pub struct ListMocapTeamsResponse {
    items: Vec<MocapTeamResponse>,
}

impl ListMocapTeamsResponse {
    pub fn from_teams(teams: Vec<mocap_team::MocapTeam>) -> Self {
        Self {
            items: teams.into_iter().map(MocapTeamResponse::from).collect(),
        }
    }
}

#[derive(Debug, Serialize)]
pub struct MocapTeamResponse {
    id: Uuid,
    external_usergroup_key: String,
    created_at: time::Timestamp,
}

impl From<mocap_team::MocapTeam> for MocapTeamResponse {
    fn from(team: mocap_team::MocapTeam) -> Self {
        Self {
            id: team.id.0,
            external_usergroup_key: team.external_usergroup_key.0,
            created_at: team.created_at,
        }
    }
}
