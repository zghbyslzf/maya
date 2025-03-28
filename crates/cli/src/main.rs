use std::path::PathBuf; // 用于处理文件路径
use clap::{Parser, Subcommand}; // clap 是一个流行的 Rust 库，用于解析命令行参数, Parser 和 Subcommand 是 clap 提供的宏和特性，用于定义命令和子命令

#[derive(Parser)]
#[command(author, version, about = "Maya CLI 工具集")]
struct Cli {
    #[command(subcommand)]
    command: Commands,
}

#[derive(Subcommand)]
enum Commands {
    /// 清理操作
    #[command(name = "c")]
    Clean {
        /// 要清理的内容类型
        #[arg(value_name = "TYPE")]
        clean_type: String,
        
        /// 操作目录路径，默认为当前目录
        #[arg(value_name = "PATH", default_value = ".")]
        path: PathBuf,
    },
}

fn main() {
    let cli = Cli::parse();

    match &cli.command {
        Commands::Clean { clean_type, path } => {
            match clean_type.as_str() {
                "node_modules" => {
                    println!("清理目录 {} 中的 node_modules 文件夹", path.display());
                    match clear_node_modules::clear_node_modules(path) {
                        Ok(count) => println!("已清理 {} 个 node_modules 文件夹", count),
                        Err(e) => eprintln!("清理过程中出错: {}", e),
                    }
                },
                "lock" => {
                    println!("清理目录 {} 中的锁文件", path.display());
                    match clear_lock::clear_lock_files(path) {
                        Ok(count) => println!("已清理 {} 个锁文件", count),
                        Err(e) => eprintln!("清理过程中出错: {}", e),
                    }
                },
                _ => {
                    eprintln!("不支持的清理类型: {}。支持的类型有: node_modules, lock", clean_type);
                }
            }
        }
    }
} 