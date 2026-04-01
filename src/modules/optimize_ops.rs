use compress_pictures;
use std::path::Path; // 需要从 crate 根引用 compress_pictures
use maya_common::error::{Error, Result};

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
    
    let (successful_compressions, failed_compressions, _avg_compression_ratio) = 
        compress_pictures::compress_images(path, img_type, create_new_file)?;
    
    if successful_compressions == 0 && failed_compressions == 0 {
        println!("未找到符合指定类型的图片进行处理。");
    } else if successful_compressions > 0 {
        // 成功，compress_images 内部已打印总结
    } else if failed_compressions > 0 && successful_compressions == 0 {
        println!("所有找到的图片都压缩失败了。");
    }
    Ok(())
}
