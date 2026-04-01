use compress_pictures;
use maya_common::error::{Error, Result};
use maya_common::file_utils::find_files_by_extension;
use std::path::Path;

const PARALLEL_THRESHOLD: usize = 10;

pub fn handle_optimize_ops(types: &[String], path: &Path) -> Result<()> {
    if types.is_empty() {
        return Err(Error::invalid_argument("请指定要压缩的图片类型 (png/jpg/jpeg/all)".to_string()));
    }

    // 检查是否有n参数，表示创建新文件而不是覆盖
    let create_new_file = types.iter().any(|t| t == "n");

    // 找到图片类型参数
    let img_type_str = types
        .iter()
        .find(|&t| t != "n")
        .map(|s| s.as_str())
        .unwrap_or("all"); // 默认压缩所有类型

    let img_type = img_type_str.parse::<compress_pictures::ImageType>()
        .map_err(|e| Error::invalid_argument(format!("图片类型参数 '{}' 错误: {}", img_type_str, e)))?;

    // 根据图片类型获取扩展名列表
    let extensions: Vec<&str> = match img_type {
        compress_pictures::ImageType::Png => vec!["png"],
        compress_pictures::ImageType::Jpg => vec!["jpg"],
        compress_pictures::ImageType::Jpeg => vec!["jpeg"],
        compress_pictures::ImageType::All => vec!["png", "jpg", "jpeg"],
    };

    // 获取文件列表以决定是否使用并行处理
    let image_files = find_files_by_extension(path, &extensions)?;
    let file_count = image_files.len();

    let (successful_compressions, failed_compressions, _avg_compression_ratio) = if file_count >= PARALLEL_THRESHOLD {
        println!("检测到 {} 个文件，启用并行压缩...", file_count);
        compress_pictures::compress_images_parallel(path, img_type, create_new_file)?
    } else {
        println!("检测到 {} 个文件，使用串行压缩...", file_count);
        compress_pictures::compress_images(path, img_type, create_new_file)?
    };
    
    if successful_compressions == 0 && failed_compressions == 0 {
        println!("未找到符合指定类型的图片进行处理。");
    } else if successful_compressions > 0 {
        // 成功，compress_images 内部已打印总结
    } else if failed_compressions > 0 && successful_compressions == 0 {
        println!("所有找到的图片都压缩失败了。");
    }
    Ok(())
}
