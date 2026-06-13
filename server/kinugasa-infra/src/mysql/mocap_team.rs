use super::unit_of_work::MySqlUnitOfWork;
use anyhow::{Context, bail};
use async_trait::async_trait;
use chrono::{NaiveDateTime, Utc};
use kinugasa_core::domain::model::{id, mocap_team};
use sqlx::{MySqlPool, Row};
use uuid::Uuid;

#[derive(Debug, Clone)]
pub struct MySqlMocapTeamRepository {
    pool: MySqlPool,
}

impl MySqlMocapTeamRepository {
    pub fn new(pool: MySqlPool) -> Self {
        Self { pool }
    }
}

#[async_trait]
impl mocap_team::MocapTeamRepository for MySqlMocapTeamRepository {
    type UoW = MySqlUnitOfWork;

    async fn get_mocap_team(
        &self,
        id: id::MocapTeamId,
    ) -> anyhow::Result<Option<mocap_team::MocapTeam>> {
        let row = sqlx::query(
            r#"
            SELECT id, external_usergroup_key, created_at
            FROM mocap_teams
            WHERE id = ?
            "#,
        )
        .bind(id.0.to_string())
        .fetch_optional(&self.pool)
        .await?;

        row.map(row_to_mocap_team).transpose()
    }

    async fn find_mocap_team_by_external_usergroup_key(
        &self,
        external_usergroup_key: id::ExternalGroupKey,
    ) -> anyhow::Result<Option<mocap_team::MocapTeam>> {
        let row = sqlx::query(
            r#"
            SELECT id, external_usergroup_key, created_at
            FROM mocap_teams
            WHERE external_usergroup_key = ?
            "#,
        )
        .bind(external_usergroup_key.0)
        .fetch_optional(&self.pool)
        .await?;

        row.map(row_to_mocap_team).transpose()
    }

    async fn list_mocap_teams(&self) -> anyhow::Result<Vec<mocap_team::MocapTeam>> {
        let rows = sqlx::query(
            r#"
            SELECT id, external_usergroup_key, created_at
            FROM mocap_teams
            ORDER BY created_at ASC, id ASC
            "#,
        )
        .fetch_all(&self.pool)
        .await?;

        rows.into_iter().map(row_to_mocap_team).collect()
    }

    async fn add_mocap_team(
        &self,
        uow: &mut Self::UoW,
        external_usergroup_key: id::ExternalGroupKey,
    ) -> anyhow::Result<id::MocapTeamId> {
        let id = Uuid::new_v4();
        sqlx::query(
            r#"
            INSERT INTO mocap_teams (id, external_usergroup_key, created_at)
            VALUES (?, ?, UTC_TIMESTAMP(6))
            "#,
        )
        .bind(id.to_string())
        .bind(external_usergroup_key.0)
        .execute(uow.connection())
        .await?;

        Ok(id::MocapTeamId(id))
    }

    async fn add_mocap_studio(
        &self,
        uow: &mut Self::UoW,
        team_id: id::MocapTeamId,
        name: String,
    ) -> anyhow::Result<id::MocapStudioId> {
        let id = Uuid::new_v4();
        sqlx::query(
            r#"
            INSERT INTO studios (
                id,
                team_id,
                name,
                status,
                last_event_sequence_number,
                created_at,
                updated_at
            )
            VALUES (?, ?, ?, 'idle', 0, UTC_TIMESTAMP(6), UTC_TIMESTAMP(6))
            "#,
        )
        .bind(id.to_string())
        .bind(team_id.0.to_string())
        .bind(name)
        .execute(uow.connection())
        .await?;

        Ok(id::MocapStudioId(id))
    }

    async fn get_mocap_studio(
        &self,
        id: id::MocapStudioId,
    ) -> anyhow::Result<Option<mocap_team::MocapStudio>> {
        let row = sqlx::query(
            r#"
            SELECT
                id,
                team_id,
                name,
                status,
                last_event_sequence_number,
                created_at,
                updated_at
            FROM studios
            WHERE id = ?
            "#,
        )
        .bind(id.0.to_string())
        .fetch_optional(&self.pool)
        .await?;

        row.map(row_to_mocap_studio).transpose()
    }

