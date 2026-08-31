use ffmpeg_sidecar::{
    command::{ffmpeg_is_installed, FfmpegCommand},
    download::auto_download,
    event::FfmpegEvent,
};
use indicatif::{ProgressBar, ProgressStyle};
use maya_common::error::{Error, Result};
use maya_common::file_utils::{atomic_replace_directory, find_files_by_extension};
use std::collections::VecDeque;
use std::fs;
use std::path::{Path, PathBuf};
use std::process::ExitStatus;
use std::sync::mpsc::{self, RecvTimeoutError};
use std::time::{Duration, Instant};

const STDERR_TAIL_LIMIT: usize = 8 * 1024;
const DEFAULT_CONVERSION_TIMEOUT: Duration = Duration::from_secs(2 * 60 * 60);

/// mp4转m3u8功能
///
/// # 参数
/// * `path` - 搜索mp4文件的目录路径
///
/// # 返回
/// * `Result<(u32, u32)>` - (成功转换的文件数量, 失败的文件数量)
pub async fn convert_mp4_to_m3u8(path: &Path) -> Result<(u32, u32)> {
    convert_mp4_to_m3u8_with_timeout(path, DEFAULT_CONVERSION_TIMEOUT).await
}

/// mp4 转 m3u8，并为每个 FFmpeg 进程设置超时。
pub async fn convert_mp4_to_m3u8_with_timeout(
    path: &Path,
    timeout: Duration,
) -> Result<(u32, u32)> {
    if timeout.is_zero() {
        return Err(Error::invalid_argument("FFmpeg转换超时必须大于0"));
    }

    // 确保FFmpeg可用，如果没有则自动下载
    ensure_ffmpeg_available().await?;

    println!("开始扫描mp4文件...");

    // 收集所有mp4文件
    let mp4_files = find_files_by_extension(path, &["mp4"])?;

    if mp4_files.is_empty() {
        println!("未找到任何mp4文件");
        return Ok((0, 0));
    }

    println!("找到 {} 个mp4文件", mp4_files.len());

    let mut successful_conversions = 0;
    let mut failed_conversions = 0;

    for (index, mp4_file) in mp4_files.iter().enumerate() {
        println!(
            "\n正在处理 ({}/{}) {}",
            index + 1,
            mp4_files.len(),
            mp4_file.display()
        );

        match convert_single_mp4(mp4_file, timeout).await {
            Ok(_) => {
                successful_conversions += 1;
                println!("✅ 成功转换: {}", mp4_file.display());
            }
            Err(e) => {
                failed_conversions += 1;
                eprintln!("❌ 转换失败 {}: {}", mp4_file.display(), e);
            }
        }
    }

    println!("\n--- 转换总结 ---");
    println!("总共处理文件数量: {}", mp4_files.len());
    println!("成功转换文件数量: {}", successful_conversions);
    println!("失败转换文件数量: {}", failed_conversions);
    println!("--------------------");

    Ok((successful_conversions, failed_conversions))
}

/// 转换单个mp4文件
async fn convert_single_mp4(mp4_file: &Path, timeout: Duration) -> Result<()> {
    let mp4_file = mp4_file.to_path_buf();
    tokio::task::spawn_blocking(move || convert_single_mp4_blocking(&mp4_file, timeout)).await?
}

fn convert_single_mp4_blocking(mp4_file: &Path, timeout: Duration) -> Result<()> {
    let file_stem = mp4_file
        .file_stem()
        .ok_or_else(|| Error::video_conversion("无法获取文件名"))?
        .to_string_lossy();

    let output_dir = mp4_file
        .parent()
        .ok_or_else(|| Error::video_conversion("无法获取文件目录"))?
        .join(&*file_stem);

    println!("🔄 开始转换...");

    // 获取视频时长用于计算进度
    let duration = get_video_duration(mp4_file)?;

    // 全部输出先写入同级临时目录；只有进程和产物验证均成功后才替换旧目录。
    atomic_replace_directory(&output_dir, |staging_dir| {
        let m3u8_file = staging_dir.join("index.m3u8");
        let mut ffmpeg = FfmpegCommand::new();
        ffmpeg
            .input(mp4_file.to_string_lossy().as_ref())
            .args(["-c", "copy"])
            .args(["-start_number", "0"])
            .args(["-hls_time", "10"])
            .args(["-hls_list_size", "0"])
            .args(["-f", "hls"])
            .output(m3u8_file.to_string_lossy().as_ref())
            .overwrite();

        run_ffmpeg_command(ffmpeg, duration, staging_dir, timeout)
    })?;

    println!("📁 输出目录: {}", output_dir.display());

    Ok(())
}

