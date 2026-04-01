use image::{self};
use maya_common::error::{Error, Result};
use maya_common::file_utils::find_files_by_extension;
use oxipng::{optimize_from_memory, Options};
use rayon::prelude::*;
use std::fs;
use std::io::{BufReader, BufWriter};
use std::path::{Path, PathBuf};
use std::str::FromStr;

const STREAMING_THRESHOLD: u64 = 10 * 1024 * 1024; // 10 MB
/// 压缩图片类型枚举
#[derive(Debug, PartialEq)]
pub enum ImageType {
    Png,
    Jpg,
    Jpeg,
    All,
}

impl FromStr for ImageType {
    type Err = String;

    fn from_str(s: &str) -> std::result::Result<Self, Self::Err> {
        match s.to_lowercase().as_str() {
            "png" => Ok(ImageType::Png),
            "jpg" => Ok(ImageType::Jpg),
            "jpeg" => Ok(ImageType::Jpeg),
            "all" => Ok(ImageType::All),
            _ => Err(format!("不支持的图片类型: {}", s)),
        }
    }
}

/// 压缩图片函数
///
/// # 参数
/// * `path` - 目录路径
/// * `img_type` - 图片类型
/// * `create_new_file` - 是否创建新文件（添加_c后缀）而不是覆盖原文件
///
/// # 返回
/// * `Result<(u32, u32, f64)>` - (成功压缩的文件数量, 失败的文件数量, 平均压缩率)
pub fn compress_images(
    path: &Path,
    img_type: ImageType,
    create_new_file: bool,
) -> Result<(u32, u32, f64)> {
    println!(
        "开始压缩 {} 图片...",
        match img_type {
            ImageType::Png => "PNG",
            ImageType::Jpg => "JPG",
            ImageType::Jpeg => "JPEG",
            ImageType::All => "所有支持的",
        }
    );

    let mut successful_compressions = 0;
    let mut failed_compressions = 0;
    let mut total_compression_ratio_sum = 0.0;
    let mut processed_files_count = 0; // 用于计算平均压缩率的分母

    // 根据图片类型获取扩展名列表
    let extensions: Vec<&str> = match img_type {
        ImageType::Png => vec!["png"],
        ImageType::Jpg => vec!["jpg"],
        ImageType::Jpeg => vec!["jpeg"],
        ImageType::All => vec!["png", "jpg", "jpeg"],
    };

    // 使用共享的文件遍历函数查找匹配的图片文件
    let image_files = find_files_by_extension(path, &extensions)?;

    for file_path in image_files {
        processed_files_count += 1; // 标记为已处理，无论成功与否
        match compress_image(&file_path, create_new_file) {
            Ok(ratio) => {
                successful_compressions += 1;
                total_compression_ratio_sum += ratio;
                println!(
                    "成功压缩: {} (压缩率: {:.2}%)",
                    file_path.display(),
                    ratio * 100.0
                );
            }
            Err(e) => {
                failed_compressions += 1;
                eprintln!("压缩失败 {}: {}", file_path.display(), e);
            }
        }
    }

    let avg_compression_ratio = if successful_compressions > 0 {
        // 平均压缩率只基于成功压缩的文件
        total_compression_ratio_sum / successful_compressions as f64
    } else {
        0.0
    };

    println!(
        "
--- 压缩总结 ---"
    );
    println!("总共处理图片数量: {}", processed_files_count);
    println!("成功压缩文件数量: {}", successful_compressions);
    println!("失败压缩文件数量: {}", failed_compressions);
    if successful_compressions > 0 {
        println!(
            "成功文件的平均压缩率: {:.2}%",
            avg_compression_ratio * 100.0
        );
    } else {
        println!("没有文件被成功压缩。");
    }
    println!("--------------------");

    Ok((
        successful_compressions,
        failed_compressions,
        avg_compression_ratio,
    ))
}

