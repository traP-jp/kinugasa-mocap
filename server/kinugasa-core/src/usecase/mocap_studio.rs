use crate::domain::model::{
    id,
    mocap_studio::log::{LogEntry, LogLevel, LogSegment},
};

#[async_trait::async_trait]
pub trait MocapStudioLogRepository {
    async fn push_log(
        &self,
        studio_id: id::MocapStudioId,
        log_entry: LogEntry,
    ) -> anyhow::Result<()>;

    async fn get_logs_from(
        &self,
        studio_id: id::MocapStudioId,
        log_level: Option<LogLevel>,
        from: usize,
    ) -> anyhow::Result<LogSegment>;
}
