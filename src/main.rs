use clap::Parser;
use std::path::PathBuf; // clap 是一个流行的 Rust 库，用于解析命令行参数
use std::path::Path;

// 导入自定义模块
mod modules {
    pub mod clean_ops;
    pub mod git_ops;
    pub mod pack_ops;
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
}

fn main() {
    let cli = Cli::parse();

    if let Some(types) = &cli.clean_types {
        modules::clean_ops::handle_clean_ops(types, &cli.path);
    } else if let Some(git_ops) = &cli.git_ops {
        modules::git_ops::handle_git_ops(git_ops, &cli.path);
    } else if let Some(pack_type) = &cli.pack_type {
        modules::pack_ops::handle_pack_ops(pack_type);
    } else if let Some(opt_types) = &cli.optimize_types {
        handle_optimize_ops(opt_types, &cli.path);
    } else {
        eprintln!("请使用 -c, -g, -p 或 -o 选项指定操作类型");
    }
}

fn handle_optimize_ops(types: &[String], path: &Path) {
    if types.is_empty() {
        eprintln!("请指定要压缩的图片类型 (png/jpg/jpeg/all)");
        return;
    }

    // 检查是否有n参数，表示创建新文件而不是覆盖
    let create_new_file = types.iter().any(|t| t == "n");
    
    // 找到图片类型参数
    let img_type_str = types.iter()
        .find(|&t| t != "n")
        .map(|s| s.as_str())
        .unwrap_or("all"); // 默认压缩所有类型
    
    match compress_pictures::ImageType::from_str(img_type_str) {
        Ok(img_type) => {
            // println!("准备压缩图片，类型: {:?}, 创建新文件: {}", img_type, create_new_file);
            match compress_pictures::compress_images(path, img_type, create_new_file) {
                Ok((successful_compressions, failed_compressions, _avg_compression_ratio)) => {
                    // 详细的总结信息已在 compress_images 内部打印
                    // 这里可以根据需要添加额外的顶层消息
                    if successful_compressions == 0 && failed_compressions == 0 {
                        println!("未找到符合指定类型的图片进行处理。");
                    } else if successful_compressions > 0 {
                        // println!("图片压缩任务部分或全部完成。"); // 可选的额外消息
                    } else if failed_compressions > 0 && successful_compressions == 0 {
                        println!("所有找到的图片都压缩失败了。");
                    }
                },
                Err(e) => {
                    eprintln!("图片压缩操作因内部错误而失败: {}", e);
                }
            }
        },
        Err(e) => {
            eprintln!("图片类型参数 '{}' 错误: {}", img_type_str, e);
        }
    }
}