/// 并行压缩图片函数
///
/// # 参数
/// * `path` - 目录路径
/// * `img_type` - 图片类型
/// * `create_new_file` - 是否创建新文件（添加_c后缀）而不是覆盖原文件
///
/// # 返回
/// * `Result<(u32, u32, f64)>` - (成功压缩的文件数量, 失败的文件数量, 平均压缩率)
pub fn compress_images_parallel(
    path: &Path,
    img_type: ImageType,
    create_new_file: bool,
) -> Result<(u32, u32, f64)> {
    println!(
        "开始并行压缩 {} 图片...",
        match img_type {
            ImageType::Png => "PNG",
            ImageType::Jpg => "JPG",
            ImageType::Jpeg => "JPEG",
            ImageType::All => "所有支持的",
        }
    );

    // 根据图片类型获取扩展名列表
    let extensions: Vec<&str> = match img_type {
        ImageType::Png => vec!["png"],
        ImageType::Jpg => vec!["jpg"],
        ImageType::Jpeg => vec!["jpeg"],
        ImageType::All => vec!["png", "jpg", "jpeg"],
    };

    // 使用共享的文件遍历函数查找匹配的图片文件
    let image_files = find_files_by_extension(path, &extensions)?;

    // 并行处理每个文件
    let results: Vec<(PathBuf, Result<f64>)> = image_files
        .par_iter()
        .map(|file_path| {
            let result = compress_image(file_path, create_new_file);
            (file_path.clone(), result)
        })
        .collect();

    let mut successful_compressions = 0;
    let mut failed_compressions = 0;
    let mut total_compression_ratio_sum = 0.0;

    // 处理结果并输出信息
    for (file_path, result) in results.iter() {
        match result {
            Ok(ratio) => {
                successful_compressions += 1;
                total_compression_ratio_sum += *ratio;
                println!(
                    "成功压缩: {} (压缩率: {:.2}%)",
                    file_path.display(),
                    *ratio * 100.0
                );
            }
            Err(e) => {
                failed_compressions += 1;
                eprintln!("压缩失败 {}: {}", file_path.display(), e);
            }
        }
    }

    let avg_compression_ratio = if successful_compressions > 0 {
        // 平均压缩率只基于成功压缩的文件
        total_compression_ratio_sum / successful_compressions as f64
    } else {
        0.0
    };

    println!(
        "
--- 压缩总结 ---"
    );
    println!("总共处理图片数量: {}", results.len());
    println!("成功压缩文件数量: {}", successful_compressions);
    println!("失败压缩文件数量: {}", failed_compressions);
    if successful_compressions > 0 {
        println!(
            "成功文件的平均压缩率: {:.2}%",
            avg_compression_ratio * 100.0
        );
    } else {
        println!("没有文件被成功压缩。");
    }
    println!("--------------------");

    Ok((
        successful_compressions,
        failed_compressions,
        avg_compression_ratio,
    ))
}

/// 压缩单个图片
fn compress_image(image_path: &Path, create_new_file: bool) -> Result<f64> {
    let original_size = fs::metadata(image_path)?.len() as f64;

    if let Some(extension) = image_path.extension() {
        let ext = extension.to_string_lossy().to_lowercase();

        match ext.as_str() {
            "png" => compress_png(image_path, create_new_file, original_size),
            "jpg" | "jpeg" => compress_jpg(image_path, create_new_file, original_size),
            _ => Err(Error::compression(format!("不支持的图片格式: {}", ext))),
        }
    } else {
        Err(Error::compression("文件没有扩展名"))
    }
}

