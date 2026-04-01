use crate::error::{Error, Result};
use std::path::{Path, PathBuf};
use walkdir::WalkDir;
#[cfg(feature = "parallel")]
use walkdir::DirEntry;

/// 查找目录中匹配特定条件的文件
///
/// # 参数
/// * `dir` - 要搜索的目录路径
/// * `filter` - 过滤函数，接受文件路径并返回是否包含该文件
///
/// # 返回
/// * `Result<Vec<PathBuf>>` - 匹配的文件路径列表
pub fn find_files<F>(dir: &Path, filter: F) -> Result<Vec<PathBuf>>
where
    F: Fn(&Path) -> bool,
{
    if !dir.is_dir() {
        return Err(Error::path(format!("路径不是目录: {}", dir.display())));
    }

    let mut files = Vec::new();
    for entry in WalkDir::new(dir)
        .into_iter()
        .filter_map(|e| e.ok())
        .filter(|e| e.file_type().is_file())
    {
        let path = entry.path();
        if filter(path) {
            files.push(path.to_path_buf());
        }
    }

    Ok(files)
}

/// 查找目录中匹配特定条件的文件（并行版本）
///
/// # 参数
/// * `dir` - 要搜索的目录路径
/// * `filter` - 过滤函数，接受文件路径并返回是否包含该文件
///
/// # 返回
/// * `Result<Vec<PathBuf>>` - 匹配的文件路径列表
#[cfg(feature = "parallel")]
pub fn find_files_parallel<F>(dir: &Path, filter: F) -> Result<Vec<PathBuf>>
where
    F: Fn(&Path) -> bool + Send + Sync,
{
    use rayon::prelude::*;

    if !dir.is_dir() {
        return Err(Error::path(format!("路径不是目录: {}", dir.display())));
    }

    let entries: Vec<DirEntry> = WalkDir::new(dir)
        .into_iter()
        .filter_map(|e| e.ok())
        .collect();

    let files: Vec<PathBuf> = entries
        .par_iter()
        .filter(|entry| entry.file_type().is_file())
        .map(|entry| entry.path())
        .filter(|path| filter(path))
        .map(|path| path.to_path_buf())
        .collect();

    Ok(files)
}

/// 查找目录中匹配特定扩展名的文件
///
/// # 参数
/// * `dir` - 要搜索的目录路径
/// * `extensions` - 扩展名列表（不包含点号，如 ["png", "jpg"]）
///
/// # 返回
/// * `Result<Vec<PathBuf>>` - 匹配的文件路径列表
pub fn find_files_by_extension(dir: &Path, extensions: &[&str]) -> Result<Vec<PathBuf>> {
    let extension_set: std::collections::HashSet<&str> = extensions.iter().copied().collect();
    
    find_files(dir, |path| {
        path.extension()
            .and_then(|ext| ext.to_str())
            .map(|ext| extension_set.contains(&ext.to_lowercase().as_str()))
            .unwrap_or(false)
    })
}

/// 递归查找目录中匹配特定名称的文件或目录
///
/// # 参数
/// * `dir` - 要搜索的目录路径
/// * `name` - 要匹配的文件或目录名
/// * `match_type` - 匹配类型：File（仅文件）、Dir（仅目录）、Any（文件或目录）
///
/// # 返回
/// * `Result<Vec<PathBuf>>` - 匹配的路径列表
pub fn find_by_name(
    dir: &Path,
    name: &str,
    match_type: MatchType,
) -> Result<Vec<PathBuf>> {
    if !dir.is_dir() {
        return Err(Error::path(format!("路径不是目录: {}", dir.display())));
    }

    let mut results = Vec::new();
    for entry in WalkDir::new(dir).into_iter().filter_map(|e| e.ok()) {
        let path = entry.path();
        
        // 检查是否匹配名称
        if path.file_name()
            .and_then(|n| n.to_str())
            .map(|n| n == name)
            .unwrap_or(false)
        {
            // 检查是否匹配类型
            let matches = match match_type {
                MatchType::File => path.is_file(),
                MatchType::Dir => path.is_dir(),
                MatchType::Any => true,
            };
            
            if matches {
                results.push(path.to_path_buf());
            }
        }
    }

    Ok(results)
}

/// 匹配类型枚举
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum MatchType {
    File,
    Dir,
    Any,
}

/// 递归删除目录中的所有空目录
///
/// # 参数
/// * `dir` - 目录路径
///
/// # 返回
/// * `Result<usize>` - 删除的空目录数量
pub fn remove_empty_dirs(dir: &Path) -> Result<usize> {
    if !dir.is_dir() {
        return Ok(0);
    }

    let mut count = 0;
    remove_empty_dirs_recursive(dir, &mut count)?;
    Ok(count)
}

fn remove_empty_dirs_recursive(dir: &Path, count: &mut usize) -> Result<()> {
    let mut has_content = false;

    for entry in std::fs::read_dir(dir)? {
        let entry = entry?;
        let path = entry.path();

        if path.is_dir() {
            // 递归处理子目录
            remove_empty_dirs_recursive(&path, count)?;
            
            // 检查子目录是否为空（可能已被删除）
            if path.exists() && std::fs::read_dir(&path)?.next().is_none() {
                std::fs::remove_dir(&path)?;
                *count += 1;
                println!("已删除空目录: {}", path.display());
            } else {
                has_content = true;
            }
        } else {
            has_content = true;
        }
    }

    // 如果当前目录为空且不是根目录，则删除
    if !has_content && dir != Path::new("") && dir != Path::new("/") {
        std::fs::remove_dir(dir)?;
        *count += 1;
        println!("已删除空目录: {}", dir.display());
    }

    Ok(())
}