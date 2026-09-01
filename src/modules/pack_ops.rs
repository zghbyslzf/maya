use crate::cli::PackMode;
use crate::presenter::Presenter;
use maya_core::{Error, Result};
use maya_fs::VitePackOptions;
use std::path::{Path, PathBuf};

pub fn handle_pack_ops(
    pack_type: PackMode,
    path: &Path,
    out_dir: Option<PathBuf>,
    presenter: &Presenter,
) -> Result<()> {
    let report = match pack_type {
        PackMode::Gitignore => {
            if out_dir.is_some() {
                return Err(Error::invalid_argument(
                    "--out-dir 只能与 Vite 打包方式 a/vite 一起使用",
                ));
            }
            maya_fs::pack_with_gitignore(path)?
        }
        PackMode::Vite => maya_fs::pack_vite(
            path,
            &VitePackOptions {
                out_dir,
                ..VitePackOptions::default()
            },
        )?,
    };
    presenter.archive(&report);
    Ok(())
}
