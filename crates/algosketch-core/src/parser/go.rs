use crate::diagnostics::RawDiagnostics;
use crate::error::{PseudoError, Result};
use crate::ir::*;
use crate::parser::common::{
    named_child_by_kind, named_children_of_kind, node_text, parse_c_family_bin_op, parse_err,
    parse_un_op, record_raw_expr, record_raw_item, record_raw_stmt,
};
use crate::parser::LanguageParser;
use crate::SourceLang;

pub struct GoParser;

impl GoParser {
    pub fn new() -> Self {
        Self
    }
}

impl Default for GoParser {
    fn default() -> Self {
        Self::new()
    }
}

impl LanguageParser for GoParser {
    fn language(&self) -> SourceLang {
        SourceLang::Go
    }

    fn parse(&self, source: &str) -> Result<(Module, RawDiagnostics)> {
        let mut parser = tree_sitter::Parser::new();
        parser
            .set_language(&tree_sitter_go::LANGUAGE.into())
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

        let mut diag = RawDiagnostics::default();
        let mut items = Vec::new();
        for i in 0..root.named_child_count() {
            let child = root.named_child(i).unwrap();
            match child.kind() {
                // package/import carry no algorithmic content: silent skip.
                // Top-level comments are named extras attached to the root
                // (header/license comments are ubiquitous in real Go): also
                // silent, not "unparsed nodes".
                "package_clause" | "import_declaration" | "comment" => continue,
                "function_declaration" | "method_declaration" => {
                    items.push(parse_function(source, child, &mut diag)?);
                }
                _ => items.push(record_raw_item(source, child, &mut diag)),
            }
        }
        Ok((
            Module {
                source_language: SourceLang::Go,
                items,
            },
            diag,
        ))
    }
}

fn parse_function(
    source: &str,
    node: tree_sitter::Node,
    diag: &mut RawDiagnostics,
) -> Result<Item> {
    let name_node = node
        .child_by_field_name("name")
        .ok_or_else(|| parse_err("function missing name"))?;
    let params_node = node
        .child_by_field_name("parameters")
        .ok_or_else(|| parse_err("function missing parameters"))?;
    // Bodyless declarations are legal Go (implemented outside Go, e.g. in
    // assembly): a loud Raw item rather than a hard error, so the rest of
    // the file still parses (mirrors java.rs's bodyless-method guard).
    let Some(body_node) = node.child_by_field_name("body") else {
        return Ok(record_raw_item(source, node, diag));
    };

    let mut params = Vec::new();
    // A method's receiver list comes first: the receiver reads as the
    // leading parameter of the function.
    if let Some(receiver) = node.child_by_field_name("receiver") {
        params.extend(parse_param_list(source, receiver));
    }
    params.extend(parse_param_list(source, params_node));

    Ok(Item::Function(Function {
        name: node_text(source, name_node).to_string(),
        params,
        return_type: None,
        body: parse_block(source, body_node, diag)?,
        span: Span {
            start: node.start_byte(),
            end: node.end_byte(),
        },
    }))
}

fn parse_param_list(source: &str, node: tree_sitter::Node) -> Vec<Param> {
    let mut params = Vec::new();
    for i in 0..node.named_child_count() {
        let child = node.named_child(i).unwrap();
        if child.kind() == "parameter_declaration" {
            // `name` is a multiple field: `func f(low, high int)` is ONE
            // parameter_declaration with two name children, so take every
            // child carrying the field rather than just the first.
            for j in 0..child.child_count() {
                if child.field_name_for_child(j as u32) == Some("name") {
                    let name = child.child(j).unwrap();
                    params.push(Param {
                        name: node_text(source, name).to_string(),
                        type_hint: None,
                    });
                }
            }
            // Unnamed parameters (`func f(int)` has no name field): skipped.
        }
    }
    params
}

/// Go `block` nodes wrap their statements in a single `statement_list` child
/// (unlike java/cpp blocks) — descend through it before iterating statements.
/// A leading comment is a named extra attached to the `block` itself, ahead of
/// the list, so the lookup must be by kind, not by position. A bare
/// `func f() {}` block holds no list: an empty body stays empty.
fn parse_block(source: &str, node: tree_sitter::Node, diag: &mut RawDiagnostics) -> Result<Block> {
    let Some(list) = named_child_by_kind(node, "statement_list") else {
        return Ok(Block(vec![]));
    };
    let mut stmts = Vec::new();
    for i in 0..list.named_child_count() {
        let child = list.named_child(i).unwrap();
        // Mid-body comments are named extras inside the list: skipped, not
        // recorded as Raw (they carry no algorithmic content).
        if child.kind() == "comment" {
            continue;
        }
        if matches!(child.kind(), "var_declaration" | "const_declaration") {
            // One declaration can carry several specs / several names, each
            // yielding its own statement.
            stmts.append(&mut parse_var_declaration(source, child, diag)?);
        } else {
            stmts.push(parse_stmt(source, child, diag)?);
        }
    }
    Ok(Block(stmts))
}