fn run_ffmpeg_command(
    mut ffmpeg: FfmpegCommand,
    duration: f64,
    output_dir: &Path,
    timeout: Duration,
) -> Result<()> {
    // 创建进度条
    let pb = ProgressBar::new(100);
    pb.set_style(
        ProgressStyle::default_bar()
            .template("{spinner:.green} [{elapsed_precise}] [{bar:40.cyan/blue}] {pos}% {msg}")
            .expect("无效的进度条模板字符串")
            .progress_chars("█▉▊▋▌▍▎▏  "),
    );
    pb.set_message("正在转换mp4到m3u8...");

    let command_description = format!("{ffmpeg:?}");
    let working_dir = std::env::current_dir()
        .map_err(|error| Error::video_conversion(format!("无法获取FFmpeg工作目录: {}", error)))?;

    let result = (|| {
        // 使用 ffmpeg-sidecar 构建命令并监听进度。
        let mut child = ffmpeg
            .spawn()
            .map_err(|e| Error::video_conversion(format!("FFmpeg启动失败: {}", e)))?;

        let mut stderr_tail = StderrTail::new(STDERR_TAIL_LIMIT);
        let mut iterator_errors = Vec::new();
        let mut last_progress = 0u64;
        let deadline = Instant::now()
            .checked_add(timeout)
            .ok_or_else(|| Error::invalid_argument("FFmpeg转换超时过大"))?;

        let iter = child
            .iter()
            .map_err(|e| Error::video_conversion(format!("FFmpeg迭代器错误: {}", e)))?;
        let (event_sender, event_receiver) = mpsc::channel();
        let event_reader = std::thread::spawn(move || {
            for event in iter {
                if event_sender.send(event).is_err() {
                    break;
                }
            }
        });

        let mut timed_out = false;
        loop {
            let remaining = deadline.saturating_duration_since(Instant::now());
            if remaining.is_zero() {
                timed_out = true;
                break;
            }

            match event_receiver.recv_timeout(remaining) {
                Ok(event) => match event {
                    FfmpegEvent::Progress(progress) => {
                        stderr_tail.push(&progress.raw_log_message);
                        update_conversion_progress(
                            &pb,
                            duration,
                            &progress.time,
                            &mut last_progress,
                        );
                    }
                    FfmpegEvent::Log(_, message) => stderr_tail.push(&message),
                    FfmpegEvent::Error(message) => {
                        stderr_tail.push(&message);
                        iterator_errors.push(message);
                    }
                    FfmpegEvent::LogEOF => {}
                    _ => {}
                },
                Err(RecvTimeoutError::Timeout) => {
                    timed_out = true;
                    break;
                }
                Err(RecvTimeoutError::Disconnected) => break,
            }
        }

        if timed_out {
            let kill_error = child.kill().err();
            let _ = child.wait();
            let _ = event_reader.join();

            let kill_details = kill_error
                .map(|error| format!("；终止进程时发生错误: {error}"))
                .unwrap_or_default();
            return Err(Error::video_conversion(format!(
                "FFmpeg转换超时（命令: {}；工作目录: {}；超时: {:?}）{}，stderr末尾: {}",
                command_description,
                working_dir.display(),
                timeout,
                kill_details,
                stderr_tail.as_string().trim()
            )));
        }

        if event_reader.join().is_err() {
            iterator_errors.push("FFmpeg事件读取线程异常退出".to_string());
        }

        let status = child
            .wait()
            .map_err(|e| Error::video_conversion(format!("等待FFmpeg退出失败: {}", e)))?;

        validate_ffmpeg_completion(
            status,
            &iterator_errors,
            stderr_tail.as_string(),
            output_dir,
            &command_description,
            &working_dir,
        )
    })();

    match result {
        Ok(()) => {
            pb.set_position(100);
            pb.finish_with_message("✅ 转换完成");
            Ok(())
        }
        Err(error) => {
            pb.abandon_with_message("❌ 转换失败");
            Err(error)
        }
    }
}

fn update_conversion_progress(
    progress_bar: &ProgressBar,
    duration: f64,
    time: &str,
    last_progress: &mut u64,
) {
    if let Ok(current_seconds) = parse_time_string(time) {
        let progress_percent = if duration > 0.0 {
            ((current_seconds / duration) * 100.0).min(99.0) as u64
        } else {
            (current_seconds as u64) % 100
        };

        if progress_percent != *last_progress {
            progress_bar.set_position(progress_percent);
            progress_bar.set_message(format!(
                "正在转换... {:.1}s{}",
                current_seconds,
                if duration > 0.0 {
                    format!(" / {:.1}s", duration)
                } else {
                    String::new()
                }
            ));
            *last_progress = progress_percent;
        }
    }
}

