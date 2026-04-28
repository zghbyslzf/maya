use compress_pictures;
use maya_common::error::{Error, Result};
use maya_common::file_utils::find_files_by_extension;
use std::path::Path;

const PARALLEL_THRESHOLD: usize = 10;

pub fn handle_optimize_ops(types: &[String], path: &Path) -> Result<()> {
    if types.is_empty() {
        return Err(Error::invalid_argument("请指定要压缩的图片类型 (png/jpg/jpeg/all)".to_string()));
    }

    let create_new_file = types.iter().any(|t| t == "n");

    let img_type_str = types
        .iter()
        .find(|&t| t != "n")
        .map(|s| s.as_str())
        .unwrap_or("all");

    let img_type = img_type_str.parse::<compress_pictures::ImageType>()
        .map_err(|e| Error::invalid_argument(format!("图片类型参数 '{}' 错误: {}", img_type_str, e)))?;

    let extensions = compress_pictures::extensions_for_type(&img_type);
    let image_files = find_files_by_extension(path, &extensions)?;
    let file_count = image_files.len();

    if file_count >= PARALLEL_THRESHOLD {
        println!("检测到 {} 个文件，启用并行压缩...", file_count);
    }
    let (successful_compressions, failed_compressions) = {
        let (s, f, _) = compress_pictures::compress_images_parallel(path, img_type, create_new_file)?;
        (s, f)
    };

    if successful_compressions == 0 && failed_compressions == 0 {
        println!("未找到符合指定类型的图片进行处理。");
    } else if failed_compressions > 0 && successful_compressions == 0 {
        println!("所有找到的图片都压缩失败了。");
    }
    Ok(())
}
