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

    pub fn render_module(&self, module: &Module) -> String {
        let mut out = String::new();
        for item in &module.items {
            if let Item::Function(f) = item {
                out.push_str(&self.render_steps(&f.body, 1));
            }
        }
        out
    }

    fn render_steps(&self, block: &Block, depth: usize) -> String {
        let mut out = String::new();
        for (i, stmt) in block.0.iter().enumerate() {
            let indent = "  ".repeat(depth);
            out.push_str(&format!("{}{}. ", indent, i + 1));
            self.render_stmt(stmt, depth, &mut out);
        }
        out
    }

    fn render_stmt(&self, stmt: &Stmt, depth: usize, out: &mut String) {
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
                let line = match (self.lang, expr) {
                    (NaturalLang::Zh, Some(e)) => format!("返回 {}", expr_to_text(e)),
                    (NaturalLang::En, Some(e)) => format!("Return {}", expr_to_text(e)),
                    (NaturalLang::Zh, None) => "返回".to_string(),
                    (NaturalLang::En, None) => "Return".to_string(),
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
            Stmt::If {
                cond,
                then_block,
                else_block,
            } => {
                let line = match self.lang {
                    NaturalLang::Zh => format!("如果 {}，则：", expr_to_text(cond)),
                    NaturalLang::En => format!("If {}, then:", expr_to_text(cond)),
                };
                out.push_str(&format!("{}\n", line));
                out.push_str(&self.render_steps(then_block, depth + 1));
                if let Some(else_block) = else_block {
                    self.render_else(else_block, depth, out);
                }
            }
            Stmt::While { cond, body } => {
                let line = match self.lang {
                    NaturalLang::Zh => format!("当 {} 时重复以下步骤：", expr_to_text(cond)),
                    NaturalLang::En => format!("While {}, repeat:", expr_to_text(cond)),
                };
                out.push_str(&format!("{}\n", line));
                out.push_str(&self.render_steps(body, depth + 1));
            }
            Stmt::For { kind, body } => {
                let line = self.render_for_header(kind);
                out.push_str(&format!("{}\n", line));
                out.push_str(&self.render_steps(body, depth + 1));
            }
        }
    }

    fn render_else(&self, else_block: &Block, depth: usize, out: &mut String) {
        let indent = "  ".repeat(depth);
        if let [Stmt::If {
            cond,
            then_block,
            else_block: nested_else,
        }] = else_block.0.as_slice()
        {
            let line = match self.lang {
                NaturalLang::Zh => format!("否则如果 {}，则：", expr_to_text(cond)),
                NaturalLang::En => format!("Otherwise if {}, then:", expr_to_text(cond)),
            };
            out.push_str(&format!("{}{}\n", indent, line));
            out.push_str(&self.render_steps(then_block, depth + 1));
            if let Some(nested) = nested_else {
                self.render_else(nested, depth, out);
            }
        } else {
            let line = match self.lang {
                NaturalLang::Zh => "否则：",
                NaturalLang::En => "Otherwise:",
            };
            out.push_str(&format!("{}{}\n", indent, line));
            out.push_str(&self.render_steps(else_block, depth + 1));
        }
    }

    fn render_for_header(&self, kind: &ForKind) -> String {
        match kind {
            ForKind::ForEach { var, iter } => match self.lang {
                NaturalLang::Zh => {
                    format!("对 {} 中的每个 {}，重复以下步骤：", expr_to_text(iter), var)
                }
                NaturalLang::En => format!("For each {} in {}, repeat:", var, expr_to_text(iter)),
            },
            ForKind::Range {
                var,
                start,
                end,
                step,
            } => match self.lang {
                NaturalLang::Zh => {
                    let step_text = step
                        .as_ref()
                        .map(|s| format!("，步长为 {}", expr_to_text(s)))
                        .unwrap_or_default();
                    format!(
                        "令 {} 从 {} 到 {}{}，重复以下步骤：",
                        var,
                        expr_to_text(start),
                        expr_to_text(end),
                        step_text
                    )
                }
                NaturalLang::En => {
                    let step_text = step
                        .as_ref()
                        .map(|s| format!(", step {}", expr_to_text(s)))
                        .unwrap_or_default();
                    format!(
                        "For {} from {} to {}{}, repeat:",
                        var,
                        expr_to_text(start),
                        expr_to_text(end),
                        step_text
                    )
                }
            },
            ForKind::CStyle { init, cond, step } => match self.lang {
                NaturalLang::Zh => format!(
                    "按初始化 {}、条件 {} 和步进 {} 循环执行：",
                    self.render_stmt_inline(init),
                    expr_to_text(cond),
                    expr_to_text(step)
                ),
                NaturalLang::En => format!(
                    "Loop with init {}, condition {}, and step {}, repeat:",
                    self.render_stmt_inline(init),
                    expr_to_text(cond),
                    expr_to_text(step)
                ),
            },
        }
    }

    fn render_stmt_inline(&self, stmt: &Stmt) -> String {
        match stmt {
            Stmt::Assign { target, value } => match self.lang {
                NaturalLang::Zh => {
                    format!("将 {} 赋值为 {}", expr_to_text(target), expr_to_text(value))
                }
                NaturalLang::En => {
                    format!("Assign {} to {}", expr_to_text(target), expr_to_text(value))
                }
            },
            Stmt::VarDecl(var) => match &var.init {
                Some(init) => match self.lang {
                    NaturalLang::Zh => format!("声明 {} 并赋值为 {}", var.name, expr_to_text(init)),
                    NaturalLang::En => {
                        format!("Declare {} and set it to {}", var.name, expr_to_text(init))
                    }
                },
                None => match self.lang {
                    NaturalLang::Zh => format!("声明 {}", var.name),
                    NaturalLang::En => format!("Declare {}", var.name),
                },
            },
            Stmt::ExprStmt(e) => expr_to_text(e),
            Stmt::Raw(text) => text.clone(),
            _ => {
                let mut out = String::new();
                self.render_stmt(stmt, 0, &mut out);
                out.trim().replace('\n', " ")
            }
        }
    }

    // Used by render_function in later tasks; kept private until then.
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
        Expr::Tuple(items) => items
            .iter()
            .map(expr_to_text)
            .collect::<Vec<_>>()
            .join(", "),
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
    fn renders_while_and_if_steps_zh() {
        let source = r#"
def foo(x):
    while x > 0:
        if x == 1:
            return x
        x = x - 1
"#;
        let module = PythonParser::new().parse(source).unwrap();
        let out = ExplainRenderer::new(NaturalLang::Zh).render_module(&module);
        assert!(out.contains("当 x > 0 时重复以下步骤"));
        assert!(out.contains("如果 x = 1，则"));
        assert!(out.contains("返回 x"));
    }

    #[test]
    fn renders_while_and_nested_if_with_numbering_zh() {
        let source = r#"
def foo(x):
    while x > 0:
        if x == 1:
            return x
        x = x - 1
"#;
        let module = PythonParser::new().parse(source).unwrap();
        let out = ExplainRenderer::new(NaturalLang::Zh).render_module(&module);
        assert_eq!(
            out,
            "  1. 当 x > 0 时重复以下步骤：\n    1. 如果 x = 1，则：\n      1. 返回 x\n    2. 将 x 赋值为 x - 1\n"
        );
    }

    #[test]
    fn render_module_starts_function_steps_at_depth_one() {
        let source = r#"
def foo():
    return 1
"#;
        let module = PythonParser::new().parse(source).unwrap();
        let out = ExplainRenderer::new(NaturalLang::Zh).render_module(&module);
        assert_eq!(out, "  1. 返回 1\n");
    }

    #[test]
    fn renders_for_each_header_en() {
        let stmt = Stmt::For {
            kind: ForKind::ForEach {
                var: "x".into(),
                iter: Expr::Ident("nums".into()),
            },
            body: Block(vec![Stmt::Break]),
        };
        let mut out = String::new();
        ExplainRenderer::new(NaturalLang::En).render_stmt(&stmt, 0, &mut out);
        assert!(out.contains("For each x in nums, repeat:"));
        assert!(out.contains("Break out of loop"));
    }

    #[test]
    fn renders_range_step_zh() {
        let stmt = Stmt::For {
            kind: ForKind::Range {
                var: "i".into(),
                start: Expr::Literal(Literal::Int(0)),
                end: Expr::Ident("n".into()),
                step: Some(Expr::Literal(Literal::Int(2))),
            },
            body: Block(vec![Stmt::Continue]),
        };
        let mut out = String::new();
        ExplainRenderer::new(NaturalLang::Zh).render_stmt(&stmt, 0, &mut out);
        assert!(out.contains("令 i 从 0 到 n，步长为 2，重复以下步骤："));
    }

    #[test]
    fn renders_cstyle_for_header_with_init_cond_step_en() {
        let stmt = Stmt::For {
            kind: ForKind::CStyle {
                init: Box::new(Stmt::Assign {
                    target: Expr::Ident("i".into()),
                    value: Expr::Literal(Literal::Int(0)),
                }),
                cond: Expr::Binary {
                    op: BinOp::Lt,
                    lhs: Box::new(Expr::Ident("i".into())),
                    rhs: Box::new(Expr::Ident("n".into())),
                },
                step: Expr::Raw("i = i + 1".into()),
            },
            body: Block(vec![Stmt::Break]),
        };
        let mut out = String::new();
        ExplainRenderer::new(NaturalLang::En).render_stmt(&stmt, 0, &mut out);
        assert!(out.contains(
            "Loop with init Assign i to 0, condition i < n, and step i = i + 1, repeat:"
        ));
    }

    #[test]
    fn renders_else_if_zh() {
        let source = r#"
def foo(x):
    if x == 1:
        return 1
    elif x == 2:
        return 2
    else:
        return 3
"#;
        let module = PythonParser::new().parse(source).unwrap();
        let out = ExplainRenderer::new(NaturalLang::Zh).render_module(&module);
        assert!(out.contains("否则如果 x = 2，则"));
        assert!(out.contains("否则："));
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
    fn renders_return_none_stmt_en_without_trailing_space() {
        let renderer = ExplainRenderer::new(NaturalLang::En);
        let mut out = String::new();
        renderer.render_stmt(&Stmt::Return(None), 0, &mut out);
        assert_eq!(out, "Return\n");
    }

    #[test]
    fn renders_return_none_stmt_zh_without_trailing_space() {
        let renderer = ExplainRenderer::new(NaturalLang::Zh);
        let mut out = String::new();
        renderer.render_stmt(&Stmt::Return(None), 0, &mut out);
        assert_eq!(out, "返回\n");
    }

    #[test]
    fn renders_break_stmt_en() {
        let renderer = ExplainRenderer::new(NaturalLang::En);
        let mut out = String::new();
        renderer.render_stmt(&Stmt::Break, 0, &mut out);
        assert_eq!(out, "Break out of loop\n");
    }

    #[test]
    fn renders_continue_stmt_en() {
        let renderer = ExplainRenderer::new(NaturalLang::En);
        let mut out = String::new();
        renderer.render_stmt(&Stmt::Continue, 0, &mut out);
        assert_eq!(out, "Continue to next iteration\n");
    }

    #[test]
    fn renders_expr_stmt() {
        let renderer = ExplainRenderer::new(NaturalLang::En);
        let mut out = String::new();
        renderer.render_stmt(
            &Stmt::ExprStmt(Expr::Ident("tick".to_string())),
            0,
            &mut out,
        );
        assert_eq!(out, "tick\n");
    }

    #[test]
    fn renders_vardecl_without_init_en() {
        let renderer = ExplainRenderer::new(NaturalLang::En);
        let mut out = String::new();
        let stmt = Stmt::VarDecl(VarDecl {
            name: "x".to_string(),
            type_hint: None,
            init: None,
        });
        renderer.render_stmt(&stmt, 0, &mut out);
        assert_eq!(out, "Declare x\n");
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