fn parse_stmt(source: &str, node: tree_sitter::Node, diag: &mut RawDiagnostics) -> Result<Stmt> {
    match node.kind() {
        "expression_statement" => {
            let inner = node
                .named_child(0)
                .ok_or_else(|| parse_err("expression_statement empty"))?;
            Ok(Stmt::ExprStmt(parse_expr(source, inner, diag)?))
        }
        "short_var_declaration" => parse_short_var(source, node, diag),
        "assignment_statement" => parse_assignment(source, node, diag),
        "return_statement" => {
            // `return` has no fields; the (optional) `expression_list` is a
            // named child — looked up by KIND, not position: a leading comment
            // (`return /* c */ 1`) attaches as the first named child and a
            // positional read would silently degrade to Return(None).
            let values = match named_child_by_kind(node, "expression_list") {
                Some(list) => parse_expression_list(source, list, diag)?,
                None => Vec::new(),
            };
            Ok(Stmt::Return(match values.len() {
                0 => None,
                1 => Some(values.into_iter().next().expect("len checked")),
                _ => Some(Expr::Tuple(values)),
            }))
        }
        "break_statement" => Ok(Stmt::Break),
        "continue_statement" => Ok(Stmt::Continue),
        "if_statement" => parse_if_stmt(source, node, diag),
        "for_statement" => parse_for_stmt(source, node, diag),
        _ => Ok(record_raw_stmt(source, node, diag)),
    }
}

/// `if cond {} else if ... {} else {}`: tree-sitter-go nests `else if` as the
/// `alternative` being directly an `if_statement` (no else_if_clause kind) —
/// re-nested as `Block(vec![If])` so the IR shape matches python/java and the
/// cross-language skeleton rendering stays uniform.
fn parse_if_stmt(source: &str, node: tree_sitter::Node, diag: &mut RawDiagnostics) -> Result<Stmt> {
    let cond = node
        .child_by_field_name("condition")
        .ok_or_else(|| parse_err("if missing condition"))?;
    let consequence = node
        .child_by_field_name("consequence")
        .ok_or_else(|| parse_err("if missing consequence"))?;
    let else_block = node
        .child_by_field_name("alternative")
        .map(|alternative| {
            if alternative.kind() == "if_statement" {
                parse_if_stmt(source, alternative, diag).map(|stmt| Block(vec![stmt]))
            } else {
                parse_block(source, alternative, diag)
            }
        })
        .transpose()?;

    Ok(Stmt::If {
        cond: parse_expr(source, cond, diag)?,
        then_block: parse_block(source, consequence, diag)?,
        else_block,
    })
}

