use maya_core::{Error, Result};
use sha2::{Digest, Sha256};
use std::fs::File;
use std::io::Read;
use std::path::Path;

const CHECKSUM_MANIFEST: &str = include_str!("../../../../FFmpeg/checksums.sha256");

pub(super) fn verify_bundled_tools() -> Result<()> {
    let ffmpeg = ffmpeg_sidecar::paths::sidecar_path()
        .map_err(|error| Error::video_conversion(format!("无法定位随包分发的 FFmpeg: {error}")))?;
    let ffprobe = ffmpeg_sidecar::ffprobe::ffprobe_sidecar_path()
        .map_err(|error| Error::video_conversion(format!("无法定位随包分发的 FFprobe: {error}")))?;

    verify_binary(&ffmpeg, expected_checksum("ffmpeg.exe")?, "FFmpeg")?;
    verify_binary(&ffprobe, expected_checksum("ffprobe.exe")?, "FFprobe")
}

fn expected_checksum(file_name: &str) -> Result<&'static str> {
    CHECKSUM_MANIFEST
        .lines()
        .filter_map(|line| line.split_once(char::is_whitespace))
        .find_map(|(hash, name)| (name.trim() == file_name).then_some(hash))
        .ok_or_else(|| Error::video_conversion(format!("FFmpeg 校验清单缺少 {file_name}")))
}

fn verify_binary(path: &Path, expected: &str, name: &str) -> Result<()> {
    let actual = sha256_file(path).map_err(|error| {
        Error::video_conversion(format!(
            "无法读取随包分发的 {name}（{}）: {error}。请重新安装 maya-cli-rs",
            path.display()
        ))
    })?;
    if !actual.eq_ignore_ascii_case(expected) {
        return Err(Error::video_conversion(format!(
            "随包分发的 {name} 完整性校验失败（路径: {}；期望 SHA-256: {expected}；实际: {actual}）。请重新安装 maya-cli-rs",
            path.display()
        )));
    }
    Ok(())
}

fn sha256_file(path: &Path) -> std::io::Result<String> {
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

#[cfg(test)]
mod tests {
    use super::*;
    use std::fs;
    use tempfile::tempdir;

    #[test]
    fn verifies_expected_hash_and_rejects_mismatch() {
        let root = tempdir().unwrap();
        let binary = root.path().join("tool.exe");
        fs::write(&binary, b"maya").unwrap();
        let hash = sha256_file(&binary).unwrap();

        assert!(verify_binary(&binary, &hash, "测试工具").is_ok());
        assert!(verify_binary(&binary, &hash.to_ascii_lowercase(), "测试工具").is_ok());
        assert!(verify_binary(&binary, &"0".repeat(64), "测试工具")
            .unwrap_err()
            .to_string()
            .contains("完整性校验失败"));
    }
}
