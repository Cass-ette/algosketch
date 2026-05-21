//! Natural-language explanation renderer.

use crate::ir::*;
use crate::NaturalLang;

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

    // Used by render_function in later tasks; kept private until then.
    #[allow(dead_code)]
    fn render_stmt(&self, stmt: &Stmt, _depth: usize, out: &mut String) {
        match stmt {
            Stmt::Assign { target, value } => {
                let line = match self.lang {
                    NaturalLang::Zh => {
                        format!("将 {} 赋值为 {}", expr_to_text(target), expr_to_text(value))
                    }
                    NaturalLang::En => {
                        format!("Assign {} to {}", expr_to_text(target), expr_to_text(value))
                    }
                };
                out.push_str(&format!("{}\n", line));
            }
            Stmt::Return(expr) => {
                let val = expr.as_ref().map(expr_to_text).unwrap_or_default();
                let line = match self.lang {
                    NaturalLang::Zh => format!("返回 {}", val),
                    NaturalLang::En => format!("Return {}", val),
                };
                out.push_str(&format!("{}\n", line));
            }
            Stmt::Break => {
                let line = match self.lang {
                    NaturalLang::Zh => "跳出循环",
                    NaturalLang::En => "Break out of loop",
                };
                out.push_str(&format!("{}\n", line));
            }
            Stmt::Continue => {
                let line = match self.lang {
                    NaturalLang::Zh => "继续下一次迭代",
                    NaturalLang::En => "Continue to next iteration",
                };
                out.push_str(&format!("{}\n", line));
            }
            Stmt::ExprStmt(e) => {
                out.push_str(&format!("{}\n", expr_to_text(e)));
            }
            Stmt::Raw(_) => {
                let line = match self.lang {
                    NaturalLang::Zh => "> 此处源代码未能结构化解析，已原样保留。",
                    NaturalLang::En => "> Unparsed source preserved as-is.",
                };
                out.push_str(&format!("{}\n", line));
            }
            Stmt::VarDecl(var) => {
                let line = match &var.init {
                    Some(init) => match self.lang {
                        NaturalLang::Zh => {
                            format!("声明 {} 并赋值为 {}", var.name, expr_to_text(init))
                        }
                        NaturalLang::En => {
                            format!("Declare {} and set it to {}", var.name, expr_to_text(init))
                        }
                    },
                    None => match self.lang {
                        NaturalLang::Zh => format!("声明 {}", var.name),
                        NaturalLang::En => format!("Declare {}", var.name),
                    },
                };
                out.push_str(&format!("{}\n", line));
            }
            _ => {}
        }
    }

    // Used by render_function in Task 7; kept private until then.
    #[allow(dead_code)]
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

fn expr_to_text(expr: &Expr) -> String {
    match expr {
        Expr::Literal(lit) => render_literal(lit),
        Expr::Ident(name) => name.clone(),
        Expr::Binary { op, lhs, rhs } => {
            format!(
                "{} {} {}",
                render_binary_operand(lhs),
                render_binop(*op),
                render_binary_operand(rhs)
            )
        }
        Expr::Unary { op, expr } => format!("{}{}", render_unop(*op), expr_to_text(expr)),
        Expr::Call { callee, args } => render_call(callee, args),
        Expr::Index { obj, index } => format!("{}[{}]", expr_to_text(obj), expr_to_text(index)),
        Expr::Field { obj, name } => format!("{}.{}", expr_to_text(obj), name),
        Expr::Tuple(items) => items.iter().map(expr_to_text).collect::<Vec<_>>().join(", "),
        Expr::Raw(text) => text.clone(),
    }
}

fn render_binary_operand(expr: &Expr) -> String {
    match expr {
        Expr::Binary { .. } => format!("({})", expr_to_text(expr)),
        _ => expr_to_text(expr),
    }
}

fn render_call(callee: &Expr, args: &[Expr]) -> String {
    if let Expr::Ident(name) = callee {
        if name == "len" && args.len() == 1 {
            return format!("LENGTH({})", expr_to_text(&args[0]));
        }
    }
    if let Expr::Field { obj, name } = callee {
        if (name == "size" || name == "length") && args.is_empty() {
            return format!("LENGTH({})", expr_to_text(obj));
        }
    }
    let args_str = args.iter().map(expr_to_text).collect::<Vec<_>>().join(", ");
    format!("{}({})", expr_to_text(callee), args_str)
}

