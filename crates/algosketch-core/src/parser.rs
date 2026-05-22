use crate::error::{PseudoError, Result};
use crate::ir::*;
use crate::SourceLang;

pub trait LanguageParser {
    fn language(&self) -> SourceLang;
    fn parse(&self, source: &str) -> Result<Module>;
}

pub struct PythonParser;

impl PythonParser {
    pub fn new() -> Self {
        Self
    }
}

impl Default for PythonParser {
    fn default() -> Self {
        Self::new()
    }
}

impl LanguageParser for PythonParser {
    fn language(&self) -> SourceLang {
        SourceLang::Python
    }

    fn parse(&self, source: &str) -> Result<Module> {
        let mut parser = tree_sitter::Parser::new();
        parser
            .set_language(&tree_sitter_python::language())
            .map_err(|e| PseudoError::Internal(format!("tree-sitter init: {e}")))?;
        let tree = parser
            .parse(source, None)
            .ok_or_else(|| PseudoError::Parse {
                file: "input".into(),
                message: "parse failed".into(),
            })?;
        let root = tree.root_node();
        if root.has_error() {
            return Err(PseudoError::Parse {
                file: "input".into(),
                message: "syntax error".into(),
            });
        }
        let mut items = Vec::new();
        for i in 0..root.named_child_count() {
            let child = root.named_child(i).unwrap();
            if child.kind() == "function_definition" {
                items.push(parse_function(source, child)?);
            } else {
                items.push(Item::Raw(node_text(source, child).to_string()));
            }
        }
        Ok(Module {
            source_language: SourceLang::Python,
            items,
        })
    }
}

fn node_text<'a>(source: &'a str, node: tree_sitter::Node<'_>) -> &'a str {
    &source[node.start_byte()..node.end_byte()]
}

fn parse_err(msg: impl Into<String>) -> PseudoError {
    PseudoError::Parse {
        file: "input".into(),
        message: msg.into(),
    }
}

fn named_child_by_kind<'a>(
    node: tree_sitter::Node<'a>,
    kind: &str,
) -> Option<tree_sitter::Node<'a>> {
    (0..node.child_count())
        .filter_map(|i| node.child(i))
        .find(|c| c.is_named() && c.kind() == kind)
}

fn parse_function(source: &str, node: tree_sitter::Node) -> Result<Item> {
    let name = node
        .child_by_field_name("name")
        .or_else(|| named_child_by_kind(node, "identifier"))
        .map(|n| node_text(source, n).to_string())
        .ok_or_else(|| parse_err("function missing name"))?;

    let params_node = node
        .child_by_field_name("parameters")
        .or_else(|| named_child_by_kind(node, "parameters"))
        .ok_or_else(|| parse_err("function missing parameters"))?;

    let mut params = Vec::new();
    for i in 0..params_node.named_child_count() {
        let param = params_node.named_child(i).unwrap();
        if param.kind() == "identifier" {
            params.push(Param {
                name: node_text(source, param).to_string(),
                type_hint: None,
            });
        }
    }

    let body_node = node
        .child_by_field_name("body")
        .or_else(|| named_child_by_kind(node, "block"))
        .ok_or_else(|| parse_err("function missing body"))?;

    let body = parse_block(source, body_node)?;

    Ok(Item::Function(Function {
        name,
        params,
        return_type: None,
        body,
        span: Span::default(),
    }))
}

fn parse_block(source: &str, node: tree_sitter::Node) -> Result<Block> {
    let mut stmts = Vec::new();
    for i in 0..node.named_child_count() {
        let child = node.named_child(i).unwrap();
        stmts.push(parse_stmt(source, child)?);
    }
    Ok(Block(stmts))
}

fn parse_stmt(source: &str, node: tree_sitter::Node) -> Result<Stmt> {
    match node.kind() {
        "expression_statement" => {
            let inner = node
                .named_child(0)
                .ok_or_else(|| parse_err("expression_statement empty"))?;
            match inner.kind() {
                "assignment" => {
                    let target = inner
                        .named_child(0)
                        .ok_or_else(|| parse_err("assignment missing target"))?;
                    let value = inner
                        .named_child(1)
                        .ok_or_else(|| parse_err("assignment missing value"))?;
                    Ok(Stmt::Assign {
                        target: parse_expr(source, target)?,
                        value: parse_expr(source, value)?,
                    })
                }
                _ => Ok(Stmt::ExprStmt(parse_expr(source, inner)?)),
            }
        }
        "while_statement" => {
            let cond = node
                .named_child(0)
                .ok_or_else(|| parse_err("while missing condition"))?;
            let body = node
                .named_child(1)
                .ok_or_else(|| parse_err("while missing body"))?;
            Ok(Stmt::While {
                cond: parse_expr(source, cond)?,
                body: parse_block(source, body)?,
            })
        }
        "if_statement" => {
            let cond = node
                .named_child(0)
                .ok_or_else(|| parse_err("if missing condition"))?;
            let then_block = node
                .named_child(1)
                .ok_or_else(|| parse_err("if missing then block"))?;

            let mut clauses = Vec::new();
            for i in 2..node.named_child_count() {
                clauses.push(node.named_child(i).unwrap());
            }
            let else_block = build_else_chain(source, &clauses)?;

            Ok(Stmt::If {
                cond: parse_expr(source, cond)?,
                then_block: parse_block(source, then_block)?,
                else_block,
            })
        }
        "return_statement" => {
            let expr = node
                .named_child(0)
                .map(|c| parse_expr(source, c))
                .transpose()?;
            Ok(Stmt::Return(expr))
        }
        _ => Ok(Stmt::Raw(node_text(source, node).to_string())),
    }
}

