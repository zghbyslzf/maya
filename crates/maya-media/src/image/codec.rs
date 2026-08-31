use super::{CompressionOptions, CompressionOutcome, OutputMode};
use maya_core::{Error, Result};
use maya_fs::atomic_write;
use oxipng::{optimize_from_memory, Options};
use std::fs;
use std::io::{BufReader, BufWriter, Read, Write};
use std::path::{Path, PathBuf};

pub(super) fn compress_image(
    path: &Path,
    options: &CompressionOptions,
) -> Result<CompressionOutcome> {
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

pub(super) fn write_image_atomically<F>(output_path: &Path, write_fn: F) -> Result<()>
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
