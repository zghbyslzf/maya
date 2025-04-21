use regex::Regex;
use std::fs;
use std::path::Path;
use swc_common::{FileName, Globals, Mark, SourceMap, GLOBALS};
use swc_ecma_ast::*;
use swc_ecma_codegen::{text_writer::JsWriter, Config as CodegenConfig, Emitter};
use swc_ecma_parser::{Parser, StringInput, Syntax};

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

/// 使用SWC解析TSX文件为AST并转换为Vue组件（AST实现）
fn parse_tsx_to_ast(content: &str) -> (String, String) {
    // use swc_common::SourceMap;
    // use swc_common::DUMMY_SP;
    // use swc_ecma_parser::{Parser, StringInput, Syntax, TsConfig};
    ('1'.to_string(), '2'.to_string())

    // let cm = SourceMap::default();
    // let fm = cm.new_source_file(
    //     FileName::Custom("input.tsx".into()).into(),
    //     content.into(),
    // );
    // let lexer = swc_ecma_parser::lexer::Lexer::new(
    //     Syntax::Typescript(TsConfig {
    //         tsx: true,
    //         ..Default::default()
    //     }),
    //     swc_ecma_ast::EsVersion::Es2020,
    //     StringInput::from(&*fm.source_file()),
    //     None,
    // );
    // let mut parser = Parser::new_from(lexer);
    // let module = parser.parse_module().expect("Failed to parse TSX module");

    // let jsx_template = extract_jsx_from_ast(&module);
    // let component_script = convert_component_from_ast(&module);
    // (jsx_template, component_script)
}

// fn extract_jsx_from_ast(module: &swc_ecma_ast::Module) -> String {
//     // use swc_ecma_ast::*;
//     // // 遍历AST查找函数组件的return语句中的JSX
//     // for item in &module.body {
//     //     if let swc_ecma_ast::ModuleItem::Stmt(swc_ecma_ast::Stmt::Decl(swc_ecma_ast::Decl::Fn(
//     //         fn_decl,
//     //     ))) = item
//     //     {
//     //         if let Some(body) = &fn_decl.function.body {
//     //             for stmt in &body.stmts {
//     //                 if let swc_ecma_ast::Stmt::Return(ret_stmt) = stmt {
//     //                     if let Some(expr) = &ret_stmt.arg {
//     //                         // 只处理JSX表达式
//     //                         if let swc_ecma_ast::Expr::JSXElement(jsx_elem) = &**expr {
//     //                             // 简单序列化输出
//     //                             // 使用swc_ecma_codegen序列化JSX
//     //                             use swc_common::GLOBALS;
//     //                             let mut buf = Vec::new();
//     //                             // GLOBALS.set(&swc_common::Globals::new(), || {
//     //                             //     let cm = swc_common::SourceMap::default();
//     //                             //     let mut emitter = Emitter {
//     //                             //         cfg: swc_ecma_codegen::Config {
//     //                             //             minify: false,
//     //                             //             ascii_only: false,
//     //                             //             omit_last_semi: false,
//     //                             //             target: swc_ecma_ast::EsVersion::Es2020,
//     //                             //         },
//     //                             //         comments: None,
//     //                             //         cm: cm.clone(),
//     //                             //         wr: Box::new(JsWriter::new(
//     //                             //             cm.clone(),
//     //                             //             "input.tsx",
//     //                             //             &mut buf,
//     //                             //             None,
//     //                             //         )),
//     //                             //     };
//     //                             //     // 用ModuleItem::Stmt包裹JSX表达式
//     //                             //     let expr_stmt =
//     //                             //         swc_ecma_ast::Stmt::Expr(swc_ecma_ast::ExprStmt {
//     //                             //             span: Default::default(),
//     //                             //             expr: Box::new(swc_ecma_ast::Expr::JSXElement(
//     //                             //                 jsx_elem.clone(),
//     //                             //             )),
//     //                             //         });
//     //                             //     emitter
//     //                             //         .emit_module_item(&swc_ecma_ast::ModuleItem::Stmt(
//     //                             //             expr_stmt,
//     //                             //         ))
//     //                             //         .unwrap();
//     //                             // });
//     //                             return String::from_utf8(buf).unwrap();
//     //                         }
//     //                     }
//     //                 }
//     //             }
//     //         }
//     //     }
//     // }
//     // String::new()

// }

// struct ComponentVisitor {
//     component_name: String,
//     props: Vec<String>,
// }

// impl ComponentVisitor {
//     fn visit_function(&mut self, func: &swc_ecma_ast::Function) {
//     // 获取组件名称
//     if let Some(swc_ecma_ast::Ident { sym, .. }) = func.ident.as_ref() {
//         self.component_name = sym.to_string();
//     }

//     // 提取props
//     if let Some(param) = func.params.first() {
//         if let Pat::Object(obj_pat) = &param.pat {
//             for prop in &obj_pat.props {
//                 if let ObjectPatProp::Assign(assign_prop) = prop {
//                     self.props.push(assign_prop.key.sym.to_string());
//                 }
//             }
//         }
//     }
// }

// fn convert_jsx_string_to_template(jsx: &str) -> String {
//     let mut template = jsx.to_string();

//     // 替换React特定语法为Vue语法

//     // 替换className为class
//     let class_re = Regex::new(r"className=").unwrap();
//     template = class_re.replace_all(&template, "class=").to_string();

