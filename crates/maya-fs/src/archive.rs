use crate::atomic_write;
use ignore::WalkBuilder;
use maya_core::{ArchiveReport, Error, Result};
use regex::Regex;
use std::fs;
use std::io::{Read, Write};
use std::path::{Component, Path, PathBuf};
use std::sync::LazyLock;
use walkdir::WalkDir;
use zip::write::{FileOptions, ZipWriter};

static STATIC_OUT_DIR_RE: LazyLock<Regex> = LazyLock::new(|| {
    Regex::new(r#"outDir\s*:\s*[\"']([^\"']+)[\"']"#).expect("STATIC_OUT_DIR_RE 正则编译失败")
});
static OUT_DIR_KEY_RE: LazyLock<Regex> =
    LazyLock::new(|| Regex::new(r"outDir\s*:").expect("OUT_DIR_KEY_RE 正则编译失败"));

#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum EmptyDirectoryPolicy {
    #[default]
    Omit,
    Preserve,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum SymlinkPolicy {
    #[default]
    Skip,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum MetadataPolicy {
    #[default]
    Omit,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub struct ArchiveOptions {
    pub empty_directories: EmptyDirectoryPolicy,
    pub symlinks: SymlinkPolicy,
    pub metadata: MetadataPolicy,
}

#[derive(Debug, Clone, Default)]
pub struct VitePackOptions {
    pub out_dir: Option<PathBuf>,
    pub archive: ArchiveOptions,
}

#[derive(Debug, Clone, PartialEq, Eq)]
enum ArchiveEntryKind {
    File,
    Directory,
}

#[derive(Debug, Clone, PartialEq, Eq)]
struct ArchiveEntry {
    source: PathBuf,
    name: String,
    kind: ArchiveEntryKind,
}

pub fn pack_with_gitignore(project_root: &Path) -> Result<ArchiveReport> {
    pack_with_gitignore_options(project_root, &ArchiveOptions::default())
}

pub fn pack_with_gitignore_options(
    project_root: &Path,
    options: &ArchiveOptions,
) -> Result<ArchiveReport> {
    if !project_root.join(".gitignore").is_file() {
        return Err(Error::config_not_found(project_root, [".gitignore"]));
    }

    let archive_path = archive_path_for(project_root, project_root);
    let walker = WalkBuilder::new(project_root)
        .hidden(false)
        .git_global(false)
        .git_ignore(true)
        .require_git(false)
        .build();
    let mut entries = Vec::new();
    for entry in walker {
        let entry = entry.map_err(|error| Error::traversal(None, error))?;
        let path = entry.path();
        if path == project_root || path == archive_path || contains_git_component(path) {
            continue;
        }
        let Some(file_type) = entry.file_type() else {
            continue;
        };
        if file_type.is_symlink() {
            continue;
        }
        if file_type.is_file() {
            entries.push(archive_entry(project_root, path, ArchiveEntryKind::File)?);
        } else if file_type.is_dir()
            && options.empty_directories == EmptyDirectoryPolicy::Preserve
            && directory_is_empty(path)?
        {
            entries.push(archive_entry(
                project_root,
                path,
                ArchiveEntryKind::Directory,
            )?);
        }
    }
    write_archive_plan(&archive_path, entries, options)
}

pub fn pack_vite(project_root: &Path, options: &VitePackOptions) -> Result<ArchiveReport> {
    let configured_dir = match &options.out_dir {
        Some(out_dir) => out_dir.clone(),
        None => {
            let config = find_vite_config(project_root)?;
            parse_out_dir(&config)?.unwrap_or_else(|| PathBuf::from("dist"))
        }
    };
    let output_dir = if configured_dir.is_absolute() {
        configured_dir
    } else {
        project_root.join(configured_dir)
    };
    if !output_dir.is_dir() {
        return Err(Error::path(format!(
            "Vite 输出目录不存在或不是目录: {}",
            output_dir.display()
        )));
    }
    let archive_path = archive_path_for(&output_dir, project_root);
    let entries = plan_walkdir_archive(&output_dir, &archive_path, &options.archive)?;
    write_archive_plan(&archive_path, entries, &options.archive)
}

fn archive_path_for(source_dir: &Path, destination: &Path) -> PathBuf {
    let folder_name = source_dir
        .file_name()
        .and_then(|name| name.to_str())
        .unwrap_or("output");
    destination.join(format!("{folder_name}.zip"))
}

fn contains_git_component(path: &Path) -> bool {
    path.components()
        .any(|component| component == Component::Normal(".git".as_ref()))
}

fn plan_walkdir_archive(
    source_dir: &Path,
    archive_path: &Path,
    options: &ArchiveOptions,
) -> Result<Vec<ArchiveEntry>> {
    let mut entries = Vec::new();
    for entry in WalkDir::new(source_dir).follow_links(false) {
        let entry =
            entry.map_err(|error| Error::traversal(error.path().map(Path::to_path_buf), error))?;
        let path = entry.path();
        if path == source_dir || path == archive_path || entry.file_type().is_symlink() {
            continue;
        }
        if entry.file_type().is_file() {
            entries.push(archive_entry(source_dir, path, ArchiveEntryKind::File)?);
        } else if entry.file_type().is_dir()
            && options.empty_directories == EmptyDirectoryPolicy::Preserve
            && directory_is_empty(path)?
        {
            entries.push(archive_entry(
                source_dir,
                path,
                ArchiveEntryKind::Directory,
            )?);
        }
    }
    Ok(entries)
}

fn archive_entry(root: &Path, path: &Path, kind: ArchiveEntryKind) -> Result<ArchiveEntry> {
    let relative = path.strip_prefix(root).map_err(|error| {
        Error::path(format!("无法计算归档相对路径 {}: {error}", path.display()))
    })?;
    let name = relative
        .to_str()
        .ok_or_else(|| Error::path(format!("归档路径不是有效 UTF-8: {}", relative.display())))?
        .replace('\\', "/");
    Ok(ArchiveEntry {
        source: path.to_path_buf(),
        name,
        kind,
    })
}

fn directory_is_empty(path: &Path) -> Result<bool> {
    let mut entries =
        fs::read_dir(path).map_err(|error| Error::io_context("读取归档目录", path, error))?;
    Ok(entries.next().transpose()?.is_none())
}

fn write_archive_plan(
    archive_path: &Path,
    entries: Vec<ArchiveEntry>,
    options: &ArchiveOptions,
) -> Result<ArchiveReport> {
    let mut files_added = 0usize;
    let mut directories_added = 0usize;
    let mut source_bytes = 0u64;

    atomic_write(archive_path, |file, _temporary_path| {
        let mut zip = ZipWriter::new(file);
        let zip_options: FileOptions<'_, ()> =
            FileOptions::default().compression_method(zip::CompressionMethod::Deflated);
        for entry in &entries {
            match entry.kind {
                ArchiveEntryKind::File => {
                    let metadata = fs::symlink_metadata(&entry.source).map_err(|error| {
                        Error::io_context("读取归档源文件元数据", &entry.source, error)
                    })?;
                    if metadata.file_type().is_symlink() {
                        continue;
                    }
                    zip.start_file(&entry.name, zip_options)
                        .map_err(|error| Error::other(format!("创建 ZIP 条目失败: {error}")))?;
                    let mut input = fs::File::open(&entry.source).map_err(|error| {
                        Error::io_context("打开归档源文件", &entry.source, error)
                    })?;
                    source_bytes += metadata.len();
                    files_added += 1;
                    let mut buffer = [0u8; 8192];
                    loop {
                        let read = input.read(&mut buffer)?;
                        if read == 0 {
                            break;
                        }
                        zip.write_all(&buffer[..read])?;
                    }
                }
                ArchiveEntryKind::Directory
                    if options.empty_directories == EmptyDirectoryPolicy::Preserve =>
                {
                    zip.add_directory(
                        format!("{}/", entry.name.trim_end_matches('/')),
                        zip_options,
                    )
                    .map_err(|error| Error::other(format!("创建 ZIP 目录条目失败: {error}")))?;
                    directories_added += 1;
                }
                ArchiveEntryKind::Directory => {}
            }
        }
        zip.finish()
            .map_err(|error| Error::other(format!("完成 ZIP 归档失败: {error}")))?;
        Ok(())
    })?;

    Ok(ArchiveReport {
        archive_bytes: fs::metadata(archive_path)?.len(),
        archive_path: archive_path.to_path_buf(),
        files_added,
        directories_added,
        source_bytes,
    })
}

fn find_vite_config(project_root: &Path) -> Result<PathBuf> {
    ["vite.config.js", "vite.config.ts"]
        .into_iter()
        .map(|name| project_root.join(name))
        .find(|path| path.is_file())
        .ok_or_else(|| Error::config_not_found(project_root, ["vite.config.js", "vite.config.ts"]))
}

fn parse_out_dir(config_path: &Path) -> Result<Option<PathBuf>> {
    let content = fs::read_to_string(config_path)
        .map_err(|error| Error::io_context("读取 Vite 配置", config_path, error))?;
    if let Some(captures) = STATIC_OUT_DIR_RE.captures(&content) {
        let value = captures
            .get(1)
            .ok_or_else(|| Error::config("无法读取 Vite build.outDir 静态值"))?
            .as_str()
            .trim();
        if value.is_empty() {
            return Err(Error::config("Vite build.outDir 不能为空"));
        }
        return Ok(Some(PathBuf::from(value)));
    }
    if OUT_DIR_KEY_RE.is_match(&content) {
        return Err(Error::unsupported_config(
            config_path,
            "build.outDir 必须是静态字符串；可使用 --out-dir 覆盖",
        ));
    }
    Ok(None)
}

#[cfg(test)]
mod tests {
    use super::*;
    use tempfile::tempdir;
    use zip::ZipArchive;

    fn archive_names(path: &Path) -> Vec<String> {
        let file = fs::File::open(path).unwrap();
        let mut archive = ZipArchive::new(file).unwrap();
        (0..archive.len())
            .map(|index| archive.by_index(index).unwrap().name().to_string())
            .collect()
    }

    #[test]
    fn gitignore_pack_requires_config_and_excludes_ignored_files() {
        let root = tempdir().unwrap();
        assert!(matches!(
            pack_with_gitignore(root.path()),
            Err(Error::ConfigNotFound { .. })
        ));
        fs::write(root.path().join(".gitignore"), "ignored.txt\n").unwrap();
        fs::write(root.path().join("kept.txt"), "kept").unwrap();
        fs::write(root.path().join("ignored.txt"), "ignored").unwrap();

        let report = pack_with_gitignore(root.path()).unwrap();
        assert_eq!(report.files_added, 2);
        assert_eq!(report.directories_added, 0);
    }

    #[test]
    fn archive_policy_can_preserve_empty_directories_and_special_paths() {
        let root = tempdir().unwrap();
        fs::write(root.path().join(".gitignore"), "").unwrap();
        fs::create_dir(root.path().join("empty folder")).unwrap();
        fs::create_dir(root.path().join("资料")).unwrap();
        fs::write(root.path().join("资料/文件 name.txt"), "content").unwrap();
        let options = ArchiveOptions {
            empty_directories: EmptyDirectoryPolicy::Preserve,
            ..ArchiveOptions::default()
        };

        let report = pack_with_gitignore_options(root.path(), &options).unwrap();
        let names = archive_names(&report.archive_path);

        assert_eq!(report.directories_added, 1);
        assert!(names.contains(&"empty folder/".to_string()));
        assert!(names.contains(&"资料/文件 name.txt".to_string()));
    }

    #[test]
    fn vite_pack_supports_static_and_explicit_output_directories() {
        let root = tempdir().unwrap();
        fs::write(
            root.path().join("vite.config.ts"),
            "export default { build: { outDir: 'build-output' } }",
        )
        .unwrap();
        fs::create_dir(root.path().join("build-output")).unwrap();
        fs::write(root.path().join("build-output/index.js"), "code").unwrap();
        assert!(pack_vite(root.path(), &VitePackOptions::default()).is_ok());

        fs::create_dir(root.path().join("custom")).unwrap();
        let options = VitePackOptions {
            out_dir: Some(PathBuf::from("custom")),
            ..VitePackOptions::default()
        };
        assert!(pack_vite(root.path(), &options).is_ok());
    }

    #[test]
    fn vite_pack_rejects_dynamic_config() {
        let root = tempdir().unwrap();
        fs::write(
            root.path().join("vite.config.js"),
            "export default { build: { outDir: process.env.OUT_DIR } }",
        )
        .unwrap();
        assert!(matches!(
            pack_vite(root.path(), &VitePackOptions::default()),
            Err(Error::UnsupportedConfig { .. })
        ));
    }

    #[test]
    fn vite_pack_reports_missing_config_and_output_directory() {
        let root = tempdir().unwrap();
        assert!(matches!(
            pack_vite(root.path(), &VitePackOptions::default()),
            Err(Error::ConfigNotFound { .. })
        ));

        fs::write(root.path().join("vite.config.js"), "export default {}").unwrap();
        assert!(matches!(
            pack_vite(root.path(), &VitePackOptions::default()),
            Err(Error::Path(_))
        ));
    }

    #[test]
    fn failed_archive_plan_does_not_replace_existing_archive() {
        let root = tempdir().unwrap();
        let source = root.path().join("source");
        fs::create_dir(&source).unwrap();
        let source_file = source.join("data.txt");
        fs::write(&source_file, "data").unwrap();
        let archive = root.path().join("source.zip");
        fs::write(&archive, "old").unwrap();
        let entries = plan_walkdir_archive(&source, &archive, &ArchiveOptions::default()).unwrap();
        fs::remove_file(source_file).unwrap();

        let result = write_archive_plan(&archive, entries, &ArchiveOptions::default());

        assert!(result.is_err());
        assert_eq!(fs::read(&archive).unwrap(), b"old");
    }
}
