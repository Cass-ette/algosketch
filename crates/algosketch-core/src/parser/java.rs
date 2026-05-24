use crate::error::{PseudoError, Result};
use crate::ir::*;
use crate::parser::common::{
    find_anon_operator, named_children_of_kind, node_text, parse_c_family_bin_op, parse_err,
    parse_un_op,
};
use crate::parser::LanguageParser;
use crate::SourceLang;

pub struct JavaParser;

impl JavaParser {
    pub fn new() -> Self {
        Self
    }
}

impl Default for JavaParser {
    fn default() -> Self {
        Self::new()
    }
}

impl LanguageParser for JavaParser {
    fn language(&self) -> SourceLang {
        SourceLang::Java
    }

    fn parse(&self, source: &str) -> Result<Module> {
        let mut parser = tree_sitter::Parser::new();
        parser
            .set_language(&tree_sitter_java::LANGUAGE.into())
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
        collect_methods(source, root, &mut items)?;
        Ok(Module {
            source_language: SourceLang::Java,
            items,
        })
    }
}

fn collect_methods(source: &str, node: tree_sitter::Node, items: &mut Vec<Item>) -> Result<()> {
    if node.kind() == "method_declaration" {
        if node.child_by_field_name("body").is_some() {
            items.push(parse_method(source, node)?);
        }
        return Ok(());
    }

    for i in 0..node.named_child_count() {
        collect_methods(source, node.named_child(i).unwrap(), items)?;
    }
    Ok(())
}

fn parse_method(source: &str, node: tree_sitter::Node) -> Result<Item> {
    let name_node = node
        .child_by_field_name("name")
        .ok_or_else(|| parse_err("method missing name"))?;
    let params_node = node
        .child_by_field_name("parameters")
        .ok_or_else(|| parse_err("method missing parameters"))?;
    let body_node = node
        .child_by_field_name("body")
        .ok_or_else(|| parse_err("method missing body"))?;

    let mut params = Vec::new();
    for i in 0..params_node.named_child_count() {
        let child = params_node.named_child(i).unwrap();
        if child.kind() == "formal_parameter" {
            if let Some(name) = child.child_by_field_name("name") {
                params.push(Param {
                    name: node_text(source, name).to_string(),
                    type_hint: child
                        .child_by_field_name("type")
                        .map(|ty| TypeHint(node_text(source, ty).to_string())),
                });
            }
        }
    }

    Ok(Item::Function(Function {
        name: node_text(source, name_node).to_string(),
        params,
        return_type: node
            .child_by_field_name("type")
            .map(|ty| TypeHint(node_text(source, ty).to_string())),
        body: parse_block(source, body_node)?,
        span: Span::default(),
    }))
}

fn parse_block(source: &str, node: tree_sitter::Node) -> Result<Block> {
    if node.kind() != "block" {
        return Ok(Block(vec![parse_stmt(source, node)?]));
    }

    let mut stmts = Vec::new();
    for i in 0..node.named_child_count() {
        stmts.push(parse_stmt(source, node.named_child(i).unwrap())?);
    }
    Ok(Block(stmts))
}

fn parse_stmt(source: &str, node: tree_sitter::Node) -> Result<Stmt> {
    match node.kind() {
        "local_variable_declaration" => parse_var_decl(source, node),
        "expression_statement" => {
            let inner = node
                .named_child(0)
                .ok_or_else(|| parse_err("expression_statement empty"))?;
            if inner.kind() == "assignment_expression" {
                parse_assignment_stmt(source, inner)
            } else {
                Ok(Stmt::ExprStmt(parse_expr(source, inner)?))
            }
        }
        "while_statement" => {
            let cond = node
                .child_by_field_name("condition")
                .ok_or_else(|| parse_err("while missing condition"))?;
            let body = node
                .child_by_field_name("body")
                .ok_or_else(|| parse_err("while missing body"))?;
            Ok(Stmt::While {
                cond: parse_expr(source, cond)?,
                body: parse_block(source, body)?,
            })
        }
        "if_statement" => parse_if_stmt(source, node),
        "return_statement" => {
            let expr = node
                .named_child(0)
                .map(|child| parse_expr(source, child))
                .transpose()?;
            Ok(Stmt::Return(expr))
        }
        "break_statement" => Ok(Stmt::Break),
        "continue_statement" => Ok(Stmt::Continue),
        "for_statement" => parse_for_stmt(source, node),
        "enhanced_for_statement" => parse_enhanced_for_stmt(source, node),
        _ => Ok(Stmt::Raw(node_text(source, node).to_string())),
    }
}