//     // 替换React的条件渲染 {condition && <div>} 为 v-if
//     let condition_re = Regex::new(r"\{(\w+)\s*&&\s*(<[^>]+>)").unwrap();
//     template = condition_re
//         .replace_all(&template, "$2 v-if=\"$1\"")
//         .to_string();

//     // 替换React的事件处理 onClick={handleClick} 为 @click="handleClick"
//     let event_re = Regex::new(r"on(\w+)=\{(\w+)\}").unwrap();
//     template = event_re
//         .replace_all(&template, "@${1:/lowercase}=\"$2\"")
//         .to_string();

//     // 替换React的状态引用 {state} 为 {{ state }}
//     let state_re = Regex::new(r"\{(\w+)\}").unwrap();
//     template = state_re.replace_all(&template, "{{ $1 }}").to_string();

//     template
// }

// fn convert_component_from_ast(module: &Module) -> String {
//     let mut vue_component = String::new();
//     let mut component_name = "Component";
//     let mut props = Vec::new();
//     let mut data_properties = Vec::new();
//     let mut methods = Vec::new();

//     // 遍历模块中的语句
//     for item in &module.body {
//         match item {
//             // 查找函数声明（函数组件）
//             ModuleItem::Stmt(Stmt::Decl(Decl::Fn(fn_decl))) => {
//                 component_name = fn_decl.ident.sym.as_ref();

//                 // 分析函数参数（props）
//                 if let Some(param) = fn_decl.function.params.first() {
//                     if let Pat::Object(obj_pat) = &param.pat {
//                         for prop in &obj_pat.props {
//                             if let ObjectPatProp::Assign(assign_prop) = prop {
//                                 props.push(assign_prop.key.sym.to_string());
//                             }
//                         }
//                     }
//                 }
//             }
//             // 查找变量声明（useState, 方法等）
//             ModuleItem::Stmt(Stmt::Decl(Decl::Var(var_decl))) => {
//                 for decl in &var_decl.decls {
//                     // 检查是否是useState
//                     if let Some(init) = &decl.init {
//                         if let Pat::Array(array_pat) = &decl.name {
//                             if array_pat.elems.len() == 2 {
//                                 if let Some(Pat::Ident(state_ident)) = &array_pat.elems[0] {
//                                     if let Some(Pat::Ident(setter_ident)) = &array_pat.elems[1] {
//                                         let state_name = state_ident.id.sym.to_string();
//                                         let setter_name = setter_ident.id.sym.to_string();

//                                         // 检查是否符合useState模式
//                                         if setter_name.starts_with("set")
//                                             && setter_name[3..].to_lowercase()
//                                                 == state_name.to_lowercase()
//                                         {
//                                             // 提取初始值
//                                             let initial_value = match &**init {
//                                                 Expr::Call(call_expr) => {
//                                                     if let Callee::Expr(expr) = &call_expr.callee {
//                                                         if let Expr::Ident(ident) = expr {
//                                                             if let Some(arg) =
//                                                                 call_expr.args.first()
//                                                             {
//                                                                 format!("{:?}", arg.expr)
//                                                             } else {
//                                                                 "null".to_string()
//                                                             }
//                                                         } else {
//                                                             "null".to_string()
//                                                         }
//                                                     } else {
//                                                         "null".to_string()
//                                                     }
//                                                 }
//                                                 _ => "null".to_string(),
//                                             };

//                                             data_properties
//                                                 .push(format!("{}: {}", state_name, initial_value));
//                                         }
//                                     }
//                                 }
//                             }
//                         } else if let Pat::Ident(ident) = &decl.name {
//                             // 检查是否是方法（箭头函数）
//                             if let Expr::Arrow(arrow_expr) = &**init {
//                                 let method_name = ident.id.sym.to_string();

//                                 // 跳过setState方法
//                                 if method_name.starts_with("set")
//                                     && data_properties
//                                         .iter()
//                                         .any(|p| p.starts_with(&method_name[3..]))
//                                 {
//                                     continue;
//                                 }

//                                 // 提取方法体
//                                 let method_body = match &*arrow_expr.body {
//                                     BlockStmtOrExpr::BlockStmt(block) => {
//                                         format!("{:?}", block)
//                                     }
//                                     BlockStmtOrExpr::Expr(expr) => {
//                                         format!("{:?}", expr)
//                                     }
//                                 };

//                                 methods.push(format!(
//                                     "{}: function() {{{}\n  }}",
//                                     method_name, method_body
//                                 ));
//                             }
//                         }
//                     }
//                 }
//             }
//             _ => {}
//         }
//     }

//     // 构建Vue组件
//     vue_component.push_str("export default {\n");
//     vue_component.push_str(&format!("  name: '{}',\n", component_name));

//     // 添加props
//     if !props.is_empty() {
//         vue_component.push_str("  props: {\n");
//         for prop in props {
//             vue_component.push_str(&format!("    {}: {{}},\n", prop));
//         }
//         vue_component.push_str("  },\n");
//     }

//     // 添加data
//     vue_component.push_str("  data() {\n");
//     vue_component.push_str("    return {\n");
//     for data_prop in &data_properties {
//         vue_component.push_str(&format!("      {},\n", data_prop));
//     }
//     vue_component.push_str("    };\n");
//     vue_component.push_str("  },\n");

//     // 添加methods
//     if !methods.is_empty() {
//         vue_component.push_str("  methods: {\n");
//         for method in methods {
//             vue_component.push_str(&format!("    {},\n", method));
//         }
//         vue_component.push_str("  },\n");
//     }

//     vue_component.push_str("};\n");

//     vue_component
// }
