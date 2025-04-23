use std::fs;
use std::io;
use swc_common::{
    errors::{EmitterWriter, Handler},
    sync::Lrc,
    FileName, Globals, SourceMap, GLOBALS,
};
use swc_ecma_ast::*;
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
        let vue_code = visitor.to_vue();
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
    pub jsx: Option<String>,
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
        n.visit_children_with(self); // 正确递归
    }
}

#[derive(Default)]
struct JsxReturnFinder {
    pub jsx: Option<String>,
}

impl Visit for JsxReturnFinder {
    fn visit_return_stmt(&mut self, n: &ReturnStmt) {
        if let Some(arg) = &n.arg {
            self.jsx = Some(format!("{:?}", arg));
        }
        n.visit_children_with(self); // 正确递归
    }
}

impl ReactComponentVisitor {
    pub fn to_vue(&self) -> String {
        let template = self.jsx.as_deref().unwrap_or("");
        let script = self.script.as_deref().unwrap_or("");
        format!(
            "<template>\n{}\n</template>\n\n<script setup lang=\"ts\">\n{}\n</script>\n",
            template, script
        )
    }
}
