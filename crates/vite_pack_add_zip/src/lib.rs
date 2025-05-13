use std::fs;
use std::path::{Path, PathBuf};

/// Vite打包模块，负责查找Vite配置并将输出目录打包为zip
pub fn handle_vite_pack() {
    // 检查vite配置文件
    let vite_config = find_vite_config();

    if let Some(config_path) = vite_config {
        // 读取outDir配置
        let out_dir = get_out_dir(&config_path).unwrap_or_else(|| "dist".to_string());

        // 检查outDir文件夹是否存在
        let current_dir = std::env::current_dir().unwrap();
        let target_dir = current_dir.join(&out_dir);
        if !target_dir.exists() {
            // 检查dist文件夹是否存在
            let dist_dir = current_dir.join("dist");
            if dist_dir.exists() {
                create_zip(&dist_dir, &current_dir).unwrap();
            } else {
                println!("没有检测到对应打包文件夹，请检查Vite配置");
            }
        } else {
            create_zip(&target_dir, &current_dir).unwrap();
        }
    } else {
        println!("没有检测到vite.config.js或vite.config.ts文件");
    }
}

/// 查找Vite配置文件
fn find_vite_config() -> Option<PathBuf> {
    let current_dir = std::env::current_dir().ok()?;
    println!("当前目录: {:?}", current_dir);
    
    // 使用共享库的find_file函数
    if let Some(js_config) = maya_common::find_file(&current_dir, "vite.config.js") {
        println!("找到vite.config.js: {:?}", js_config);
        Some(js_config)
    } else if let Some(ts_config) = maya_common::find_file(&current_dir, "vite.config.ts") {
        println!("找到vite.config.ts: {:?}", ts_config);
        Some(ts_config)
    } else {
        None
    }
}

/// 从Vite配置文件中获取输出目录
fn get_out_dir(config_path: &Path) -> Option<String> {
    let content = fs::read_to_string(config_path).ok()?;

    // 增强配置解析逻辑
    let re = regex::Regex::new(r#"(outDir\s*[:=]\s*['"]?)([^,'"}\s]+)['"]?"#).unwrap();

    // 先尝试正则匹配
    if let Some(caps) = re.captures(&content) {
        if let Some(m) = caps.get(2) {
            return Some(
                m.as_str()
                    .trim_matches(|c| c == '"' || c == '\'')
                    .to_string(),
            );
        }
    }

    // 处理嵌套对象结构
    let build_block_re =
        regex::Regex::new(r#"(?s)build\s*:\s*\{.*?outDir\s*[:=]\s*['"]?([^'"},]+)"#).unwrap();
    if let Some(caps) = build_block_re.captures(&content) {
        if let Some(block) = caps.get(1) {
            if let Some(m) = regex::Regex::new(r#"outDir[=:]s*['"]?([^,'"}\s]+)['"]?"#)
                .unwrap()
                .find(block.as_str())
            {
                let dir = m
                    .as_str()
                    .split(&[':', '='][..])
                    .nth(1)
                    .unwrap()
                    .trim()
                    .trim_matches(|c| c == '"' || c == '\'' || c == ',' || c == ' ');
                return Some(dir.to_string());
            }
        }
    }

    None
}

/// 创建ZIP文件
fn create_zip(source_dir: &Path, dest_path: &Path) -> std::io::Result<PathBuf> {
    // 使用共享库的create_zip_archive函数
    let zip_path = maya_common::create_zip_archive(
        source_dir,
        dest_path,
        |path| path.is_file() // 包含所有文件
    )?;
    
    println!("成功打包到: {:?}", zip_path);
    Ok(zip_path)
} 