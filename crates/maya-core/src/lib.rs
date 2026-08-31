mod error;
mod report;

pub use error::{Error, Result};
pub use report::{
    ArchiveReport, FailurePolicy, NoopProgress, OperationWarning, ProgressEvent, ProgressSink,
    RemovalReport,
};
