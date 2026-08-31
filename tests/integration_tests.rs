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
    cmd.arg("clean").arg(temp_path).arg("--types").arg("n");
    cmd.assert()
        .success()
        .stdout(predicate::str::contains("已清理 0 个 node_modules 文件夹"));
}

// 测试 clean 子命令无效类型
#[test]
fn test_clean_invalid_type() {
    let mut cmd = Command::cargo_bin("maya").unwrap();
    cmd.arg("clean").arg("--types").arg("invalid").arg(".");
    cmd.assert()
        .failure()
        .code(2)
        .stderr(predicate::str::contains("invalid value 'invalid'"));
}

// 测试 clean lock 子命令实际删除锁文件
#[test]
fn test_clean_lock_files() {
    use std::fs::File;
    use std::io::Write;
    use tempfile::tempdir;

    let temp_dir = tempdir().unwrap();
    let temp_path = temp_dir.path();
    let lock_path = temp_path.join("package-lock.json");

    let mut file = File::create(&lock_path).unwrap();
    writeln!(file, "{{}}").unwrap();
    assert!(lock_path.exists());

    let mut cmd = Command::cargo_bin("maya").unwrap();
    cmd.arg("clean")
        .arg(temp_path.to_str().unwrap())
        .arg("--types")
        .arg("lock");
    cmd.assert()
        .success()
        .stdout(predicate::str::contains("已清理 1 个锁文件"));

    assert!(!lock_path.exists());
}

// 测试 clean node_modules 子命令实际删除目录
#[test]
fn test_clean_node_modules_in_temp_dir() {
    use std::fs;
    use tempfile::tempdir;

    let temp_dir = tempdir().unwrap();
    let temp_path = temp_dir.path();
    let nm_path = temp_path.join("node_modules");
    fs::create_dir(&nm_path).unwrap();
    assert!(nm_path.exists());

    let mut cmd = Command::cargo_bin("maya").unwrap();
    cmd.arg("clean")
        .arg(temp_path.to_str().unwrap())
        .arg("--types")
        .arg("n");
    cmd.assert()
        .success()
        .stdout(predicate::str::contains("已清理 1 个 node_modules 文件夹"));

    assert!(!nm_path.exists());
}

// 测试别名 c 等效于 clean
#[test]
fn test_clean_alias_works() {
    let mut cmd = Command::cargo_bin("maya").unwrap();
    cmd.arg("c").arg("--help");
    cmd.assert()
        .success()
        .stdout(predicate::str::contains("清理操作"));
}

// 测试别名 o 等效于 optimize
#[test]
fn test_optimize_alias_works() {
    let mut cmd = Command::cargo_bin("maya").unwrap();
    cmd.arg("o").arg("--help");
    cmd.assert()
        .success()
        .stdout(predicate::str::contains("图片压缩操作"));
}

// 测试 git 子命令缺少必需参数
#[test]
fn test_git_missing_required_args() {
    let mut cmd = Command::cargo_bin("maya").unwrap();
    cmd.arg("git");
    cmd.assert()
        .failure()
        .stderr(predicate::str::contains("required"));
}

// 测试优化别名
#[test]
fn test_optimize_alias_execution() {
    use tempfile::tempdir;

    let temp_dir = tempdir().unwrap();
    let temp_path = temp_dir.path().to_str().unwrap();

    let mut cmd = Command::cargo_bin("maya").unwrap();
    cmd.arg("o").arg(temp_path).arg("-t").arg("png");
    cmd.assert()
        .success()
        .stdout(predicate::str::contains("开始压缩"));
}

#[test]
fn quiet_suppresses_success_output() {
    use tempfile::tempdir;

    let temp_dir = tempdir().unwrap();
    let mut cmd = Command::cargo_bin("maya").unwrap();
    cmd.args(["--quiet", "clean"])
        .arg(temp_dir.path())
        .args(["--types", "n"]);

    cmd.assert().success().stdout(predicate::str::is_empty());
}

#[test]
fn no_progress_keeps_summary_but_hides_progress_events() {
    use tempfile::tempdir;

    let temp_dir = tempdir().unwrap();
    let mut cmd = Command::cargo_bin("maya").unwrap();
    cmd.args(["--no-progress", "optimize"])
        .arg(temp_dir.path())
        .args(["--types", "png"]);

    cmd.assert()
        .success()
        .stdout(predicate::str::contains("图片处理完成"))
        .stdout(predicate::str::contains("开始压缩图片").not());
}

#[test]
fn corrupt_image_returns_partial_failure_exit_code() {
    use std::fs;
    use tempfile::tempdir;

    let temp_dir = tempdir().unwrap();
    fs::write(temp_dir.path().join("broken.png"), b"not a png").unwrap();
    let mut cmd = Command::cargo_bin("maya").unwrap();
    cmd.arg("optimize")
        .arg(temp_dir.path())
        .args(["--types", "png"]);

    cmd.assert()
        .code(3)
        .stderr(predicate::str::contains("图片压缩部分失败"));
}

#[test]
fn pack_accepts_explicit_project_path_and_vite_out_dir() {
    use std::fs;
    use tempfile::tempdir;

    let temp_dir = tempdir().unwrap();
    fs::create_dir(temp_dir.path().join("custom-build")).unwrap();
    fs::write(temp_dir.path().join("custom-build/index.js"), "code").unwrap();
    let mut cmd = Command::cargo_bin("maya").unwrap();
    cmd.arg("pack")
        .arg(temp_dir.path())
        .args(["--type", "a", "--out-dir", "custom-build"]);

    cmd.assert().success();
    assert!(temp_dir.path().join("custom-build.zip").is_file());
}

#[test]
fn vite_pack_without_config_returns_input_exit_code() {
    use tempfile::tempdir;

    let temp_dir = tempdir().unwrap();
    let mut cmd = Command::cargo_bin("maya").unwrap();
    cmd.arg("pack").arg(temp_dir.path()).args(["--type", "a"]);

    cmd.assert()
        .code(4)
        .stderr(predicate::str::contains("未找到配置文件"));
}

#[test]
fn optimize_new_file_flag_creates_suffixed_image() {
    use image::{ImageBuffer, Rgba};
    use tempfile::tempdir;

    let temp_dir = tempdir().unwrap();
    let source = temp_dir.path().join("image.png");
    let image: ImageBuffer<Rgba<u8>, _> = ImageBuffer::from_pixel(16, 16, Rgba([255, 0, 0, 255]));
    image.save(&source).unwrap();

    let mut cmd = Command::cargo_bin("maya").unwrap();
    cmd.arg("optimize")
        .arg(temp_dir.path())
        .args(["--types", "png", "--new-file"]);

    cmd.assert().success();
    assert!(temp_dir.path().join("image_c.png").is_file());
}

#[test]
fn optimize_legacy_n_value_still_creates_new_file() {
    use image::{ImageBuffer, Rgba};
    use tempfile::tempdir;

    let temp_dir = tempdir().unwrap();
    let source = temp_dir.path().join("legacy.png");
    let image: ImageBuffer<Rgba<u8>, _> = ImageBuffer::from_pixel(16, 16, Rgba([0, 255, 0, 255]));
    image.save(&source).unwrap();

    let mut cmd = Command::cargo_bin("maya").unwrap();
    cmd.arg("optimize")
        .arg(temp_dir.path())
        .args(["--types", "png", "n"]);

    cmd.assert().success();
    assert!(temp_dir.path().join("legacy_c.png").is_file());
}
