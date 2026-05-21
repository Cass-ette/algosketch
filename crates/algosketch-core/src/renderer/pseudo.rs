//! Pseudocode renderer (CLRS-style).
//!
//! Walks an algosketch IR and emits CLRS-style pseudocode:
//! uppercase keywords, `←` for assignment, `≤ ≥ ≠`, `DIV`, `MOD`,
//! and `LENGTH(...)` for length-of operations.

use crate::ir::*;

#[derive(Debug, Default, Clone, Copy)]
pub struct PseudoRenderer {
    pub indent_width: usize,
}

impl PseudoRenderer {
    pub fn new() -> Self {
        Self { indent_width: 4 }
    }

    pub fn render_module(&self, module: &Module) -> String {
        let mut out = String::new();
        for item in &module.items {
            self.render_item(item, &mut out);
        }
        out
    }

    pub fn render_function(&self, f: &Function) -> String {
        let mut out = String::new();
        self.render_function_into(f, &mut out);
        out
    }

    fn indent_step(&self) -> usize {
        if self.indent_width == 0 {
            4
        } else {
            self.indent_width
        }
    }

    fn render_item(&self, item: &Item, out: &mut String) {
        match item {
            Item::Function(f) => self.render_function_into(f, out),
            Item::Class(_) | Item::Import(_) | Item::GlobalVar(_) => {}
            Item::Raw(text) => {
                out.push_str(text);
                if !text.ends_with('\n') {
                    out.push('\n');
                }
            }
        }
    }

    fn render_function_into(&self, f: &Function, out: &mut String) {
        let params = f
            .params
            .iter()
            .map(|p| p.name.clone())
            .collect::<Vec<_>>()
            .join(", ");
        out.push_str(&format!("FUNCTION {}({})\n", f.name, params));
        self.render_block(&f.body, 1, out);
        out.push_str("END FUNCTION\n");
    }

    fn render_block(&self, block: &Block, depth: usize, out: &mut String) {
        for stmt in &block.0 {
            self.render_stmt(stmt, depth, out);
        }
    }

    fn pad(&self, depth: usize) -> String {
        " ".repeat(depth * self.indent_step())
    }

    fn render_stmt(&self, stmt: &Stmt, depth: usize, out: &mut String) {
        let pad = self.pad(depth);
        match stmt {
            Stmt::Assign { target, value } => {
                out.push_str(&format!(
                    "{pad}{} ← {}\n",
                    render_expr(target),
                    render_expr(value)
                ));
            }
            Stmt::While { cond, body } => {
                out.push_str(&format!("{pad}WHILE {}\n", render_expr(cond)));
                self.render_block(body, depth + 1, out);
                out.push_str(&format!("{pad}END WHILE\n"));
            }
            Stmt::If {
                cond,
                then_block,
                else_block,
            } => {
                out.push_str(&format!("{pad}IF {} THEN\n", render_expr(cond)));
                self.render_block(then_block, depth + 1, out);
                if let Some(else_blk) = else_block {
                    self.render_else(else_blk, depth, out);
                }
                out.push_str(&format!("{pad}END IF\n"));
            }
            Stmt::Return(expr) => {
                match expr {
                    Some(e) => out.push_str(&format!("{pad}RETURN {}\n", render_expr(e))),
                    None => out.push_str(&format!("{pad}RETURN\n")),
                };
            }
            Stmt::Break => out.push_str(&format!("{pad}BREAK\n")),
            Stmt::Continue => out.push_str(&format!("{pad}CONTINUE\n")),
            Stmt::ExprStmt(e) => out.push_str(&format!("{pad}{}\n", render_expr(e))),
            Stmt::Raw(text) => {
                out.push_str(&format!("{pad}{text} // <unparsed>\n"));
            }
            Stmt::For { .. } | Stmt::VarDecl(_) => {
                out.push_str(&format!("{pad}// <unsupported stmt>\n"));
            }
        }
    }

    fn render_else(&self, else_block: &Block, depth: usize, out: &mut String) {
        let pad = self.pad(depth);
        // Detect `else { if ... }` shape produced by elif lowering and emit ELSE IF.
        if let [Stmt::If {
            cond,
            then_block,
            else_block: nested_else,
        }] = else_block.0.as_slice()
        {
            out.push_str(&format!("{pad}ELSE IF {} THEN\n", render_expr(cond)));
            self.render_block(then_block, depth + 1, out);
            if let Some(nested) = nested_else {
                self.render_else(nested, depth, out);
            }
        } else {
            out.push_str(&format!("{pad}ELSE\n"));
            self.render_block(else_block, depth + 1, out);
        }
    }
}

fn render_expr(expr: &Expr) -> String {
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
        Expr::Unary { op, expr } => {
            format!("{}{}", render_unop(*op), render_expr(expr))
        }
        Expr::Call { callee, args } => render_call(callee, args),
        Expr::Index { obj, index } => format!("{}[{}]", render_expr(obj), render_expr(index)),
        Expr::Field { obj, name } => format!("{}.{}", render_expr(obj), name),
        Expr::Tuple(items) => items.iter().map(render_expr).collect::<Vec<_>>().join(", "),
        Expr::Raw(text) => text.clone(),
    }
}

fn render_binary_operand(expr: &Expr) -> String {
    match expr {
        Expr::Binary { .. } => format!("({})", render_expr(expr)),
        _ => render_expr(expr),
    }
}

fn render_call(callee: &Expr, args: &[Expr]) -> String {
    if let Expr::Ident(name) = callee {
        if name == "len" && args.len() == 1 {
            return format!("LENGTH({})", render_expr(&args[0]));
        }
    }
    if let Expr::Field { obj, name } = callee {
        if (name == "size" || name == "length") && args.is_empty() {
            return format!("LENGTH({})", render_expr(obj));
        }
    }
    let args_str = args.iter().map(render_expr).collect::<Vec<_>>().join(", ");
    format!("{}({})", render_expr(callee), args_str)
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
        BinOp::Ne => "≠",
        BinOp::Lt => "<",
        BinOp::Le => "≤",
        BinOp::Gt => ">",
        BinOp::Ge => "≥",
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
    fn renders_binary_search_pseudocode() {
        let source = r#"
def binary_search(nums, target):
    left, right = 0, len(nums) - 1
    while left <= right:
        mid = (left + right) // 2
        if nums[mid] == target:
            return mid
        elif nums[mid] < target:
            left = mid + 1
        else:
            right = mid - 1
    return -1
"#;

        let module = PythonParser::new().parse(source).unwrap();
        let out = PseudoRenderer::new().render_module(&module);

        assert!(
            out.contains("FUNCTION binary_search(nums, target)"),
            "missing FUNCTION header in:\n{out}"
        );
        assert!(
            out.contains("WHILE left ≤ right"),
            "missing WHILE line in:\n{out}"
        );
        assert!(
            out.contains("mid ← (left + right) DIV 2"),
            "missing mid line in:\n{out}"
        );
        assert!(
            out.contains("IF nums[mid] = target THEN"),
            "missing IF line in:\n{out}"
        );
        assert!(
            out.contains("ELSE IF nums[mid] < target THEN"),
            "missing ELSE IF line in:\n{out}"
        );
        assert!(out.contains("ELSE\n"), "missing ELSE line in:\n{out}");
        assert!(out.contains("RETURN -1"), "missing RETURN -1 in:\n{out}");
        assert!(
            out.contains("END FUNCTION"),
            "missing END FUNCTION in:\n{out}"
        );
    }
}
