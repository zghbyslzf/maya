use ignore::WalkBuilder;
use maya_common::error::{Error, Result};
use maya_common::ArchiveReport;
use std::collections::HashSet;
use std::path::{Component, Path, PathBuf};

/// 按项目根目录中的 `.gitignore` 规则创建 ZIP。
pub fn pack_with_gitignore(project_root: &Path) -> Result<ArchiveReport> {
    let gitignore = project_root.join(".gitignore");
    if !gitignore.is_file() {
        return Err(Error::config_not_found(project_root, [".gitignore"]));
    }

    let walker = WalkBuilder::new(project_root)
        .hidden(false)
        .git_global(false)
        .git_ignore(true)
        .require_git(false)
        .build();
    let mut allowed_files: HashSet<PathBuf> = HashSet::new();

    for entry in walker {
        let entry = entry.map_err(|error| {
            Error::path(format!(
                "按 .gitignore 遍历目录 {} 失败: {error}",
                project_root.display()
            ))
        })?;
        let path = entry.path();
        if path
            .components()
            .any(|component| component == Component::Normal(".git".as_ref()))
        {
            continue;
        }
        if entry
            .file_type()
            .is_some_and(|file_type| file_type.is_file())
        {
            allowed_files.insert(path.to_path_buf());
        }
    }

    maya_common::create_zip_archive(project_root, project_root, |path| {
        allowed_files.contains(path)
    })
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::fs;
    use tempfile::tempdir;

    #[test]
    fn requires_gitignore() {
        let root = tempdir().unwrap();
        assert!(matches!(
            pack_with_gitignore(root.path()),
            Err(Error::ConfigNotFound { .. })
        ));
    }

    #[test]
    fn creates_archive_and_excludes_ignored_file() {
        let root = tempdir().unwrap();
        fs::write(root.path().join(".gitignore"), "ignored.txt\n").unwrap();
        fs::write(root.path().join("kept.txt"), "kept").unwrap();
        fs::write(root.path().join("ignored.txt"), "ignored").unwrap();

        let report = pack_with_gitignore(root.path()).unwrap();

        assert!(report.archive_path.is_file());
        assert_eq!(report.files_added, 2);
    }
}