/// `for`'s ONLY field is `body`; the loop shape comes from named children:
/// a `for_clause` child (C-style), a `range_clause` child (foreach), exactly
/// one other child (`while` with that child as the condition — it carries no
/// field, so it must be found positionally among the named children), or
/// nothing (infinite loop → `While(true)`).
fn parse_for_stmt(
    source: &str,
    node: tree_sitter::Node,
    diag: &mut RawDiagnostics,
) -> Result<Stmt> {
    let body = node
        .child_by_field_name("body")
        .ok_or_else(|| parse_err("for missing body"))?;

    if let Some(clause) = named_child_by_kind(node, "for_clause") {
        // C-style: `for i := 0; i < n; i++`. A partial clause (any of the
        // three fields omitted, e.g. `for i := 0; ; i++`) stays a loud Raw
        // rather than guessing a missing part (cpp parity).
        let (Some(init), Some(cond), Some(update)) = (
            clause.child_by_field_name("initializer"),
            clause.child_by_field_name("condition"),
            clause.child_by_field_name("update"),
        ) else {
            return Ok(record_raw_stmt(source, node, diag));
        };
        return Ok(Stmt::For {
            kind: ForKind::CStyle {
                init: Box::new(parse_stmt(source, init, diag)?),
                cond: parse_expr(source, cond, diag)?,
                // `i++`/`i--` are inc/dec statements with no structured
                // mapping: recorded Raw regardless of the inner kind
                // (cpp/java parity, drives fixture budgets).
                step: record_raw_expr(source, update, diag),
            },
            body: parse_block(source, body, diag)?,
        });
    }

    if let Some(clause) = named_child_by_kind(node, "range_clause") {
        // `for v := range xs`: the left side is an expression_list holding
        // the (single) loop variable. Two variables (`for i, v := range xs`)
        // have no structured shape — the whole statement stays a loud Raw.
        let (Some(left), Some(right)) = (
            clause.child_by_field_name("left"),
            clause.child_by_field_name("right"),
        ) else {
            return Ok(record_raw_stmt(source, node, diag));
        };
        let names = identifier_names(source, left);
        return match names.as_deref() {
            Some([var]) => Ok(Stmt::For {
                kind: ForKind::ForEach {
                    var: var.to_string(),
                    iter: parse_expr(source, right, diag)?,
                },
                body: parse_block(source, body, diag)?,
            }),
            _ => Ok(record_raw_stmt(source, node, diag)),
        };
    }

    // No clause child: collect the named children besides the body block.
    // Comments are named extras attached to the for_statement itself
    // (`for /* c */ n > 0 {}`) and carry no algorithmic content: skipped.
    let rest: Vec<tree_sitter::Node> = (0..node.named_child_count())
        .map(|i| node.named_child(i).unwrap())
        .filter(|c| c.kind() != "comment" && c.id() != body.id())
        .collect();
    match rest.as_slice() {
        [cond] => Ok(Stmt::While {
            cond: parse_expr(source, *cond, diag)?,
            body: parse_block(source, body, diag)?,
        }),
        [] => Ok(Stmt::While {
            cond: Expr::Literal(Literal::Bool(true)),
            body: parse_block(source, body, diag)?,
        }),
        // Not a shape valid Go produces (gated by the upstream has_error
        // check): keep the statement loud, not misparsed.
        _ => Ok(record_raw_stmt(source, node, diag)),
    }
}

/// `y := 4` / `a, b := 1, 2`: single name → declaration; multiple names →
/// assignment over tuples (spec §2).
fn parse_short_var(
    source: &str,
    node: tree_sitter::Node,
    diag: &mut RawDiagnostics,
) -> Result<Stmt> {
    let (Some(left), Some(right)) = (
        node.child_by_field_name("left"),
        node.child_by_field_name("right"),
    ) else {
        return Ok(record_raw_stmt(source, node, diag));
    };
    let values = parse_expression_list(source, right, diag)?;
    let stmts = identifier_names(source, left).and_then(|names| declare_or_assign(names, values));
    match stmts {
        Some(mut stmts) if stmts.len() == 1 => Ok(stmts.pop().expect("len checked")),
        _ => Ok(record_raw_stmt(source, node, diag)),
    }
}

/// `x = y` / `q, r = a, b`: either side may hold multiple expressions.
fn parse_assignment(
    source: &str,
    node: tree_sitter::Node,
    diag: &mut RawDiagnostics,
) -> Result<Stmt> {
    // `+=` & friends reuse the same kind: keep them Raw rather than silently
    // dropping the operator (java/cpp parity).
    if let Some(op) = node.child_by_field_name("operator") {
        if node_text(source, op) != "=" {
            return Ok(record_raw_stmt(source, node, diag));
        }
    }
    let (Some(left), Some(right)) = (
        node.child_by_field_name("left"),
        node.child_by_field_name("right"),
    ) else {
        return Ok(record_raw_stmt(source, node, diag));
    };
    let targets = parse_expression_list(source, left, diag)?;
    let values = parse_expression_list(source, right, diag)?;
    Ok(Stmt::Assign {
        target: single_or_tuple(targets),
        value: single_or_tuple(values),
    })
}

/// `var x int = 3` / `var y, z int` / `const k = 10` — one declaration per
/// inner spec; grouped declarations (`var ( ... )`) nest their specs under a
/// `var_spec_list` wrapper child and yield several statements.
fn parse_var_declaration(
    source: &str,
    node: tree_sitter::Node,
    diag: &mut RawDiagnostics,
) -> Result<Vec<Stmt>> {
    let spec_kind = if node.kind() == "var_declaration" {
        "var_spec"
    } else {
        "const_spec"
    };
    let mut specs = named_children_of_kind(node, spec_kind);
    if specs.is_empty() {
        if let Some(list) = named_child_by_kind(node, "var_spec_list") {
            specs = named_children_of_kind(list, spec_kind);
        }
    }
    let mut stmts = Vec::new();
    for spec in specs {
        let names: Vec<String> = (0..spec.child_count())
            .filter_map(|i| {
                (spec.field_name_for_child(i as u32) == Some("name"))
                    .then(|| spec.child(i))
                    .flatten()
            })
            .map(|name| node_text(source, name).to_string())
            .collect();
        let values = spec
            .child_by_field_name("value")
            .map(|list| parse_expression_list(source, list, diag))
            .transpose()?
            .unwrap_or_default();
        match declare_or_assign(names, values) {
            Some(mut spec_stmts) => stmts.append(&mut spec_stmts),
            None => stmts.push(record_raw_stmt(source, spec, diag)),
        }
    }
    if stmts.is_empty() {
        // A declaration whose specs did not map: keep it loud, not lost.
        stmts.push(record_raw_stmt(source, node, diag));
    }
    Ok(stmts)
}

