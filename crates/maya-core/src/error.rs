use std::error::Error as StdError;
use std::path::PathBuf;
use thiserror::Error;

#[derive(Error, Debug)]
pub enum Error {
    #[error("I/O错误: {0}")]
    Io(#[from] std::io::Error),

    #[error("{operation}失败（路径: {path}）: {source}")]
    IoContext {
        operation: String,
        path: PathBuf,
        #[source]
        source: std::io::Error,
    },

    #[error("目录遍历错误（路径: {path:?}）: {source}")]
    Traversal {
        path: Option<PathBuf>,
        #[source]
        source: Box<dyn StdError + Send + Sync>,
    },

    #[error("路径错误: {0}")]
    Path(String),

    #[error("压缩错误: {0}")]
    Compression(String),

    #[error("视频转换错误: {0}")]
    VideoConversion(String),

    #[error("无效参数: {0}")]
    InvalidArgument(String),

    #[error(
        "外部命令失败（程序: {program}；参数: {args:?}；工作目录: {cwd}；退出码: {status:?}）: {stderr}"
    )]
    CommandFailed {
        program: String,
        args: Vec<String>,
        cwd: PathBuf,
        status: Option<i32>,
        stderr: String,
    },

    #[error("{operation}部分失败：成功 {succeeded}，失败 {failed}")]
    PartialFailure {
        operation: String,
        succeeded: usize,
        failed: usize,
    },

    #[error("配置文件错误: {0}")]
    Config(String),

    #[error("在 {root} 下未找到配置文件（候选: {candidates:?}）")]
    ConfigNotFound {
        root: PathBuf,
        candidates: Vec<String>,
    },

    #[error("不支持的动态配置（文件: {path}）: {reason}")]
    UnsupportedConfig { path: PathBuf, reason: String },

    #[error("错误: {0}")]
    Other(String),
}

pub type Result<T> = std::result::Result<T, Error>;

impl Error {
    pub fn io_context(
        operation: impl Into<String>,
        path: impl Into<PathBuf>,
        source: std::io::Error,
    ) -> Self {
        Self::IoContext {
            operation: operation.into(),
            path: path.into(),
            source,
        }
    }

    pub fn traversal(path: Option<PathBuf>, source: impl StdError + Send + Sync + 'static) -> Self {
        Self::Traversal {
            path,
            source: Box::new(source),
        }
    }

    pub fn path(message: impl Into<String>) -> Self {
        Self::Path(message.into())
    }

    pub fn compression(message: impl Into<String>) -> Self {
        Self::Compression(message.into())
    }

    pub fn video_conversion(message: impl Into<String>) -> Self {
        Self::VideoConversion(message.into())
    }

    pub fn invalid_argument(message: impl Into<String>) -> Self {
        Self::InvalidArgument(message.into())
    }

    pub fn command_failed(
        program: impl Into<String>,
        args: Vec<String>,
        cwd: impl Into<PathBuf>,
        status: Option<i32>,
        stderr: impl Into<String>,
    ) -> Self {
        Self::CommandFailed {
            program: program.into(),
            args,
            cwd: cwd.into(),
            status,
            stderr: stderr.into(),
        }
    }

    pub fn partial_failure(operation: impl Into<String>, succeeded: usize, failed: usize) -> Self {
        Self::PartialFailure {
            operation: operation.into(),
            succeeded,
            failed,
        }
    }

    pub fn exit_code(&self) -> u8 {
        match self {
            Self::InvalidArgument(_) => 2,
            Self::PartialFailure { .. } => 3,
            Self::Path(_)
            | Self::Config(_)
            | Self::ConfigNotFound { .. }
            | Self::UnsupportedConfig { .. } => 4,
            Self::CommandFailed { .. } => 5,
            _ => 1,
        }
    }

    pub fn config(message: impl Into<String>) -> Self {
        Self::Config(message.into())
    }

    pub fn config_not_found(
        root: impl Into<PathBuf>,
        candidates: impl IntoIterator<Item = impl Into<String>>,
    ) -> Self {
        Self::ConfigNotFound {
            root: root.into(),
            candidates: candidates.into_iter().map(Into::into).collect(),
        }
    }

    pub fn unsupported_config(path: impl Into<PathBuf>, reason: impl Into<String>) -> Self {
        Self::UnsupportedConfig {
            path: path.into(),
            reason: reason.into(),
        }
    }

    pub fn other(message: impl Into<String>) -> Self {
        Self::Other(message.into())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn exit_codes_are_stable_by_error_category() {
        assert_eq!(Error::invalid_argument("bad").exit_code(), 2);
        assert_eq!(Error::partial_failure("batch", 1, 1).exit_code(), 3);
        assert_eq!(Error::path("missing").exit_code(), 4);
        assert_eq!(
            Error::command_failed("tool", vec![], ".", Some(1), "failed").exit_code(),
            5
        );
    }
}
