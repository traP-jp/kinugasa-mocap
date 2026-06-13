use super::{
    errors::MocapTeamApiError,
    responses::{ListMocapTeamsResponse, MocapTeamResponse},
    state::MocapTeamApiState,
};
use crate::domain::model::{
    ext_user::AuthenticatedExternalUser,
    id, mocap_team,
    unit_of_work::{self, UnitOfWork},
};
use axum::{
    Extension, Json,
    extract::{Path, State},
    http::StatusCode,
    response::IntoResponse,
};
use uuid::Uuid;

pub async fn list_mocap_teams<R, U>(
    State(state): State<MocapTeamApiState<R, U>>,
) -> Result<Json<ListMocapTeamsResponse>, MocapTeamApiError>
where
    R: mocap_team::MocapTeamRepository<UoW = U::UoW>,
    U: unit_of_work::UnitOfWorkProvider,
{
    let teams = state.repository().list_mocap_teams().await?;

    Ok(Json(ListMocapTeamsResponse::from_teams(teams)))
}

pub async fn create_mocap_team<R, U>(
    State(state): State<MocapTeamApiState<R, U>>,
    Extension(user): Extension<AuthenticatedExternalUser>,
    Path(external_usergroup_key): Path<String>,
) -> Result<impl IntoResponse, MocapTeamApiError>
where
    R: mocap_team::MocapTeamRepository<UoW = U::UoW>,
    U: unit_of_work::UnitOfWorkProvider,
{
    let external_usergroup_key = validate_external_usergroup_key(
        id::ExternalGroupKey(external_usergroup_key),
        "externalUsergroupKey",
    )?;
    ensure_user_belongs_to_group(&user, &external_usergroup_key)?;
    let mut uow = state.unit_of_work_provider().start().await?;

    if state
        .repository()
        .find_mocap_team_by_external_usergroup_key(external_usergroup_key.clone())
        .await?
        .is_some()
    {
        return Err(MocapTeamApiError::Conflict(
            "mocap team with the external_usergroup_key already exists".to_owned(),
        ));
    }

    let id = state
        .repository()
        .add_mocap_team(&mut uow, external_usergroup_key)
        .await?;
    uow.commit().await?;
    let team = state
        .repository()
        .get_mocap_team(id)
        .await?
        .ok_or_else(|| {
            MocapTeamApiError::NotFound("created mocap team was not found".to_owned())
        })?;

    Ok((StatusCode::CREATED, Json(MocapTeamResponse::from(team))))
}

pub async fn get_mocap_team_by_external_usergroup_key<R, U>(
    State(state): State<MocapTeamApiState<R, U>>,
    Path(external_usergroup_key): Path<String>,
) -> Result<Json<MocapTeamResponse>, MocapTeamApiError>
where
    R: mocap_team::MocapTeamRepository<UoW = U::UoW>,
    U: unit_of_work::UnitOfWorkProvider,
{
    let external_usergroup_key = validate_external_usergroup_key(
        id::ExternalGroupKey(external_usergroup_key),
        "externalUsergroupKey",
    )?;
    let team = state
        .repository()
        .find_mocap_team_by_external_usergroup_key(external_usergroup_key)
        .await?
        .ok_or_else(|| MocapTeamApiError::NotFound("mocap team was not found".to_owned()))?;

    Ok(Json(MocapTeamResponse::from(team)))
}

pub async fn get_mocap_team<R, U>(
    State(state): State<MocapTeamApiState<R, U>>,
    Path(team_id): Path<Uuid>,
) -> Result<Json<MocapTeamResponse>, MocapTeamApiError>
where
    R: mocap_team::MocapTeamRepository<UoW = U::UoW>,
    U: unit_of_work::UnitOfWorkProvider,
{
    let team = state
        .repository()
        .get_mocap_team(id::MocapTeamId(team_id))
        .await?
        .ok_or_else(|| MocapTeamApiError::NotFound("mocap team was not found".to_owned()))?;

    Ok(Json(MocapTeamResponse::from(team)))
}

fn validate_external_usergroup_key(
    external_usergroup_key: id::ExternalGroupKey,
    field_name: &'static str,
) -> Result<id::ExternalGroupKey, MocapTeamApiError> {
    if external_usergroup_key.0.trim().is_empty() {
        return Err(MocapTeamApiError::Validation(format!(
            "{field_name} must not be empty"
        )));
    }

    Ok(external_usergroup_key)
}

fn ensure_user_belongs_to_group(
    user: &AuthenticatedExternalUser,
    external_usergroup_key: &id::ExternalGroupKey,
) -> Result<(), MocapTeamApiError> {
    if user
        .groups
        .iter()
        .any(|group| group == external_usergroup_key)
    {
        return Ok(());
    }

    Err(MocapTeamApiError::Forbidden(
        "authenticated user does not belong to the external user group".to_owned(),
    ))
}
