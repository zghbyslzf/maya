use std::fs;
// use std::io::Write;
use std::path::{Path, PathBuf};
use walkdir::WalkDir;
use zip::write::FileOptions;
use zip::ZipWriter;

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

fn find_vite_config() -> Option<PathBuf> {
    let current_dir = std::env::current_dir().ok()?;
    println!("当前目录: {:?}", current_dir);
    let js_config = current_dir.join("vite.config.js");
    let ts_config = current_dir.join("vite.config.ts");
    println!("检查js配置路径: {:?}", js_config);
    println!("检查js配置路径2: {:?}", js_config.exists());
    if js_config.exists() {
        Some(js_config)
    } else if ts_config.exists() {
        Some(ts_config)
    } else {
        None
    }
}

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

fn create_zip(source_dir: &Path, dest_path: &Path) -> std::io::Result<()> {
    let folder_name = source_dir.file_name().unwrap_or_default().to_str().unwrap();
    let zip_path = dest_path.join(format!("{}.zip", folder_name));
    let file = fs::File::create(&zip_path)?;
    let mut zip = ZipWriter::new(file);

    // 在新版zip中，需要明确指定FileOptions的类型参数为()
    let options: FileOptions<'_, ()> = FileOptions::default()
        .compression_method(zip::CompressionMethod::Deflated);

    let walkdir = WalkDir::new(source_dir);
    let it = walkdir.into_iter().filter_map(|e| e.ok());

    for entry in it {
        let path = entry.path();
        let name = path.strip_prefix(source_dir).unwrap();

        if path.is_file() {
            // 确保文件名是有效的Unicode
            if let Some(name_str) = name.to_str() {
                zip.start_file(name_str, options)?;
                let mut f = fs::File::open(path)?;
                std::io::copy(&mut f, &mut zip)?;
            }
        }
    }

    zip.finish()?;
    println!("成功打包到: {:?}", zip_path);
    Ok(())
}
