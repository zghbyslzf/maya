mod binaries;
mod ffmpeg;
mod model;
mod probe;

pub use model::{ConversionOptions, ConversionOutcome, ConversionReport};

use ffmpeg::run_ffmpeg_command;
#[cfg(test)]
use ffmpeg::{validate_ffmpeg_completion, validate_hls_output, StderrTail};
use ffmpeg_sidecar::command::FfmpegCommand;
use maya_core::{Error, FailurePolicy, NoopProgress, ProgressEvent, ProgressSink, Result};
use maya_fs::{atomic_replace_directory, find_files_by_extension};
use model::ConvertedVideo;
use probe::get_video_duration;
#[cfg(test)]
use probe::parse_time_string;
use std::path::Path;
use std::sync::Arc;
use std::time::Duration;

pub async fn convert_mp4_to_m3u8(
    path: &Path,
    options: &ConversionOptions,
) -> Result<ConversionReport> {
    convert_mp4_to_m3u8_with_progress(path, options, Arc::new(NoopProgress)).await
}

pub async fn convert_mp4_to_m3u8_with_progress(
    path: &Path,
    options: &ConversionOptions,
    progress: Arc<dyn ProgressSink>,
) -> Result<ConversionReport> {
    if options.timeout.is_zero() {
        return Err(Error::invalid_argument("FFmpeg转换超时必须大于0"));
    }

    let files = find_files_by_extension(path, &["mp4"])?;
    let total = files.len();
    if total == 0 {
        progress.emit(ProgressEvent::Started {
            operation: "MP4 转 M3U8".to_string(),
            total: Some(0),
        });
        progress.emit(ProgressEvent::Finished);
        return Ok(ConversionReport::default());
    }
    ensure_ffmpeg_available(progress.as_ref()).await?;
    progress.emit(ProgressEvent::Started {
        operation: "MP4 转 M3U8".to_string(),
        total: Some(total as u64),
    });

    let mut report = ConversionReport {
        scanned: total,
        ..ConversionReport::default()
    };
    for input in files {
        match convert_single_mp4(&input, options.timeout, Arc::clone(&progress)).await {
            Ok(converted) => {
                report.succeeded += 1;
                report.warning_count += converted.warnings.len();
                report.items.push(ConversionOutcome::Converted {
                    input: input.clone(),
                    output_dir: converted.output_dir,
                    warnings: converted.warnings,
                });
            }
            Err(error) if options.failure_policy == FailurePolicy::Continue => {
                report.failed += 1;
                report.items.push(ConversionOutcome::Failed {
                    input: input.clone(),
                    error,
                });
            }
            Err(error) => {
                progress.emit(ProgressEvent::Finished);
                return Err(error);
            }
        }
        progress.emit(ProgressEvent::Advanced {
            increment: 1,
            total: Some(total as u64),
            message: Some(input.display().to_string()),
        });
    }
    progress.emit(ProgressEvent::Finished);
    Ok(report)
}

async fn convert_single_mp4(
    input: &Path,
    timeout: Duration,
    progress: Arc<dyn ProgressSink>,
) -> Result<ConvertedVideo> {
    let input = input.to_path_buf();
    tokio::task::spawn_blocking(move || {
        convert_single_mp4_blocking(&input, timeout, progress.as_ref())
    })
    .await
    .map_err(|error| Error::other(format!("视频转换任务异常退出: {error}")))?
}

fn convert_single_mp4_blocking(
    input: &Path,
    timeout: Duration,
    progress: &dyn ProgressSink,
) -> Result<ConvertedVideo> {
    let stem = input
        .file_stem()
        .ok_or_else(|| Error::video_conversion("无法获取文件名"))?;
    let working_dir = input
        .parent()
        .ok_or_else(|| Error::video_conversion("无法获取文件目录"))?;
    let output_dir = working_dir.join(stem);
    let (duration, warning) = get_video_duration(input)?;

    atomic_replace_directory(&output_dir, |staging_dir| {
        let playlist = staging_dir.join("index.m3u8");
        let mut command = FfmpegCommand::new();
        command
            .input(input.to_string_lossy().as_ref())
            .args(["-c", "copy"])
            .args(["-start_number", "0"])
            .args(["-hls_time", "10"])
            .args(["-hls_list_size", "0"])
            .args(["-f", "hls"])
            .output(playlist.to_string_lossy().as_ref())
            .overwrite();
        run_ffmpeg_command(
            command,
            duration,
            staging_dir,
            working_dir,
            timeout,
            progress,
        )
    })?;

    Ok(ConvertedVideo {
        output_dir,
        warnings: warning.into_iter().collect(),
    })
}

