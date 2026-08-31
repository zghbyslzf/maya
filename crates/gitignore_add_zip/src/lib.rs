use ignore::WalkBuilder;
use maya_common::error::{Error, Result};
use std::collections::HashSet;
use std::path::{Path, PathBuf};

pub fn handle_gitignore_pack() -> Result<()> {
    // 检查当前目录下是否有.gitignore文件
    let current_dir = std::env::current_dir()?;

    if let Some(gitignore_path) = maya_common::find_file(&current_dir, ".gitignore") {
        println!("找到.gitignore文件: {:?}", gitignore_path);

        // 创建zip文件
        let zip_path = create_zip_from_gitignore(&current_dir, &current_dir)?;
        println!("成功打包文件到: {:?}", zip_path);
    } else {
        println!("没有找到.gitignore文件");
    }
    Ok(())
}

fn create_zip_from_gitignore(source_dir: &Path, dest_path: &Path) -> Result<PathBuf> {
    let walker = WalkBuilder::new(source_dir)
        .hidden(false)
        .git_global(false)
        .git_ignore(true)
        .require_git(false)
        .build();

    let mut allowed_files: HashSet<PathBuf> = HashSet::new();

    for entry in walker {
        let entry = entry.map_err(|error| {
            Error::path(format!(
                "按 .gitignore 遍历目录 {} 失败: {}",
                source_dir.display(),
                error
            ))
        })?;
        let path = entry.path();

        if path
            .components()
            .any(|c| c == std::path::Component::Normal(".git".as_ref()))
        {
            continue;
        }

        allowed_files.insert(path.to_path_buf());
    }

    let zip_path = maya_common::create_zip_archive(source_dir, dest_path, |path| {
        path.is_file() && allowed_files.contains(path)
    })?;

    Ok(zip_path)
}
