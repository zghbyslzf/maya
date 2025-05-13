use std::path::Path;
use ignore::WalkBuilder;
use maya_common;

pub fn handle_gitignore_pack() {
    // 检查当前目录下是否有.gitignore文件
    let current_dir = std::env::current_dir().unwrap();
    
    if let Some(gitignore_path) = maya_common::find_file(&current_dir, ".gitignore") {
        println!("找到.gitignore文件: {:?}", gitignore_path);
        
        // 创建zip文件
        let result = create_zip_from_gitignore(&current_dir, &current_dir);
        match result {
            Ok(zip_path) => println!("成功打包文件到: {:?}", zip_path),
            Err(e) => println!("打包文件时出错: {}", e),
        }
    } else {
        println!("没有找到.gitignore文件");
    }
}

fn create_zip_from_gitignore(source_dir: &Path, dest_path: &Path) -> std::io::Result<std::path::PathBuf> {
    // 使用ignore库来尊重.gitignore规则
    let walker = WalkBuilder::new(source_dir)
        .hidden(false) // 不跳过隐藏文件，让.gitignore规则处理
        .git_global(false) // 忽略全局git规则
        .git_ignore(true) // 使用.gitignore规则
        .require_git(false) // 不需要Git仓库
        .build();
    
    // 收集忽略规则允许的文件
    let mut allowed_files = vec![];
    
    for entry in walker {
        if let Ok(entry) = entry {
            let path = entry.path();
            
            // 跳过.git目录
            if path.to_str().unwrap_or("").contains("/.git/") || 
               path.to_str().unwrap_or("").starts_with(".git/") {
                continue;
            }
            
            allowed_files.push(path.to_path_buf());
        }
    }
    
    // 创建zip文件 - 使用maya_common中的函数
    let zip_path = maya_common::create_zip_archive(
        source_dir, 
        dest_path,
        |path| {
            // 只包含在allowed_files中的文件
            path.is_file() && allowed_files.iter().any(|p| p == path)
        }
    )?;
    
    Ok(zip_path)
}
