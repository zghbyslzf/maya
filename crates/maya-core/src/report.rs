use std::path::PathBuf;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum FailurePolicy {
    #[default]
    Continue,
    FailFast,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum ProgressEvent {
    Started {
        operation: String,
        total: Option<u64>,
    },
    Advanced {
        increment: u64,
        total: Option<u64>,
        message: Option<String>,
    },
    Message(String),
    Finished,
}

pub trait ProgressSink: Send + Sync {
    fn emit(&self, event: ProgressEvent);
}

#[derive(Debug, Default, Clone, Copy)]
pub struct NoopProgress;

impl ProgressSink for NoopProgress {
    fn emit(&self, _event: ProgressEvent) {}
}

#[derive(Debug, Clone, PartialEq, Eq, Default)]
pub struct RemovalReport {
    pub removed: Vec<PathBuf>,
    pub skipped: Vec<PathBuf>,
}

impl RemovalReport {
    pub fn removed_count(&self) -> usize {
        self.removed.len()
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ArchiveReport {
    pub archive_path: PathBuf,
    pub files_added: usize,
    pub source_bytes: u64,
    pub archive_bytes: u64,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct OperationWarning {
    pub path: Option<PathBuf>,
    pub message: String,
}

impl OperationWarning {
    pub fn new(path: Option<PathBuf>, message: impl Into<String>) -> Self {
        Self {
            path,
            message: message.into(),
        }
    }
}
