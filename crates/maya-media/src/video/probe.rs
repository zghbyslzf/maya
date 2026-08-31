use maya_core::{Error, OperationWarning, Result};
use std::path::Path;

pub(super) fn get_video_duration(mp4_file: &Path) -> Result<(f64, Option<OperationWarning>)> {
    let output = std::process::Command::new(ffmpeg_sidecar::ffprobe::ffprobe_path())
        .args([
            "-v",
            "quiet",
            "-show_entries",
            "format=duration",
            "-of",
            "csv=p=0",
            mp4_file.to_string_lossy().as_ref(),
        ])
        .output();

    match output {
        Ok(result) if result.status.success() => {
            let duration = String::from_utf8(result.stdout)
                .map_err(|error| {
                    Error::video_conversion(format!("解析 ffprobe 输出失败: {error}"))
                })?
                .trim()
                .parse()
                .map_err(|error| Error::video_conversion(format!("解析视频时长失败: {error}")))?;
            Ok((duration, None))
        }
        Ok(_) => Ok((
            0.0,
            Some(OperationWarning::new(
                Some(mp4_file.to_path_buf()),
                "无法获取视频时长，将使用非确定进度",
            )),
        )),
        Err(_) => Ok((
            0.0,
            Some(OperationWarning::new(
                Some(mp4_file.to_path_buf()),
                "ffprobe 不可用，将使用非确定进度",
            )),
        )),
    }
}

pub(super) fn parse_time_string(time: &str) -> Result<f64> {
    let parts: Vec<&str> = time.split(':').collect();
    if parts.len() != 3 {
        return Err(Error::video_conversion(format!("无效的时间格式: {time}")));
    }
    let hours: f64 = parts[0]
        .parse()
        .map_err(|_| Error::video_conversion(format!("无效的小时值: {}", parts[0])))?;
    let minutes: f64 = parts[1]
        .parse()
        .map_err(|_| Error::video_conversion(format!("无效的分钟值: {}", parts[1])))?;
    let seconds: f64 = parts[2]
        .parse()
        .map_err(|_| Error::video_conversion(format!("无效的秒值: {}", parts[2])))?;
    Ok(hours * 3600.0 + minutes * 60.0 + seconds)
}