/// 压缩PNG图片
fn compress_png(image_path: &Path, create_new_file: bool, original_size: f64) -> Result<f64> {
    if original_size as u64 > STREAMING_THRESHOLD {
        println!("文件大小超过阈值，使用缓冲IO处理: {}", image_path.display());
    }
    // 使用 BufReader 读取文件
    let file = fs::File::open(image_path)?;
    let mut reader = BufReader::new(file);
    let mut input_data = Vec::new();
    use std::io::Read;
    reader.read_to_end(&mut input_data)?;

    // 使用默认优化选项
    let options = Options::default();

    // 优化PNG到内存
    let output_data_in_memory = optimize_from_memory(&input_data, &options)
        .map_err(|e| Error::compression(format!("PNG优化失败: {}", e)))?;
    let compressed_size_in_memory = output_data_in_memory.len() as f64;

    if !create_new_file {
        // 覆写模式
        if compressed_size_in_memory >= original_size {
            println!(
                "提示: 文件 {} (原始大小: {:.0} bytes) 压缩后大小为 {:.0} bytes，未变小或反而变大，跳过覆写。",
                image_path.display(),
                original_size,
                compressed_size_in_memory
            );
            return Ok(0.0); // 返回0%压缩率，表示未进行有效压缩
        }
        // 体积变小，执行覆写，使用 BufWriter
        let file = fs::File::create(image_path)?;
        let mut writer = BufWriter::new(file);
        use std::io::Write;
        writer.write_all(&output_data_in_memory)?;
        writer.flush()?;
        let compression_ratio = 1.0 - (compressed_size_in_memory / original_size);
        Ok(compression_ratio)
    } else {
        // 创建新文件模式
        let output_path = create_output_path(image_path, "_c");
        let file = fs::File::create(&output_path)?;
        let mut writer = BufWriter::new(file);
        use std::io::Write;
        writer.write_all(&output_data_in_memory)?;
        writer.flush()?;

        // 对于新文件，我们仍然基于其在磁盘上的最终大小计算压缩率
        let final_compressed_size_on_disk = fs::metadata(&output_path)?.len() as f64;
        let compression_ratio = 1.0 - (final_compressed_size_on_disk / original_size);
        Ok(compression_ratio)
    }
}

/// 压缩JPG/JPEG图片
fn compress_jpg(image_path: &Path, create_new_file: bool, original_size: f64) -> Result<f64> {
    if original_size as u64 > STREAMING_THRESHOLD {
        println!("文件大小超过阈值，使用缓冲IO处理: {}", image_path.display());
    }
    // 使用 BufReader 打开图片
    let file = fs::File::open(image_path)?;
    let reader = BufReader::new(file);
    let img = image::load(reader, image::ImageFormat::Jpeg)
        .map_err(|e| Error::compression(format!("无法打开图片: {}", e)))?;

    // 尝试将图片编码到内存缓冲区
    let mut buffer = Vec::new();
    let mut cursor = std::io::Cursor::new(&mut buffer);
    img.write_to(&mut cursor, image::ImageFormat::Jpeg)
        .map_err(|e| Error::compression(format!("图片编码失败: {}", e)))?;
    let compressed_size_in_memory = buffer.len() as f64;

    if !create_new_file {
        // 覆写模式
        if compressed_size_in_memory >= original_size {
            println!(
                "提示: 文件 {} (原始大小: {:.0} bytes) 压缩后大小为 {:.0} bytes，未变小或反而变大，跳过覆写。",
                image_path.display(),
                original_size,
                compressed_size_in_memory
            );
            return Ok(0.0); // 返回0%压缩率
        }
        // 体积变小，执行覆写，使用 BufWriter
        let file = fs::File::create(image_path)?;
        let mut writer = BufWriter::new(file);
        use std::io::Write;
        writer.write_all(&buffer)?;
        writer.flush()?;
        let compression_ratio = 1.0 - (compressed_size_in_memory / original_size);
        Ok(compression_ratio)
    } else {
        // 创建新文件模式
        let output_path = create_output_path(image_path, "_c");
        let file = fs::File::create(&output_path)?;
        let mut writer = BufWriter::new(file);
        use std::io::Write;
        img.write_to(&mut writer, image::ImageFormat::Jpeg)
            .map_err(|e| Error::compression(format!("图片保存失败: {}", e)))?;
        writer.flush()?;

        let final_compressed_size_on_disk = fs::metadata(&output_path)?.len() as f64;
        let compression_ratio = 1.0 - (final_compressed_size_on_disk / original_size);
        Ok(compression_ratio)
    }
}

