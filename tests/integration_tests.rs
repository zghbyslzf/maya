use assert_cmd::Command;
use predicates::prelude::*;

#[test]
fn test_help() {
    let mut cmd = Command::cargo_bin("maya").unwrap();
    cmd.arg("--help");
    cmd.assert()
        .success()
        .stdout(predicate::str::contains("Maya CLI 工具集"));
}

#[test]
fn test_version() {
    let mut cmd = Command::cargo_bin("maya").unwrap();
    cmd.arg("--version");
    cmd.assert()
        .success()
        .stdout(predicate::str::contains("maya"));
}

#[test]
fn test_clean_subcommand_help() {
    let mut cmd = Command::cargo_bin("maya").unwrap();
    cmd.arg("clean").arg("--help");
    cmd.assert()
        .success()
        .stdout(predicate::str::contains("清理操作"));
}

#[test]
fn test_optimize_subcommand_help() {
    let mut cmd = Command::cargo_bin("maya").unwrap();
    cmd.arg("optimize").arg("--help");
    cmd.assert()
        .success()
        .stdout(predicate::str::contains("图片压缩操作"));
}

#[test]
fn test_git_subcommand_help() {
    let mut cmd = Command::cargo_bin("maya").unwrap();
    cmd.arg("git").arg("--help");
    cmd.assert()
        .success()
        .stdout(predicate::str::contains("Git相关操作"));
}

#[test]
fn test_pack_subcommand_help() {
    let mut cmd = Command::cargo_bin("maya").unwrap();
    cmd.arg("pack").arg("--help");
    cmd.assert()
        .success()
        .stdout(predicate::str::contains("打包操作"));
}

#[test]
fn test_transform_subcommand_help() {
    let mut cmd = Command::cargo_bin("maya").unwrap();
    cmd.arg("transform").arg("--help");
    cmd.assert()
        .success()
        .stdout(predicate::str::contains("视频转换操作"));
}

// 测试无效子命令
#[test]
fn test_invalid_subcommand() {
    let mut cmd = Command::cargo_bin("maya").unwrap();
    cmd.arg("invalid");
    cmd.assert()
        .failure()
        .stderr(predicate::str::contains("unrecognized subcommand"));
}

// 测试 clean 子命令缺少必需参数
#[test]
fn test_clean_missing_required_args() {
    let mut cmd = Command::cargo_bin("maya").unwrap();
    cmd.arg("clean");
    cmd.assert()
        .failure()
        .stderr(predicate::str::contains("required"));
}

// 测试 optimize 子命令缺少必需参数
#[test]
fn test_optimize_missing_required_args() {
    let mut cmd = Command::cargo_bin("maya").unwrap();
    cmd.arg("optimize");
    cmd.assert()
        .failure()
        .stderr(predicate::str::contains("required"));
}

// 测试 clean 子命令在实际目录中执行（无 node_modules 目录）
#[test]
fn test_clean_execution_no_node_modules() {
    use tempfile::tempdir;

    let temp_dir = tempdir().unwrap();
    let temp_path = temp_dir.path().to_str().unwrap();

    let mut cmd = Command::cargo_bin("maya").unwrap();
    cmd.arg("clean")
        .arg(temp_path)
        .arg("--types")
        .arg("n");
    cmd.assert()
        .success()
        .stdout(predicate::str::contains("已清理 0 个 node_modules 文件夹"));
}

// 测试 clean 子命令无效类型
#[test]
fn test_clean_invalid_type() {
    let mut cmd = Command::cargo_bin("maya").unwrap();
    cmd.arg("clean")
        .arg("--types")
        .arg("invalid")
        .arg(".");
    cmd.assert()
        .failure()
        .stderr(predicate::str::contains("不支持的清理类型"));
}