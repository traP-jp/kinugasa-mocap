use async_trait::async_trait;

#[async_trait]
pub trait UnitOfWork {
    async fn commit(self) -> anyhow::Result<()>;
}

#[async_trait]
pub trait UnitOfWorkProvider {
    type UoW: UnitOfWork;
    async fn start(&self) -> anyhow::Result<Self::UoW>;
}
