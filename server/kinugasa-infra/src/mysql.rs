mod mocap_team;
mod unit_of_work;

use sqlx::{MySqlPool, mysql::MySqlPoolOptions};

pub use mocap_team::MySqlMocapTeamRepository;
pub use unit_of_work::{MySqlUnitOfWork, MySqlUnitOfWorkProvider};

#[derive(Debug, Clone)]
pub struct MySqlInfra {
    pool: MySqlPool,
}

impl MySqlInfra {
    pub async fn connect(database_url: &str) -> anyhow::Result<Self> {
        let pool = MySqlPoolOptions::new()
            .max_connections(10)
            .connect(database_url)
            .await?;

        Ok(Self { pool })
    }

    pub fn mocap_team_repository(&self) -> MySqlMocapTeamRepository {
        MySqlMocapTeamRepository::new(self.pool.clone())
    }

    pub fn unit_of_work_provider(&self) -> MySqlUnitOfWorkProvider {
        MySqlUnitOfWorkProvider::new(self.pool.clone())
    }
}
