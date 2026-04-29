use crate::error::{Error, Result};
use std::fs;
use std::io::{Read, Write};
use std::path::{Path, PathBuf};
use walkdir::WalkDir;
use zip::write::{FileOptions, ZipWriter};
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
    find_files(dir, |path| {
        path.extension()
            .and_then(|ext| ext.to_str())
            .map(|ext| extensions.iter().any(|e| e.eq_ignore_ascii_case(ext)))
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
            remove_empty_dirs_recursive(&path, count)?;
            
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

    if !has_content && dir.parent().is_some() {
        std::fs::remove_dir(dir)?;
        *count += 1;
        println!("已删除空目录: {}", dir.display());
    }

    Ok(())
}

pub fn create_zip_archive<F>(
    source_dir: &Path,
    dest_path: &Path,
    file_filter: F,
) -> Result<PathBuf>
where
    F: Fn(&Path) -> bool,
{
    let folder_name = source_dir
        .file_name()
        .unwrap_or_default()
        .to_str()
        .unwrap_or("output");
    
    let zip_path = dest_path.join(format!("{}.zip", folder_name));
    
    let file = fs::File::create(&zip_path)?;
    let mut zip = ZipWriter::new(file);
    
    let options: FileOptions<'_, ()> = FileOptions::default()
        .compression_method(zip::CompressionMethod::Deflated);
    
    let zip_filename = zip_path.file_name().unwrap_or_default().to_str().unwrap_or("");
    
    let walkdir = WalkDir::new(source_dir);
    let it = walkdir.into_iter().filter_map(|e| e.ok());
    
    for entry in it {
        let path = entry.path();
        
        if path.file_name().unwrap_or_default().to_str().unwrap_or("") == zip_filename {
            continue;
        }
        
        if !file_filter(path) {
            continue;
        }
        
        if let Ok(name) = path.strip_prefix(source_dir) {
            if path.is_file() {
                if let Some(name_str) = name.to_str() {
                    zip.start_file(name_str, options)?;
                    let mut f = fs::File::open(path)?;
                    let mut buffer = [0u8; 8192];
                    loop {
                        let n = f.read(&mut buffer)?;
                        if n == 0 { break; }
                        zip.write_all(&buffer[..n])?;
                    }
                }
            }
        }
    }
    
    zip.finish()?;
    Ok(zip_path)
}

pub fn find_file(dir: &Path, filename: &str) -> Option<PathBuf> {
    let file_path = dir.join(filename);
    if file_path.exists() {
        Some(file_path)
    } else {
        None
    }
}