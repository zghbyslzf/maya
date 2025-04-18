use std::path::PathBuf;

/// 处理清理操作的模块
pub fn handle_clean_ops(clean_types: &[String], path: &PathBuf) {
    for clean_type in clean_types {
        match clean_type.as_str() {
            "n" | "node_modules" => {
                println!("清理目录 {} 中的 node_modules 文件夹", path.display());
                match clear_node_modules::clear_node_modules(path.to_string_lossy().to_string()) {
                    Ok(count) => println!("已清理 {} 个 node_modules 文件夹", count),
                    Err(e) => eprintln!("清理过程中出错: {:?}", e),
                }
            }
            "lock" => {
                println!("清理目录 {} 中的锁文件", path.display());
                match clear_lock::clear_lock_files(path.to_string_lossy().to_string()) {
                    Ok(count) => println!("已清理 {} 个锁文件", count),
                    Err(e) => eprintln!("清理过程中出错: {:?}", e),
                }
            }
            _ => {
                eprintln!("跳过不支持的清理类型: {}", clean_type);
            }
        }
    }
}
