use maya_common::error::{Error, Result};
use maya_common::file_utils::{atomic_write, find_files_by_extension};
use maya_common::{FailurePolicy, NoopProgress, ProgressEvent, ProgressSink};
use oxipng::{optimize_from_memory, Options};
use rayon::prelude::*;
use std::fs;
use std::io::{BufReader, BufWriter, Read, Write};
use std::path::{Path, PathBuf};
use std::str::FromStr;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ImageType {
    Png,
    Jpeg,
    All,
}

impl FromStr for ImageType {
    type Err = String;

    fn from_str(value: &str) -> std::result::Result<Self, Self::Err> {
        match value.to_ascii_lowercase().as_str() {
            "png" => Ok(Self::Png),
            "jpg" | "jpeg" => Ok(Self::Jpeg),
            "all" => Ok(Self::All),
            _ => Err(format!("不支持的图片类型: {value}")),
        }
    }
}

pub fn extensions_for_type(image_type: ImageType) -> &'static [&'static str] {
    match image_type {
        ImageType::Png => &["png"],
        ImageType::Jpeg => &["jpg", "jpeg"],
        ImageType::All => &["png", "jpg", "jpeg"],
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum OutputMode {
    #[default]
    Overwrite,
    NewFile,
}

#[derive(Debug, Clone)]
pub struct CompressionOptions {
    pub image_type: ImageType,
    pub output_mode: OutputMode,
    pub jpeg_quality: u8,
    pub failure_policy: FailurePolicy,
}

impl Default for CompressionOptions {
    fn default() -> Self {
        Self {
            image_type: ImageType::All,
            output_mode: OutputMode::Overwrite,
            jpeg_quality: 80,
            failure_policy: FailurePolicy::Continue,
        }
    }
}

#[derive(Debug)]
pub enum CompressionOutcome {
    Compressed {
        path: PathBuf,
        output_path: PathBuf,
        bytes_before: u64,
        bytes_after: u64,
    },
    Skipped {
        path: PathBuf,
        reason: String,
    },
    Failed {
        path: PathBuf,
        error: Error,
    },
}

#[derive(Debug, Default)]
pub struct CompressionReport {
    pub scanned: usize,
    pub succeeded: usize,
    pub failed: usize,
    pub skipped: usize,
    pub bytes_before: u64,
    pub bytes_after: u64,
    pub items: Vec<CompressionOutcome>,
}

impl CompressionReport {
    pub fn compression_ratio(&self) -> f64 {
        if self.bytes_before == 0 {
            0.0
        } else {
            1.0 - self.bytes_after as f64 / self.bytes_before as f64
        }
    }
}

/// 压缩路径下的图片；终端展示由调用方负责。
pub fn compress_images(path: &Path, options: &CompressionOptions) -> Result<CompressionReport> {
    compress_images_with_progress(path, options, &NoopProgress)
}

pub fn compress_images_with_progress(
    path: &Path,
    options: &CompressionOptions,
    progress: &dyn ProgressSink,
) -> Result<CompressionReport> {
    if !(1..=100).contains(&options.jpeg_quality) {
        return Err(Error::invalid_argument("JPEG 质量必须在 1 到 100 之间"));
    }

    let files = find_files_by_extension(path, extensions_for_type(options.image_type))?;
    let total = files.len();
    progress.emit(ProgressEvent::Started {
        operation: "压缩图片".to_string(),
        total: Some(total as u64),
    });

    let items = match options.failure_policy {
        FailurePolicy::Continue => files
            .par_iter()
            .map(|path| {
                let outcome = compress_image(path, options).unwrap_or_else(|error| {
                    CompressionOutcome::Failed {
                        path: path.clone(),
                        error,
                    }
                });
                progress.emit(ProgressEvent::Advanced {
                    increment: 1,
                    total: Some(total as u64),
                    message: Some(path.display().to_string()),
                });
                outcome
            })
            .collect(),
        FailurePolicy::FailFast => {
            let mut items = Vec::with_capacity(total);
            for path in files {
                let outcome = match compress_image(&path, options) {
                    Ok(outcome) => outcome,
                    Err(error) => {
                        progress.emit(ProgressEvent::Finished);
                        return Err(error);
                    }
                };
                progress.emit(ProgressEvent::Advanced {
                    increment: 1,
                    total: Some(total as u64),
                    message: Some(path.display().to_string()),
                });
                items.push(outcome);
            }
            items
        }
    };

    progress.emit(ProgressEvent::Finished);
    Ok(summarize(items))
}

fn summarize(items: Vec<CompressionOutcome>) -> CompressionReport {
    let mut report = CompressionReport {
        scanned: items.len(),
        items,
        ..CompressionReport::default()
    };

    for outcome in &report.items {
        match outcome {
            CompressionOutcome::Compressed {
                bytes_before,
                bytes_after,
                ..
            } => {
                report.succeeded += 1;
                report.bytes_before += bytes_before;
                report.bytes_after += bytes_after;
            }
            CompressionOutcome::Skipped { .. } => report.skipped += 1,
            CompressionOutcome::Failed { .. } => report.failed += 1,
        }
    }

    report
}

fn compress_image(path: &Path, options: &CompressionOptions) -> Result<CompressionOutcome> {
    let bytes_before = fs::metadata(path)
        .map_err(|error| Error::io_context("读取图片元数据", path, error))?
        .len();
    let extension = path
        .extension()
        .and_then(|value| value.to_str())
        .ok_or_else(|| Error::path(format!("图片路径没有有效扩展名: {}", path.display())))?;

    let encoded = if extension.eq_ignore_ascii_case("png") {
        encode_png(path)?
    } else if extension.eq_ignore_ascii_case("jpg") || extension.eq_ignore_ascii_case("jpeg") {
        encode_jpeg(path, options.jpeg_quality)?
    } else {
        return Err(Error::compression(format!("不支持的图片格式: {extension}")));
    };

    if options.output_mode == OutputMode::Overwrite && encoded.len() as u64 >= bytes_before {
        return Ok(CompressionOutcome::Skipped {
            path: path.to_path_buf(),
            reason: "压缩后文件未变小".to_string(),
        });
    }

    let output_path = match options.output_mode {
        OutputMode::Overwrite => path.to_path_buf(),
        OutputMode::NewFile => create_output_path(path, "_c")?,
    };
    write_image_atomically(&output_path, |writer| {
        writer.write_all(&encoded)?;
        Ok(())
    })?;
    let bytes_after = fs::metadata(&output_path)
        .map_err(|error| Error::io_context("读取压缩图片元数据", &output_path, error))?
        .len();

    Ok(CompressionOutcome::Compressed {
        path: path.to_path_buf(),
        output_path,
        bytes_before,
        bytes_after,
    })
}

fn encode_png(path: &Path) -> Result<Vec<u8>> {
    let file =
        fs::File::open(path).map_err(|error| Error::io_context("打开 PNG 图片", path, error))?;
    let mut reader = BufReader::new(file);
    let mut input = Vec::new();
    reader
        .read_to_end(&mut input)
        .map_err(|error| Error::io_context("读取 PNG 图片", path, error))?;
    optimize_from_memory(&input, &Options::default())
        .map_err(|error| Error::compression(format!("PNG 优化失败: {error}")))
}

fn encode_jpeg(path: &Path, quality: u8) -> Result<Vec<u8>> {
    let file =
        fs::File::open(path).map_err(|error| Error::io_context("打开 JPEG 图片", path, error))?;
    let reader = BufReader::new(file);
    let image = image::load(reader, image::ImageFormat::Jpeg)
        .map_err(|error| Error::compression(format!("JPEG 解码失败: {error}")))?;
    let mut buffer = Vec::new();
    image::codecs::jpeg::JpegEncoder::new_with_quality(&mut buffer, quality)
        .encode_image(&image)
        .map_err(|error| Error::compression(format!("JPEG 编码失败: {error}")))?;
    Ok(buffer)
}

fn write_image_atomically<F>(output_path: &Path, write_fn: F) -> Result<()>
where
    F: FnOnce(&mut BufWriter<&mut fs::File>) -> Result<()>,
{
    atomic_write(output_path, |file, _| {
        let mut writer = BufWriter::new(file);
        write_fn(&mut writer)?;
        writer.flush()?;
        Ok(())
    })
}

fn create_output_path(input_path: &Path, suffix: &str) -> Result<PathBuf> {
    let stem = input_path
        .file_stem()
        .ok_or_else(|| Error::path(format!("图片路径没有文件名: {}", input_path.display())))?;
    let extension = input_path
        .extension()
        .ok_or_else(|| Error::path(format!("图片路径没有扩展名: {}", input_path.display())))?;

    Ok(input_path.with_file_name(format!(
        "{}{}.{}",
        stem.to_string_lossy(),
        suffix,
        extension.to_string_lossy()
    )))
}

#[cfg(test)]
mod tests {
    use super::*;
    use image::{ImageBuffer, Rgba};
    use tempfile::tempdir;

    #[test]
    fn jpg_and_jpeg_are_the_same_format() {
        assert_eq!(ImageType::from_str("jpg").unwrap(), ImageType::Jpeg);
        assert_eq!(ImageType::from_str("JPEG").unwrap(), ImageType::Jpeg);
    }

    #[test]
    fn rejects_invalid_jpeg_quality() {
        let temp_dir = tempdir().unwrap();
        let options = CompressionOptions {
            jpeg_quality: 0,
            ..CompressionOptions::default()
        };
        assert!(matches!(
            compress_images(temp_dir.path(), &options),
            Err(Error::InvalidArgument(_))
        ));
    }

    #[test]
    fn creates_new_png_and_reports_it() {
        let temp_dir = tempdir().unwrap();
        let path = temp_dir.path().join("test.png");
        let image: ImageBuffer<Rgba<u8>, _> =
            ImageBuffer::from_pixel(16, 16, Rgba([255, 0, 0, 255]));
        image.save(&path).unwrap();
        let options = CompressionOptions {
            image_type: ImageType::Png,
            output_mode: OutputMode::NewFile,
            ..CompressionOptions::default()
        };

        let report = compress_images(temp_dir.path(), &options).unwrap();

        assert_eq!(report.scanned, 1);
        assert_eq!(report.succeeded, 1);
        assert_eq!(report.failed, 0);
        assert!(temp_dir.path().join("test_c.png").is_file());
    }

    #[test]
    fn corrupt_png_is_a_reported_failure_in_continue_mode() {
        let temp_dir = tempdir().unwrap();
        fs::write(temp_dir.path().join("broken.png"), b"not a png").unwrap();
        let options = CompressionOptions {
            image_type: ImageType::Png,
            ..CompressionOptions::default()
        };

        let report = compress_images(temp_dir.path(), &options).unwrap();

        assert_eq!(report.scanned, 1);
        assert_eq!(report.failed, 1);
        assert!(matches!(report.items[0], CompressionOutcome::Failed { .. }));
    }

    #[test]
    fn fail_fast_returns_the_first_error() {
        let temp_dir = tempdir().unwrap();
        fs::write(temp_dir.path().join("broken.png"), b"not a png").unwrap();
        let options = CompressionOptions {
            image_type: ImageType::Png,
            failure_policy: FailurePolicy::FailFast,
            ..CompressionOptions::default()
        };

        assert!(compress_images(temp_dir.path(), &options).is_err());
    }

    #[test]
    fn atomic_image_write_preserves_existing_file_after_failure() {
        let temp_dir = tempdir().unwrap();
        let path = temp_dir.path().join("test.png");
        fs::write(&path, b"original image bytes").unwrap();

        let result = write_image_atomically(&path, |writer| {
            writer.write_all(b"partial image bytes")?;
            Err(Error::compression("模拟图片编码失败"))
        });

        assert!(result.is_err());
        assert_eq!(fs::read(&path).unwrap(), b"original image bytes");
        assert_eq!(fs::read_dir(temp_dir.path()).unwrap().count(), 1);
    }
}
