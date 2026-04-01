use std::path::Path;
use mp4_to_m3u8;
use maya_common::error::{Error, Result};

pub async fn handle_transform_ops(types: &[String], path: &Path) -> Result<()> {
    if types.len() < 2 {
        return Err(Error::invalid_argument("请指定源格式和目标格式，例如: maya -t mp4 m3u8".to_string()));
    }

    let source_format = &types[0].to_lowercase();
    let target_format = &types[1].to_lowercase();

    match (source_format.as_str(), target_format.as_str()) {
        ("mp4", "m3u8") => {
            let (successful_conversions, failed_conversions) = mp4_to_m3u8::convert_mp4_to_m3u8(path).await?;
            if successful_conversions == 0 && failed_conversions == 0 {
                println!("未找到任何mp4文件进行转换。");
            } else if successful_conversions > 0 {
                println!("✅ mp4到m3u8转换任务完成！");
            } else if failed_conversions > 0 && successful_conversions == 0 {
                println!("❌ 所有找到的mp4文件都转换失败了。");
            }
        },
        _ => {
            return Err(Error::invalid_argument(format!("暂不支持从 {} 转换到 {} 的格式。目前支持的转换: mp4 -> m3u8", source_format, target_format)));
        }
    }
    Ok(())
} 