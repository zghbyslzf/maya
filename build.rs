use sha2::{Digest, Sha256};
use std::env;
use std::fs::{self, File};
use std::io::{self, Read};
use std::path::{Path, PathBuf};

const SIDECAR_DIRECTORY: &str = "FFmpeg";
const CHECKSUM_MANIFEST: &str = "FFmpeg/checksums.sha256";

fn main() {
    println!("cargo:rerun-if-changed={CHECKSUM_MANIFEST}");
    println!("cargo:rerun-if-changed={SIDECAR_DIRECTORY}/ffmpeg.exe");
    println!("cargo:rerun-if-changed={SIDECAR_DIRECTORY}/ffprobe.exe");

    if env::var("PROFILE").as_deref() != Ok("release") {
        return;
    }
    if env::var("CARGO_CFG_TARGET_OS").as_deref() != Ok("windows") {
        panic!("Maya 只支持 Windows 目标平台");
    }

    deploy_release_sidecars()
        .unwrap_or_else(|error| panic!("无法为 release 产物部署 FFmpeg sidecar: {error}"));
}

fn deploy_release_sidecars() -> Result<(), String> {
    let manifest_dir = required_path("CARGO_MANIFEST_DIR")?;
    let output_dir = required_path("OUT_DIR")?;
    let release_dir = output_dir
        .ancestors()
        .nth(3)
        .ok_or_else(|| format!("无法从 OUT_DIR 推导 release 目录: {}", output_dir.display()))?;
    let checksums = fs::read_to_string(manifest_dir.join(CHECKSUM_MANIFEST))
        .map_err(|error| format!("读取校验清单失败: {error}"))?;

    for (expected, file_name) in parse_checksums(&checksums)? {
        let source = manifest_dir.join(SIDECAR_DIRECTORY).join(file_name);
        verify_checksum(&source, expected)?;
        let destination = release_dir.join(file_name);
        fs::copy(&source, &destination).map_err(|error| {
            format!(
                "复制 {} 到 {} 失败: {error}",
                source.display(),
                destination.display()
            )
        })?;
        verify_checksum(&destination, expected)?;
    }
    Ok(())
}

fn required_path(name: &str) -> Result<PathBuf, String> {
    env::var_os(name)
        .map(PathBuf::from)
        .ok_or_else(|| format!("Cargo 未提供环境变量 {name}"))
}

fn parse_checksums(content: &str) -> Result<Vec<(&str, &str)>, String> {
    let mut entries = Vec::new();
    for line in content.lines().filter(|line| !line.trim().is_empty()) {
        let (hash, file_name) = line
            .split_once(char::is_whitespace)
            .ok_or_else(|| format!("无效的校验清单行: {line}"))?;
        if hash.len() != 64 || !hash.bytes().all(|byte| byte.is_ascii_hexdigit()) {
            return Err(format!("无效的 SHA-256: {hash}"));
        }
        let file_name = file_name.trim();
        if !matches!(file_name, "ffmpeg.exe" | "ffprobe.exe") {
            return Err(format!("校验清单包含非预期文件: {file_name}"));
        }
        entries.push((hash, file_name));
    }
    let ffmpeg_count = entries
        .iter()
        .filter(|(_, file_name)| *file_name == "ffmpeg.exe")
        .count();
    let ffprobe_count = entries
        .iter()
        .filter(|(_, file_name)| *file_name == "ffprobe.exe")
        .count();
    if ffmpeg_count != 1 || ffprobe_count != 1 {
        return Err(format!(
            "校验清单必须且只能各包含一个 ffmpeg.exe 和 ffprobe.exe，实际计数为 {ffmpeg_count}/{ffprobe_count}"
        ));
    }
    Ok(entries)
}

fn verify_checksum(path: &Path, expected: &str) -> Result<(), String> {
    let actual = sha256_file(path)
        .map_err(|error| format!("计算 {} 的 SHA-256 失败: {error}", path.display()))?;
    if !actual.eq_ignore_ascii_case(expected) {
        return Err(format!(
            "{} 的 SHA-256 不匹配（期望: {expected}；实际: {actual}）",
            path.display()
        ));
    }
    Ok(())
}

fn sha256_file(path: &Path) -> io::Result<String> {
    let mut file = File::open(path)?;
    let mut hasher = Sha256::new();
    let mut buffer = [0_u8; 64 * 1024];
    loop {
        let count = file.read(&mut buffer)?;
        if count == 0 {
            break;
        }
        hasher.update(&buffer[..count]);
    }
    Ok(format!("{:X}", hasher.finalize()))
}