/// Shared `:=` / spec shape: one name → declaration; several names with a
/// single multi-value call → assignment to a tuple; several values → tuple
/// assignment; several names with nothing to assign → one declaration each.
/// `None` marks a count combination valid Go cannot produce — the caller
/// keeps the whole statement as `Raw`.
fn declare_or_assign(names: Vec<String>, values: Vec<Expr>) -> Option<Vec<Stmt>> {
    match (names.len(), values.len()) {
        (1, 0) => Some(vec![Stmt::VarDecl(VarDecl {
            name: names.into_iter().next()?,
            type_hint: None,
            init: None,
        })]),
        (1, 1) => {
            let init = values.into_iter().next()?;
            Some(vec![Stmt::VarDecl(VarDecl {
                name: names.into_iter().next()?,
                type_hint: None,
                init: Some(init),
            })])
        }
        (_, 0) => Some(
            names
                .into_iter()
                .map(|name| {
                    Stmt::VarDecl(VarDecl {
                        name,
                        type_hint: None,
                        init: None,
                    })
                })
                .collect(),
        ),
        (_, 1) => {
            let value = values.into_iter().next()?;
            Some(vec![Stmt::Assign {
                target: Expr::Tuple(names.into_iter().map(Expr::Ident).collect()),
                value,
            }])
        }
        (n, m) if n == m => Some(vec![Stmt::Assign {
            target: Expr::Tuple(names.into_iter().map(Expr::Ident).collect()),
            value: Expr::Tuple(values),
        }]),
        _ => None,
    }
}

fn parse_expression_list(
    source: &str,
    node: tree_sitter::Node,
    diag: &mut RawDiagnostics,
) -> Result<Vec<Expr>> {
    let mut exprs = Vec::new();
    for i in 0..node.named_child_count() {
        let child = node.named_child(i).unwrap();
        // Comments attach inside the list between items (`a, /* c */ b`):
        // skipped, or tuple alignment silently shifts.
        if child.kind() == "comment" {
            continue;
        }
        exprs.push(parse_expr(source, child, diag)?);
    }
    Ok(exprs)
}

/// Names on the left of `:=` must all be plain identifiers; anything else
/// (invalid Go, gated by the upstream `has_error` check) → `None`.
fn identifier_names(source: &str, list: tree_sitter::Node) -> Option<Vec<String>> {
    let mut names = Vec::new();
    for i in 0..list.named_child_count() {
        let child = list.named_child(i).unwrap();
        if child.kind() != "identifier" {
            return None;
        }
        names.push(node_text(source, child).to_string());
    }
    if names.is_empty() {
        None
    } else {
        Some(names)
    }
}

fn single_or_tuple(exprs: Vec<Expr>) -> Expr {
    match exprs.len() {
        1 => exprs.into_iter().next().expect("len checked"),
        _ => Expr::Tuple(exprs),
    }
}

