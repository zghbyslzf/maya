use clap::{Parser, Subcommand, ValueEnum};
use std::path::PathBuf;

#[derive(Debug, Parser)]
#[command(author, version, about = "Maya CLI 工具集", long_about = None)]
pub struct Cli {
    /// 不输出非错误信息
    #[arg(long, global = true)]
    pub quiet: bool,

    /// 禁用长任务进度输出
    #[arg(long, global = true)]
    pub no_progress: bool,

    #[command(subcommand)]
    pub command: Command,
}

#[derive(Debug, Subcommand)]
pub enum Command {
    /// 清理操作 (别名为 c)
    #[command(visible_alias = "c")]
    Clean {
        /// 操作目录路径，默认为当前目录
        #[arg(default_value = ".", value_name = "PATH")]
        path: PathBuf,

        /// 清理类型（支持多个类型）
        #[arg(short = 't', long, num_args = 1.., value_enum, required = true)]
        types: Vec<CleanTarget>,
    },

    /// Git相关操作 (别名为 g)
    #[command(visible_alias = "g")]
    Git {
        /// 操作目录路径，默认为当前目录
        #[arg(default_value = ".", value_name = "PATH")]
        path: PathBuf,

        /// Git 操作类型
        #[arg(short = 'o', long, value_enum, required = true)]
        ops: GitOperation,

        /// 自定义 commit message
        #[arg(short = 'm', long, default_value = "feat: update")]
        message: String,
    },

    /// 打包操作 (别名为 p)
    #[command(visible_alias = "p")]
    Pack {
        /// 项目根目录，默认为当前目录
        #[arg(default_value = ".", value_name = "PATH")]
        path: PathBuf,

        /// 打包方式
        #[arg(short = 't', long = "type", value_enum, required = true)]
        pack_type: PackMode,

        /// 显式指定 Vite 输出目录
        #[arg(long, value_name = "PATH")]
        out_dir: Option<PathBuf>,
    },

    /// 图片压缩操作 (别名为 o)
    #[command(visible_alias = "o")]
    Optimize {
        /// 操作目录路径，默认为当前目录
        #[arg(default_value = ".", value_name = "PATH")]
        path: PathBuf,

        /// 图片格式；旧值 n 仍表示创建新文件
        #[arg(short = 't', long, num_args = 1.., value_enum, required = true)]
        types: Vec<OptimizeType>,

        /// 创建带 `_c` 后缀的新文件
        #[arg(long)]
        new_file: bool,

        /// JPEG 有损编码质量
        #[arg(long, default_value_t = 80, value_parser = clap::value_parser!(u8).range(1..=100))]
        jpeg_quality: u8,

        /// 单项失败后的执行策略
        #[arg(long, value_enum, default_value_t = FailureMode::Continue)]
        failure_policy: FailureMode,
    },

    /// 视频转换操作 (别名为 t)
    #[command(visible_alias = "t")]
    Transform {
        /// 操作目录路径，默认为当前目录
        #[arg(default_value = ".", value_name = "PATH")]
        path: PathBuf,

        /// 源格式和目标格式，例如 mp4 m3u8
        #[arg(short = 't', long, num_args = 2, value_enum, required = true)]
        types: Vec<MediaFormat>,

        /// 单项失败后的执行策略
        #[arg(long, value_enum, default_value_t = FailureMode::Continue)]
        failure_policy: FailureMode,
    },
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, ValueEnum)]
pub enum CleanTarget {
    #[value(name = "node_modules", alias = "node-modules", alias = "n")]
    NodeModules,
    #[value(name = "lock")]
    LockFiles,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, ValueEnum)]
pub enum GitOperation {
    #[value(name = "m", alias = "add-commit-push")]
    AddCommitPush,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, ValueEnum)]
pub enum PackMode {
    #[value(name = "g", alias = "gitignore")]
    Gitignore,
    #[value(name = "a", alias = "vite")]
    Vite,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, ValueEnum)]
pub enum OptimizeType {
    #[value(name = "png")]
    Png,
    #[value(name = "jpeg", alias = "jpg")]
    Jpeg,
    #[value(name = "all")]
    All,
    #[value(name = "n", alias = "new-file")]
    LegacyNewFile,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, ValueEnum)]
pub enum MediaFormat {
    #[value(name = "mp4")]
    Mp4,
    #[value(name = "m3u8")]
    M3u8,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, ValueEnum)]
pub enum FailureMode {
    #[value(name = "continue")]
    Continue,
    #[value(name = "fail-fast")]
    FailFast,
}

impl From<FailureMode> for maya_common::FailurePolicy {
    fn from(value: FailureMode) -> Self {
        match value {
            FailureMode::Continue => Self::Continue,
            FailureMode::FailFast => Self::FailFast,
        }
    }
}
