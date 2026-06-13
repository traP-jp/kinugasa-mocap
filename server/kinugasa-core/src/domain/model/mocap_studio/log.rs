use crate::domain::model::{id, time, unit_of_work};

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum LogLevel {
    Error,
    Warn,
    Info,
    Debug,
    Trace,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct LogEntry {
    pub level: LogLevel,
    pub message: String,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct StudioLog {
    pub id: id::StudioLogId,
    pub studio_id: id::MocapStudioId,
    pub entry: LogEntry,
    pub created_at: time::Timestamp,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct LogSegment {
    pub top_index: usize,
    pub length: usize,
    pub log_entries: Vec<LogEntry>,
}

#[async_trait::async_trait]
pub trait MocapStudioLogRepository {
    type UoW: unit_of_work::UnitOfWork;

    async fn push_log(
        &self,
        uow: &mut Self::UoW,
        studio_id: id::MocapStudioId,
        log_entry: LogEntry,
    ) -> anyhow::Result<()>;

    async fn get_logs_from(
        &self,
        uow: &mut Self::UoW,
        studio_id: id::MocapStudioId,
        log_level: Option<LogLevel>,
        from: usize,
    ) -> anyhow::Result<LogSegment>;
}
