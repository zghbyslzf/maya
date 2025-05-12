use std::fs::{self, File};
use std::io::{self, Read, Write};
use std::path::Path;
use zip::write::{FileOptions, ZipWriter};
use ignore::WalkBuilder;

pub fn handle_gitignore_pack() {
    // 检查当前目录下是否有.gitignore文件
    let current_dir = std::env::current_dir().unwrap();
    let gitignore_path = current_dir.join(".gitignore");
    
    if !gitignore_path.exists() {
        println!("没有找到.gitignore文件");
        return;
    }
    
    println!("找到.gitignore文件: {:?}", gitignore_path);
    
    // 创建zip文件名，用当前目录名
    let folder_name = current_dir.file_name().unwrap_or_default().to_str().unwrap();
    let zip_name = format!("{}.zip", folder_name);
    let zip_path = current_dir.join(&zip_name);
    
    match create_zip_from_gitignore(&current_dir, &zip_path) {
        Ok(_) => println!("成功打包文件到: {}", zip_name),
        Err(e) => println!("打包文件时出错: {}", e),
    }
}

fn create_zip_from_gitignore(source_dir: &Path, zip_path: &Path) -> io::Result<()> {
    let file = File::create(zip_path)?;
    let mut zip = ZipWriter::new(file);
    
    // 使用ignore库来尊重.gitignore规则
    let walker = WalkBuilder::new(source_dir)
        .hidden(false) // 不跳过隐藏文件，让.gitignore规则处理
        .git_global(false) // 忽略全局git规则
        .git_ignore(true) // 使用.gitignore规则
        .require_git(false) // 不需要Git仓库
        .build();
    
    let options: FileOptions<'_, ()> = FileOptions::default()
        .compression_method(zip::CompressionMethod::Deflated);
    
    // 避免将生成的zip文件本身包含在新的zip中
    let zip_filename = zip_path.file_name().unwrap_or_default().to_str().unwrap();
    
    for entry in walker {
        match entry {
            Ok(entry) => {
                let path = entry.path();
                
                // 跳过.git目录
                if path.to_str().unwrap_or("").contains("/.git/") || 
                   path.to_str().unwrap_or("").starts_with(".git/") {
                    continue;
                }
                
                // 跳过zip文件本身
                if path.file_name().unwrap_or_default().to_str().unwrap_or("") == zip_filename {
                    continue;
                }
                
                let metadata = fs::metadata(path)?;
                
                // 只处理文件，不处理目录
                if metadata.is_file() {
                    let relative_path = path.strip_prefix(source_dir).unwrap();
                    let name = relative_path.to_str().unwrap_or("");
                    
                    // 将文件添加到zip中
                    zip.start_file(name, options)?;
                    let mut file = File::open(path)?;
                    let mut buffer = Vec::new();
                    file.read_to_end(&mut buffer)?;
                    zip.write_all(&buffer)?;
                }
            },
            Err(e) => {
                println!("遍历文件时出错: {}", e);
            }
        }
    }
    
    zip.finish()?;
    Ok(())
}
