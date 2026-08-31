use maya_core::{Error, FailurePolicy};
use std::path::PathBuf;
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
