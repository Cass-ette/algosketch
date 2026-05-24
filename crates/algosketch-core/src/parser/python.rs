use crate::error::{PseudoError, Result};
use crate::ir::*;
use crate::parser::common::{
    find_anon_operator, named_child_by_kind, node_text, parse_bin_op, parse_comparison_op,
    parse_err, parse_un_op,
};
use crate::parser::LanguageParser;
use crate::SourceLang;

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
                        .child_by_field_name("left")
                        .or_else(|| inner.named_child(0))
                        .ok_or_else(|| parse_err("assignment missing target"))?;
                    let value = inner
                        .child_by_field_name("right")
                        .or_else(|| inner.named_child(1))
                        .ok_or_else(|| parse_err("assignment missing value"))?;
                    if let Some(type_node) = inner.child_by_field_name("type") {
                        if target.kind() == "identifier" {
                            return Ok(Stmt::VarDecl(VarDecl {
                                name: node_text(source, target).to_string(),
                                type_hint: Some(TypeHint(node_text(source, type_node).to_string())),
                                init: Some(parse_expr(source, value)?),
                            }));
                        }
                    }
                    Ok(Stmt::Assign {
                        target: parse_expr(source, target)?,
                        value: parse_expr(source, value)?,
                    })
                }
                "typed_assignment" => parse_typed_assignment(source, inner),
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
        "for_statement" => parse_for_stmt(source, node),
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
        "break_statement" => Ok(Stmt::Break),
        "continue_statement" => Ok(Stmt::Continue),
        _ => Ok(Stmt::Raw(node_text(source, node).to_string())),
    }
}

fn parse_typed_assignment(source: &str, node: tree_sitter::Node) -> Result<Stmt> {
    let target = node
        .child_by_field_name("left")
        .ok_or_else(|| parse_err("typed_assignment missing target"))?;
    let type_node = node.child_by_field_name("type");
    let value = node
        .child_by_field_name("right")
        .map(|value| parse_expr(source, value))
        .transpose()?;

    if target.kind() != "identifier" {
        return Ok(Stmt::Raw(node_text(source, node).to_string()));
    }

    Ok(Stmt::VarDecl(VarDecl {
        name: node_text(source, target).to_string(),
        type_hint: type_node.map(|ty| TypeHint(node_text(source, ty).to_string())),
        init: value,
    }))
}

fn parse_for_stmt(source: &str, node: tree_sitter::Node) -> Result<Stmt> {
    let target = node
        .named_child(0)
        .ok_or_else(|| parse_err("for missing target"))?;
    let iter = node
        .named_child(1)
        .ok_or_else(|| parse_err("for missing iterable"))?;
    let body = node
        .named_child(2)
        .ok_or_else(|| parse_err("for missing body"))?;

    let var = match target.kind() {
        "identifier" => node_text(source, target).to_string(),
        _ => return Ok(Stmt::Raw(node_text(source, node).to_string())),
    };

    Ok(Stmt::For {
        kind: python_for_kind(source, var, iter)?,
        body: parse_block(source, body)?,
    })
}

fn python_for_kind(source: &str, var: String, iter: tree_sitter::Node) -> Result<ForKind> {
    if iter.kind() == "call" {
        let callee = iter
            .named_child(0)
            .ok_or_else(|| parse_err("call missing callee"))?;
        if callee.kind() == "identifier" && node_text(source, callee) == "range" {
            let args_node = iter
                .named_child(1)
                .ok_or_else(|| parse_err("call missing args"))?;
            let mut args = Vec::new();
            for i in 0..args_node.named_child_count() {
                args.push(parse_expr(source, args_node.named_child(i).unwrap())?);
            }
            return match args.as_slice() {
                [end] => Ok(ForKind::Range {
                    var,
                    start: Expr::Literal(Literal::Int(0)),
                    end: end.clone(),
                    step: None,
                }),
                [start, end] => Ok(ForKind::Range {
                    var,
                    start: start.clone(),
                    end: end.clone(),
                    step: None,
                }),
                [start, end, step] => Ok(ForKind::Range {
                    var,
                    start: start.clone(),
                    end: end.clone(),
                    step: Some(step.clone()),
                }),
                _ => Ok(ForKind::ForEach {
                    var,
                    iter: parse_expr(source, iter)?,
                }),
            };
        }
    }

    Ok(ForKind::ForEach {
        var,
        iter: parse_expr(source, iter)?,
    })
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
        "string" => {
            let text = node_text(source, node);
            Ok(Expr::Literal(Literal::Str(
                text.trim_matches(['\"', '\'']).to_string(),
            )))
        }
        "true" => Ok(Expr::Literal(Literal::Bool(true))),
        "false" => Ok(Expr::Literal(Literal::Bool(false))),
        "none" => Ok(Expr::Literal(Literal::None)),
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

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn java_grammar_loads() {
        let mut parser = tree_sitter::Parser::new();
        parser
            .set_language(&tree_sitter_java::LANGUAGE.into())
            .expect("java grammar should load");
    }

    #[test]
    fn cpp_grammar_loads() {
        let mut parser = tree_sitter::Parser::new();
        parser
            .set_language(&tree_sitter_cpp::LANGUAGE.into())
            .expect("cpp grammar should load");
    }

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
    fn parses_python_range_for_loop() {
        let source = r#"
def first_even(nums):
    for i in range(0, len(nums)):
        if nums[i] % 2 == 0:
            return nums[i]
    return -1
"#;

        let module = PythonParser::new().parse(source).unwrap();
        let Item::Function(function) = &module.items[0] else {
            panic!("expected function");
        };
        let Stmt::For { kind, body } = &function.body.0[0] else {
            panic!("expected for loop");
        };
        let ForKind::Range {
            var,
            start,
            end,
            step,
        } = kind
        else {
            panic!("expected range for loop");
        };

        assert_eq!(var, "i");
        assert_eq!(start, &Expr::Literal(Literal::Int(0)));
        assert_eq!(
            end,
            &Expr::Call {
                callee: Box::new(Expr::Ident("len".to_string())),
                args: vec![Expr::Ident("nums".to_string())]
            }
        );
        assert_eq!(step, &None);
        assert!(matches!(body.0[0], Stmt::If { .. }));
    }

    #[test]
    fn parses_python_for_each_loop() {
        let source = r#"
def total(nums):
    sum = 0
    for value in nums:
        sum = sum + value
    return sum
"#;

        let module = PythonParser::new().parse(source).unwrap();
        let Item::Function(function) = &module.items[0] else {
            panic!("expected function");
        };
        let Stmt::For { kind, body } = &function.body.0[1] else {
            panic!("expected for loop");
        };
        assert_eq!(
            kind,
            &ForKind::ForEach {
                var: "value".to_string(),
                iter: Expr::Ident("nums".to_string())
            }
        );
        assert!(matches!(body.0[0], Stmt::Assign { .. }));
    }

    #[test]
    fn parses_python_break_and_continue() {
        let source = r#"
def scan(nums):
    while True:
        continue
        break
"#;

        let module = PythonParser::new().parse(source).unwrap();
        let Item::Function(function) = &module.items[0] else {
            panic!("expected function");
        };
        let Stmt::While { body, .. } = &function.body.0[0] else {
            panic!("expected while");
        };
        assert!(matches!(body.0[0], Stmt::Continue));
        assert!(matches!(body.0[1], Stmt::Break));
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
