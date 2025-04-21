use regex::Regex;
use std::fs;
use std::path::{Path, PathBuf};

/// 处理React到Vue的转换操作
pub fn handle_react_to_vue(file_path: &str) {
    println!("正在将React文件转换为Vue: {}", file_path);

    // 检查文件是否存在
    let path = Path::new(file_path);
    if !path.exists() {
        eprintln!("错误: 文件 '{}' 不存在", file_path);
        return;
    }

    // 检查文件是否为.tsx文件
    if !file_path.ends_with(".tsx") {
        eprintln!("错误: 文件 '{}' 不是.tsx文件", file_path);
        return;
    }

    // 读取文件内容
    let content = match fs::read_to_string(path) {
        Ok(content) => content,
        Err(e) => {
            eprintln!("读取文件失败: {}", e);
            return;
        }
    };

    // 转换内容
    let vue_content = convert_react_to_vue(&content);

    // 创建新的Vue文件路径
    let vue_path = path.with_extension("vue");

    // 写入新文件
    match fs::write(&vue_path, vue_content) {
        Ok(_) => println!("成功创建Vue文件: {}", vue_path.display()),
        Err(e) => eprintln!("创建Vue文件失败: {}", e),
    }
}

/// 将React代码转换为Vue代码
fn convert_react_to_vue(content: &str) -> String {
    let mut vue_content = String::new();

    // 添加Vue模板基本结构
    vue_content.push_str("<template>\n");

    // 提取JSX部分并转换为Vue模板
    let jsx = extract_jsx(content);
    vue_content.push_str(&convert_jsx_to_template(&jsx));

    vue_content.push_str("</template>\n\n<script>\n");

    // 转换React组件为Vue组件
    vue_content.push_str(&convert_component_definition(content));

    vue_content.push_str("</script>\n\n<style scoped>\n</style>\n");

    vue_content
}

/// 提取React组件中的JSX部分
fn extract_jsx(content: &str) -> String {
    // 简单实现：查找return语句后的JSX
    let re = Regex::new(r"return\s*\(([\s\S]*?)\);\s*\}").unwrap();

    if let Some(captures) = re.captures(content) {
        if let Some(jsx_match) = captures.get(1) {
            return jsx_match.as_str().to_string();
        }
    }

    // 如果没有找到，返回空字符串
    String::new()
}

/// 将JSX转换为Vue模板
fn convert_jsx_to_template(jsx: &str) -> String {
    let mut template = jsx.to_string();

    // 替换React特定语法为Vue语法

    // 替换className为class
    let class_re = Regex::new(r"className=").unwrap();
    template = class_re.replace_all(&template, "class=").to_string();

    // 替换React的条件渲染 {condition && <div>} 为 v-if
    let condition_re = Regex::new(r"\{(\w+)\s*&&\s*(<[^>]+>)").unwrap();
    template = condition_re
        .replace_all(&template, "$2 v-if=\"$1\"")
        .to_string();

    // 替换React的事件处理 onClick={handleClick} 为 @click="handleClick"
    let event_re = Regex::new(r"on(\w+)=\{(\w+)\}").unwrap();
    template = event_re
        .replace_all(&template, "@${1:/lowercase}=\"$2\"")
        .to_string();

    // 替换React的状态引用 {state} 为 {{ state }}
    let state_re = Regex::new(r"\{(\w+)\}").unwrap();
    template = state_re.replace_all(&template, "{{ $1 }}").to_string();

    template
}

/// 将React组件定义转换为Vue组件定义
fn convert_component_definition(content: &str) -> String {
    let mut vue_component = String::new();

    // 提取组件名称
    let component_re = Regex::new(r"function\s+(\w+)\s*\(").unwrap();
    let component_name = if let Some(captures) = component_re.captures(content) {
        captures.get(1).map_or("Component", |m| m.as_str())
    } else {
        "Component"
    };

    // 提取状态定义
    let state_re = Regex::new(r"const\s+\[(\w+),\s*set(\w+)\]\s*=\s*useState\(([^)]*?)\)").unwrap();
    let mut data_properties = Vec::new();

    for captures in state_re.captures_iter(content) {
        let state_name = captures.get(1).map_or("", |m| m.as_str());
        let initial_value = captures.get(3).map_or("null", |m| m.as_str());
        data_properties.push(format!("{}: {}", state_name, initial_value));
    }

    // 提取props
    let props_re = Regex::new(r"\{([^}]+)\}\s*=\s*props").unwrap();
    let mut props = Vec::new();

    if let Some(captures) = props_re.captures(content) {
        if let Some(props_match) = captures.get(1) {
            for prop in props_match.as_str().split(',') {
                props.push(prop.trim().to_string());
            }
        }
    }

    // 构建Vue组件
    vue_component.push_str("export default {\n");
    vue_component.push_str(&format!("  name: '{}',\n", component_name));

    // 添加props
    if !props.is_empty() {
        vue_component.push_str("  props: {\n");
        for prop in props {
            vue_component.push_str(&format!("    {}: {{}},\n", prop));
        }
        vue_component.push_str("  },\n");
    }

    // 添加data
    vue_component.push_str("  data() {\n");
    vue_component.push_str("    return {\n");
    for data_prop in &data_properties {
        vue_component.push_str(&format!("      {},\n", data_prop));
    }
    vue_component.push_str("    };\n");
    vue_component.push_str("  },\n");

    // 提取方法
    let methods_re = Regex::new(r"const\s+(\w+)\s*=\s*\([^)]*\)\s*=>\s*\{([\s\S]*?)\};").unwrap();
    let mut methods = Vec::new();

    for captures in methods_re.captures_iter(content) {
        let method_name = captures.get(1).map_or("", |m| m.as_str());
        let method_body = captures.get(2).map_or("", |m| m.as_str());

        // 跳过setState方法
        if method_name.starts_with("set")
            && data_properties
                .iter()
                .any(|p| p.starts_with(&method_name[3..]))
        {
            continue;
        }

        methods.push(format!(
            "{}: function() {{{}\n  }}",
            method_name, method_body
        ));
    }

    // 添加methods
    if !methods.is_empty() {
        vue_component.push_str("  methods: {\n");
        for method in methods {
            vue_component.push_str(&format!("    {},\n", method));
        }
        vue_component.push_str("  },\n");
    }

    vue_component.push_str("};\n");

    vue_component
}
