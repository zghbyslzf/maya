use crate::atomic_write;
use ignore::WalkBuilder;
use maya_core::{ArchiveReport, Error, Result};
use regex::Regex;
use std::collections::HashSet;
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

#[derive(Debug, Clone, Default)]
pub struct VitePackOptions {
    pub out_dir: Option<PathBuf>,
}

pub fn pack_with_gitignore(project_root: &Path) -> Result<ArchiveReport> {
    if !project_root.join(".gitignore").is_file() {
        return Err(Error::config_not_found(project_root, [".gitignore"]));
    }

    let walker = WalkBuilder::new(project_root)
        .hidden(false)
        .git_global(false)
        .git_ignore(true)
        .require_git(false)
        .build();
    let mut files = HashSet::new();
    for entry in walker {
        let entry = entry.map_err(|error| Error::traversal(None, error))?;
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
            files.insert(path.to_path_buf());
        }
    }
    create_zip_archive(project_root, project_root, |path| files.contains(path))
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
    create_zip_archive(&output_dir, project_root, Path::is_file)
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

fn create_zip_archive<F>(source_dir: &Path, destination: &Path, filter: F) -> Result<ArchiveReport>
where
    F: Fn(&Path) -> bool,
{
    if !source_dir.is_dir() || !destination.is_dir() {
        return Err(Error::path("归档源路径和目标路径都必须是目录"));
    }
    let folder_name = source_dir
        .file_name()
        .and_then(|name| name.to_str())
        .unwrap_or("output");
    let archive_path = destination.join(format!("{folder_name}.zip"));
    let mut files_added = 0usize;
    let mut source_bytes = 0u64;

    atomic_write(&archive_path, |file, temporary_path| {
        let mut zip = ZipWriter::new(file);
        let options: FileOptions<'_, ()> =
            FileOptions::default().compression_method(zip::CompressionMethod::Deflated);
        for entry in WalkDir::new(source_dir) {
            let entry = entry
                .map_err(|error| Error::traversal(error.path().map(Path::to_path_buf), error))?;
            let path = entry.path();
            if path == archive_path || path == temporary_path || !filter(path) {
                continue;
            }
            if entry.file_type().is_file() {
                let relative = path.strip_prefix(source_dir).map_err(|error| {
                    Error::path(format!("无法计算归档相对路径 {}: {error}", path.display()))
                })?;
                let name = relative.to_str().ok_or_else(|| {
                    Error::path(format!("归档路径不是有效 UTF-8: {}", relative.display()))
                })?;
                zip.start_file(name.replace('\\', "/"), options)
                    .map_err(|error| Error::other(format!("创建 ZIP 条目失败: {error}")))?;
                let mut input = fs::File::open(path)
                    .map_err(|error| Error::io_context("打开归档源文件", path, error))?;
                source_bytes += entry
                    .metadata()
                    .map_err(|error| Error::other(format!("读取归档元数据失败: {error}")))?
                    .len();
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
        }
        zip.finish()
            .map_err(|error| Error::other(format!("完成 ZIP 归档失败: {error}")))?;
        Ok(())
    })?;

    Ok(ArchiveReport {
        archive_bytes: fs::metadata(&archive_path)?.len(),
        archive_path,
        files_added,
        source_bytes,
    })
}

#[cfg(test)]
mod tests {
    use super::*;
    use tempfile::tempdir;

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
    fn failed_zip_does_not_replace_existing_archive() {
        let root = tempdir().unwrap();
        let source = root.path().join("source");
        fs::create_dir(&source).unwrap();
        let source_file = source.join("data.txt");
        fs::write(&source_file, "data").unwrap();
        let archive = root.path().join("source.zip");
        fs::write(&archive, "old").unwrap();

        let result = create_zip_archive(&source, root.path(), |path| {
            if path == source_file {
                fs::remove_file(path).unwrap();
                true
            } else {
                false
            }
        });

        assert!(result.is_err());
        assert_eq!(fs::read(&archive).unwrap(), b"old");
    }
}
