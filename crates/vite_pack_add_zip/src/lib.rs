use std::fs;
// use std::io::Write;
use std::path::{Path, PathBuf};
use walkdir::WalkDir;
use zip::write::FileOptions;
use zip::ZipWriter;

pub fn handle_vite_pack(path: &PathBuf) {
    // 检查vite配置文件
    let vite_config = find_vite_config(path);

    if let Some(config_path) = vite_config {
        // 读取outDir配置
        let out_dir = get_out_dir(&config_path).unwrap_or_else(|| "dist".to_string());

        // 检查outDir文件夹是否存在
        let target_dir = path.join(&out_dir);
        if !target_dir.exists() {
            // 检查dist文件夹是否存在
            let dist_dir = path.join("dist");
            if dist_dir.exists() {
                create_zip(&dist_dir, path).unwrap();
            } else {
                println!("没有检测到对应打包文件夹，请检查Vite配置");
            }
        } else {
            create_zip(&target_dir, path).unwrap();
        }
    } else {
        println!("没有检测到vite.config.js或vite.config.ts文件");
    }
}

fn find_vite_config(path: &Path) -> Option<PathBuf> {
    let js_config = path.join("vite.config.js");
    let ts_config = path.join("vite.config.ts");

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

    // 简单解析outDir配置
    if content.contains("outDir") {
        let lines: Vec<&str> = content.lines().collect();
        for line in lines {
            if line.trim().starts_with("outDir") {
                let parts: Vec<&str> = line.split(':').collect();
                if parts.len() > 1 {
                    let dir = parts[1]
                        .trim()
                        .trim_matches(|c| c == '"' || c == '\'' || c == ',' || c == ' ');
                    return Some(dir.to_string());
                }
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

    let options = FileOptions::default().compression_method(zip::CompressionMethod::Deflated);

    let walkdir = WalkDir::new(source_dir);
    let it = walkdir.into_iter().filter_map(|e| e.ok());

    for entry in it {
        let path = entry.path();
        let name = path.strip_prefix(source_dir).unwrap();

        if path.is_file() {
            zip.start_file(name.to_str().unwrap(), options)?;
            let mut f = fs::File::open(path)?;
            std::io::copy(&mut f, &mut zip)?;
        }
    }

    zip.finish()?;
    println!("成功打包到: {:?}", zip_path);
    Ok(())
}
