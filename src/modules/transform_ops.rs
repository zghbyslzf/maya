use std::path::Path;
use mp4_to_m3u8;

pub async fn handle_transform_ops(types: &[String], path: &Path) {
    if types.len() < 2 {
        eprintln!("请指定源格式和目标格式，例如: maya -t mp4 m3u8");
        return;
    }

    let source_format = &types[0].to_lowercase();
    let target_format = &types[1].to_lowercase();

    match (source_format.as_str(), target_format.as_str()) {
        ("mp4", "m3u8") => {
            match mp4_to_m3u8::convert_mp4_to_m3u8(path).await {
                Ok((successful_conversions, failed_conversions)) => {
                    if successful_conversions == 0 && failed_conversions == 0 {
                        println!("未找到任何mp4文件进行转换。");
                    } else if successful_conversions > 0 {
                        println!("✅ mp4到m3u8转换任务完成！");
                    } else if failed_conversions > 0 && successful_conversions == 0 {
                        println!("❌ 所有找到的mp4文件都转换失败了。");
                    }
                },
                Err(e) => {
                    eprintln!("视频转换操作因内部错误而失败: {}", e);
                }
            }
        },
        _ => {
            eprintln!("暂不支持从 {} 转换到 {} 的格式", source_format, target_format);
            eprintln!("目前支持的转换: mp4 -> m3u8");
        }
    }
} 