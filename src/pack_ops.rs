/// 处理打包操作的模块
pub fn handle_pack_ops(pack_type: &str) {
    match pack_type {
        "g" => {
            println!("根据.gitignore规则打包文件");
            gitignore_add_zip::handle_gitignore_pack();
        }
        "a" => {
            println!("打包Vite项目输出目录");
            vite_pack_add_zip::handle_vite_pack();
        }
        _ => {
            println!("未知的打包类型: {}。可用选项: g (gitignore) 或 a (vite)。", pack_type);
        }
    }
} 