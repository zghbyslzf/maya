use regex::Regex;
use std::fs;
use std::path::Path;
use swc_common::sync::Lrc;
use swc_common::{FileName, SourceMap};
use swc_ecma_ast::*;
use swc_ecma_parser::{lexer::Lexer, Parser, StringInput, Syntax, TsConfig};

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

    // 使用SWC解析TSX文件
    let (jsx_template, component_script) = parse_tsx_to_ast(content);

    // 添加Vue模板基本结构
    vue_content.push_str("<template>\n");
    vue_content.push_str(&jsx_template);
    vue_content.push_str("</template>\n\n<script>\n");
    vue_content.push_str(&component_script);
    vue_content.push_str("</script>\n\n<style scoped>\n</style>\n");

    vue_content
}

/// 使用SWC解析TSX文件为AST并转换为Vue组件
fn parse_tsx_to_ast(content: &str) -> (String, String) {
    // 由于SWC库版本兼容性问题，我们使用正则表达式直接解析内容
    let jsx_template = extract_jsx_with_regex(content);
    let component_script = extract_component_with_regex(content);

    return (jsx_template, component_script);
}

/// 使用正则表达式提取JSX部分
fn extract_jsx_with_regex(content: &str) -> String {
    let mut template = String::new();

    // 尝试匹配return语句中的JSX
    let re = Regex::new(r"return\s*\((([\s\S])*?)\);\s*\}").unwrap();

    if let Some(captures) = re.captures(content) {
        if let Some(jsx_match) = captures.get(1) {
            template = convert_jsx_string_to_template(jsx_match.as_str());
        }
    }

    template
}

/// 使用正则表达式提取组件定义
fn extract_component_with_regex(content: &str) -> String {
    let mut vue_component = String::new();
    let mut component_name = "Component";

    // 尝试匹配函数组件名称
    let fn_re = Regex::new(r"function\s+([A-Za-z_][A-Za-z0-9_]*)\s*\(").unwrap();
    if let Some(captures) = fn_re.captures(content) {
        if let Some(name_match) = captures.get(1) {
            component_name = name_match.as_str();
        }
    }

    // 构建Vue组件
    vue_component.push_str("export default {\n");
    vue_component.push_str(&format!("  name: '{}',\n", component_name));

    // 添加props (简化处理)
    vue_component.push_str("  props: {},\n");

    // 添加data
    vue_component.push_str("  data() {\n");
    vue_component.push_str("    return {\n");
    vue_component.push_str("    };\n");
    vue_component.push_str("  },\n");

    // 添加methods
    vue_component.push_str("  methods: {},\n");

    vue_component.push_str("};\n");

    vue_component
}

// 此函数已被extract_jsx_with_regex替代

// 此函数已被extract_jsx_with_regex替代，保留convert_jsx_string_to_template函数用于转换

/// 将JSX字符串转换为Vue模板
fn convert_jsx_string_to_template(jsx: &str) -> String {
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

/// 从AST中提取组件定义并转换为Vue组件脚本
fn convert_component_from_ast(module: &Module) -> String {
    let mut vue_component = String::new();
    let mut component_name = "Component";
    let mut props = Vec::new();
    let mut data_properties = Vec::new();
    let mut methods = Vec::new();

    // 遍历模块中的语句
    for item in &module.body {
        match item {
            // 查找函数声明（函数组件）
            ModuleItem::Stmt(Stmt::Decl(Decl::Fn(fn_decl))) => {
                component_name = fn_decl.ident.sym.as_ref();

                // 分析函数参数（props）
                if let Some(param) = fn_decl.function.params.first() {
                    if let Pat::Object(obj_pat) = &param.pat {
                        for prop in &obj_pat.props {
                            if let ObjectPatProp::Assign(assign_prop) = prop {
                                props.push(assign_prop.key.sym.to_string());
                            }
                        }
                    }
                }
            }
            // 查找变量声明（useState, 方法等）
            ModuleItem::Stmt(Stmt::Decl(Decl::Var(var_decl))) => {
                for decl in &var_decl.decls {
                    // 检查是否是useState
                    if let Some(init) = &decl.init {
                        if let Pat::Array(array_pat) = &decl.name {
                            if array_pat.elems.len() == 2 {
                                if let Some(Pat::Ident(state_ident)) = &array_pat.elems[0] {
                                    if let Some(Pat::Ident(setter_ident)) = &array_pat.elems[1] {
                                        let state_name = state_ident.id.sym.to_string();
                                        let setter_name = setter_ident.id.sym.to_string();

                                        // 检查是否符合useState模式
                                        if setter_name.starts_with("set")
                                            && setter_name[3..].to_lowercase()
                                                == state_name.to_lowercase()
                                        {
                                            // 提取初始值
                                            let initial_value = match &**init {
                                                Expr::Call(call_expr) => {
                                                    if let Some(arg) = call_expr.args.first() {
                                                        format!("{:?}", arg.expr)
                                                    } else {
                                                        "null".to_string()
                                                    }
                                                }
                                                _ => "null".to_string(),
                                            };

                                            data_properties
                                                .push(format!("{}: {}", state_name, initial_value));
                                        }
                                    }
                                }
                            }
                        } else if let Pat::Ident(ident) = &decl.name {
                            // 检查是否是方法（箭头函数）
                            if let Expr::Arrow(arrow_expr) = &**init {
                                let method_name = ident.id.sym.to_string();

                                // 跳过setState方法
                                if method_name.starts_with("set")
                                    && data_properties
                                        .iter()
                                        .any(|p| p.starts_with(&method_name[3..]))
                                {
                                    continue;
                                }

                                // 提取方法体
                                let method_body = match &*arrow_expr.body {
                                    BlockStmtOrExpr::BlockStmt(block) => {
                                        format!("{:?}", block)
                                    }
                                    BlockStmtOrExpr::Expr(expr) => {
                                        format!("{:?}", expr)
                                    }
                                };

                                methods.push(format!(
                                    "{}: function() {{{}\n  }}",
                                    method_name, method_body
                                ));
                            }
                        }
                    }
                }
            }
            _ => {}
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
