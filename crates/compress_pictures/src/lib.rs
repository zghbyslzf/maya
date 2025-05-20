use anyhow::{anyhow, Result};
use image::{self};
use oxipng::{optimize_from_memory, Options};
use std::fs;
use std::path::{Path, PathBuf};
use walkdir::WalkDir;

/// 压缩图片类型枚举
#[derive(Debug, PartialEq)]
pub enum ImageType {
    Png,
    Jpg,
    Jpeg,
    All,
}

impl ImageType {
    pub fn from_str(s: &str) -> Result<Self> {
        match s.to_lowercase().as_str() {
            "png" => Ok(ImageType::Png),
            "jpg" => Ok(ImageType::Jpg),
            "jpeg" => Ok(ImageType::Jpeg),
            "all" => Ok(ImageType::All),
            _ => Err(anyhow!("不支持的图片类型: {}", s)),
        }
    }
    
    fn is_supported_extension(&self, ext: &str) -> bool {
        let ext = ext.to_lowercase();
        match self {
            ImageType::Png => ext == "png",
            ImageType::Jpg => ext == "jpg",
            ImageType::Jpeg => ext == "jpeg",
            ImageType::All => ext == "png" || ext == "jpg" || ext == "jpeg",
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
pub fn compress_images(path: &Path, img_type: ImageType, create_new_file: bool) -> Result<(u32, u32, f64)> {
    println!("开始压缩 {} 图片...", match img_type {
        ImageType::Png => "PNG",
        ImageType::Jpg => "JPG",
        ImageType::Jpeg => "JPEG",
        ImageType::All => "所有支持的",
    });

    let mut successful_compressions = 0;
    let mut failed_compressions = 0;
    let mut total_compression_ratio_sum = 0.0;
    let mut processed_files_count = 0; // 用于计算平均压缩率的分母

    for entry in WalkDir::new(path).into_iter().filter_map(|e| e.ok()) {
        let file_path = entry.path();
        
        if file_path.is_file() {
            if let Some(extension) = file_path.extension() {
                let ext_str = extension.to_string_lossy().to_string();
                
                if img_type.is_supported_extension(&ext_str) {
                    processed_files_count += 1; // 标记为已处理，无论成功与否
                    match compress_image(file_path, create_new_file) {
                        Ok(ratio) => {
                            successful_compressions += 1;
                            total_compression_ratio_sum += ratio;
                            println!("成功压缩: {} (压缩率: {:.2}%)", file_path.display(), ratio * 100.0);
                        }
                        Err(e) => {
                            failed_compressions += 1;
                            eprintln!("压缩失败 {}: {}", file_path.display(), e);
                        }
                    }
                }
            }
        }
    }

    let avg_compression_ratio = if successful_compressions > 0 { // 平均压缩率只基于成功压缩的文件
        total_compression_ratio_sum / successful_compressions as f64
    } else {
        0.0
    };

    println!("
--- 压缩总结 ---");
    println!("总共处理图片数量: {}", processed_files_count);
    println!("成功压缩文件数量: {}", successful_compressions);
    println!("失败压缩文件数量: {}", failed_compressions);
    if successful_compressions > 0 {
        println!("成功文件的平均压缩率: {:.2}%", avg_compression_ratio * 100.0);
    } else {
        println!("没有文件被成功压缩。");
    }
    println!("--------------------");

    Ok((successful_compressions, failed_compressions, avg_compression_ratio))
}

/// 压缩单个图片
fn compress_image(image_path: &Path, create_new_file: bool) -> Result<f64> {
    let original_size = fs::metadata(image_path)?.len() as f64;
    
    if let Some(extension) = image_path.extension() {
        let ext = extension.to_string_lossy().to_lowercase();
        
        match ext.as_str() {
            "png" => compress_png(image_path, create_new_file, original_size),
            "jpg" | "jpeg" => compress_jpg(image_path, create_new_file, original_size),
            _ => Err(anyhow!("不支持的图片格式: {}", ext)),
        }
    } else {
        Err(anyhow!("文件没有扩展名"))
    }
}

/// 压缩PNG图片
fn compress_png(image_path: &Path, create_new_file: bool, original_size: f64) -> Result<f64> {
    let input_data = fs::read(image_path)?;
    
    // 使用默认优化选项
    let options = Options::default();
    
    // 优化PNG到内存
    let output_data_in_memory = optimize_from_memory(&input_data, &options)?;
    let compressed_size_in_memory = output_data_in_memory.len() as f64;

    if !create_new_file { // 覆写模式
        if compressed_size_in_memory >= original_size {
            println!(
                "提示: 文件 {} (原始大小: {:.0} bytes) 压缩后大小为 {:.0} bytes，未变小或反而变大，跳过覆写。",
                image_path.display(),
                original_size,
                compressed_size_in_memory
            );
            return Ok(0.0); // 返回0%压缩率，表示未进行有效压缩
        }
        // 体积变小，执行覆写
        fs::write(image_path, output_data_in_memory)?;
        let compression_ratio = 1.0 - (compressed_size_in_memory / original_size);
        Ok(compression_ratio)
    } else { // 创建新文件模式
        let output_path = create_output_path(image_path, "_c");
        fs::write(&output_path, output_data_in_memory)?;
        
        // 对于新文件，我们仍然基于其在磁盘上的最终大小计算压缩率
        let final_compressed_size_on_disk = fs::metadata(&output_path)?.len() as f64;
        let compression_ratio = 1.0 - (final_compressed_size_on_disk / original_size);
        Ok(compression_ratio)
    }
}

/// 压缩JPG/JPEG图片
fn compress_jpg(image_path: &Path, create_new_file: bool, original_size: f64) -> Result<f64> {
    // 打开图片
    let img = image::open(image_path)?;
    
    // 尝试将图片编码到内存缓冲区
    let mut buffer = Vec::new();
    // // 使用 image::ImageEncoder::write_image 进行更通用的编码
    // // 需要选择一个 Encoder, 对于 Jpeg， image::codecs::jpeg::JpegEncoder 适用
    // let mut encoder = image::codecs::jpeg::JpegEncoder::new_with_quality(&mut buffer, 85); // 假设质量为85
    
    // 获取图像的原始数据和尺寸
    // 注意：image crate 的 `img.save_with_format` 或 `img.write_to` 对于Jpeg是简便方法。
    // 为了获取到内存中的字节流，我们需要使用更底层的 `ImageEncoder` API
    // 或者使用 `img.write_to(&mut Cursor::new(&mut buffer), image::ImageFormat::Jpeg)`

    // 使用 img.write_to 更简单直接
    let mut cursor = std::io::Cursor::new(&mut buffer);
    img.write_to(&mut cursor, image::ImageFormat::Jpeg)?; // 使用image crate的默认质量或特定质量
    let compressed_size_in_memory = buffer.len() as f64;

    if !create_new_file { // 覆写模式
        if compressed_size_in_memory >= original_size {
            println!(
                "提示: 文件 {} (原始大小: {:.0} bytes) 压缩后大小为 {:.0} bytes，未变小或反而变大，跳过覆写。",
                image_path.display(),
                original_size,
                compressed_size_in_memory
            );
            return Ok(0.0); // 返回0%压缩率
        }
        // 体积变小，执行覆写
        fs::write(image_path, buffer)?;
        let compression_ratio = 1.0 - (compressed_size_in_memory / original_size);
        Ok(compression_ratio)
    } else { // 创建新文件模式
        let output_path = create_output_path(image_path, "_c");
        // 可以直接写入内存中的buffer，或者让image库再次保存（可能更保险，确保元数据等正确写入）
        // fs::write(&output_path, buffer)?;
        img.save_with_format(&output_path, image::ImageFormat::Jpeg)?; // 保持与之前逻辑一致性

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
