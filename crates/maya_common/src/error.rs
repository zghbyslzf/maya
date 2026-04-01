use thiserror::Error;

/// Maya CLI 统一错误类型
#[derive(Error, Debug)]
pub enum Error {
    /// I/O 错误
    #[error("I/O错误: {0}")]
    Io(#[from] std::io::Error),

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

    /// 配置文件错误
    #[error("配置文件错误: {0}")]
    Config(String),

    /// 其他未分类错误
    #[error("错误: {0}")]
    Other(String),
}

/// 统一结果类型别名
pub type Result<T> = std::result::Result<T, Error>;

/// 为 Path 错误提供便捷构造函数
impl Error {
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

    /// 创建配置文件错误
    pub fn config(msg: impl Into<String>) -> Self {
        Self::Config(msg.into())
    }

    /// 创建其他错误
    pub fn other(msg: impl Into<String>) -> Self {
        Self::Other(msg.into())
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

