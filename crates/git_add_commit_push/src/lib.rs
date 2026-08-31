use maya_common::error::{Error, Result};
use std::path::Path;
use std::process::Command;

const STDERR_TAIL_LIMIT: usize = 8 * 1024;

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum GitOutcome {
    CommittedAndPushed { commit_summary: String },
    NothingToCommit,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ProcessOutput {
    pub success: bool,
    pub status: Option<i32>,
    pub stdout: Vec<u8>,
    pub stderr: Vec<u8>,
}

/// 外部进程边界，允许 Git 工作流在不启动真实进程时进行契约测试。
pub trait ProcessRunner {
    fn run(&self, program: &str, args: &[String], cwd: &Path) -> Result<ProcessOutput>;
}

#[derive(Debug, Default, Clone, Copy)]
pub struct SystemProcessRunner;

impl ProcessRunner for SystemProcessRunner {
    fn run(&self, program: &str, args: &[String], cwd: &Path) -> Result<ProcessOutput> {
        let output = Command::new(program)
            .args(args)
            .current_dir(cwd)
            .output()
            .map_err(|error| {
                Error::command_failed(
                    program,
                    args.to_vec(),
                    cwd,
                    None,
                    format!("无法启动进程: {error}"),
                )
            })?;
        Ok(ProcessOutput {
            success: output.status.success(),
            status: output.status.code(),
            stdout: output.stdout,
            stderr: output.stderr,
        })
    }
}

/// 在指定仓库执行 add、commit 和 push；不依赖 Git 本地化输出判断无变更。
pub fn git_add_commit_push(path: &Path, message: &str) -> Result<GitOutcome> {
    git_add_commit_push_with_runner(path, message, &SystemProcessRunner)
}

pub fn git_add_commit_push_with_runner<R: ProcessRunner>(
    path: &Path,
    message: &str,
    runner: &R,
) -> Result<GitOutcome> {
    run_checked(runner, path, &["add", "."])?;

    let diff_args = strings(&["diff", "--cached", "--quiet", "--exit-code"]);
    let diff = runner.run("git", &diff_args, path)?;
    match (diff.success, diff.status) {
        (true, _) => return Ok(GitOutcome::NothingToCommit),
        (false, Some(1)) => {}
        _ => return Err(command_error(path, diff_args, diff)),
    }

    let commit_args = vec!["commit".to_string(), "-m".to_string(), message.to_string()];
    let commit = runner.run("git", &commit_args, path)?;
    if !commit.success {
        return Err(command_error(path, commit_args, commit));
    }
    let commit_summary = String::from_utf8_lossy(&commit.stdout).trim().to_string();

    run_checked(runner, path, &["push"])?;
    Ok(GitOutcome::CommittedAndPushed { commit_summary })
}

fn run_checked<R: ProcessRunner>(runner: &R, cwd: &Path, args: &[&str]) -> Result<ProcessOutput> {
    let args = strings(args);
    let output = runner.run("git", &args, cwd)?;
    if output.success {
        Ok(output)
    } else {
        Err(command_error(cwd, args, output))
    }
}

fn strings(values: &[&str]) -> Vec<String> {
    values.iter().map(|value| (*value).to_string()).collect()
}

fn command_error(cwd: &Path, args: Vec<String>, output: ProcessOutput) -> Error {
    Error::command_failed(
        "git",
        args,
        cwd,
        output.status,
        bounded_tail(&output.stderr, STDERR_TAIL_LIMIT),
    )
}

fn bounded_tail(bytes: &[u8], limit: usize) -> String {
    let text = String::from_utf8_lossy(bytes);
    if text.len() <= limit {
        return text.trim().to_string();
    }
    let mut start = text.len() - limit;
    while !text.is_char_boundary(start) {
        start += 1;
    }
    text[start..].trim().to_string()
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::collections::VecDeque;
    use std::sync::Mutex;

    struct FakeRunner {
        outputs: Mutex<VecDeque<ProcessOutput>>,
        calls: Mutex<Vec<Vec<String>>>,
    }

    impl FakeRunner {
        fn new(outputs: Vec<ProcessOutput>) -> Self {
            Self {
                outputs: Mutex::new(outputs.into()),
                calls: Mutex::new(Vec::new()),
            }
        }
    }

    impl ProcessRunner for FakeRunner {
        fn run(&self, _program: &str, args: &[String], _cwd: &Path) -> Result<ProcessOutput> {
            self.calls.lock().unwrap().push(args.to_vec());
            Ok(self.outputs.lock().unwrap().pop_front().unwrap())
        }
    }

    fn output(success: bool, status: i32, stdout: &str, stderr: &str) -> ProcessOutput {
        ProcessOutput {
            success,
            status: Some(status),
            stdout: stdout.as_bytes().to_vec(),
            stderr: stderr.as_bytes().to_vec(),
        }
    }

    #[test]
    fn detects_no_changes_from_diff_exit_status() {
        let runner = FakeRunner::new(vec![
            output(true, 0, "", ""),
            output(true, 0, "", "任何语言的输出都不影响判断"),
        ]);

        let result = git_add_commit_push_with_runner(Path::new("."), "message", &runner).unwrap();

        assert_eq!(result, GitOutcome::NothingToCommit);
        assert_eq!(runner.calls.lock().unwrap().len(), 2);
    }

    #[test]
    fn commits_and_pushes_when_diff_returns_one() {
        let runner = FakeRunner::new(vec![
            output(true, 0, "", ""),
            output(false, 1, "", ""),
            output(true, 0, "commit summary", ""),
            output(true, 0, "", ""),
        ]);

        let result = git_add_commit_push_with_runner(Path::new("."), "message", &runner).unwrap();

        assert_eq!(
            result,
            GitOutcome::CommittedAndPushed {
                commit_summary: "commit summary".to_string()
            }
        );
        assert_eq!(runner.calls.lock().unwrap().len(), 4);
    }

    #[test]
    fn unexpected_diff_status_is_a_structured_command_error() {
        let runner = FakeRunner::new(vec![
            output(true, 0, "", ""),
            output(false, 128, "", "repository error"),
        ]);

        let error =
            git_add_commit_push_with_runner(Path::new("."), "message", &runner).unwrap_err();

        assert!(matches!(
            error,
            Error::CommandFailed {
                status: Some(128),
                ..
            }
        ));
    }
}
