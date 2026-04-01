use std::path::Path;
use maya_common::error::{Error, Result};

/// 处理清理操作的模块
pub fn handle_clean_ops(clean_types: &[String], path: &Path) -> Result<()> {
    for clean_type in clean_types {
        match clean_type.as_str() {
            "n" | "node_modules" => {
                println!("清理目录 {} 中的 node_modules 文件夹", path.display());
                let count = clear_node_modules::clear_node_modules(path.to_string_lossy().to_string())?;
                println!("已清理 {} 个 node_modules 文件夹", count);
            }
            "lock" => {
                println!("清理目录 {} 中的锁文件", path.display());
                let count = clear_lock::clear_lock_files(path.to_string_lossy().to_string())?;
                println!("已清理 {} 个锁文件", count);
            }
            _ => {
                return Err(Error::invalid_argument(format!("不支持的清理类型: {}", clean_type)));
            }
        }
    }
    Ok(())
}
