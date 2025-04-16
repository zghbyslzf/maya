use std::io::{self, Error, ErrorKind};
use std::process::{Command, Stdio};

/// 在指定目录依次执行 git add .、git commit -m "feat: update"、git push
pub fn git_add_commit_push(path: String) -> io::Result<()> {
    // git add .
    let add_status = Command::new("git")
        .arg("add")
        .arg(".")
        .current_dir(&path)
        .stdout(Stdio::inherit())
        .stderr(Stdio::inherit())
        .status()?;
    if !add_status.success() {
        return Err(Error::new(ErrorKind::Other, "git add 失败"));
    }

    // git commit -m "feat: update"
    let commit_status = Command::new("git")
        .arg("commit")
        .arg("-m")
        .arg("feat: update")
        .current_dir(&path)
        .stdout(Stdio::inherit())
        .stderr(Stdio::inherit())
        .status()?;
    if !commit_status.success() {
        // 如果没有变更，git commit 会返回非 0，可以忽略
        println!("git commit 可能没有变更，无需提交");
    }

    // git push
    let push_status = Command::new("git")
        .arg("push")
        .current_dir(&path)
        .stdout(Stdio::inherit())
        .stderr(Stdio::inherit())
        .status()?;
    if !push_status.success() {
        return Err(Error::new(ErrorKind::Other, "git push 失败"));
    }
    Ok(())
}