fn parse_expr(source: &str, node: tree_sitter::Node, diag: &mut RawDiagnostics) -> Result<Expr> {
    match node.kind() {
        "identifier" => Ok(Expr::Ident(node_text(source, node).to_string())),
        // `true`/`false`/`nil` are keyword literals with their own kinds in
        // tree-sitter-go, not identifiers.
        "true" => Ok(Expr::Literal(Literal::Bool(true))),
        "false" => Ok(Expr::Literal(Literal::Bool(false))),
        "nil" => Ok(Expr::Literal(Literal::None)),
        "int_literal" => {
            let text = node_text(source, node);
            match text.parse::<i64>() {
                Ok(n) => Ok(Expr::Literal(Literal::Int(n))),
                Err(_) => Ok(record_raw_expr(source, node, diag)),
            }
        }
        "float_literal" => Ok(Expr::Literal(Literal::Float(
            node_text(source, node).to_string(),
        ))),
        "interpreted_string_literal" | "raw_string_literal" => {
            let text = node_text(source, node);
            Ok(Expr::Literal(Literal::Str(
                text.trim_matches(['"', '`']).to_string(),
            )))
        }
        "parenthesized_expression" => {
            // A comment right after the opening paren attaches as the first
            // named child (`(/* c */ x)`): take the first non-comment child.
            let inner = (0..node.named_child_count())
                .map(|i| node.named_child(i).unwrap())
                .find(|c| c.kind() != "comment")
                .ok_or_else(|| parse_err("parenthesized_expression empty"))?;
            parse_expr(source, inner, diag)
        }
        "binary_expression" => parse_binary_expr(source, node, diag),
        "unary_expression" => {
            // `parse_un_op` only knows `!`/`-`(/`~`); operators like `^`,
            // `&`, `*` (deref) fall back to Raw — never a hard error (spec §2).
            let Some(op) = node
                .child_by_field_name("operator")
                .and_then(|op| parse_un_op(node_text(source, op)).ok())
            else {
                return Ok(record_raw_expr(source, node, diag));
            };
            let operand = node
                .child_by_field_name("operand")
                .ok_or_else(|| parse_err("unary_expression missing operand"))?;
            Ok(Expr::Unary {
                op,
                expr: Box::new(parse_expr(source, operand, diag)?),
            })
        }
        "call_expression" => {
            let function = node
                .child_by_field_name("function")
                .ok_or_else(|| parse_err("call_expression missing function"))?;
            let arguments = node
                .child_by_field_name("arguments")
                .ok_or_else(|| parse_err("call_expression missing arguments"))?;
            let mut args = Vec::new();
            for i in 0..arguments.named_child_count() {
                let child = arguments.named_child(i).unwrap();
                // Comments attach inside the argument list (`g(/* c */ x)`):
                // skipped, or the call gains a phantom Raw argument.
                if child.kind() == "comment" {
                    continue;
                }
                args.push(parse_expr(source, child, diag)?);
            }
            Ok(Expr::Call {
                callee: Box::new(parse_expr(source, function, diag)?),
                args,
            })
        }
        "index_expression" => {
            let operand = node
                .child_by_field_name("operand")
                .ok_or_else(|| parse_err("index_expression missing operand"))?;
            let index = node
                .child_by_field_name("index")
                .ok_or_else(|| parse_err("index_expression missing index"))?;
            Ok(Expr::Index {
                obj: Box::new(parse_expr(source, operand, diag)?),
                index: Box::new(parse_expr(source, index, diag)?),
            })
        }
        "selector_expression" => {
            let operand = node
                .child_by_field_name("operand")
                .ok_or_else(|| parse_err("selector_expression missing operand"))?;
            let field = node
                .child_by_field_name("field")
                .ok_or_else(|| parse_err("selector_expression missing field"))?;
            Ok(Expr::Field {
                obj: Box::new(parse_expr(source, operand, diag)?),
                name: node_text(source, field).to_string(),
            })
        }
        _ => Ok(record_raw_expr(source, node, diag)),
    }
}

