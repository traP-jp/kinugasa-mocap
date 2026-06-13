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
