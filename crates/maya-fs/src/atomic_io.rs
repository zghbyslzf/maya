use maya_core::{Error, Result};
use std::fs;
use std::io::Write;
use std::path::Path;
use tempfile::Builder;

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
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use tempfile::tempdir;

    #[test]
    fn failed_file_write_preserves_existing_target() {
        let root = tempdir().unwrap();
        let target = root.path().join("result.bin");
        fs::write(&target, b"old").unwrap();

        let result = atomic_write(&target, |file, _| {
            file.write_all(b"partial")?;
            Err(Error::other("模拟失败"))
        });

        assert!(result.is_err());
        assert_eq!(fs::read(&target).unwrap(), b"old");
        assert_eq!(fs::read_dir(root.path()).unwrap().count(), 1);
    }

    #[test]
    fn successful_file_write_replaces_target_without_temporary_files() {
        let root = tempdir().unwrap();
        let target = root.path().join("result.bin");
        fs::write(&target, b"old").unwrap();

        atomic_write(&target, |file, _| {
            file.write_all(b"new")?;
            Ok(())
        })
        .unwrap();

        assert_eq!(fs::read(&target).unwrap(), b"new");
        assert_eq!(fs::read_dir(root.path()).unwrap().count(), 1);
    }

    #[test]
    fn failed_directory_build_preserves_existing_target() {
        let root = tempdir().unwrap();
        let target = root.path().join("video");
        fs::create_dir(&target).unwrap();
        fs::write(target.join("index.m3u8"), b"old").unwrap();

        let result = atomic_replace_directory(&target, |staging| {
            fs::write(staging.join("index.m3u8"), b"partial")?;
            Err(Error::other("模拟失败"))
        });

        assert!(result.is_err());
        assert_eq!(fs::read(target.join("index.m3u8")).unwrap(), b"old");
    }

    #[test]
    fn successful_directory_build_replaces_target_without_staging_directories() {
        let root = tempdir().unwrap();
        let target = root.path().join("video");
        fs::create_dir(&target).unwrap();
        fs::write(target.join("old.ts"), b"old").unwrap();

        atomic_replace_directory(&target, |staging| {
            fs::write(staging.join("index.m3u8"), b"new")?;
            Ok(())
        })
        .unwrap();

        assert!(!target.join("old.ts").exists());
        assert_eq!(fs::read(target.join("index.m3u8")).unwrap(), b"new");
        assert_eq!(fs::read_dir(root.path()).unwrap().count(), 1);
    }
}