fn build_else_chain(source: &str, clauses: &[tree_sitter::Node]) -> Result<Option<Block>> {
    if clauses.is_empty() {
        return Ok(None);
    }
    let first = clauses[0];
    match first.kind() {
        "elif_clause" => {
            let cond = first
                .named_child(0)
                .ok_or_else(|| parse_err("elif missing condition"))?;
            let body = first
                .named_child(1)
                .ok_or_else(|| parse_err("elif missing body"))?;
            let nested_if = Stmt::If {
                cond: parse_expr(source, cond)?,
                then_block: parse_block(source, body)?,
                else_block: build_else_chain(source, &clauses[1..])?,
            };
            Ok(Some(Block(vec![nested_if])))
        }
        "else_clause" => {
            let body = first
                .named_child(0)
                .ok_or_else(|| parse_err("else missing body"))?;
            Ok(Some(parse_block(source, body)?))
        }
        _ => Ok(None),
    }
}

fn parse_expr(source: &str, node: tree_sitter::Node) -> Result<Expr> {
    match node.kind() {
        "identifier" => Ok(Expr::Ident(node_text(source, node).to_string())),
        "integer" => {
            let text = node_text(source, node);
            let n = text
                .parse::<i64>()
                .map_err(|_| parse_err(format!("invalid integer literal: {text}")))?;
            Ok(Expr::Literal(Literal::Int(n)))
        }
        "call" => {
            let callee = node
                .named_child(0)
                .ok_or_else(|| parse_err("call missing callee"))?;
            let args_node = node
                .named_child(1)
                .ok_or_else(|| parse_err("call missing args"))?;
            let mut args = Vec::new();
            for i in 0..args_node.named_child_count() {
                let arg = args_node.named_child(i).unwrap();
                args.push(parse_expr(source, arg)?);
            }
            Ok(Expr::Call {
                callee: Box::new(parse_expr(source, callee)?),
                args,
            })
        }
        "subscript" => {
            let obj = node
                .named_child(0)
                .ok_or_else(|| parse_err("subscript missing object"))?;
            let index = node
                .named_child(1)
                .ok_or_else(|| parse_err("subscript missing index"))?;
            Ok(Expr::Index {
                obj: Box::new(parse_expr(source, obj)?),
                index: Box::new(parse_expr(source, index)?),
            })
        }
        "attribute" => {
            let obj = node
                .named_child(0)
                .ok_or_else(|| parse_err("attribute missing object"))?;
            let name = node
                .named_child(1)
                .ok_or_else(|| parse_err("attribute missing name"))?;
            Ok(Expr::Field {
                obj: Box::new(parse_expr(source, obj)?),
                name: node_text(source, name).to_string(),
            })
        }
        "parenthesized_expression" => {
            let inner = node
                .named_child(0)
                .ok_or_else(|| parse_err("parenthesized_expression empty"))?;
            parse_expr(source, inner)
        }
        "binary_operator" => {
            let lhs = node
                .named_child(0)
                .ok_or_else(|| parse_err("binary_operator missing lhs"))?;
            let rhs = node
                .named_child(1)
                .ok_or_else(|| parse_err("binary_operator missing rhs"))?;
            let op_text = find_anon_operator(source, node)
                .ok_or_else(|| parse_err("binary_operator missing operator"))?;
            let op = parse_bin_op(op_text)?;
            Ok(Expr::Binary {
                op,
                lhs: Box::new(parse_expr(source, lhs)?),
                rhs: Box::new(parse_expr(source, rhs)?),
            })
        }
        "comparison_operator" => {
            let lhs = node
                .named_child(0)
                .ok_or_else(|| parse_err("comparison_operator missing lhs"))?;
            let rhs = node
                .named_child(1)
                .ok_or_else(|| parse_err("comparison_operator missing rhs"))?;
            let op_text = find_anon_operator(source, node).ok_or_else(|| PseudoError::Parse {
                file: "input".into(),
                message: "comparison_operator missing operator".into(),
            })?;
            let Ok(op) = parse_comparison_op(op_text) else {
                return Ok(Expr::Raw(node_text(source, node).to_string()));
            };
            Ok(Expr::Binary {
                op,
                lhs: Box::new(parse_expr(source, lhs)?),
                rhs: Box::new(parse_expr(source, rhs)?),
            })
        }
        "unary_operator" => {
            let op_text = find_anon_operator(source, node)
                .ok_or_else(|| parse_err("unary_operator missing operator"))?;
            let op = parse_un_op(op_text)?;
            let expr = node
                .named_child(0)
                .ok_or_else(|| parse_err("unary_operator missing operand"))?;
            Ok(Expr::Unary {
                op,
                expr: Box::new(parse_expr(source, expr)?),
            })
        }
        "pattern_list" | "expression_list" | "tuple" => {
            let mut elements = Vec::new();
            for i in 0..node.named_child_count() {
                let child = node.named_child(i).unwrap();
                elements.push(parse_expr(source, child)?);
            }
            Ok(Expr::Tuple(elements))
        }
        _ => Ok(Expr::Raw(node_text(source, node).to_string())),
    }
}