fn parse_var_decl(source: &str, node: tree_sitter::Node) -> Result<Stmt> {
    let declarators = named_children_of_kind(node, "variable_declarator");
    if declarators.len() != 1 {
        return Ok(Stmt::Raw(node_text(source, node).to_string()));
    }

    let declarator = declarators[0];
    let name = declarator
        .child_by_field_name("name")
        .ok_or_else(|| parse_err("variable_declarator missing name"))?;
    Ok(Stmt::VarDecl(VarDecl {
        name: node_text(source, name).to_string(),
        type_hint: node
            .child_by_field_name("type")
            .map(|ty| TypeHint(node_text(source, ty).to_string())),
        init: declarator
            .child_by_field_name("value")
            .map(|value| parse_expr(source, value))
            .transpose()?,
    }))
}

fn parse_assignment_stmt(source: &str, node: tree_sitter::Node) -> Result<Stmt> {
    let op_node = node
        .child_by_field_name("operator")
        .ok_or_else(|| parse_err("assignment missing operator"))?;
    if node_text(source, op_node) != "=" {
        return Ok(Stmt::Raw(node_text(source, node).to_string()));
    }
    let target = node
        .child_by_field_name("left")
        .ok_or_else(|| parse_err("assignment missing target"))?;
    let value = node
        .child_by_field_name("right")
        .ok_or_else(|| parse_err("assignment missing value"))?;
    Ok(Stmt::Assign {
        target: parse_expr(source, target)?,
        value: parse_expr(source, value)?,
    })
}

fn parse_if_stmt(source: &str, node: tree_sitter::Node) -> Result<Stmt> {
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
                parse_if_stmt(source, alternative).map(|stmt| Block(vec![stmt]))
            } else {
                parse_block(source, alternative)
            }
        })
        .transpose()?;

    Ok(Stmt::If {
        cond: parse_expr(source, cond)?,
        then_block: parse_block(source, consequence)?,
        else_block,
    })
}

fn parse_for_stmt(source: &str, node: tree_sitter::Node) -> Result<Stmt> {
    let Some(init) = node.child_by_field_name("init") else {
        return Ok(Stmt::Raw(node_text(source, node).to_string()));
    };
    let Some(cond) = node.child_by_field_name("condition") else {
        return Ok(Stmt::Raw(node_text(source, node).to_string()));
    };
    let Some(update) = node.child_by_field_name("update") else {
        return Ok(Stmt::Raw(node_text(source, node).to_string()));
    };
    let body = node
        .child_by_field_name("body")
        .ok_or_else(|| parse_err("for missing body"))?;

    let init = if init.kind() == "local_variable_declaration" {
        parse_var_decl(source, init)?
    } else if init.kind() == "assignment_expression" {
        parse_assignment_stmt(source, init)?
    } else {
        Stmt::ExprStmt(parse_expr(source, init)?)
    };

    Ok(Stmt::For {
        kind: ForKind::CStyle {
            init: Box::new(init),
            cond: parse_expr(source, cond)?,
            step: parse_expr(source, update)?,
        },
        body: parse_block(source, body)?,
    })
}

fn parse_enhanced_for_stmt(source: &str, node: tree_sitter::Node) -> Result<Stmt> {
    let name = node
        .child_by_field_name("name")
        .ok_or_else(|| parse_err("enhanced for missing name"))?;
    let value = node
        .child_by_field_name("value")
        .ok_or_else(|| parse_err("enhanced for missing value"))?;
    let body = node
        .child_by_field_name("body")
        .ok_or_else(|| parse_err("enhanced for missing body"))?;

    Ok(Stmt::For {
        kind: ForKind::ForEach {
            var: node_text(source, name).to_string(),
            iter: parse_expr(source, value)?,
        },
        body: parse_block(source, body)?,
    })
}