fn validate_ffmpeg_completion(
    status: ExitStatus,
    iterator_errors: &[String],
    stderr_tail: String,
    output_dir: &Path,
    command_description: &str,
    working_dir: &Path,
) -> Result<()> {
    if !status.success() {
        let exit_code = status
            .code()
            .map(|code| code.to_string())
            .unwrap_or_else(|| "被信号终止".to_string());
        let details = if stderr_tail.trim().is_empty() {
            "没有可用的 stderr 输出".to_string()
        } else {
            stderr_tail
        };
        return Err(Error::video_conversion(format!(
            "FFmpeg退出失败（命令: {}；工作目录: {}；退出码: {}），stderr末尾: {}",
            command_description,
            working_dir.display(),
            exit_code,
            details.trim()
        )));
    }

    if !iterator_errors.is_empty() {
        return Err(Error::video_conversion(format!(
            "FFmpeg输出解析失败: {}",
            iterator_errors.join(" | ")
        )));
    }

    validate_hls_output(output_dir)
}

fn validate_hls_output(output_dir: &Path) -> Result<()> {
    let playlist_path = output_dir.join("index.m3u8");
    let playlist = fs::read_to_string(&playlist_path).map_err(|error| {
        Error::video_conversion(format!(
            "无法读取输出播放列表 {}: {}",
            playlist_path.display(),
            error
        ))
    })?;

    if playlist.trim().is_empty() {
        return Err(Error::video_conversion(format!(
            "输出播放列表为空: {}",
            playlist_path.display()
        )));
    }

    let segment_paths: Vec<PathBuf> = playlist
        .lines()
        .map(str::trim)
        .filter(|line| !line.is_empty() && !line.starts_with('#'))
        .map(|line| line.split('?').next().unwrap_or(line))
        .map(|line| output_dir.join(line))
        .collect();

    if segment_paths.is_empty() {
        return Err(Error::video_conversion(format!(
            "输出播放列表未引用任何分片: {}",
            playlist_path.display()
        )));
    }

    for segment_path in segment_paths {
        let metadata = fs::metadata(&segment_path).map_err(|error| {
            Error::video_conversion(format!(
                "播放列表引用的分片不存在或不可读 {}: {}",
                segment_path.display(),
                error
            ))
        })?;
        if !metadata.is_file() || metadata.len() == 0 {
            return Err(Error::video_conversion(format!(
                "播放列表引用的分片为空或不是文件: {}",
                segment_path.display()
            )));
        }
    }

    Ok(())
}

struct StderrTail {
    lines: VecDeque<String>,
    bytes: usize,
    limit: usize,
}

impl StderrTail {
    fn new(limit: usize) -> Self {
        Self {
            lines: VecDeque::new(),
            bytes: 0,
            limit,
        }
    }

    fn push(&mut self, message: &str) {
        if message.is_empty() {
            return;
        }

        let message = message.to_string();
        self.bytes += message.len();
        self.lines.push_back(message);

        while self.bytes > self.limit && self.lines.len() > 1 {
            if let Some(removed) = self.lines.pop_front() {
                self.bytes = self.bytes.saturating_sub(removed.len());
            }
        }
    }

    fn as_string(&self) -> String {
        let joined = self.lines.iter().cloned().collect::<Vec<_>>().join("\n");
        if joined.len() <= self.limit {
            return joined;
        }

        let mut start = joined.len() - self.limit;
        while !joined.is_char_boundary(start) {
            start += 1;
        }
        joined[start..].to_string()
    }
}

