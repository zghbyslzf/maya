use std::path::PathBuf;
use clap::Parser; // clap 是一个流行的 Rust 库，用于解析命令行参数

#[derive(Parser)]
#[command(author, version, about = "Maya CLI 工具集")]
struct Cli {
    /// 执行清理操作（支持多个类型：n/node_modules, lock）
    #[arg(short = 'c', long, num_args = 1.., value_name = "TYPES")]
    clean_types: Option<Vec<String>>,

    /// 操作目录路径，默认为当前目录
    #[arg(default_value = ".", value_name = "PATH")]
    path: PathBuf,
}

fn main() {
    let cli = Cli::parse();

    if let Some(types) = &cli.clean_types {
        for clean_type in types {
            match clean_type.as_str() {
                "n" | "node_modules" => {
                    println!("清理目录 {} 中的 node_modules 文件夹", cli.path.display());
                    match clear_node_modules::clear_node_modules(cli.path.to_string_lossy().to_string()) {
                        Ok(count) => println!("已清理 {} 个 node_modules 文件夹", count),
                        Err(e) => eprintln!("清理过程中出错: {:?}", e),
                    }
                },
                "lock" => {
                    println!("清理目录 {} 中的锁文件", cli.path.display());
                    match clear_lock::clear_lock_files(cli.path.to_string_lossy().to_string()) {
                        Ok(count) => println!("已清理 {} 个锁文件", count),
                        Err(e) => eprintln!("清理过程中出错: {:?}", e),
                    }
                },
                _ => {
                    eprintln!("跳过不支持的清理类型: {}", clean_type);
                }
            }
        }
    } else {
        eprintln!("请使用 -c 选项指定清理类型");
    }
}