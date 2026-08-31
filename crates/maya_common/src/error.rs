use std::path::PathBuf;
use thiserror::Error;

/// Maya CLI 统一错误类型
#[derive(Error, Debug)]
pub enum Error {
    /// I/O 错误
    #[error("I/O错误: {0}")]
    Io(#[from] std::io::Error),

    /// 带操作和路径上下文的 I/O 错误
    #[error("{operation}失败（路径: {path}）: {source}")]
    IoContext {
        operation: String,
        path: PathBuf,
        #[source]
        source: std::io::Error,
    },

    /// 目录遍历错误（保留出错路径和底层 I/O 原因）
    #[error("目录遍历错误: {0}")]
    Walk(#[from] walkdir::Error),

    /// 路径相关错误
    #[error("路径错误: {0}")]
    Path(String),

    /// 图片压缩错误
    #[error("压缩错误: {0}")]
    Compression(String),

    /// 视频转换错误
    #[error("视频转换错误: {0}")]
    VideoConversion(String),

    /// Git 操作错误
    #[error("Git操作错误: {0}")]
    Git(String),

    /// 无效参数错误
    #[error("无效参数: {0}")]
    InvalidArgument(String),

    /// 外部命令执行错误
    #[error("外部命令执行错误: {0}")]
    CommandExecution(String),

    /// 外部命令非正常完成
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

    /// 批处理存在一个或多个失败项目
    #[error("{operation}部分失败：成功 {succeeded}，失败 {failed}")]
    PartialFailure {
        operation: String,
        succeeded: usize,
        failed: usize,
    },

    /// 配置文件错误
    #[error("配置文件错误: {0}")]
    Config(String),

    /// 必需配置文件不存在
    #[error("在 {root} 下未找到配置文件（候选: {candidates:?}）")]
    ConfigNotFound {
        root: PathBuf,
        candidates: Vec<String>,
    },

    /// 配置存在，但使用了当前实现无法可靠解析的动态表达式
    #[error("不支持的动态配置（文件: {path}）: {reason}")]
    UnsupportedConfig { path: PathBuf, reason: String },

    /// 其他未分类错误
    #[error("错误: {0}")]
    Other(String),
}

/// 统一结果类型别名
pub type Result<T> = std::result::Result<T, Error>;

/// 为 Path 错误提供便捷构造函数
impl Error {
    /// 创建带路径上下文的 I/O 错误。
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

    /// 创建路径错误
    pub fn path(msg: impl Into<String>) -> Self {
        Self::Path(msg.into())
    }

    /// 创建压缩错误
    pub fn compression(msg: impl Into<String>) -> Self {
        Self::Compression(msg.into())
    }

    /// 创建视频转换错误
    pub fn video_conversion(msg: impl Into<String>) -> Self {
        Self::VideoConversion(msg.into())
    }

    /// 创建 Git 错误
    pub fn git(msg: impl Into<String>) -> Self {
        Self::Git(msg.into())
    }

    /// 创建无效参数错误
    pub fn invalid_argument(msg: impl Into<String>) -> Self {
        Self::InvalidArgument(msg.into())
    }

    /// 创建命令执行错误
    pub fn command_execution(msg: impl Into<String>) -> Self {
        Self::CommandExecution(msg.into())
    }

    /// 创建结构化外部命令错误。
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

    /// 创建批处理部分失败错误。
    pub fn partial_failure(operation: impl Into<String>, succeeded: usize, failed: usize) -> Self {
        Self::PartialFailure {
            operation: operation.into(),
            succeeded,
            failed,
        }
    }

    /// CLI 使用的稳定退出码。
    pub fn exit_code(&self) -> u8 {
        match self {
            Self::InvalidArgument(_) => 2,
            Self::PartialFailure { .. } => 3,
            Self::Path(_)
            | Self::Config(_)
            | Self::ConfigNotFound { .. }
            | Self::UnsupportedConfig { .. } => 4,
            Self::CommandExecution(_) | Self::CommandFailed { .. } | Self::Git(_) => 5,
            _ => 1,
        }
    }

    /// 创建配置文件错误
    pub fn config(msg: impl Into<String>) -> Self {
        Self::Config(msg.into())
    }

    /// 创建“配置文件不存在”错误。
    pub fn config_not_found(
        root: impl Into<PathBuf>,
        candidates: impl IntoIterator<Item = impl Into<String>>,
    ) -> Self {
        Self::ConfigNotFound {
            root: root.into(),
            candidates: candidates.into_iter().map(Into::into).collect(),
        }
    }

    /// 创建“不支持的动态配置”错误。
    pub fn unsupported_config(path: impl Into<PathBuf>, reason: impl Into<String>) -> Self {
        Self::UnsupportedConfig {
            path: path.into(),
            reason: reason.into(),
        }
    }

    /// 创建其他错误
    pub fn other(msg: impl Into<String>) -> Self {
        Self::Other(msg.into())
    }
}

/// 从 zip::result::ZipError 转换
impl From<zip::result::ZipError> for Error {
    fn from(err: zip::result::ZipError) -> Self {
        match err {
            zip::result::ZipError::Io(e) => Self::Io(e),
            _ => Self::Other(err.to_string()),
        }
    }
}

/// 从 anyhow::Error 转换
#[cfg(feature = "anyhow")]
impl From<anyhow::Error> for Error {
    fn from(err: anyhow::Error) -> Self {
        Self::Other(err.to_string())
    }
}

/// 从 tokio::task::JoinError 转换
#[cfg(feature = "tokio")]
impl From<tokio::task::JoinError> for Error {
    fn from(err: tokio::task::JoinError) -> Self {
        Self::Other(format!("任务执行错误: {}", err))
    }
}
