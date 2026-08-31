mod codec;
mod model;

pub use model::*;

use codec::compress_image;
#[cfg(test)]
use codec::write_image_atomically;
use maya_core::{Error, FailurePolicy, NoopProgress, ProgressEvent, ProgressSink, Result};
use maya_fs::find_files_by_extension;
use rayon::prelude::*;
use std::path::Path;

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

#[cfg(test)]
mod tests {
    use super::*;
    use image::{ImageBuffer, Rgba};
    use std::fs;
    use std::io::Write;
    use std::str::FromStr;
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