/// 确保FFmpeg可用，如果不可用则自动下载
async fn ensure_ffmpeg_available() -> Result<()> {
    // 尝试检查FFmpeg是否已经可用
    if ffmpeg_is_installed() {
        return Ok(());
    }

    println!("🔍 FFmpeg未找到，正在自动下载...");

    // 创建下载进度条
    let pb = ProgressBar::new(100);
    pb.set_style(
        ProgressStyle::default_bar()
            .template("{spinner:.blue} [{elapsed_precise}] [{bar:40.cyan/blue}] {pos}% {msg}")
            .expect("无效的进度条模板字符串")
            .progress_chars("█▉▊▋▌▍▎▏  "),
    );
    pb.set_message("正在下载FFmpeg二进制文件...");

    let progress_handle = tokio::spawn({
        let pb = pb.clone();
        async move {
            for i in 0..=100 {
                pb.set_position(i);
                tokio::time::sleep(Duration::from_millis(50)).await;
                if i == 100 {
                    break;
                }
            }
        }
    });

    // 自动下载FFmpeg
    let download_result = tokio::task::spawn_blocking(auto_download)
        .await
        .map_err(|e| Error::video_conversion(format!("FFmpeg下载任务失败: {}", e)))?;

    progress_handle.abort();

    match download_result {
        Ok(_) => {
            pb.finish_with_message("✅ FFmpeg下载完成");
            Ok(())
        }
        Err(e) => {
            pb.abandon_with_message("❌ FFmpeg下载失败");
            Err(Error::video_conversion(format!("FFmpeg下载失败: {}", e)))
        }
    }
}

/// 获取视频时长
fn get_video_duration(mp4_file: &Path) -> Result<f64> {
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
            let duration_str = String::from_utf8(result.stdout)
                .map_err(|e| Error::video_conversion(format!("解析ffprobe输出失败: {}", e)))?;
            let duration: f64 = duration_str
                .trim()
                .parse()
                .map_err(|e| Error::video_conversion(format!("解析视频时长失败: {}", e)))?;
            Ok(duration)
        }
        Ok(_) => {
            // 如果ffprobe失败，返回默认值0（将使用无进度模式）
            println!("⚠️  无法获取视频时长，将使用简化进度显示");
            Ok(0.0)
        }
        Err(_) => {
            // 如果ffprobe不可用，返回默认值0
            println!("⚠️  ffprobe不可用，将使用简化进度显示");
            Ok(0.0)
        }
    }
}