fn find_anon_operator<'a>(source: &'a str, node: tree_sitter::Node<'_>) -> Option<&'a str> {
    for i in 0..node.child_count() {
        if let Some(child) = node.child(i) {
            if !child.is_named() {
                let text = node_text(source, child);
                if !["(", ")", "[", "]", "{", "}", ",", ".", ":", ";"].contains(&text) {
                    return Some(text);
                }
            }
        }
    }
    None
}

fn parse_bin_op(text: &str) -> Result<BinOp> {
    match text {
        "+" => Ok(BinOp::Add),
        "-" => Ok(BinOp::Sub),
        "*" => Ok(BinOp::Mul),
        "/" => Ok(BinOp::Div),
        "//" => Ok(BinOp::IntDiv),
        "%" => Ok(BinOp::Mod),
        "&&" | "and" => Ok(BinOp::And),
        "||" | "or" => Ok(BinOp::Or),
        "&" => Ok(BinOp::BitAnd),
        "|" => Ok(BinOp::BitOr),
        "^" => Ok(BinOp::BitXor),
        "<<" => Ok(BinOp::Shl),
        ">>" => Ok(BinOp::Shr),
        _ => Err(parse_err(format!("unknown binary operator: {text}"))),
    }
}

fn parse_comparison_op(text: &str) -> Result<BinOp> {
    match text {
        "==" => Ok(BinOp::Eq),
        "!=" => Ok(BinOp::Ne),
        "<" => Ok(BinOp::Lt),
        "<=" => Ok(BinOp::Le),
        ">" => Ok(BinOp::Gt),
        ">=" => Ok(BinOp::Ge),
        _ => Err(parse_err(format!("unknown comparison operator: {text}"))),
    }
}

fn parse_un_op(text: &str) -> Result<UnOp> {
    match text {
        "-" => Ok(UnOp::Neg),
        "not" => Ok(UnOp::Not),
        "~" => Ok(UnOp::BitNot),
        _ => Err(parse_err(format!("unknown unary operator: {text}"))),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parses_python_binary_search_function_shape() {
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
        let Item::Function(function) = &module.items[0] else {
            panic!("expected function");
        };

        assert_eq!(function.name, "binary_search");
        assert_eq!(
            function
                .params
                .iter()
                .map(|p| p.name.as_str())
                .collect::<Vec<_>>(),
            vec!["nums", "target"]
        );
        assert!(matches!(function.body.0[1], Stmt::While { .. }));
    }

    #[test]
    fn preserves_unsupported_comparison_as_raw_expr() {
        let source = r#"
def rebuild_path(came_from, current):
    while current in came_from:
        current = came_from[current]
    return current
"#;

        let module = PythonParser::new().parse(source).unwrap();
        let Item::Function(function) = &module.items[0] else {
            panic!("expected function");
        };
        let Stmt::While { cond, .. } = &function.body.0[0] else {
            panic!("expected while");
        };
        assert_eq!(cond, &Expr::Raw("current in came_from".to_string()));
    }

    #[test]
    fn returns_parse_error_for_invalid_python() {
        let result = PythonParser::new().parse("def f(:\n    pass\n");
        match result {
            Err(PseudoError::Parse { .. }) => {}
            other => panic!("expected Parse error, got {other:?}"),
        }
    }
}
