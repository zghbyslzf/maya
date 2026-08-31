use crate::cli::{FailureMode, MediaFormat};
use crate::presenter::Presenter;
use maya_core::{Error, ProgressSink, Result};
use maya_media::video::ConversionOptions;
use std::path::Path;
use std::sync::Arc;

pub async fn handle_transform_ops(
    types: &[MediaFormat],
    path: &Path,
    failure_policy: FailureMode,
    presenter: Arc<Presenter>,
) -> Result<()> {
    if types != [MediaFormat::Mp4, MediaFormat::M3u8] {
        return Err(Error::invalid_argument("目前只支持 mp4 -> m3u8 转换"));
    }
    let options = ConversionOptions {
        failure_policy: failure_policy.into(),
        ..ConversionOptions::default()
    };
    let progress: Arc<dyn ProgressSink> = presenter.clone();
    let report =
        maya_media::video::convert_mp4_to_m3u8_with_progress(path, &options, progress).await?;
    presenter.conversion(&report);
    if report.failed > 0 {
        return Err(Error::partial_failure(
            "视频转换",
            report.succeeded,
            report.failed,
        ));
    }
    Ok(())
}