fn render_literal(lit: &Literal) -> String {
    match lit {
        Literal::Int(n) => n.to_string(),
        Literal::Float(s) => s.clone(),
        Literal::Str(s) => format!("\"{s}\""),
        Literal::Bool(b) => if *b { "TRUE" } else { "FALSE" }.to_string(),
        Literal::None => "NIL".to_string(),
    }
}

fn render_binop(op: BinOp) -> &'static str {
    match op {
        BinOp::Add => "+",
        BinOp::Sub => "-",
        BinOp::Mul => "*",
        BinOp::Div => "/",
        BinOp::IntDiv => "DIV",
        BinOp::Mod => "MOD",
        BinOp::Eq => "=",
        BinOp::Ne => "\u{2260}",
        BinOp::Lt => "<",
        BinOp::Le => "\u{2264}",
        BinOp::Gt => ">",
        BinOp::Ge => "\u{2265}",
        BinOp::And => "AND",
        BinOp::Or => "OR",
        BinOp::BitAnd => "&",
        BinOp::BitOr => "|",
        BinOp::BitXor => "^",
        BinOp::Shl => "<<",
        BinOp::Shr => ">>",
    }
}

fn render_unop(op: UnOp) -> &'static str {
    match op {
        UnOp::Neg => "-",
        UnOp::Not => "NOT ",
        UnOp::BitNot => "~",
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

    #[test]
    fn renders_assign_stmt_zh() {
        let source = r#"
def foo():
    x = 5
"#;
        let module = PythonParser::new().parse(source).unwrap();
        let renderer = ExplainRenderer::new(NaturalLang::Zh);
        if let Some(Item::Function(f)) = module.items.first() {
            let mut out = String::new();
            renderer.render_stmt(&f.body.0[0], 0, &mut out);
            assert!(out.contains("将 x 赋值为 5"));
        } else {
            panic!("Expected function");
        }
    }

    #[test]
    fn renders_return_stmt_en() {
        let source = r#"
def foo():
    return 42
"#;
        let module = PythonParser::new().parse(source).unwrap();
        let renderer = ExplainRenderer::new(NaturalLang::En);
        if let Some(Item::Function(f)) = module.items.first() {
            let mut out = String::new();
            renderer.render_stmt(&f.body.0[0], 0, &mut out);
            assert!(out.contains("Return 42"));
        } else {
            panic!("Expected function");
        }
    }

    #[test]
    fn renders_raw_stmt_zh() {
        let renderer = ExplainRenderer::new(NaturalLang::Zh);
        let mut out = String::new();
        renderer.render_stmt(&Stmt::Raw("...".to_string()), 0, &mut out);
        assert!(out.contains("此处源代码未能结构化解析"));
    }

    #[test]
    fn renders_vardecl_with_init_en() {
        let renderer = ExplainRenderer::new(NaturalLang::En);
        let mut out = String::new();
        let stmt = Stmt::VarDecl(VarDecl {
            name: "x".to_string(),
            type_hint: None,
            init: Some(Expr::Literal(Literal::Int(1))),
        });
        renderer.render_stmt(&stmt, 0, &mut out);
        assert!(out.contains("Declare x and set it to 1"));
    }

    #[test]
    fn expr_to_text_uses_pseudocode_operators() {
        let source = r#"
def foo(nums):
    mid = (left + right) // 2
    size = len(nums) - 1
"#;
        let module = PythonParser::new().parse(source).unwrap();
        if let Some(Item::Function(f)) = module.items.first() {
            // First assignment: mid = (left + right) // 2
            let first_assign = f.body.0.first().unwrap();
            if let Stmt::Assign { value, .. } = first_assign {
                assert_eq!(expr_to_text(value), "(left + right) DIV 2");
            } else {
                panic!("Expected Assign");
            }

            // Second assignment: size = len(nums) - 1
            let second_assign = f.body.0.get(1).unwrap();
            if let Stmt::Assign { value, .. } = second_assign {
                assert_eq!(expr_to_text(value), "LENGTH(nums) - 1");
            } else {
                panic!("Expected Assign");
            }
        } else {
            panic!("Expected function");
        }
    }

    #[test]
    fn expr_to_text_renders_comparisons() {
        let source = r#"
def foo(nums, target):
    if nums[mid] == target:
        return mid
"#;
        let module = PythonParser::new().parse(source).unwrap();
        if let Some(Item::Function(f)) = module.items.first() {
            let if_stmt = f.body.0.first().unwrap();
            if let Stmt::If { cond, .. } = if_stmt {
                assert_eq!(expr_to_text(cond), "nums[mid] = target");
            } else {
                panic!("Expected If");
            }
        } else {
            panic!("Expected function");
        }
    }
}
