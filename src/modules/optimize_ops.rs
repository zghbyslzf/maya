use crate::cli::{FailureMode, OptimizeType};
use crate::presenter::Presenter;
use maya_core::{Error, Result};
use maya_media::image::{CompressionOptions, ImageType, OutputMode};
use std::path::Path;

pub fn handle_optimize_ops(
    types: &[OptimizeType],
    path: &Path,
    new_file: bool,
    jpeg_quality: u8,
    failure_policy: FailureMode,
    presenter: &Presenter,
) -> Result<()> {
    let legacy_new_file = types.contains(&OptimizeType::LegacyNewFile);
    let formats: Vec<OptimizeType> = types
        .iter()
        .copied()
        .filter(|value| *value != OptimizeType::LegacyNewFile)
        .collect();
    if formats.len() != 1 {
        return Err(Error::invalid_argument(
            "必须且只能指定一种图片格式（png、jpg/jpeg 或 all）；n 仅表示新文件模式",
        ));
    }

    let image_type = match formats[0] {
        OptimizeType::Png => ImageType::Png,
        OptimizeType::Jpeg => ImageType::Jpeg,
        OptimizeType::All => ImageType::All,
        OptimizeType::LegacyNewFile => {
            return Err(Error::invalid_argument("n 必须与一种图片格式同时使用"));
        }
    };
    let options = CompressionOptions {
        image_type,
        output_mode: if new_file || legacy_new_file {
            OutputMode::NewFile
        } else {
            OutputMode::Overwrite
        },
        jpeg_quality,
        failure_policy: failure_policy.into(),
    };
    let report = maya_media::image::compress_images_with_progress(path, &options, presenter)?;
    presenter.compression(&report);
    if report.failed > 0 {
        return Err(Error::partial_failure(
            "图片压缩",
            report.succeeded,
            report.failed,
        ));
    }
    Ok(())
}
