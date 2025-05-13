use std::fs;
use std::io::{self, Read, Write};
use std::path::{Path, PathBuf};
use walkdir::WalkDir;
use zip::write::{FileOptions, ZipWriter};

/// 创建ZIP归档。可以被不同的zip功能共享使用。
pub fn create_zip_archive<F>(
    source_dir: &Path,
    dest_path: &Path,
    file_filter: F,
) -> io::Result<PathBuf>
where
    F: Fn(&Path) -> bool,
{
    // 获取源目录名称
    let folder_name = source_dir
        .file_name()
        .unwrap_or_default()
        .to_str()
        .unwrap_or("output");
    
    // 构建zip文件路径
    let zip_path = dest_path.join(format!("{}.zip", folder_name));
    
    // 创建zip文件
    let file = fs::File::create(&zip_path)?;
    let mut zip = ZipWriter::new(file);
    
    // 设置压缩选项
    let options: FileOptions<'_, ()> = FileOptions::default()
        .compression_method(zip::CompressionMethod::Deflated);
    
    // 获取zip文件名，避免将自身包含在压缩文件中
    let zip_filename = zip_path.file_name().unwrap_or_default().to_str().unwrap_or("");
    
    // 遍历源目录中的所有文件
    let walkdir = WalkDir::new(source_dir);
    let it = walkdir.into_iter().filter_map(|e| e.ok());
    
    for entry in it {
        let path = entry.path();
        
        // 如果是zip文件本身，跳过
        if path.file_name().unwrap_or_default().to_str().unwrap_or("") == zip_filename {
            continue;
        }
        
        // 应用文件过滤器
        if !file_filter(path) {
            continue;
        }
        
        // 获取相对路径
        if let Ok(name) = path.strip_prefix(source_dir) {
            if path.is_file() {
                if let Some(name_str) = name.to_str() {
                    // 将文件添加到zip中
                    zip.start_file(name_str, options)?;
                    let mut f = fs::File::open(path)?;
                    let mut buffer = Vec::new();
                    f.read_to_end(&mut buffer)?;
                    zip.write_all(&buffer)?;
                }
            }
        }
    }
    
    // 完成zip文件创建
    zip.finish()?;
    Ok(zip_path)
}

/// 查找指定目录中的文件
pub fn find_file(dir: &Path, filename: &str) -> Option<PathBuf> {
    let file_path = dir.join(filename);
    if file_path.exists() {
        Some(file_path)
    } else {
        None
    }
}

/// 格式化结果输出
pub fn format_result<T, E: std::fmt::Display>(
    result: Result<T, E>, 
    success_msg: &str, 
    error_prefix: &str
) -> T
where 
    T: std::fmt::Debug,
{
    match result {
        Ok(value) => {
            println!("{}", success_msg);
            value
        },
        Err(e) => {
            eprintln!("{}: {}", error_prefix, e);
            panic!("操作失败");
        }
    }
} 