#[derive(Debug, Clone)]
pub enum LogLevel {
    Error,
    Warn,
    Info,
    Debug,
    Trace,
}

#[derive(Debug, Clone)]
pub struct LogEntry {
    pub level: LogLevel,
    pub message: String,
}

#[derive(Debug, Clone)]
pub struct LogSegment {
    pub top_index: usize,
    pub length: usize,
    pub log_entries: Vec<LogEntry>,
}

#[async_trait::async_trait]
pub trait MocapStudioLogRepository {
    async fn push_log(
        &self,
        studio_id: crate::id::MocapStudioId,
        log_entry: LogEntry,
    ) -> anyhow::Result<()>;
    async fn get_logs_from(
        &self,
        studio_id: crate::id::MocapStudioId,
        log_level: Option<LogLevel>,
        from: usize,
    ) -> anyhow::Result<LogSegment>;
}
