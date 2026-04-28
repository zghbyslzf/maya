use maya_common::error::{Error, Result};
use std::path::Path;
use std::process::{Command, Stdio};

/// 在指定目录依次执行 git add .、git commit、git push
pub fn git_add_commit_push(path: impl AsRef<Path>, message: &str) -> Result<()> {
    let path = path.as_ref();

    let add_status = Command::new("git")
        .arg("add")
        .arg(".")
        .current_dir(path)
        .stdout(Stdio::inherit())
        .stderr(Stdio::inherit())
        .status()?;
    if !add_status.success() {
        return Err(Error::command_execution("git add 失败"));
    }

    let commit_output = Command::new("git")
        .arg("commit")
        .arg("-m")
        .arg(message)
        .current_dir(path)
        .output()?;
    if !commit_output.status.success() {
        let stderr = String::from_utf8_lossy(&commit_output.stderr);
        if stderr.contains("nothing to commit") {
            println!("没有变更，无需提交");
            return Ok(());
        } else {
            eprintln!("{}", stderr);
            return Err(Error::git("git commit 失败"));
        }
    } else {
        let stdout = String::from_utf8_lossy(&commit_output.stdout);
        if !stdout.trim().is_empty() {
            println!("{}", stdout.trim());
        }
    }

    let push_status = Command::new("git")
        .arg("push")
        .current_dir(path)
        .stdout(Stdio::inherit())
        .stderr(Stdio::inherit())
        .status()?;
    if !push_status.success() {
        return Err(Error::command_execution("git push 失败"));
    }
    Ok(())
}