fn parse_expr(source: &str, node: tree_sitter::Node) -> Result<Expr> {
    match node.kind() {
        "identifier" => Ok(Expr::Ident(node_text(source, node).to_string())),
        "decimal_integer_literal" => {
            let text = node_text(source, node);
            match text.parse::<i64>() {
                Ok(n) => Ok(Expr::Literal(Literal::Int(n))),
                Err(_) => Ok(Expr::Raw(text.to_string())),
            }
        }
        "string_literal" => {
            let text = node_text(source, node);
            Ok(Expr::Literal(Literal::Str(
                text.trim_matches(['\"', '\'']).to_string(),
            )))
        }
        "true" => Ok(Expr::Literal(Literal::Bool(true))),
        "false" => Ok(Expr::Literal(Literal::Bool(false))),
        "null_literal" => Ok(Expr::Literal(Literal::None)),
        "binary_expression" => parse_binary_expr(source, node),
        "unary_expression" => {
            let op_text = find_anon_operator(source, node)
                .ok_or_else(|| parse_err("unary_expression missing operator"))?;
            let op = parse_un_op(op_text)?;
            let operand = node
                .child_by_field_name("operand")
                .ok_or_else(|| parse_err("unary_expression missing operand"))?;
            Ok(Expr::Unary {
                op,
                expr: Box::new(parse_expr(source, operand)?),
            })
        }
        "parenthesized_expression" => {
            let inner = node
                .named_child(0)
                .ok_or_else(|| parse_err("parenthesized_expression empty"))?;
            parse_expr(source, inner)
        }
        "method_invocation" => parse_method_invocation(source, node),
        "array_access" => {
            let array = node
                .child_by_field_name("array")
                .ok_or_else(|| parse_err("array_access missing array"))?;
            let index = node
                .child_by_field_name("index")
                .ok_or_else(|| parse_err("array_access missing index"))?;
            Ok(Expr::Index {
                obj: Box::new(parse_expr(source, array)?),
                index: Box::new(parse_expr(source, index)?),
            })
        }
        "field_access" => {
            let object = node
                .child_by_field_name("object")
                .ok_or_else(|| parse_err("field_access missing object"))?;
            let field = node
                .child_by_field_name("field")
                .ok_or_else(|| parse_err("field_access missing field"))?;
            Ok(Expr::Field {
                obj: Box::new(parse_expr(source, object)?),
                name: node_text(source, field).to_string(),
            })
        }
        "assignment_expression" | "update_expression" => {
            Ok(Expr::Raw(node_text(source, node).to_string()))
        }
        _ => Ok(Expr::Raw(node_text(source, node).to_string())),
    }
}

fn parse_binary_expr(source: &str, node: tree_sitter::Node) -> Result<Expr> {
    let lhs = node
        .child_by_field_name("left")
        .ok_or_else(|| parse_err("binary_expression missing lhs"))?;
    let rhs = node
        .child_by_field_name("right")
        .ok_or_else(|| parse_err("binary_expression missing rhs"))?;
    let op_text = find_anon_operator(source, node)
        .ok_or_else(|| parse_err("binary_expression missing operator"))?;
    let Some(op) = parse_c_family_bin_op(op_text) else {
        return Ok(Expr::Raw(node_text(source, node).to_string()));
    };
    Ok(Expr::Binary {
        op,
        lhs: Box::new(parse_expr(source, lhs)?),
        rhs: Box::new(parse_expr(source, rhs)?),
    })
}

