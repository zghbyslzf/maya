use std::path::Path;
use maya_common::error::{Error, Result};

/// 处理Git操作的模块
pub fn handle_git_ops(git_ops: &[String], path: &Path) -> Result<()> {
    if git_ops.len() == 1 && (git_ops[0] == "m" || git_ops[0] == "M") {
        git_add_commit_push::git_add_commit_push(path.to_string_lossy().to_string())?;
        println!("已完成 git add/commit/push 操作");
    } else {
        return Err(Error::invalid_argument(format!("暂不支持的 git 操作参数: {:?}", git_ops)));
    }
    Ok(())
}
