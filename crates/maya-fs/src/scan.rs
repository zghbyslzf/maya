use maya_core::{Error, Result};
use std::path::{Path, PathBuf};
use walkdir::WalkDir;

pub fn find_files<F>(dir: &Path, filter: F) -> Result<Vec<PathBuf>>
where
    F: Fn(&Path) -> bool,
{
    if !dir.is_dir() {
        return Err(Error::path(format!("路径不是目录: {}", dir.display())));
    }

    let mut files = Vec::new();
    for entry in WalkDir::new(dir).sort_by_file_name() {
        let entry =
            entry.map_err(|error| Error::traversal(error.path().map(Path::to_path_buf), error))?;
        if entry.file_type().is_file() && filter(entry.path()) {
            files.push(entry.into_path());
        }
    }
    Ok(files)
}

pub fn find_files_by_extension(dir: &Path, extensions: &[&str]) -> Result<Vec<PathBuf>> {
    find_files(dir, |path| {
        path.extension()
            .and_then(|extension| extension.to_str())
            .is_some_and(|extension| {
                extensions
                    .iter()
                    .any(|expected| expected.eq_ignore_ascii_case(extension))
            })
    })
}

pub fn find_directories_by_name_pruned(dir: &Path, name: &str) -> Result<Vec<PathBuf>> {
    if !dir.is_dir() {
        return Err(Error::path(format!("路径不是目录: {}", dir.display())));
    }

    let mut results = Vec::new();
    let mut entries = WalkDir::new(dir).into_iter();
    while let Some(entry) = entries.next() {
        let entry =
            entry.map_err(|error| Error::traversal(error.path().map(Path::to_path_buf), error))?;
        let is_match = entry.file_type().is_dir()
            && entry
                .file_name()
                .to_str()
                .is_some_and(|entry_name| entry_name == name);
        if is_match {
            results.push(entry.path().to_path_buf());
            entries.skip_current_dir();
        }
    }
    Ok(results)
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::fs;
    use tempfile::tempdir;

    #[test]
    fn traversal_errors_are_not_silently_discarded() {
        let root = tempdir().unwrap();
        let trigger = root.path().join("a-trigger.txt");
        let doomed = root.path().join("z-doomed");
        fs::write(&trigger, b"trigger").unwrap();
        fs::create_dir(&doomed).unwrap();
        fs::write(doomed.join("data.txt"), b"data").unwrap();

        let result = find_files(root.path(), |path| {
            if path == trigger {
                fs::remove_dir_all(&doomed).unwrap();
            }
            true
        });

        assert!(matches!(result, Err(Error::Traversal { .. })));
    }

    #[test]
    fn pruned_search_omits_nested_matches() {
        let root = tempdir().unwrap();
        let outer = root.path().join("node_modules");
        let nested = outer.join("dependency/node_modules");
        fs::create_dir_all(&nested).unwrap();
        fs::create_dir_all(root.path().join("app/node_modules")).unwrap();

        let matches = find_directories_by_name_pruned(root.path(), "node_modules").unwrap();

        assert_eq!(matches.len(), 2);
        assert!(matches.contains(&outer));
        assert!(!matches.contains(&nested));
    }
}
