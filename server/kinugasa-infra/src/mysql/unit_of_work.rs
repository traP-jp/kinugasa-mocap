use async_trait::async_trait;
use kinugasa_core::domain::model::unit_of_work::{UnitOfWork, UnitOfWorkProvider};
use sqlx::{MySql, MySqlConnection, MySqlPool, Transaction};

pub struct MySqlUnitOfWork {
    transaction: Transaction<'static, MySql>,
}

impl MySqlUnitOfWork {
    pub(crate) fn connection(&mut self) -> &mut MySqlConnection {
        &mut self.transaction
    }
}

#[async_trait]
impl UnitOfWork for MySqlUnitOfWork {
    async fn commit(self) -> anyhow::Result<()> {
        self.transaction.commit().await?;
        Ok(())
    }
}

#[derive(Debug, Clone)]
pub struct MySqlUnitOfWorkProvider {
    pool: MySqlPool,
}

impl MySqlUnitOfWorkProvider {
    pub fn new(pool: MySqlPool) -> Self {
        Self { pool }
    }
}

#[async_trait]
impl UnitOfWorkProvider for MySqlUnitOfWorkProvider {
    type UoW = MySqlUnitOfWork;

    async fn start(&self) -> anyhow::Result<Self::UoW> {
        Ok(MySqlUnitOfWork {
            transaction: self.pool.begin().await?,
        })
    }
}