async fn ensure_ffmpeg_available(progress: &dyn ProgressSink) -> Result<()> {
    progress.emit(ProgressEvent::Started {
        operation: "校验 FFmpeg 与 FFprobe".to_string(),
        total: None,
    });
    let result = tokio::task::spawn_blocking(binaries::verify_bundled_tools).await;
    progress.emit(ProgressEvent::Finished);
    result.map_err(|error| Error::video_conversion(format!("FFmpeg校验任务失败: {error}")))?
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::fs;
    use std::process::{Command, ExitStatus, Stdio};
    use tempfile::tempdir;

    fn exit_status(code: i32) -> ExitStatus {
        Command::new("cmd")
            .args(["/C", "exit", &code.to_string()])
            .status()
            .unwrap()
    }

    #[cfg(windows)]
    fn fake_ffmpeg_command(output_dir: &Path, exit_code: i32) -> FfmpegCommand {
        let literal = |path: std::path::PathBuf| path.to_string_lossy().replace('%', "%%");
        let playlist = literal(output_dir.join("index.m3u8"));
        let segment = literal(output_dir.join("segment0.ts"));
        let script = format!(
            "(echo #EXTM3U&echo segment0.ts)>\"{playlist}\" & \
             echo segment bytes>\"{segment}\" & \
             echo simulated ffmpeg failure 1>&2 & exit /b {exit_code}"
        );
        let mut command = Command::new("cmd");
        command
            .args(["/D", "/S", "/C", &script, "-nostdin"])
            .stdin(Stdio::piped())
            .stdout(Stdio::piped())
            .stderr(Stdio::piped());
        FfmpegCommand::from(command)
    }

    #[test]
    fn parses_ffmpeg_time_values() {
        assert_eq!(parse_time_string("01:30:45.5").unwrap(), 5445.5);
        assert!(parse_time_string("00:00").is_err());
        assert!(parse_time_string("abc:def:ghi").is_err());
    }

    #[test]
    fn validates_complete_hls_output() {
        let root = tempdir().unwrap();
        fs::write(root.path().join("index.m3u8"), "#EXTM3U\nsegment0.ts\n").unwrap();
        fs::write(root.path().join("segment0.ts"), "segment").unwrap();
        assert!(validate_hls_output(root.path()).is_ok());
    }

    #[test]
    fn rejects_missing_hls_segment() {
        let root = tempdir().unwrap();
        fs::write(root.path().join("index.m3u8"), "#EXTM3U\nmissing.ts\n").unwrap();
        assert!(validate_hls_output(root.path())
            .unwrap_err()
            .to_string()
            .contains("分片不存在"));
    }

    #[test]
    fn non_zero_exit_is_failure_even_with_output() {
        let root = tempdir().unwrap();
        fs::write(root.path().join("index.m3u8"), "#EXTM3U\nsegment0.ts\n").unwrap();
        fs::write(root.path().join("segment0.ts"), "segment").unwrap();
        let error = validate_ffmpeg_completion(
            exit_status(7),
            &[],
            "conversion failed".to_string(),
            root.path(),
            "ffmpeg command",
            root.path(),
        )
        .unwrap_err();
        assert!(error.to_string().contains("Some(7)"));
        assert!(error.to_string().contains("conversion failed"));
    }

    #[test]
    fn successful_exit_still_requires_complete_output() {
        let root = tempdir().unwrap();
        fs::write(root.path().join("index.m3u8"), "#EXTM3U\n").unwrap();

        let error = validate_ffmpeg_completion(
            exit_status(0),
            &[],
            String::new(),
            root.path(),
            "ffmpeg command",
            root.path(),
        )
        .unwrap_err();

        assert!(error.to_string().contains("未引用任何分片"));
    }

    #[cfg(windows)]
    #[test]
    fn failed_process_preserves_existing_output() {
        let root = tempdir().unwrap();
        let target = root.path().join("video");
        fs::create_dir(&target).unwrap();
        fs::write(target.join("index.m3u8"), "old").unwrap();
        let result = atomic_replace_directory(&target, |staging| {
            run_ffmpeg_command(
                fake_ffmpeg_command(staging, 7),
                0.0,
                staging,
                root.path(),
                Duration::from_secs(10),
                &NoopProgress,
            )
        });
        assert!(result.unwrap_err().to_string().contains("Some(7)"));
        assert_eq!(fs::read(target.join("index.m3u8")).unwrap(), b"old");
    }

    #[cfg(windows)]
    #[test]
    fn timed_out_process_preserves_existing_output() {
        let root = tempdir().unwrap();
        let target = root.path().join("video");
        fs::create_dir(&target).unwrap();
        fs::write(target.join("index.m3u8"), "old").unwrap();
        let mut command = Command::new("powershell");
        command
            .args([
                "-NoLogo",
                "-NoProfile",
                "-NonInteractive",
                "-File",
                "-",
                "-nostdin",
            ])
            .stdin(Stdio::piped())
            .stdout(Stdio::piped())
            .stderr(Stdio::piped());
        let result = atomic_replace_directory(&target, |staging| {
            run_ffmpeg_command(
                FfmpegCommand::from(command),
                0.0,
                staging,
                root.path(),
                Duration::from_millis(100),
                &NoopProgress,
            )
        });
        assert!(result.unwrap_err().to_string().contains("转换超时"));
        assert_eq!(fs::read(target.join("index.m3u8")).unwrap(), b"old");
    }

    #[test]
    fn stderr_tail_is_bounded() {
        let mut tail = StderrTail::new(8);
        tail.push("first");
        tail.push("second");
        tail.push("末尾");
        let output = tail.as_string();
        assert!(output.len() <= 8);
        assert!(output.contains("末尾"));
    }
}
