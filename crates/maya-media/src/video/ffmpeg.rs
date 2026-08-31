use super::probe::parse_time_string;
use ffmpeg_sidecar::{command::FfmpegCommand, event::FfmpegEvent};
use maya_core::{Error, ProgressEvent, ProgressSink, Result};
use std::collections::VecDeque;
use std::fs;
use std::path::{Path, PathBuf};
use std::process::ExitStatus;
use std::sync::mpsc::{self, RecvTimeoutError};
use std::time::{Duration, Instant};

const STDERR_TAIL_LIMIT: usize = 8 * 1024;

pub(super) fn run_ffmpeg_command(
    mut ffmpeg: FfmpegCommand,
    duration: f64,
    output_dir: &Path,
    working_dir: &Path,
    timeout: Duration,
    progress: &dyn ProgressSink,
) -> Result<()> {
    let command_description = format!("{ffmpeg:?}");
    let mut child = ffmpeg
        .spawn()
        .map_err(|error| Error::video_conversion(format!("FFmpeg 启动失败: {error}")))?;
    let mut stderr_tail = StderrTail::new(STDERR_TAIL_LIMIT);
    let mut iterator_errors = Vec::new();
    let mut last_progress = 0u64;
    let deadline = Instant::now()
        .checked_add(timeout)
        .ok_or_else(|| Error::invalid_argument("FFmpeg 转换超时过大"))?;
    let iterator = child
        .iter()
        .map_err(|error| Error::video_conversion(format!("FFmpeg 迭代器错误: {error}")))?;
    let (event_sender, event_receiver) = mpsc::channel();
    let event_reader = std::thread::spawn(move || {
        for event in iterator {
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
                FfmpegEvent::Progress(event_progress) => {
                    stderr_tail.push(&event_progress.raw_log_message);
                    update_conversion_progress(
                        progress,
                        duration,
                        &event_progress.time,
                        &mut last_progress,
                    );
                }
                FfmpegEvent::Log(_, message) => stderr_tail.push(&message),
                FfmpegEvent::Error(message) => {
                    stderr_tail.push(&message);
                    iterator_errors.push(message);
                }
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
        .map_err(|error| Error::video_conversion(format!("等待 FFmpeg 退出失败: {error}")))?;
    validate_ffmpeg_completion(
        status,
        &iterator_errors,
        stderr_tail.as_string(),
        output_dir,
        &command_description,
        working_dir,
    )
}

fn update_conversion_progress(
    progress: &dyn ProgressSink,
    duration: f64,
    time: &str,
    last_progress: &mut u64,
) {
    if let Ok(current_seconds) = parse_time_string(time) {
        let percent = if duration > 0.0 {
            ((current_seconds / duration) * 100.0).min(99.0) as u64
        } else {
            (current_seconds as u64) % 100
        };
        if percent != *last_progress {
            progress.emit(ProgressEvent::Message(format!(
                "FFmpeg 转换进度 {percent}%：{current_seconds:.1}s{}",
                if duration > 0.0 {
                    format!(" / {duration:.1}s")
                } else {
                    String::new()
                }
            )));
            *last_progress = percent;
        }
    }
}

pub(super) fn validate_ffmpeg_completion(
    status: ExitStatus,
    iterator_errors: &[String],
    stderr_tail: String,
    output_dir: &Path,
    command_description: &str,
    working_dir: &Path,
) -> Result<()> {
    if !status.success() {
        let details = if stderr_tail.trim().is_empty() {
            "没有可用的 stderr 输出".to_string()
        } else {
            stderr_tail
        };
        return Err(Error::command_failed(
            "ffmpeg",
            vec![command_description.to_string()],
            working_dir,
            status.code(),
            details.trim(),
        ));
    }
    if !iterator_errors.is_empty() {
        return Err(Error::video_conversion(format!(
            "FFmpeg输出解析失败: {}",
            iterator_errors.join(" | ")
        )));
    }
    validate_hls_output(output_dir)
}

pub(super) fn validate_hls_output(output_dir: &Path) -> Result<()> {
    let playlist_path = output_dir.join("index.m3u8");
    let playlist = fs::read_to_string(&playlist_path).map_err(|error| {
        Error::video_conversion(format!(
            "无法读取输出播放列表 {}: {error}",
            playlist_path.display()
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
                "播放列表引用的分片不存在或不可读 {}: {error}",
                segment_path.display()
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

pub(super) struct StderrTail {
    lines: VecDeque<String>,
    bytes: usize,
    limit: usize,
}

impl StderrTail {
    pub(super) fn new(limit: usize) -> Self {
        Self {
            lines: VecDeque::new(),
            bytes: 0,
            limit,
        }
    }

    pub(super) fn push(&mut self, message: &str) {
        if message.is_empty() {
            return;
        }
        self.bytes += message.len();
        self.lines.push_back(message.to_string());
        while self.bytes > self.limit && self.lines.len() > 1 {
            if let Some(removed) = self.lines.pop_front() {
                self.bytes = self.bytes.saturating_sub(removed.len());
            }
        }
    }

    pub(super) fn as_string(&self) -> String {
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
