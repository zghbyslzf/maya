use crate::{find_directories_by_name_pruned, find_files};
use maya_core::{Error, RemovalReport, Result};
use std::collections::HashSet;
use std::fs;
use std::path::Path;
use std::sync::LazyLock;

static LOCK_FILES: LazyLock<HashSet<&str>> = LazyLock::new(|| {
    ["package-lock.json", "yarn.lock", "pnpm-lock.yaml"]
        .into_iter()
        .collect()
});

pub fn clear_node_modules(root: &Path) -> Result<RemovalReport> {
    let targets = find_directories_by_name_pruned(root, "node_modules")?;
    remove_targets(targets, true, "删除 node_modules 目录")
}

pub fn clear_lock_files(root: &Path) -> Result<RemovalReport> {
    let targets = find_files(root, |path| {
        path.file_name()
            .and_then(|name| name.to_str())
            .is_some_and(|name| LOCK_FILES.contains(name))
    })?;
    remove_targets(targets, false, "删除锁文件")
}

fn remove_targets(
    targets: Vec<std::path::PathBuf>,
    directories: bool,
    operation: &str,
) -> Result<RemovalReport> {
    let mut report = RemovalReport::default();
    for target in targets {
        let result = if directories {
            fs::remove_dir_all(&target)
        } else {
            fs::remove_file(&target)
        };
        match result {
            Ok(()) => report.removed.push(target),
            Err(error) if error.kind() == std::io::ErrorKind::NotFound => {
                report.skipped.push(target);
            }
            Err(error) => return Err(Error::io_context(operation, target, error)),
        }
    }
    Ok(report)
}

#[cfg(test)]
mod tests {
    use super::*;
    use tempfile::tempdir;

    #[test]
    fn clears_outer_nested_and_sibling_node_modules_once() {
        let root = tempdir().unwrap();
        let outer = root.path().join("node_modules");
        let nested = outer.join("dependency/node_modules");
        let sibling = root.path().join("app/node_modules");
        fs::create_dir_all(&nested).unwrap();
        fs::create_dir_all(&sibling).unwrap();

        let report = clear_node_modules(root.path()).unwrap();

        assert_eq!(report.removed_count(), 2);
        assert!(!outer.exists());
        assert!(!sibling.exists());
    }

    #[test]
    fn clears_supported_lock_files() {
        let root = tempdir().unwrap();
        fs::write(root.path().join("package-lock.json"), "{}").unwrap();
        fs::write(root.path().join("yarn.lock"), "lock").unwrap();
        fs::write(root.path().join("other.txt"), "keep").unwrap();

        let report = clear_lock_files(root.path()).unwrap();

        assert_eq!(report.removed_count(), 2);
        assert!(root.path().join("other.txt").exists());
    }

    #[test]
    fn disappeared_target_is_reported_as_skipped() {
        let root = tempdir().unwrap();
        let target = root.path().join("node_modules");

        let report = remove_targets(vec![target.clone()], true, "删除 node_modules 目录").unwrap();

        assert!(report.removed.is_empty());
        assert_eq!(report.skipped, vec![target]);
    }
}
