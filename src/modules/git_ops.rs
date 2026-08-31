use crate::cli::GitOperation;
use crate::presenter::Presenter;
use maya_common::error::Result;
use std::path::Path;

pub fn handle_git_ops(
    operation: GitOperation,
    path: &Path,
    message: &str,
    presenter: &Presenter,
) -> Result<()> {
    match operation {
        GitOperation::AddCommitPush => {
            let outcome = git_add_commit_push::git_add_commit_push(path, message)?;
            presenter.git(&outcome);
        }
    }
    Ok(())
}