/// 解析FFmpeg输出的时间字符串 (HH:MM:SS.ss) 为秒数
fn parse_time_string(time_str: &str) -> Result<f64> {
    let parts: Vec<&str> = time_str.split(':').collect();
    if parts.len() == 3 {
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
    } else {
        Err(Error::video_conversion(format!(
            "无效的时间格式: {}",
            time_str
        )))
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::process::{Command, Stdio};
    use tempfile::tempdir;

    fn exit_status(code: i32) -> ExitStatus {
        if cfg!(windows) {
            Command::new("cmd")
                .args(["/C", "exit", &code.to_string()])
                .status()
                .unwrap()
        } else {
            Command::new("sh")
                .args(["-c", &format!("exit {code}")])
                .status()
                .unwrap()
        }
    }

    #[cfg(windows)]
    fn fake_ffmpeg_command(output_dir: &Path, exit_code: i32) -> FfmpegCommand {
        fn cmd_literal(path: &Path) -> String {
            path.to_string_lossy().replace('%', "%%")
        }

        let playlist = cmd_literal(&output_dir.join("index.m3u8"));
        let segment = cmd_literal(&output_dir.join("segment0.ts"));
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
    fn test_parse_time_string_valid() {
        assert_eq!(parse_time_string("00:00:00").unwrap(), 0.0);
        assert_eq!(parse_time_string("00:00:01").unwrap(), 1.0);
        assert_eq!(parse_time_string("00:01:00").unwrap(), 60.0);
        assert_eq!(parse_time_string("01:00:00").unwrap(), 3600.0);
        assert_eq!(parse_time_string("01:30:45").unwrap(), 5445.0);
        assert_eq!(parse_time_string("01:30:45.5").unwrap(), 5445.5);
    }

    #[test]
    fn test_parse_time_string_invalid() {
        assert!(parse_time_string("").is_err());
        assert!(parse_time_string("00:00").is_err());
        assert!(parse_time_string("00:00:00:00").is_err());
        assert!(parse_time_string("abc:def:ghi").is_err());
        assert!(parse_time_string("xx:yy:zz").is_err());
        assert_eq!(parse_time_string("00:00:60").unwrap(), 60.0);
    }

    #[test]
    fn test_parse_time_string_edge_cases() {
        assert!(parse_time_string("::").is_err());
        assert!(parse_time_string("01::").is_err());
        assert!(parse_time_string(":02:").is_err());
        assert!(parse_time_string("::03").is_err());
    }

    #[test]
    fn validate_hls_output_accepts_complete_playlist() {
        let temp_dir = tempdir().unwrap();
        fs::write(
            temp_dir.path().join("index.m3u8"),
            b"#EXTM3U\n#EXTINF:10.0,\nsegment0.ts\n#EXT-X-ENDLIST\n",
        )
        .unwrap();
        fs::write(temp_dir.path().join("segment0.ts"), b"segment bytes").unwrap();

        assert!(validate_hls_output(temp_dir.path()).is_ok());
    }

    #[test]
    fn validate_hls_output_rejects_missing_segment() {
        let temp_dir = tempdir().unwrap();
        fs::write(
            temp_dir.path().join("index.m3u8"),
            b"#EXTM3U\n#EXTINF:10.0,\nmissing.ts\n",
        )
        .unwrap();

        let error = validate_hls_output(temp_dir.path()).unwrap_err();

        assert!(error.to_string().contains("分片不存在"));
    }

    #[test]
    fn validate_ffmpeg_completion_rejects_non_zero_status_even_with_output() {
        let temp_dir = tempdir().unwrap();
        fs::write(
            temp_dir.path().join("index.m3u8"),
            b"#EXTM3U\nsegment0.ts\n",
        )
        .unwrap();
        fs::write(temp_dir.path().join("segment0.ts"), b"segment bytes").unwrap();

        let error = validate_ffmpeg_completion(
            exit_status(7),
            &[],
            "conversion failed".to_string(),
            temp_dir.path(),
            "ffmpeg -i input.mp4 output.m3u8",
            temp_dir.path(),
        )
        .unwrap_err();

        assert!(error.to_string().contains("退出码: 7"));
        assert!(error.to_string().contains("conversion failed"));
        assert!(error.to_string().contains("ffmpeg -i input.mp4"));
        assert!(error
            .to_string()
            .contains(&temp_dir.path().display().to_string()));
    }

    #[test]
    fn validate_ffmpeg_completion_requires_complete_output_after_success() {
        let temp_dir = tempdir().unwrap();
        fs::write(temp_dir.path().join("index.m3u8"), b"#EXTM3U\n").unwrap();

        let error = validate_ffmpeg_completion(
            exit_status(0),
            &[],
            String::new(),
            temp_dir.path(),
            "ffmpeg",
            temp_dir.path(),
        )
        .unwrap_err();

        assert!(error.to_string().contains("未引用任何分片"));
    }

    #[cfg(windows)]
    #[test]
    fn failed_ffmpeg_process_does_not_replace_existing_output_directory() {
        let temp_dir = tempdir().unwrap();
        let target = temp_dir.path().join("video");
        fs::create_dir(&target).unwrap();
        fs::write(target.join("index.m3u8"), b"old playlist").unwrap();

        let result = atomic_replace_directory(&target, |staging_dir| {
            run_ffmpeg_command(
                fake_ffmpeg_command(staging_dir, 7),
                0.0,
                staging_dir,
                Duration::from_secs(10),
            )
        });

        let error = result.unwrap_err();
        let error_message = error.to_string();
        assert!(
            error_message.contains("退出码: 7"),
            "实际错误: {error_message}"
        );
        assert!(
            error_message.contains("simulated ffmpeg failure"),
            "实际错误: {error_message}"
        );
        assert_eq!(
            fs::read(target.join("index.m3u8")).unwrap(),
            b"old playlist"
        );
        assert_eq!(fs::read_dir(temp_dir.path()).unwrap().count(), 1);
    }

    #[cfg(windows)]
    #[test]
    fn timed_out_ffmpeg_process_does_not_replace_existing_output_directory() {
        let temp_dir = tempdir().unwrap();
        let target = temp_dir.path().join("video");
        fs::create_dir(&target).unwrap();
        fs::write(target.join("index.m3u8"), b"old playlist").unwrap();

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

        // PowerShell 的 `-File -` 会等待 stdin；sidecar 保持该管道打开，适合验证超时终止。

        let result = atomic_replace_directory(&target, |staging_dir| {
            run_ffmpeg_command(
                FfmpegCommand::from(command),
                0.0,
                staging_dir,
                Duration::from_millis(100),
            )
        });

        let error_message = result.unwrap_err().to_string();
        assert!(
            error_message.contains("转换超时"),
            "实际错误: {error_message}"
        );
        assert_eq!(
            fs::read(target.join("index.m3u8")).unwrap(),
            b"old playlist"
        );
        assert_eq!(fs::read_dir(temp_dir.path()).unwrap().count(), 1);
    }

    #[test]
    fn stderr_tail_is_bounded_and_keeps_latest_messages() {
        let mut tail = StderrTail::new(8);
        tail.push("first");
        tail.push("second");
        tail.push("末尾");

        let output = tail.as_string();

        assert!(output.len() <= 8);
        assert!(output.contains("末尾"));
        assert!(!output.contains("first"));
    }
}
