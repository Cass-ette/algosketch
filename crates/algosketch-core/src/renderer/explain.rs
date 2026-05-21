//! Natural-language explanation renderer.

use crate::ir::*;
use crate::NaturalLang;

#[derive(Debug)]
pub struct ExplainRenderer {
    pub lang: NaturalLang,
}

#[allow(dead_code)]
impl ExplainRenderer {
    pub fn new(lang: NaturalLang) -> Self {
        Self { lang }
    }

    pub fn render_module(&self, _module: &Module) -> String {
        String::new()
    }

    fn detect_purpose(&self, f: &Function) -> String {
        let name_lower = f.name.to_lowercase();
        let has_loop = self.has_loop(&f.body);
        let has_recursion = self.has_recursion(f);

        let action = if name_lower.contains("search") || name_lower.contains("find") {
            match self.lang {
                NaturalLang::Zh => "查找",
                NaturalLang::En => "search for",
            }
        } else if name_lower.contains("sort") {
            match self.lang {
                NaturalLang::Zh => "排序",
                NaturalLang::En => "sort",
            }
        } else if name_lower.contains("reverse") {
            match self.lang {
                NaturalLang::Zh => "反转",
                NaturalLang::En => "reverse",
            }
        } else {
            match self.lang {
                NaturalLang::Zh => "处理",
                NaturalLang::En => "process",
            }
        };

        let method = if has_recursion {
            match self.lang {
                NaturalLang::Zh => "（递归）",
                NaturalLang::En => " (recursively)",
            }
        } else if has_loop {
            match self.lang {
                NaturalLang::Zh => "（迭代）",
                NaturalLang::En => " (iteratively)",
            }
        } else {
            ""
        };

        match self.lang {
            NaturalLang::Zh => format!("{}输入数据{}", action, method),
            NaturalLang::En => format!("{} the input{}", action, method),
        }
    }

    fn has_loop(&self, block: &Block) -> bool {
        block.0.iter().any(|stmt| self.stmt_has_loop(stmt))
    }

    fn stmt_has_loop(&self, stmt: &Stmt) -> bool {
        match stmt {
            Stmt::If {
                then_block,
                else_block,
                ..
            } => self.has_loop(then_block) || else_block.as_ref().is_some_and(|b| self.has_loop(b)),
            Stmt::While { .. } | Stmt::For { .. } => true,
            _ => false,
        }
    }

    fn has_recursion(&self, f: &Function) -> bool {
        self.block_calls_function(&f.body, &f.name)
    }

    fn block_calls_function(&self, block: &Block, fname: &str) -> bool {
        block
            .0
            .iter()
            .any(|stmt| self.stmt_calls_function(stmt, fname))
    }

    fn stmt_calls_function(&self, stmt: &Stmt, fname: &str) -> bool {
        match stmt {
            Stmt::Return(Some(e)) | Stmt::ExprStmt(e) => self.expr_calls_function(e, fname),
            Stmt::Assign { value, .. } => self.expr_calls_function(value, fname),
            Stmt::VarDecl(v) => v
                .init
                .as_ref()
                .is_some_and(|e| self.expr_calls_function(e, fname)),
            Stmt::If {
                then_block,
                else_block,
                ..
            } => {
                self.block_calls_function(then_block, fname)
                    || else_block
                        .as_ref()
                        .is_some_and(|b| self.block_calls_function(b, fname))
            }
            Stmt::While { body, .. } | Stmt::For { body, .. } => {
                self.block_calls_function(body, fname)
            }
            _ => false,
        }
    }

    fn expr_calls_function(&self, expr: &Expr, fname: &str) -> bool {
        match expr {
            Expr::Call { callee, args } => {
                matches!(callee.as_ref(), Expr::Ident(name) if name == fname)
                    || args.iter().any(|arg| self.expr_calls_function(arg, fname))
            }
            Expr::Binary { lhs, rhs, .. } => {
                self.expr_calls_function(lhs, fname) || self.expr_calls_function(rhs, fname)
            }
            Expr::Unary { expr, .. } => self.expr_calls_function(expr, fname),
            Expr::Tuple(items) => items.iter().any(|e| self.expr_calls_function(e, fname)),
            Expr::Index { obj, index } => {
                self.expr_calls_function(obj, fname) || self.expr_calls_function(index, fname)
            }
            Expr::Field { obj, .. } => self.expr_calls_function(obj, fname),
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
    fn detects_loop_returns_false_without_loop() {
        let source = r#"
def foo():
    x = 1
    return x
"#;
        let module = PythonParser::new().parse(source).unwrap();
        let renderer = ExplainRenderer::new(NaturalLang::Zh);
        if let Some(Item::Function(f)) = module.items.first() {
            assert!(!renderer.has_loop(&f.body));
        } else {
            panic!("Expected function");
        }
    }

    #[test]
    fn detects_loop_nested_under_if() {
        let source = r#"
def foo(flag):
    if flag:
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

    #[test]
    fn detects_recursion_returns_false_for_different_function_call() {
        let source = r#"
def factorial(n):
    return helper(n - 1)
"#;
        let module = PythonParser::new().parse(source).unwrap();
        let renderer = ExplainRenderer::new(NaturalLang::Zh);
        if let Some(Item::Function(f)) = module.items.first() {
            assert!(!renderer.has_recursion(f));
        } else {
            panic!("Expected function");
        }
    }

    #[test]
    fn detects_recursion_under_unary_expression() {
        let source = r#"
def factorial(n):
    return -factorial(n - 1)
"#;
        let module = PythonParser::new().parse(source).unwrap();
        let renderer = ExplainRenderer::new(NaturalLang::Zh);
        if let Some(Item::Function(f)) = module.items.first() {
            assert!(renderer.has_recursion(f));
        } else {
            panic!("Expected function");
        }
    }

    #[test]
    fn detects_recursion_in_call_argument() {
        let source = r#"
def factorial(n):
    return helper(factorial(n - 1))
"#;
        let module = PythonParser::new().parse(source).unwrap();
        let renderer = ExplainRenderer::new(NaturalLang::Zh);
        if let Some(Item::Function(f)) = module.items.first() {
            assert!(renderer.has_recursion(f));
        } else {
            panic!("Expected function");
        }
    }

    #[test]
    fn detects_search_purpose_zh() {
        let source = r#"
def binary_search(nums, target):
    while True:
        pass
"#;
        let module = PythonParser::new().parse(source).unwrap();
        let renderer = ExplainRenderer::new(NaturalLang::Zh);
        if let Some(Item::Function(f)) = module.items.first() {
            let purpose = renderer.detect_purpose(f);
            assert!(purpose.contains("查找"));
            assert!(purpose.contains("迭代"));
        } else {
            panic!("Expected function");
        }
    }

    #[test]
    fn detects_search_purpose_en() {
        let source = r#"
def binary_search(nums, target):
    while True:
        pass
"#;
        let module = PythonParser::new().parse(source).unwrap();
        let renderer = ExplainRenderer::new(NaturalLang::En);
        if let Some(Item::Function(f)) = module.items.first() {
            let purpose = renderer.detect_purpose(f);
            assert!(purpose.contains("search"));
            assert!(purpose.contains("iteratively"));
        } else {
            panic!("Expected function");
        }
    }
}
