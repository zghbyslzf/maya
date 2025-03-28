use std::fs;
use std::path::Path;
use std::io;

/// 清除目录中的锁文件 (package-lock.json, yarn.lock 等)
pub fn clear_lock_files<P: AsRef<Path>>(dir: P) -> io::Result<usize> {
    let lock_files = ["package-lock.json", "yarn.lock", "pnpm-lock.yaml"];
    let mut count = 0;
    
    // 遍历目录
    clear_lock_files_in_dir(dir.as_ref(), &lock_files, &mut count)?;
    
    Ok(count)
}

fn clear_lock_files_in_dir(dir: &Path, lock_files: &[&str], count: &mut usize) -> io::Result<()> {
    if !dir.is_dir() {
        return Ok(());
    }
    
    // 检查当前目录中的锁文件
    for &lock_file in lock_files {
        let file_path = dir.join(lock_file);
        if file_path.exists() {
            fs::remove_file(&file_path)?;
            *count += 1;
            println!("已删除: {}", file_path.display());
        }
    }
    
    // 遍历子目录
    for entry in fs::read_dir(dir)? {
        let entry = entry?;
        let path = entry.path();
        
        if path.is_dir() && path.file_name().map_or(false, |name| name != "node_modules") {
            // 跳过 node_modules 目录，避免不必要的递归
            clear_lock_files_in_dir(&path, lock_files, count)?;
        }
    }
    
    Ok(())
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