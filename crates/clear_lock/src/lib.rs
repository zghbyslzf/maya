use std::fs;
use std::path::Path;
use maya_common::error::Result;
use maya_common::file_utils::find_files;

/// 清除目录中的锁文件 (package-lock.json, yarn.lock 等)
pub fn clear_lock_files<P: AsRef<Path>>(dir: P) -> Result<usize> {
    let lock_files = ["package-lock.json", "yarn.lock", "pnpm-lock.yaml"];
    let lock_files_set: std::collections::HashSet<&str> = lock_files.iter().copied().collect();

    // 使用共享的文件遍历函数查找所有文件
    let all_files = find_files(dir.as_ref(), |path| {
        // 只检查文件
        if !path.is_file() {
            return false;
        }
        
        // 获取文件名
        if let Some(file_name) = path.file_name().and_then(|n| n.to_str()) {
            lock_files_set.contains(file_name)
        } else {
            false
        }
    })?;

    let mut count = 0;
    for file_path in all_files {
        fs::remove_file(&file_path)?;
        count += 1;
        println!("已删除: {}", file_path.display());
    }

    Ok(count)
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::fs::File;
    use std::io::Write;
    use tempfile::tempdir;

    #[test]
    fn test_clear_lock_files() {
        let temp_dir = tempdir().unwrap();
        let lock_path = temp_dir.path().join("package-lock.json");

        // 创建测试锁文件
        let mut file = File::create(&lock_path).unwrap();
        writeln!(file, "{{}}").unwrap();

        assert!(lock_path.exists());

        // 清除锁文件
        let count = clear_lock_files(temp_dir.path()).unwrap();

        assert_eq!(count, 1);
        assert!(!lock_path.exists());
    }
}
