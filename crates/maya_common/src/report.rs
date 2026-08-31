use std::path::PathBuf;

/// 批处理遇到单项失败时的执行策略。
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum FailurePolicy {
    /// 记录失败并继续处理其余项目。
    #[default]
    Continue,
    /// 遇到首个失败时返回错误。
    FailFast,
}

/// 长任务向调用方报告的进度事件。具体终端展示由 CLI 决定。
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

/// 长任务进度边界。库只发送事件，不依赖终端或具体进度条实现。
pub trait ProgressSink: Send + Sync {
    fn emit(&self, event: ProgressEvent);
}

/// 不需要进度通知时使用的空实现。
#[derive(Debug, Default, Clone, Copy)]
pub struct NoopProgress;

impl ProgressSink for NoopProgress {
    fn emit(&self, _event: ProgressEvent) {}
}

/// 文件或目录删除操作的结构化结果。
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

/// ZIP 归档操作的结构化结果。
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ArchiveReport {
    pub archive_path: PathBuf,
    pub files_added: usize,
    pub source_bytes: u64,
    pub archive_bytes: u64,
}

/// 不阻止操作完成、但调用方应能够结构化展示或记录的警告。
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
