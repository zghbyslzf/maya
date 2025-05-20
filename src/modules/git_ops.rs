use std::path::PathBuf;

/// 处理Git操作的模块
pub fn handle_git_ops(git_ops: &[String], path: &PathBuf) {
    if git_ops.len() == 1 && (git_ops[0] == "m" || git_ops[0] == "M") {
        match git_add_commit_push::git_add_commit_push(path.to_string_lossy().to_string()) {
            Ok(_) => println!("已完成 git add/commit/push 操作"),
            Err(e) => eprintln!("git 操作出错: {:?}", e),
        }
    } else {
        eprintln!("暂不支持的 git 操作参数: {:?}", git_ops);
    }
}
