use crate::error::{Error, Result};
use crate::report::ArchiveReport;
use std::fs;
use std::io::{Read, Write};
use std::path::{Path, PathBuf};
use tempfile::Builder;
#[cfg(feature = "parallel")]
use walkdir::DirEntry;
use walkdir::WalkDir;
use zip::write::{FileOptions, ZipWriter};

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
    for entry in WalkDir::new(dir).sort_by_file_name() {
        let entry = entry?;
        if !entry.file_type().is_file() {
            continue;
        }

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
        .collect::<std::result::Result<_, walkdir::Error>>()?;

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
pub fn find_by_name(dir: &Path, name: &str, match_type: MatchType) -> Result<Vec<PathBuf>> {
    if !dir.is_dir() {
        return Err(Error::path(format!("路径不是目录: {}", dir.display())));
    }

    let mut results = Vec::new();
    for entry in WalkDir::new(dir) {
        let entry = entry?;
        let path = entry.path();

        // 检查是否匹配名称
        if path
            .file_name()
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

/// 递归查找指定名称的目录，并在命中后停止进入该目录。
///
/// 适用于删除 `node_modules` 这类整棵目录树的场景，避免同时返回父目录和
/// 已包含在父目录中的嵌套目标。
pub fn find_directories_by_name_pruned(dir: &Path, name: &str) -> Result<Vec<PathBuf>> {
    if !dir.is_dir() {
        return Err(Error::path(format!("路径不是目录: {}", dir.display())));
    }

    let mut results = Vec::new();
    let mut entries = WalkDir::new(dir).into_iter();

    while let Some(entry) = entries.next() {
        let entry = entry?;
        let is_match = entry.file_type().is_dir()
            && entry
                .file_name()
                .to_str()
                .map(|entry_name| entry_name == name)
                .unwrap_or(false);

        if is_match {
            results.push(entry.path().to_path_buf());
            entries.skip_current_dir();
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

/// 在目标文件所在目录中完成写入，并在全部写入成功后替换目标文件。
///
/// 回调失败、刷新失败或持久化失败时，临时文件会被删除，已有目标文件保持不变。
/// 同目录临时文件保证最终移动不会跨文件系统。
pub fn atomic_write<F>(target_path: &Path, write_fn: F) -> Result<()>
where
    F: FnOnce(&mut fs::File, &Path) -> Result<()>,
{
    let parent = target_path
        .parent()
        .filter(|path| !path.as_os_str().is_empty())
        .unwrap_or_else(|| Path::new("."));

    if !parent.is_dir() {
        return Err(Error::path(format!(
            "目标文件的父目录不存在或不是目录: {}",
            parent.display()
        )));
    }

    let mut temp_file = Builder::new().prefix(".maya-tmp-").tempfile_in(parent)?;
    let temp_path = temp_file.path().to_path_buf();

    write_fn(temp_file.as_file_mut(), &temp_path)?;
    temp_file.as_file_mut().flush()?;
    temp_file.as_file().sync_all()?;

    temp_file
        .into_temp_path()
        .persist(target_path)
        .map_err(|error| Error::Io(error.error))?;

    Ok(())
}

/// 在目标目录的同级临时目录中构建完整产物，成功后再提交到目标路径。
///
/// 构建失败时旧目录保持不变。目标已存在时会先移动到同级备份路径；如果提交
/// 新目录失败，会立即尝试恢复备份。
pub fn atomic_replace_directory<F>(target_path: &Path, build_fn: F) -> Result<()>
where
    F: FnOnce(&Path) -> Result<()>,
{
    let parent = target_path
        .parent()
        .filter(|path| !path.as_os_str().is_empty())
        .unwrap_or_else(|| Path::new("."));

    if !parent.is_dir() {
        return Err(Error::path(format!(
            "目标目录的父目录不存在或不是目录: {}",
            parent.display()
        )));
    }

    let staging_dir = Builder::new().prefix(".maya-tmp-").tempdir_in(parent)?;
    build_fn(staging_dir.path())?;
    let staging_path = staging_dir.keep();

    if !target_path.exists() {
        let result = fs::rename(&staging_path, target_path).map_err(|error| {
            let _ = fs::remove_dir_all(&staging_path);
            Error::Io(error)
        });
        if result.is_ok() {
            sync_directory(parent)?;
        }
        return result;
    }

    if !target_path.is_dir() {
        let _ = fs::remove_dir_all(&staging_path);
        return Err(Error::path(format!(
            "目标路径已存在但不是目录: {}",
            target_path.display()
        )));
    }

    let backup_reservation = Builder::new().prefix(".maya-backup-").tempdir_in(parent)?;
    let backup_path = backup_reservation.path().to_path_buf();
    backup_reservation.close()?;

    if let Err(error) = fs::rename(target_path, &backup_path) {
        let _ = fs::remove_dir_all(&staging_path);
        return Err(Error::Io(error));
    }

    if let Err(commit_error) = fs::rename(&staging_path, target_path) {
        let rollback_result = fs::rename(&backup_path, target_path);
        let _ = fs::remove_dir_all(&staging_path);

        return match rollback_result {
            Ok(()) => Err(Error::Io(commit_error)),
            Err(rollback_error) => Err(Error::other(format!(
                "提交目录 {} 失败: {}；恢复旧目录也失败: {}（备份位于 {}）",
                target_path.display(),
                commit_error,
                rollback_error,
                backup_path.display()
            ))),
        };
    }

    sync_directory(parent)?;
    fs::remove_dir_all(&backup_path)?;
    Ok(())
}

#[cfg(not(windows))]
fn sync_directory(path: &Path) -> Result<()> {
    fs::File::open(path)?.sync_all()?;
    Ok(())
}

#[cfg(windows)]
fn sync_directory(_path: &Path) -> Result<()> {
    // Windows 不允许通过普通 File API 打开目录；目录重命名由操作系统完成提交。
    Ok(())
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
    }

    Ok(())
}

pub fn create_zip_archive<F>(
    source_dir: &Path,
    dest_path: &Path,
    file_filter: F,
) -> Result<ArchiveReport>
where
    F: Fn(&Path) -> bool,
{
    if !source_dir.is_dir() {
        return Err(Error::path(format!(
            "归档源路径不是目录: {}",
            source_dir.display()
        )));
    }
    if !dest_path.is_dir() {
        return Err(Error::path(format!(
            "归档目标路径不是目录: {}",
            dest_path.display()
        )));
    }

    let folder_name = source_dir
        .file_name()
        .unwrap_or_default()
        .to_str()
        .unwrap_or("output");

    let zip_path = dest_path.join(format!("{}.zip", folder_name));

    let mut files_added = 0usize;
    let mut source_bytes = 0u64;
    atomic_write(&zip_path, |file, temp_path| {
        let mut zip = ZipWriter::new(file);
        let options: FileOptions<'_, ()> =
            FileOptions::default().compression_method(zip::CompressionMethod::Deflated);

        for entry in WalkDir::new(source_dir) {
            let entry = entry?;
            let path = entry.path();

            if path == zip_path || path == temp_path || !file_filter(path) {
                continue;
            }

            let name = path.strip_prefix(source_dir).map_err(|error| {
                Error::path(format!(
                    "无法计算归档相对路径 {}: {}",
                    path.display(),
                    error
                ))
            })?;

            if entry.file_type().is_file() {
                let name_str = name.to_str().ok_or_else(|| {
                    Error::path(format!("归档路径不是有效 UTF-8: {}", name.display()))
                })?;
                zip.start_file(name_str.replace('\\', "/"), options)?;
                let mut source_file = fs::File::open(path)?;
                source_bytes += entry.metadata()?.len();
                files_added += 1;
                let mut buffer = [0u8; 8192];
                loop {
                    let bytes_read = source_file.read(&mut buffer)?;
                    if bytes_read == 0 {
                        break;
                    }
                    zip.write_all(&buffer[..bytes_read])?;
                }
            }
        }

        zip.finish()?;
        Ok(())
    })?;

    let archive_bytes = fs::metadata(&zip_path)?.len();
    Ok(ArchiveReport {
        archive_path: zip_path,
        files_added,
        source_bytes,
        archive_bytes,
    })
}

pub fn find_file(dir: &Path, filename: &str) -> Option<PathBuf> {
    let file_path = dir.join(filename);
    if file_path.exists() {
        Some(file_path)
    } else {
        None
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::io::Write;
    use tempfile::tempdir;

    #[test]
    fn atomic_write_replaces_existing_file_after_success() {
        let temp_dir = tempdir().unwrap();
        let target = temp_dir.path().join("result.bin");
        fs::write(&target, b"old content").unwrap();

        atomic_write(&target, |file, _| {
            file.write_all(b"new content")?;
            Ok(())
        })
        .unwrap();

        assert_eq!(fs::read(&target).unwrap(), b"new content");
    }

    #[test]
    fn find_files_propagates_error_when_directory_disappears_during_walk() {
        let temp_dir = tempdir().unwrap();
        let trigger = temp_dir.path().join("a-trigger.txt");
        let doomed = temp_dir.path().join("z-doomed");
        fs::write(&trigger, b"trigger").unwrap();
        fs::create_dir(&doomed).unwrap();
        fs::write(doomed.join("data.txt"), b"data").unwrap();

        let result = find_files(temp_dir.path(), |path| {
            if path == trigger {
                fs::remove_dir_all(&doomed).unwrap();
            }
            true
        });

        assert!(matches!(result, Err(Error::Walk(_))));
    }

    #[test]
    fn atomic_write_preserves_existing_file_after_callback_failure() {
        let temp_dir = tempdir().unwrap();
        let target = temp_dir.path().join("result.bin");
        fs::write(&target, b"old content").unwrap();

        let result = atomic_write(&target, |file, _| {
            file.write_all(b"partial content")?;
            Err(Error::other("模拟写入失败"))
        });

        assert!(result.is_err());
        assert_eq!(fs::read(&target).unwrap(), b"old content");
        assert_eq!(fs::read_dir(temp_dir.path()).unwrap().count(), 1);
    }

    #[test]
    fn pruned_directory_search_does_not_return_nested_matches() {
        let temp_dir = tempdir().unwrap();
        let outer = temp_dir.path().join("node_modules");
        let nested = outer.join("dependency").join("node_modules");
        fs::create_dir_all(&nested).unwrap();
        fs::create_dir_all(temp_dir.path().join("app").join("node_modules")).unwrap();

        let mut matches = find_directories_by_name_pruned(temp_dir.path(), "node_modules").unwrap();
        matches.sort();

        assert_eq!(matches.len(), 2);
        assert!(matches.contains(&outer));
        assert!(!matches.contains(&nested));
    }

    #[test]
    fn atomic_replace_directory_preserves_existing_directory_after_build_failure() {
        let temp_dir = tempdir().unwrap();
        let target = temp_dir.path().join("video");
        fs::create_dir(&target).unwrap();
        fs::write(target.join("index.m3u8"), b"old playlist").unwrap();

        let result = atomic_replace_directory(&target, |staging_path| {
            fs::write(staging_path.join("index.m3u8"), b"partial playlist")?;
            Err(Error::other("模拟转换失败"))
        });

        assert!(result.is_err());
        assert_eq!(
            fs::read(target.join("index.m3u8")).unwrap(),
            b"old playlist"
        );
        assert_eq!(fs::read_dir(temp_dir.path()).unwrap().count(), 1);
    }

    #[test]
    fn atomic_replace_directory_commits_complete_directory() {
        let temp_dir = tempdir().unwrap();
        let target = temp_dir.path().join("video");
        fs::create_dir(&target).unwrap();
        fs::write(target.join("old.ts"), b"old segment").unwrap();

        atomic_replace_directory(&target, |staging_path| {
            fs::write(staging_path.join("index.m3u8"), b"new playlist")?;
            fs::write(staging_path.join("index0.ts"), b"new segment")?;
            Ok(())
        })
        .unwrap();

        assert!(!target.join("old.ts").exists());
        assert_eq!(
            fs::read(target.join("index.m3u8")).unwrap(),
            b"new playlist"
        );
        assert_eq!(fs::read_dir(temp_dir.path()).unwrap().count(), 1);
    }

    #[test]
    fn zip_failure_preserves_existing_archive() {
        let temp_dir = tempdir().unwrap();
        let source = temp_dir.path().join("source");
        fs::create_dir(&source).unwrap();
        let source_file = source.join("data.txt");
        fs::write(&source_file, b"source data").unwrap();
        let archive = temp_dir.path().join("source.zip");
        fs::write(&archive, b"old archive").unwrap();

        let result = create_zip_archive(&source, temp_dir.path(), |path| {
            if path == source_file {
                fs::remove_file(path).unwrap();
                true
            } else {
                false
            }
        });

        assert!(result.is_err());
        assert_eq!(fs::read(&archive).unwrap(), b"old archive");
        assert_eq!(fs::read_dir(temp_dir.path()).unwrap().count(), 2);
    }
}
