//! Natural-language explanation renderer.

use crate::NaturalLang;
use crate::ir::*;

#[derive(Debug)]
pub struct ExplainRenderer {
    pub lang: NaturalLang,
}

impl ExplainRenderer {
    pub fn new(lang: NaturalLang) -> Self {
        Self { lang }
    }

    pub fn render_module(&self, _module: &Module) -> String {
        String::new()
    }

    fn has_loop(&self, block: &Block) -> bool {
        block.0.iter().any(|stmt| matches!(stmt, Stmt::While { .. } | Stmt::For { .. }))
    }

    fn has_recursion(&self, f: &Function) -> bool {
        self.block_calls_function(&f.body, &f.name)
    }

    fn block_calls_function(&self, block: &Block, fname: &str) -> bool {
        block.0.iter().any(|stmt| self.stmt_calls_function(stmt, fname))
    }

    fn stmt_calls_function(&self, stmt: &Stmt, fname: &str) -> bool {
        match stmt {
            Stmt::Return(Some(e)) | Stmt::ExprStmt(e) => self.expr_calls_function(e, fname),
            Stmt::Assign { value, .. } => self.expr_calls_function(value, fname),
            Stmt::If { then_block, else_block, .. } => {
                self.block_calls_function(then_block, fname)
                    || else_block.as_ref().map_or(false, |b| self.block_calls_function(b, fname))
            }
            Stmt::While { body, .. } | Stmt::For { body, .. } => self.block_calls_function(body, fname),
            _ => false,
        }
    }

    fn expr_calls_function(&self, expr: &Expr, fname: &str) -> bool {
        match expr {
            Expr::Call { callee, .. } => matches!(callee.as_ref(), Expr::Ident(name) if name == fname),
            Expr::Binary { lhs, rhs, .. } => self.expr_calls_function(lhs, fname) || self.expr_calls_function(rhs, fname),
            _ => false,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::parser::{LanguageParser, PythonParser};

    #[test]
    fn renders_empty_module() {
        let source = "";
        let module = PythonParser::new().parse(source).unwrap();
        let out = ExplainRenderer::new(NaturalLang::Zh).render_module(&module);
        assert_eq!(out, "");
    }

    #[test]
    fn detects_loop_in_function() {
        let source = r#"
def foo():
    while True:
        pass
"#;
        let module = PythonParser::new().parse(source).unwrap();
        let renderer = ExplainRenderer::new(NaturalLang::Zh);
        if let Some(Item::Function(f)) = module.items.first() {
            assert!(renderer.has_loop(&f.body));
        } else {
            panic!("Expected function");
        }
    }

    #[test]
    fn detects_recursion_in_function() {
        let source = r#"
def factorial(n):
    return factorial(n - 1)
"#;
        let module = PythonParser::new().parse(source).unwrap();
        let renderer = ExplainRenderer::new(NaturalLang::Zh);
        if let Some(Item::Function(f)) = module.items.first() {
            assert!(renderer.has_recursion(f));
        } else {
            panic!("Expected function");
        }
    }
}
