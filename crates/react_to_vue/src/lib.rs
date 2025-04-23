use std::fs;
use std::io;
use std::io::Cursor;
use swc_common::DUMMY_SP;
use swc_common::{
    errors::{EmitterWriter, Handler},
    sync::Lrc,
    FileName, Globals, SourceMap, GLOBALS,
};
use swc_ecma_ast::*;
use swc_ecma_codegen::{text_writer::JsWriter, Config as CodegenConfig, Emitter};
use swc_ecma_parser::{lexer::Lexer, Parser, StringInput, Syntax, TsSyntax};
use swc_ecma_visit::{Visit, VisitWith};

pub fn handle_react_to_vue(input_path: &str) {
    GLOBALS.set(&Globals::new(), || {
        let input = match fs::read_to_string(input_path) {
            Ok(s) => s,
            Err(e) => {
                eprintln!("读取文件失败: {}", e);
                return;
            }
        };
        let cm: Lrc<SourceMap> = Default::default();
        let emitter = Box::new(EmitterWriter::new(
            Box::new(io::stderr()),
            Some(cm.clone()),
            false, // short_message
            false, // teach
        ));
        let handler = Handler::with_emitter(true, false, emitter);
        let fm = cm.new_source_file(
            FileName::Custom(input_path.to_string()).into(),
            input.clone(),
        );
        let lexer = Lexer::new(
            Syntax::Typescript(TsSyntax {
                tsx: true,
                ..Default::default()
            }),
            Default::default(),
            StringInput::from(&*fm),
            None,
        );
        let mut parser = Parser::new_from(lexer);

        for e in parser.take_errors() {
            e.into_diagnostic(&handler).emit();
        }

        let module = match parser.parse_module() {
            Ok(m) => m,
            Err(e) => {
                e.into_diagnostic(&handler).emit();
                return;
            }
        };

        let mut visitor = ReactComponentVisitor::default();
        module.visit_with(&mut visitor);
        let vue_code = visitor.to_vue_with_cm(&cm);
        let vue_path = if input_path.ends_with(".tsx") {
            input_path.trim_end_matches(".tsx").to_string() + ".vue"
        } else {
            format!("{}.vue", input_path)
        };
        if let Err(e) = fs::write(&vue_path, vue_code) {
            eprintln!("写入 Vue 文件失败: {}", e);
        } else {
            println!("转换完成: {} -> {}", input_path, vue_path);
        }
    });
}

#[derive(Default)]
struct ReactComponentVisitor {
    pub jsx: Option<Box<Expr>>,
    pub script: Option<String>,
}

impl Visit for ReactComponentVisitor {
    fn visit_module_item(&mut self, n: &ModuleItem) {
        if let ModuleItem::ModuleDecl(ModuleDecl::ExportDecl(ed)) = n {
            if let Decl::Fn(f) = &ed.decl {
                let mut jsx_finder = JsxReturnFinder::default();
                f.function.visit_with(&mut jsx_finder);
                self.jsx = jsx_finder.jsx;
                self.script = Some("// props, methods...\n// ...\n".to_string());
            }
        }

        // 新增：处理顶层表达式
        if let ModuleItem::Stmt(Stmt::Expr(expr_stmt)) = n {
            let mut jsx_finder = JsxReturnFinder::default();
            expr_stmt.expr.visit_with(&mut jsx_finder);
            if jsx_finder.jsx.is_some() {
                self.jsx = jsx_finder.jsx;
                self.script = Some("// 入口渲染逻辑已提取\n".to_string());
            }
        }

        n.visit_children_with(self); // 正确递归
    }
}

#[derive(Default)]
struct JsxReturnFinder {
    pub jsx: Option<Box<Expr>>,
}

impl Visit for JsxReturnFinder {
    fn visit_return_stmt(&mut self, n: &ReturnStmt) {
        if let Some(arg) = &n.arg {
            self.jsx = Some(arg.clone());
        }
        n.visit_children_with(self);
    }
    fn visit_call_expr(&mut self, n: &CallExpr) {
        for arg in &n.args {
            arg.expr.visit_with(self);
        }
        n.visit_children_with(self);
    }
    fn visit_jsx_element(&mut self, n: &JSXElement) {
        self.jsx = Some(Box::new(Expr::JSXElement(Box::new(n.clone()))));
    }
    fn visit_jsx_fragment(&mut self, n: &JSXFragment) {
        self.jsx = Some(Box::new(Expr::JSXFragment(n.clone())));
    }
}

impl JsxReturnFinder {
    fn jsx_to_string(&self, cm: &Lrc<SourceMap>) -> Option<String> {
        if let Some(ref jsx) = self.jsx {
            let mut buf = Cursor::new(Vec::new());
            let mut cfg = CodegenConfig::default();
            cfg.minify = false;
            cfg.ascii_only = false;
            cfg.emit_assert_for_import_attributes = false;
            cfg.inline_script = false;
            cfg.omit_last_semi = false;
            cfg.reduce_escaped_newline = false;
            let mut emitter = Emitter {
                cfg,
                comments: None,
                cm: cm.clone(),
                wr: JsWriter::new(cm.clone(), "\n", &mut buf, None),
            };
            // 包装为一个表达式语句 -> 语句 -> 模块项 -> 模块
            let expr_stmt = Stmt::Expr(ExprStmt {
                span: DUMMY_SP,
                expr: jsx.clone(),
            });
            let module_item = ModuleItem::Stmt(expr_stmt);
            let module = Module {
                span: DUMMY_SP,
                body: vec![module_item],
                shebang: None,
            };
            emitter.emit_module(&module).ok()?;
            return String::from_utf8(buf.into_inner()).ok();
        }
        None
    }
}

impl ReactComponentVisitor {
    pub fn to_vue_with_cm(&self, cm: &Lrc<SourceMap>) -> String {
        let template = self
            .jsx
            .as_ref()
            .and_then(|jsx| {
                let jsx_finder = JsxReturnFinder {
                    jsx: Some(jsx.clone()),
                };
                jsx_finder.jsx_to_string(cm)
            })
            .unwrap_or_else(|| "".to_string());
        let script = self.script.as_deref().unwrap_or("");
        format!(
            "<template>\n{}\n</template>\n\n<script setup lang=\"ts\">\n{}\n</script>\n",
            template, script
        )
    }
}