fn parse_binary_expr(
    source: &str,
    node: tree_sitter::Node,
    diag: &mut RawDiagnostics,
) -> Result<Expr> {
    let (Some(lhs), Some(rhs)) = (
        node.child_by_field_name("left"),
        node.child_by_field_name("right"),
    ) else {
        return Ok(record_raw_expr(source, node, diag));
    };
    let Some(op) = node
        .child_by_field_name("operator")
        .and_then(|op| parse_c_family_bin_op(node_text(source, op)))
    else {
        return Ok(record_raw_expr(source, node, diag));
    };
    Ok(Expr::Binary {
        op,
        lhs: Box::new(parse_expr(source, lhs, diag)?),
        rhs: Box::new(parse_expr(source, rhs, diag)?),
    })
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::ir::Item;

    #[test]
    fn parses_go_function_shape() {
        let source = "package main\n\nfunc add(a int, b int) int {\n\treturn a + b\n}\n";
        let (module, diag) = GoParser::new().parse(source).unwrap();
        assert_eq!(diag.total(), 0);
        assert_eq!(module.items.len(), 1);
        let Item::Function(f) = &module.items[0] else {
            panic!("expected function");
        };
        assert_eq!(f.name, "add");
        assert_eq!(f.params.len(), 2);
        assert_eq!(f.params[0].name, "a");
    }

    #[test]
    fn parses_go_method_receiver_as_param() {
        let source = "package main\n\nfunc (n *Node) set(v int) {\n\tn.v = v\n}\n";
        let (module, _) = GoParser::new().parse(source).unwrap();
        let Item::Function(f) = &module.items[0] else {
            panic!("expected function");
        };
        assert_eq!(f.name, "set");
        // Receiver first, then regular params — the full sequence is pinned.
        let names: Vec<&str> = f.params.iter().map(|p| p.name.as_str()).collect();
        assert_eq!(names, vec!["n", "v"]);
    }

    #[test]
    fn parses_go_grouped_parameter_names() {
        let source = "package main\n\nfunc f(low, high int) int {\n\treturn low + high\n}\n";
        let (module, diag) = GoParser::new().parse(source).unwrap();
        assert_eq!(diag.total(), 0);
        let Item::Function(f) = &module.items[0] else {
            panic!("expected function");
        };
        assert_eq!(f.params.len(), 2);
        let names: Vec<&str> = f.params.iter().map(|p| p.name.as_str()).collect();
        assert_eq!(names, vec!["low", "high"]);
    }

    #[test]
    fn skips_package_and_import_silently() {
        let source = "package main\n\nimport \"fmt\"\n\nfunc f() {\n\tfmt.Println(1)\n}\n";
        let (module, diag) = GoParser::new().parse(source).unwrap();
        assert_eq!(diag.total(), 0); // package/import: silent, no Raw
        assert_eq!(module.items.len(), 1);
    }

    #[test]
    fn returns_parse_error_for_invalid_go() {
        let source = "func broken( {\n";
        let err = GoParser::new().parse(source).unwrap_err();
        assert!(matches!(err, crate::error::PseudoError::Parse { .. }));
    }

    #[test]
    fn parses_go_declarations_and_assignments() {
        let source =
            "package main\n\nfunc f() {\n\tvar x int = 3\n\ty := 4\n\tx = y\n\ta, b := 1, 2\n}\n";
        let (module, diag) = GoParser::new().parse(source).unwrap();
        let Item::Function(f) = &module.items[0] else {
            panic!()
        };
        use crate::ir::{Expr, Stmt};
        assert!(matches!(f.body.0[0], Stmt::VarDecl(_))); // var x int = 3
        assert!(matches!(f.body.0[1], Stmt::VarDecl(_))); // y := 4 (single-name :=)
        assert!(matches!(f.body.0[2], Stmt::Assign { .. })); // x = y
                                                             // a, b := 1, 2: both sides carry two entries — tuple semantics pinned.
        let Stmt::Assign {
            target: Expr::Tuple(targets),
            value: Expr::Tuple(values),
        } = &f.body.0[3]
        else {
            panic!("expected tuple assignment for a, b := 1, 2");
        };
        assert_eq!(targets.len(), 2);
        assert_eq!(values.len(), 2);
        assert_eq!(diag.total(), 0);
    }

    #[test]
    fn parses_go_literals_and_operators() {
        let source = "package main\n\nfunc f() {\n\tx := 1 + 2 * 3\n\ts := \"hi\"\n\tb := true\n\tn := nil\n}\n";
        let (module, diag) = GoParser::new().parse(source).unwrap();
        let Item::Function(f) = &module.items[0] else {
            panic!()
        };
        assert_eq!(f.body.0.len(), 4);
        assert_eq!(diag.total(), 0);
    }

    #[test]
    fn parses_go_return_break_continue() {
        let source = "package main\n\nfunc f() {\n\treturn 1\n}\n\nfunc g() {\n\treturn\n}\n\nfunc h() {\n\treturn 1, 2\n}\n";
        let (module, _) = GoParser::new().parse(source).unwrap();
        use crate::ir::{Expr, Stmt};
        let Item::Function(f) = &module.items[0] else {
            panic!()
        };
        assert!(matches!(&f.body.0[0], Stmt::Return(Some(_))));
        let Item::Function(g) = &module.items[1] else {
            panic!()
        };
        assert!(matches!(&g.body.0[0], Stmt::Return(None)));
        let Item::Function(h) = &module.items[2] else {
            panic!()
        };
        // `return 1, 2`: multi-value return wraps as a tuple.
        let Stmt::Return(Some(Expr::Tuple(values))) = &h.body.0[0] else {
            panic!("expected tuple return");
        };
        assert_eq!(values.len(), 2);
    }

    #[test]
    fn parses_go_calls_index_and_selectors() {
        let source =
            "package main\n\nfunc f(xs []int) {\n\tlen(xs)\n\tv := xs[0]\n\tw := xs[0].field\n}\n";
        let (module, diag) = GoParser::new().parse(source).unwrap();
        let Item::Function(f) = &module.items[0] else {
            panic!()
        };
        assert_eq!(f.body.0.len(), 3);
        assert_eq!(diag.total(), 0);
    }

    #[test]
    fn parses_go_grouped_var_declaration() {
        // Grouped declarations nest their specs under a `var_spec_list`
        // wrapper: each spec still expands to its own VarDecl statement.
        let source = "package main\n\nfunc f() {\n\tvar (\n\t\ta int = 1\n\t\tb int = 2\n\t)\n}\n";
        let (module, diag) = GoParser::new().parse(source).unwrap();
        assert_eq!(diag.total(), 0);
        let Item::Function(f) = &module.items[0] else {
            panic!("expected function");
        };
        use crate::ir::Stmt;
        assert_eq!(f.body.0.len(), 2);
        assert!(matches!(&f.body.0[0], Stmt::VarDecl(d) if d.name == "a"));
        assert!(matches!(&f.body.0[1], Stmt::VarDecl(d) if d.name == "b"));
    }

    #[test]
    fn parses_go_compound_assignment_as_raw() {
        // `+=` & friends keep the operator visible: one loud Raw with a
        // diagnostic, never a plain Assign that silently drops the update
        // (mirror of java.rs's compound-assignment parity test).
        let source = "package main\n\nfunc f() {\n\tx := 1\n\tx += 2\n}\n";
        let (module, diag) = GoParser::new().parse(source).unwrap();
        let Item::Function(f) = &module.items[0] else {
            panic!("expected function");
        };
        use crate::ir::Stmt;
        assert_eq!(f.body.0[1], Stmt::Raw("x += 2".to_string()));
        assert_eq!(diag.total(), 1);
    }

    #[test]
    fn parses_go_body_with_leading_comment() {
        // Comments are named extras: a leading one attaches as a direct child
        // of `block` (ahead of `statement_list`), mid-body ones sit inside the
        // list — neither may disturb or replace the statements around them.
        let source = "package main\n\nfunc f() {\n\t// leading\n\tx := 1\n\t// mid\n\ty := x\n}\n";
        let (module, diag) = GoParser::new().parse(source).unwrap();
        assert_eq!(diag.total(), 0);
        let Item::Function(f) = &module.items[0] else {
            panic!("expected function");
        };
        assert_eq!(f.body.0.len(), 2);
        use crate::ir::Stmt;
        assert!(matches!(f.body.0[0], Stmt::VarDecl(_))); // x := 1
        assert!(matches!(f.body.0[1], Stmt::VarDecl(_))); // y := x
    }

    #[test]
    fn bodyless_function_falls_back_to_raw_item() {
        // Bodyless declarations are legal Go (external/assembly impls): one
        // loud Raw item, never a hard error, and the rest of the file still
        // parses (carried from the Task 2 review).
        let source = "package main\n\nfunc f()\n\nfunc g() int {\n\treturn 1\n}\n";
        let (module, diag) = GoParser::new().parse(source).unwrap();
        assert_eq!(module.items.len(), 2);
        let Item::Raw(text) = &module.items[0] else {
            panic!("expected raw item for the bodyless function");
        };
        assert!(text.contains("func f()"));
        let Item::Function(g) = &module.items[1] else {
            panic!("expected the concrete function to still parse");
        };
        assert_eq!(g.name, "g");
        assert_eq!(diag.items, 1);
    }

    #[test]
    fn parses_go_if_else_chain() {
        let source = "package main\n\nfunc f(x int) int {\n\tif x == 1 {\n\t\treturn 1\n\t} else if x == 2 {\n\t\treturn 2\n\t} else {\n\t\treturn 3\n\t}\n}\n";
        let (module, diag) = GoParser::new().parse(source).unwrap();
        let Item::Function(f) = &module.items[0] else {
            panic!()
        };
        use crate::ir::Stmt;
        let Stmt::If {
            then_block,
            else_block: Some(else_block),
            ..
        } = &f.body.0[0]
        else {
            panic!("expected if with else");
        };
        assert_eq!(then_block.0.len(), 1);
        // else-if must nest an If inside the else block (matches python/java shape)
        assert!(matches!(else_block.0[0], Stmt::If { .. }));
        assert_eq!(diag.total(), 0);
    }

    #[test]
    fn parses_go_cstyle_and_cond_fors() {
        let source = "package main\n\nfunc f(n int) {\n\tfor i := 0; i < n; i++ {\n\t\tg(i)\n\t}\n\tfor n > 0 {\n\t\tn = n - 1\n\t}\n}\n";
        let (module, diag) = GoParser::new().parse(source).unwrap();
        let Item::Function(f) = &module.items[0] else {
            panic!()
        };
        use crate::ir::{ForKind, Stmt};
        let Stmt::For {
            kind: ForKind::CStyle { .. },
            ..
        } = &f.body.0[0]
        else {
            panic!("expected c-style for");
        };
        let Stmt::While { cond, .. } = &f.body.0[1] else {
            panic!("expected while")
        };
        // pin the condition actually parsed (guards against silently dropping it)
        assert!(matches!(cond, crate::ir::Expr::Binary { .. }));
        // the i++ update clause records one Raw expression (cpp/java parity)
        assert_eq!(diag.expressions, 1);
        assert_eq!(diag.statements, 0);
    }

    #[test]
    fn parses_go_range_and_infinite_for() {
        let source = "package main\n\nfunc f(xs []int) {\n\tfor v := range xs {\n\t\tg(v)\n\t}\n\tfor {\n\t\tbreak\n\t}\n}\n";
        let (module, diag) = GoParser::new().parse(source).unwrap();
        let Item::Function(f) = &module.items[0] else {
            panic!()
        };
        use crate::ir::{ForKind, Literal, Stmt};
        let Stmt::For {
            kind: ForKind::ForEach { var, .. },
            ..
        } = &f.body.0[0]
        else {
            panic!("expected foreach");
        };
        assert_eq!(var, "v");
        let Stmt::While { cond, .. } = &f.body.0[1] else {
            panic!("expected while")
        };
        assert_eq!(cond, &crate::ir::Expr::Literal(Literal::Bool(true)));
        assert_eq!(diag.total(), 0);
    }

    #[test]
    fn two_var_range_falls_back_to_raw() {
        let source = "package main\n\nfunc f(xs []int) {\n\tfor i, v := range xs {\n\t\tg(i)\n\t\tg(v)\n\t}\n}\n";
        let (_, diag) = GoParser::new().parse(source).unwrap();
        assert_eq!(diag.statements, 1);
        assert_eq!(diag.sorted_unique_lines(), vec![4]);
    }

    #[test]
    fn parses_go_return_with_leading_comment() {
        // `return /* c */ 1`: the comment lands as the FIRST named child of
        // return_statement — the value list must be found by kind, not by
        // position, or the return silently degrades to Return(None)
        // (carried from the Task 3 review).
        let source = "package main\n\nfunc f() int {\n\treturn /* c */ 1\n}\n";
        let (module, diag) = GoParser::new().parse(source).unwrap();
        let Item::Function(f) = &module.items[0] else {
            panic!("expected function");
        };
        use crate::ir::{Expr, Literal, Stmt};
        assert_eq!(
            f.body.0[0],
            Stmt::Return(Some(Expr::Literal(Literal::Int(1))))
        );
        assert_eq!(diag.total(), 0);
    }

    #[test]
    fn top_level_comments_skip_silently() {
        // A `// comment` at file scope (header/license comments are ubiquitous
        // in real Go) attaches as a named extra child of the root: it must
        // skip silently like package/import, not surface as an unparsed Raw
        // item (carried from the Task 5 review).
        let source =
            "// Algotorial sketch.\npackage main\n\nfunc f() {\n\tg()\n}\n// trailing note\n";
        let (module, diag) = GoParser::new().parse(source).unwrap();
        assert_eq!(module.items.len(), 1);
        assert_eq!(diag.total(), 0);
    }

    #[test]
    fn comments_inside_expressions_do_not_disturb_parsing() {
        // Comments are named extras that attach INSIDE expression containers
        // (parenthesized_expression, argument_list, expression_list between
        // items): skipping them keeps value alignment intact (Task 4
        // positional-access audit — empirically dumped CST shapes).
        let source =
            "package main\n\nfunc f(n int) {\n\tx := (/* c */ n)\n\tg(/* d */ n)\n\treturn /* a */ n, /* b */ n\n}\n";
        let (module, diag) = GoParser::new().parse(source).unwrap();
        let Item::Function(f) = &module.items[0] else {
            panic!("expected function");
        };
        use crate::ir::{Expr, Stmt};
        assert!(matches!(&f.body.0[0], Stmt::VarDecl(d) if matches!(d.init, Some(Expr::Ident(_)))));
        let Stmt::ExprStmt(Expr::Call { args, .. }) = &f.body.0[1] else {
            panic!("expected call");
        };
        assert_eq!(args.len(), 1);
        let Stmt::Return(Some(Expr::Tuple(values))) = &f.body.0[2] else {
            panic!("expected tuple return");
        };
        assert_eq!(values.len(), 2);
        assert_eq!(diag.total(), 0);
    }
}
