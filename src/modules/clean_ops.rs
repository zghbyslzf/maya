use crate::cli::CleanTarget;
use crate::presenter::Presenter;
use maya_core::Result;
use std::path::Path;

pub fn handle_clean_ops(
    clean_types: &[CleanTarget],
    path: &Path,
    presenter: &Presenter,
) -> Result<()> {
    for clean_type in clean_types {
        match clean_type {
            CleanTarget::NodeModules => {
                let report = maya_fs::clear_node_modules(path)?;
                presenter.removal(" node_modules 文件夹", &report);
            }
            CleanTarget::LockFiles => {
                let report = maya_fs::clear_lock_files(path)?;
                presenter.removal("锁文件", &report);
            }
        }
    }
    Ok(())
}
