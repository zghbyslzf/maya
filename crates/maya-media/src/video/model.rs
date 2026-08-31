use maya_core::{Error, FailurePolicy, OperationWarning};
use std::path::PathBuf;
use std::time::Duration;

pub(super) const DEFAULT_CONVERSION_TIMEOUT: Duration = Duration::from_secs(2 * 60 * 60);

#[derive(Debug, Clone)]
pub struct ConversionOptions {
    pub timeout: Duration,
    pub failure_policy: FailurePolicy,
}

impl Default for ConversionOptions {
    fn default() -> Self {
        Self {
            timeout: DEFAULT_CONVERSION_TIMEOUT,
            failure_policy: FailurePolicy::Continue,
        }
    }
}

#[derive(Debug)]
pub enum ConversionOutcome {
    Converted {
        input: PathBuf,
        output_dir: PathBuf,
        warnings: Vec<OperationWarning>,
    },
    Failed {
        input: PathBuf,
        error: Error,
    },
}

#[derive(Debug, Default)]
pub struct ConversionReport {
    pub scanned: usize,
    pub succeeded: usize,
    pub failed: usize,
    pub warning_count: usize,
    pub items: Vec<ConversionOutcome>,
}

pub(super) struct ConvertedVideo {
    pub output_dir: PathBuf,
    pub warnings: Vec<OperationWarning>,
}
