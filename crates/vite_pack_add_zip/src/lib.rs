use maya_common::error::{Error, Result};
use maya_common::ArchiveReport;
use regex::Regex;
use std::fs;
use std::path::{Path, PathBuf};
use std::sync::LazyLock;

static STATIC_OUT_DIR_RE: LazyLock<Regex> = LazyLock::new(|| {
    Regex::new(r#"outDir\s*:\s*[\"']([^\"']+)[\"']"#).expect("STATIC_OUT_DIR_RE 正则编译失败")
});
static OUT_DIR_KEY_RE: LazyLock<Regex> =
    LazyLock::new(|| Regex::new(r"outDir\s*:").expect("OUT_DIR_KEY_RE 正则编译失败"));

#[derive(Debug, Clone, Default)]
pub struct VitePackOptions {
    /// 显式指定构建输出目录；相对路径以项目根目录为基准。
    pub out_dir: Option<PathBuf>,
}

/// 打包 Vite 构建目录。仅解析静态字符串形式的 `build.outDir`。
pub fn pack_vite(project_root: &Path, options: &VitePackOptions) -> Result<ArchiveReport> {
    let out_dir = match &options.out_dir {
        Some(out_dir) => out_dir.clone(),
        None => {
            let config_path = find_vite_config(project_root)?;
            parse_out_dir(&config_path)?.unwrap_or_else(|| PathBuf::from("dist"))
        }
    };
    let output_dir = if out_dir.is_absolute() {
        out_dir
    } else {
        project_root.join(out_dir)
    };

    if !output_dir.is_dir() {
        return Err(Error::path(format!(
            "Vite 输出目录不存在或不是目录: {}",
            output_dir.display()
        )));
    }

    maya_common::create_zip_archive(&output_dir, project_root, Path::is_file)
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
            "build.outDir 必须是单引号或双引号包围的静态字符串；可使用 --out-dir 覆盖",
        ));
    }

    Ok(None)
}

#[cfg(test)]
mod tests {
    use super::*;
    use tempfile::tempdir;

    #[test]
    fn packs_static_out_dir_from_typescript_config() {
        let root = tempdir().unwrap();
        fs::write(
            root.path().join("vite.config.ts"),
            "export default { build: { outDir: 'build-output' } }",
        )
        .unwrap();
        fs::create_dir(root.path().join("build-output")).unwrap();
        fs::write(root.path().join("build-output/index.js"), "code").unwrap();

        let report = pack_vite(root.path(), &VitePackOptions::default()).unwrap();

        assert_eq!(report.archive_path, root.path().join("build-output.zip"));
        assert_eq!(report.files_added, 1);
    }

    #[test]
    fn missing_config_is_an_error() {
        let root = tempdir().unwrap();
        assert!(matches!(
            pack_vite(root.path(), &VitePackOptions::default()),
            Err(Error::ConfigNotFound { .. })
        ));
    }

    #[test]
    fn explicit_out_dir_does_not_require_config() {
        let root = tempdir().unwrap();
        fs::create_dir(root.path().join("custom")).unwrap();
        fs::write(root.path().join("custom/index.js"), "code").unwrap();
        let options = VitePackOptions {
            out_dir: Some(PathBuf::from("custom")),
        };

        assert!(pack_vite(root.path(), &options).is_ok());
    }

    #[test]
    fn rejects_dynamic_out_dir() {
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
    fn missing_output_directory_is_an_error() {
        let root = tempdir().unwrap();
        fs::write(root.path().join("vite.config.js"), "export default {}").unwrap();

        assert!(matches!(
            pack_vite(root.path(), &VitePackOptions::default()),
            Err(Error::Path(_))
        ));
    }
}
