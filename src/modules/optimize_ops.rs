use std::path::Path;
use compress_pictures; // 需要从 crate 根引用 compress_pictures

pub fn handle_optimize_ops(types: &[String], path: &Path) {
    if types.is_empty() {
        eprintln!("请指定要压缩的图片类型 (png/jpg/jpeg/all)");
        return;
    }

    // 检查是否有n参数，表示创建新文件而不是覆盖
    let create_new_file = types.iter().any(|t| t == "n");
    
    // 找到图片类型参数
    let img_type_str = types.iter()
        .find(|&t| t != "n")
        .map(|s| s.as_str())
        .unwrap_or("all"); // 默认压缩所有类型
    
    match compress_pictures::ImageType::from_str(img_type_str) {
        Ok(img_type) => {
            // println!("准备压缩图片，类型: {:?}, 创建新文件: {}", img_type, create_new_file);
            match compress_pictures::compress_images(path, img_type, create_new_file) {
                Ok((successful_compressions, failed_compressions, _avg_compression_ratio)) => {
                    // 详细的总结信息已在 compress_images 内部打印
                    // 这里可以根据需要添加额外的顶层消息
                    if successful_compressions == 0 && failed_compressions == 0 {
                        println!("未找到符合指定类型的图片进行处理。");
                    } else if successful_compressions > 0 {
                        // println!("图片压缩任务部分或全部完成。"); // 可选的额外消息
                    } else if failed_compressions > 0 && successful_compressions == 0 {
                        println!("所有找到的图片都压缩失败了。");
                    }
                },
                Err(e) => {
                    eprintln!("图片压缩操作因内部错误而失败: {}", e);
                }
            }
        },
        Err(e) => {
            eprintln!("图片类型参数 '{}' 错误: {}", img_type_str, e);
        }
    }
} 