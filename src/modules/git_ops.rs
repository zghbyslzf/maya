use crate::cli::GitOperation;
use crate::presenter::Presenter;
use maya_core::Result;
use std::path::Path;

pub fn handle_git_ops(
    operation: GitOperation,
    path: &Path,
    message: &str,
    presenter: &Presenter,
) -> Result<()> {
    match operation {
        GitOperation::AddCommitPush => {
            let outcome = maya_git::git_add_commit_push(path, message)?;
            presenter.git(&outcome);
        }
    }
    Ok(())
}
