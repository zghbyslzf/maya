use clap::{Parser, Subcommand};
use maya_common::error::Result;
use std::path::PathBuf;

// 导入自定义模块
mod modules {
    pub mod clean_ops;
    pub mod git_ops;
    pub mod optimize_ops;
    pub mod pack_ops;
    pub mod transform_ops;
}

/// Maya CLI 工具集
#[derive(Parser)]
#[command(author, version, about, long_about = None)]
struct Cli {
    #[command(subcommand)]
    command: Command,
}

#[derive(Subcommand)]
enum Command {
    /// 清理操作
    Clean {
        /// 操作目录路径，默认为当前目录
        #[arg(default_value = ".", value_name = "PATH")]
        path: PathBuf,

        /// 清理类型（支持多个类型：n/node_modules, lock）
        #[arg(short = 't', long, num_args = 1.., value_name = "TYPES", required = true)]
        types: Vec<String>,
    },

    /// Git相关操作
    Git {
        /// 操作目录路径，默认为当前目录
        #[arg(default_value = ".", value_name = "PATH")]
        path: PathBuf,

        /// Git操作类型
        #[arg(short = 'o', long, num_args = 1.., value_name = "GIT_OPS", required = true)]
        ops: Vec<String>,
    },

    /// 打包操作
    Pack {
        /// 打包操作类型 (g: 根据gitignore打包, a: Vite项目打包)
        #[arg(short = 't', long, value_name = "PACK_TYPE", required = true)]
        pack_type: String,
    },

    /// 图片压缩操作
    Optimize {
        /// 操作目录路径，默认为当前目录
        #[arg(default_value = ".", value_name = "PATH")]
        path: PathBuf,

        /// 图片压缩类型 (png/jpg/jpeg/all, 添加n参数创建新文件)
        #[arg(short = 't', long, num_args = 1.., value_name = "OPT_TYPES", required = true)]
        types: Vec<String>,
    },

    /// 视频转换操作
    Transform {
        /// 操作目录路径，默认为当前目录
        #[arg(default_value = ".", value_name = "PATH")]
        path: PathBuf,

        /// 源格式 目标格式，例如: mp4 m3u8
        #[arg(short = 't', long, num_args = 2.., value_name = "TRANSFORM_TYPES", required = true)]
        types: Vec<String>,
    },
}

#[tokio::main]
async fn main() -> Result<()> {
    let cli = Cli::parse();

    match cli.command {
        Command::Clean { types, path } => {
            modules::clean_ops::handle_clean_ops(&types, &path)?;
        }
        Command::Git { ops, path } => {
            modules::git_ops::handle_git_ops(&ops, &path)?;
        }
        Command::Pack { pack_type } => {
            modules::pack_ops::handle_pack_ops(&pack_type)?;
        }
        Command::Optimize { types, path } => {
            modules::optimize_ops::handle_optimize_ops(&types, &path)?;
        }
        Command::Transform { types, path } => {
            modules::transform_ops::handle_transform_ops(&types, &path).await?;
        }
    }
    Ok(())
}
