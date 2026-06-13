use async_trait::async_trait;

#[async_trait]
pub trait UnitOfWork: Send + 'static {
    async fn commit(self) -> anyhow::Result<()>;
}

#[async_trait]
pub trait UnitOfWorkProvider: Clone + Send + Sync + 'static {
    type UoW: UnitOfWork;

    async fn start(&self) -> anyhow::Result<Self::UoW>;
}
