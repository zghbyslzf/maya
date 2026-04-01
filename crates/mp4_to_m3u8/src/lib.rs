use ffmpeg_sidecar::{command::FfmpegCommand, download::auto_download, event::FfmpegEvent};
use indicatif::{ProgressBar, ProgressStyle};
use maya_common::error::{Error, Result};
use maya_common::file_utils::find_files_by_extension;
use std::fs;
use std::path::Path;
use std::time::Duration;

/// mp4转m3u8功能
///
/// # 参数
/// * `path` - 搜索mp4文件的目录路径
///
/// # 返回
/// * `Result<(u32, u32)>` - (成功转换的文件数量, 失败的文件数量)
pub async fn convert_mp4_to_m3u8(path: &Path) -> Result<(u32, u32)> {
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

        match convert_single_mp4(mp4_file).await {
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
async fn convert_single_mp4(mp4_file: &Path) -> Result<()> {
    // 创建输出目录
    let file_stem = mp4_file
        .file_stem()
        .ok_or_else(|| Error::video_conversion("无法获取文件名"))?
        .to_string_lossy();

    let output_dir = mp4_file
        .parent()
        .ok_or_else(|| Error::video_conversion("无法获取文件目录"))?
        .join(&*file_stem);

    // 如果目录已存在，先删除
    if output_dir.exists() {
        fs::remove_dir_all(&output_dir)?;
    }
    fs::create_dir_all(&output_dir)?;

    let m3u8_file = output_dir.join("index.m3u8");

    println!("🔄 开始转换...");

    // 获取视频时长用于计算进度
    let duration = get_video_duration(mp4_file)?;

    // 创建进度条
    let pb = ProgressBar::new(100);
    pb.set_style(
        ProgressStyle::default_bar()
            .template("{spinner:.green} [{elapsed_precise}] [{bar:40.cyan/blue}] {pos}% {msg}")
            .unwrap()
            .progress_chars("█▉▊▋▌▍▎▏  "),
    );
    pb.set_message("正在转换mp4到m3u8...");

    // 使用ffmpeg-sidecar构建命令并监听进度
    let mut ffmpeg = FfmpegCommand::new()
        .input(mp4_file.to_string_lossy().as_ref())
        .args(["-c", "copy"])
        .args(["-start_number", "0"])
        .args(["-hls_time", "10"]) // 每个片段10秒
        .args(["-hls_list_size", "0"]) // 保留所有片段
        .args(["-f", "hls"])
        .args(["-progress", "pipe:1"]) // 输出进度到stdout
        .output(m3u8_file.to_string_lossy().as_ref())
        .overwrite() // 覆盖已存在的文件
        .spawn()
        .map_err(|e| Error::video_conversion(format!("FFmpeg启动失败: {}", e)))?;

    // 监听FFmpeg进度
    let iter = ffmpeg
        .iter()
        .map_err(|e| Error::video_conversion(format!("FFmpeg迭代器错误: {}", e)))?;
    let mut last_progress = 0u64;

    for event in iter {
        match event {
            FfmpegEvent::Progress(progress) => {
                // 解析时间字符串（格式如 "00:01:23.45"）
                if let Ok(current_seconds) = parse_time_string(&progress.time) {
                    let progress_percent = if duration > 0.0 {
                        ((current_seconds / duration) * 100.0).min(100.0) as u64
                    } else {
                        // 如果没有总时长，显示经过的时间
                        (current_seconds as u64) % 100
                    };

                    if progress_percent != last_progress {
                        pb.set_position(progress_percent);
                        pb.set_message(format!(
                            "正在转换... {:.1}s{}",
                            current_seconds,
                            if duration > 0.0 {
                                format!(" / {:.1}s", duration)
                            } else {
                                String::new()
                            }
                        ));
                        last_progress = progress_percent;
                    }
                }
            }
            FfmpegEvent::LogEOF => {
                pb.set_position(100);
                pb.set_message("转换完成");
                break;
            }
            _ => {}
        }
    }

    pb.finish_with_message("✅ 转换完成");

    // 检查输出文件是否存在
    if !m3u8_file.exists() {
        return Err(Error::video_conversion("转换完成但未找到输出文件"));
    }

    println!("📁 输出目录: {}", output_dir.display());

    Ok(())
}

/// 确保FFmpeg可用，如果不可用则自动下载
async fn ensure_ffmpeg_available() -> Result<()> {
    // 尝试检查FFmpeg是否已经可用
    if is_ffmpeg_available() {
        return Ok(());
    }

    println!("🔍 FFmpeg未找到，正在自动下载...");

    // 创建下载进度条
    let pb = ProgressBar::new(100);
    pb.set_style(
        ProgressStyle::default_bar()
            .template("{spinner:.blue} [{elapsed_precise}] [{bar:40.cyan/blue}] {pos}% {msg}")
            .unwrap()
            .progress_chars("█▉▊▋▌▍▎▏  "),
    );
    pb.set_message("正在下载FFmpeg二进制文件...");

    // 模拟下载进度（因为auto_download不支持进度回调）
    tokio::spawn({
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
    let download_result = tokio::task::spawn_blocking(|| auto_download())
        .await
        .map_err(|e| Error::video_conversion(format!("FFmpeg下载任务失败: {}", e)))?;

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

/// 检查ffmpeg是否可用
fn is_ffmpeg_available() -> bool {
    // 简单的检查方式：尝试执行FFmpeg版本命令
    match std::process::Command::new("ffmpeg")
        .args(["-version"])
        .output()
    {
        Ok(output) => output.status.success(),
        Err(_) => false,
    }
}

/// 获取视频时长
fn get_video_duration(mp4_file: &Path) -> Result<f64> {
    let output = std::process::Command::new("ffprobe")
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
        let hours: f64 = parts[0].parse().unwrap_or(0.0);
        let minutes: f64 = parts[1].parse().unwrap_or(0.0);
        let seconds: f64 = parts[2].parse().unwrap_or(0.0);
        Ok(hours * 3600.0 + minutes * 60.0 + seconds)
    } else {
        Err(Error::video_conversion(format!(
            "无效的时间格式: {}",
            time_str
        )))
    }
}