/// 创建输出路径（添加后缀）
fn create_output_path(input_path: &Path, suffix: &str) -> PathBuf {
    let stem = input_path.file_stem().unwrap_or_default();
    let extension = input_path.extension().unwrap_or_default();

    let new_filename = format!(
        "{}{}.{}",
        stem.to_string_lossy(),
        suffix,
        extension.to_string_lossy()
    );

    input_path.with_file_name(new_filename)
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::str::FromStr;

    #[test]
    fn test_image_type_from_str_valid() {
        assert_eq!(ImageType::from_str("png").unwrap(), ImageType::Png);
        assert_eq!(ImageType::from_str("PNG").unwrap(), ImageType::Png);
        assert_eq!(ImageType::from_str("jpg").unwrap(), ImageType::Jpg);
        assert_eq!(ImageType::from_str("JPG").unwrap(), ImageType::Jpg);
        assert_eq!(ImageType::from_str("jpeg").unwrap(), ImageType::Jpeg);
        assert_eq!(ImageType::from_str("JPEG").unwrap(), ImageType::Jpeg);
        assert_eq!(ImageType::from_str("all").unwrap(), ImageType::All);
        assert_eq!(ImageType::from_str("ALL").unwrap(), ImageType::All);
    }

    #[test]
    fn test_image_type_from_str_invalid() {
        assert!(ImageType::from_str("gif").is_err());
        assert!(ImageType::from_str("bmp").is_err());
        assert!(ImageType::from_str("").is_err());
        assert!(ImageType::from_str("png ").is_err());
    }

    #[test]
    fn test_image_type_partial_eq() {
        assert_eq!(ImageType::Png, ImageType::Png);
        assert_ne!(ImageType::Png, ImageType::Jpg);
        assert_ne!(ImageType::Jpg, ImageType::Jpeg);
    }

    #[test]
    fn test_compress_image_unsupported_extension() {
        use std::fs::File;
        use tempfile::tempdir;

        let temp_dir = tempdir().unwrap();
        let file_path = temp_dir.path().join("test.bmp");
        File::create(&file_path).unwrap();

        let result = compress_image(&file_path, false);
        assert!(result.is_err());
        let err = result.unwrap_err();
        assert!(err.to_string().contains("不支持的图片格式"));
    }

    #[test]
    fn test_compress_image_file_not_found() {
        let non_existent_path = std::path::Path::new("/non/existent/file.png");
        let result = compress_image(non_existent_path, false);
        assert!(result.is_err());
        // 应该是Io错误，但我们的错误类型会包装它
        let err = result.unwrap_err();
        assert!(err.to_string().contains("I/O错误"));
    }

    #[test]
    fn test_compress_image_png_create_new() {
        use image::{ImageBuffer, Rgba};
        use tempfile::tempdir;

        let temp_dir = tempdir().unwrap();
        let file_path = temp_dir.path().join("test.png");

        // 创建一个1x1像素的PNG图像
        let img: ImageBuffer<Rgba<u8>, _> = ImageBuffer::from_pixel(1, 1, Rgba([255, 0, 0, 255]));
        img.save(&file_path).unwrap();

        // 使用create_new_file=true进行压缩，这样不会修改原文件
        let result = compress_image(&file_path, true);
        // 压缩应该成功，但可能没有压缩率（因为图像很小）
        assert!(result.is_ok());
        let compression_ratio = result.unwrap();
        // 压缩率应该在0.0到1.0之间（可能是0.0，因为图像太小无法压缩）
        assert!(compression_ratio >= 0.0 && compression_ratio <= 1.0);

        // 检查新文件是否被创建（带有_c后缀）
        let new_file_path = temp_dir.path().join("test_c.png");
        assert!(new_file_path.exists());
    }
}
