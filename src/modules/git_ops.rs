use maya_common::error::{Error, Result};
use std::path::Path;

/// 处理Git操作的模块
pub fn handle_git_ops(git_ops: &[String], path: &Path, message: &str) -> Result<()> {
    if git_ops.len() == 1 && (git_ops[0] == "m" || git_ops[0] == "M") {
        git_add_commit_push::git_add_commit_push(path, message)?;
        println!("已完成 git add/commit/push 操作");
    } else {
        return Err(Error::invalid_argument(format!(
            "暂不支持的 git 操作参数: {:?}",
            git_ops
        )));
    }
    Ok(())
}
