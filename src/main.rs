﻿use clap::Parser;
use std::path::PathBuf; // clap 是一个流行的 Rust 库，用于解析命令行参数

// 导入自定义模块
mod modules {
    pub mod clean_ops;
    pub mod git_ops;
    pub mod pack_ops;
    pub mod optimize_ops;
    pub mod transform_ops;
}

#[derive(Parser)]
#[command(author, version, about = "Maya CLI 工具集")]
struct Cli {
    /// 执行清理操作（支持多个类型：n/node_modules, lock）
    #[arg(short = 'c', long, num_args = 1.., value_name = "TYPES")]
    clean_types: Option<Vec<String>>,

    /// 操作目录路径，默认为当前目录
    #[arg(default_value = ".", value_name = "PATH")]
    path: PathBuf,
    
    /// Git相关操作
    #[arg(short = 'g', long, num_args = 1.., value_name = "GIT_OPS")]
    git_ops: Option<Vec<String>>,

    /// 打包操作类型 (g: 根据gitignore打包, a: Vite项目打包)
    #[arg(short = 'p', long, value_name = "PACK_TYPE")]
    pack_type: Option<String>,
    
    /// 图片压缩操作 (png/jpg/jpeg/all, 添加n参数创建新文件)
    #[arg(short = 'o', long, num_args = 1.., value_name = "OPT_TYPES")]
    optimize_types: Option<Vec<String>>,

    /// 视频转换操作 (源格式 目标格式，例如: mp4 m3u8)
    #[arg(short = 't', long, num_args = 2.., value_name = "TRANSFORM_TYPES")]
    transform_types: Option<Vec<String>>,
}

#[tokio::main]
async fn main() {
    let cli = Cli::parse();

    if let Some(types) = &cli.clean_types {
        modules::clean_ops::handle_clean_ops(types, &cli.path);
    } else if let Some(git_ops) = &cli.git_ops {
        modules::git_ops::handle_git_ops(git_ops, &cli.path);
    } else if let Some(pack_type) = &cli.pack_type {
        modules::pack_ops::handle_pack_ops(pack_type);
    } else if let Some(opt_types) = &cli.optimize_types {
        modules::optimize_ops::handle_optimize_ops(opt_types, &cli.path);
    } else if let Some(transform_types) = &cli.transform_types {
        modules::transform_ops::handle_transform_ops(transform_types, &cli.path).await;
    } else {
        eprintln!("请使用 -c, -g, -p, -o 或 -t 选项指定操作类型");
    }
}