fn parse_method_invocation(source: &str, node: tree_sitter::Node) -> Result<Expr> {
    let name = node
        .child_by_field_name("name")
        .ok_or_else(|| parse_err("method_invocation missing name"))?;
    let callee = if let Some(object) = node.child_by_field_name("object") {
        Expr::Field {
            obj: Box::new(parse_expr(source, object)?),
            name: node_text(source, name).to_string(),
        }
    } else {
        Expr::Ident(node_text(source, name).to_string())
    };

    let args_node = node
        .child_by_field_name("arguments")
        .ok_or_else(|| parse_err("method_invocation missing arguments"))?;
    let mut args = Vec::new();
    for i in 0..args_node.named_child_count() {
        args.push(parse_expr(source, args_node.named_child(i).unwrap())?);
    }

    Ok(Expr::Call {
        callee: Box::new(callee),
        args,
    })
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::ir::{ForKind, Item, Stmt};

    #[test]
    fn parses_java_method_shape() {
        let source = r#"
class Solution {
    int answer(int x) {
        return x;
    }
}
"#;

        let module = JavaParser::new().parse(source).unwrap();
        assert_eq!(module.source_language, SourceLang::Java);
        let Item::Function(function) = &module.items[0] else {
            panic!("expected function");
        };

        assert_eq!(function.name, "answer");
        assert_eq!(
            function
                .params
                .iter()
                .map(|p| p.name.as_str())
                .collect::<Vec<_>>(),
            vec!["x"]
        );
        assert!(matches!(function.body.0[0], Stmt::Return(_)));
    }

    #[test]
    fn parses_java_binary_search_control_flow() {
        let source = r#"
class Solution {
    int binary_search(int[] nums, int target) {
        int left = 0;
        int right = nums.length - 1;
        while (left <= right) {
            int mid = (left + right) / 2;
            if (nums[mid] == target) {
                return mid;
            } else if (nums[mid] < target) {
                left = mid + 1;
            } else {
                right = mid - 1;
            }
        }
        return -1;
    }
}
"#;

        let module = JavaParser::new().parse(source).unwrap();
        let Item::Function(function) = &module.items[0] else {
            panic!("expected function");
        };

        assert!(matches!(function.body.0[0], Stmt::VarDecl(_)));
        assert!(matches!(function.body.0[2], Stmt::While { .. }));
    }

    #[test]
    fn parses_java_cstyle_for_loop() {
        let source = r#"
class Solution {
    int first_even(int[] nums) {
        for (int i = 0; i < nums.length; i++) {
            if (nums[i] % 2 == 0) {
                return nums[i];
            }
        }
        return -1;
    }
}
"#;

        let module = JavaParser::new().parse(source).unwrap();
        let Item::Function(function) = &module.items[0] else {
            panic!("expected function");
        };
        let Stmt::For { kind, .. } = &function.body.0[0] else {
            panic!("expected for loop");
        };
        assert!(matches!(kind, ForKind::CStyle { .. }));
    }

    #[test]
    fn parses_java_enhanced_for_loop() {
        let source = r#"
class Solution {
    int total(int[] nums) {
        int total = 0;
        for (int value : nums) {
            total = total + value;
        }
        return total;
    }
}
"#;

        let module = JavaParser::new().parse(source).unwrap();
        let Item::Function(function) = &module.items[0] else {
            panic!("expected function");
        };
        let Stmt::For { kind, .. } = &function.body.0[1] else {
            panic!("expected for loop");
        };
        assert!(matches!(kind, ForKind::ForEach { .. }));
    }

    #[test]
    fn parses_java_skips_bodyless_method_and_parses_valid_one() {
        let source = r#"
abstract class Base {
    abstract int compute(int x);
    int answer(int x) {
        return x;
    }
}
"#;

        let module = JavaParser::new().parse(source).unwrap();
        assert_eq!(
            module.items.len(),
            1,
            "should only parse the concrete method"
        );
        let Item::Function(function) = &module.items[0] else {
            panic!("expected function");
        };
        assert_eq!(function.name, "answer");
    }

    #[test]
    fn parses_java_unknown_binary_as_raw() {
        // >>> is a binary_expression in tree-sitter Java but not in our parse_bin_op table.
        let source = r#"
class Solution {
    int check(int x) {
        return x >>> 1;
    }
}
"#;

        let module = JavaParser::new().parse(source).unwrap();
        let Item::Function(function) = &module.items[0] else {
            panic!("expected function");
        };
        let Stmt::Return(Some(expr)) = &function.body.0[0] else {
            panic!("expected return");
        };
        assert!(matches!(expr, Expr::Raw(_)));
    }

    #[test]
    fn parses_java_compound_assignment_as_raw() {
        let source = r#"
class Solution {
    int total(int[] nums) {
        int total = 0;
        for (int value : nums) {
            total += value;
        }
        return total;
    }
}
"#;

        let module = JavaParser::new().parse(source).unwrap();
        let Item::Function(function) = &module.items[0] else {
            panic!("expected function");
        };
        let Stmt::For { body, .. } = &function.body.0[1] else {
            panic!("expected for loop");
        };
        assert_eq!(body.0[0], Stmt::Raw("total += value".to_string()));
    }
}
