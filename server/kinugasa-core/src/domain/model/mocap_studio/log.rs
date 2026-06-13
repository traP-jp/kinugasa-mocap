use crate::domain::model::{id, mocap_studio::state, time, unit_of_work};

#[derive(Debug, Clone, Copy, PartialEq, Eq, serde::Serialize, serde::Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum LogLevel {
    Error,
    Warn,
    Info,
    Debug,
    Trace,
}

#[derive(Debug, Clone, PartialEq, Eq, serde::Serialize, serde::Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum LogMessage {
    StudioCreated,
    StudioClosed,
    CameraCreated { name: String },
    CameraDeleted { name: String },
    TakeStarted { take_number: state::TakeNumber },
    TakeCompleted { take_number: state::TakeNumber },
    CaptureStarted { camera_name: String },
    CaptureStopped { camera_name: String },
    CaptureFailed { camera_name: String },
    StorageUnavailable,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct LogEntry {
    pub level: LogLevel,
    pub message: LogMessage,
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
    pub logs: Vec<StudioLog>,
}

#[async_trait::async_trait]
pub trait MocapStudioLogRepository {
    type UoW: unit_of_work::UnitOfWork;

    async fn append_studio_log(
        &self,
        uow: &mut Self::UoW,
        studio_id: id::MocapStudioId,
        log_entry: LogEntry,
    ) -> anyhow::Result<StudioLog>;

    async fn get_studio_logs_from(
        &self,
        uow: &mut Self::UoW,
        studio_id: id::MocapStudioId,
        log_level: Option<LogLevel>,
        from: usize,
    ) -> anyhow::Result<LogSegment>;
}