    async fn list_mocap_studios_by_team(
        &self,
        team_id: id::MocapTeamId,
    ) -> anyhow::Result<Vec<mocap_team::MocapStudio>> {
        let rows = sqlx::query(
            r#"
            SELECT
                id,
                team_id,
                name,
                status,
                last_event_sequence_number,
                created_at,
                updated_at
            FROM studios
            WHERE team_id = ?
            ORDER BY created_at ASC, id ASC
            "#,
        )
        .bind(team_id.0.to_string())
        .fetch_all(&self.pool)
        .await?;

        rows.into_iter().map(row_to_mocap_studio).collect()
    }

    async fn update_mocap_studio(
        &self,
        uow: &mut Self::UoW,
        id: id::MocapStudioId,
        params: mocap_team::UpdateMocapStudioParams,
    ) -> anyhow::Result<()> {
        match (params.status, params.last_event_sequence_number) {
            (None, None) => {}
            (Some(status), None) => {
                sqlx::query(
                    r#"
                    UPDATE studios
                    SET status = ?, updated_at = UTC_TIMESTAMP(6)
                    WHERE id = ?
                    "#,
                )
                .bind(mocap_studio_status_to_str(status))
                .bind(id.0.to_string())
                .execute(uow.connection())
                .await?;
            }
            (None, Some(sequence_number)) => {
                sqlx::query(
                    r#"
                    UPDATE studios
                    SET last_event_sequence_number = ?, updated_at = UTC_TIMESTAMP(6)
                    WHERE id = ?
                    "#,
                )
                .bind(sequence_number.0)
                .bind(id.0.to_string())
                .execute(uow.connection())
                .await?;
            }
            (Some(status), Some(sequence_number)) => {
                sqlx::query(
                    r#"
                    UPDATE studios
                    SET
                        status = ?,
                        last_event_sequence_number = ?,
                        updated_at = UTC_TIMESTAMP(6)
                    WHERE id = ?
                    "#,
                )
                .bind(mocap_studio_status_to_str(status))
                .bind(sequence_number.0)
                .bind(id.0.to_string())
                .execute(uow.connection())
                .await?;
            }
        }

        Ok(())
    }
}

fn row_to_mocap_team(row: sqlx::mysql::MySqlRow) -> anyhow::Result<mocap_team::MocapTeam> {
    Ok(mocap_team::MocapTeam {
        id: id::MocapTeamId(parse_uuid(row.try_get("id")?)?),
        external_usergroup_key: id::ExternalGroupKey(row.try_get("external_usergroup_key")?),
        created_at: mysql_timestamp_to_utc(row.try_get("created_at")?),
    })
}

fn row_to_mocap_studio(row: sqlx::mysql::MySqlRow) -> anyhow::Result<mocap_team::MocapStudio> {
    Ok(mocap_team::MocapStudio {
        id: id::MocapStudioId(parse_uuid(row.try_get("id")?)?),
        team_id: id::MocapTeamId(parse_uuid(row.try_get("team_id")?)?),
        name: row.try_get("name")?,
        status: parse_mocap_studio_status(row.try_get("status")?)?,
        last_event_sequence_number: mocap_team::StudioEventSequenceNumber(
            row.try_get::<i64, _>("last_event_sequence_number")?
                .try_into()
                .context("last_event_sequence_number must be non-negative")?,
        ),
        created_at: mysql_timestamp_to_utc(row.try_get("created_at")?),
        updated_at: mysql_timestamp_to_utc(row.try_get("updated_at")?),
    })
}

fn parse_uuid(value: String) -> anyhow::Result<Uuid> {
    Uuid::parse_str(&value).with_context(|| format!("invalid uuid value: {value}"))
}

fn mysql_timestamp_to_utc(value: NaiveDateTime) -> chrono::DateTime<Utc> {
    value.and_utc()
}

fn parse_mocap_studio_status(value: String) -> anyhow::Result<mocap_team::MocapStudioStatus> {
    match value.as_str() {
        "capturing" => Ok(mocap_team::MocapStudioStatus::Capturing),
        "idle" => Ok(mocap_team::MocapStudioStatus::Idle),
        "closed" => Ok(mocap_team::MocapStudioStatus::Closed),
        _ => bail!("invalid mocap studio status: {value}"),
    }
}

fn mocap_studio_status_to_str(status: mocap_team::MocapStudioStatus) -> &'static str {
    match status {
        mocap_team::MocapStudioStatus::Capturing => "capturing",
        mocap_team::MocapStudioStatus::Idle => "idle",
        mocap_team::MocapStudioStatus::Closed => "closed",
    }
}
