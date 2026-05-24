use crate::error::{PseudoError, Result};
use crate::ir::*;
use crate::parser::common::{
    find_anon_operator, named_child_by_kind, named_children_of_kind, node_text,
    parse_c_family_bin_op, parse_err, parse_un_op,
};
use crate::parser::LanguageParser;
use crate::SourceLang;

pub struct CppParser;

impl CppParser {
    pub fn new() -> Self {
        Self
    }
}

impl Default for CppParser {
    fn default() -> Self {
        Self::new()
    }
}

impl LanguageParser for CppParser {
    fn language(&self) -> SourceLang {
        SourceLang::Cpp
    }

    fn parse(&self, source: &str) -> Result<Module> {
        let mut parser = tree_sitter::Parser::new();
        parser
            .set_language(&tree_sitter_cpp::LANGUAGE.into())
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
        collect_functions(source, root, &mut items)?;
        Ok(Module {
            source_language: SourceLang::Cpp,
            items,
        })
    }
}

fn collect_functions(source: &str, node: tree_sitter::Node, items: &mut Vec<Item>) -> Result<()> {
    if node.kind() == "function_definition" {
        if node.child_by_field_name("body").is_some() {
            items.push(parse_function(source, node)?);
        }
        return Ok(());
    }

    for i in 0..node.named_child_count() {
        collect_functions(source, node.named_child(i).unwrap(), items)?;
    }
    Ok(())
}

fn parse_function(source: &str, node: tree_sitter::Node) -> Result<Item> {
    let declarator = node
        .child_by_field_name("declarator")
        .ok_or_else(|| parse_err("function_definition missing declarator"))?;
    let function_declarator = if declarator.kind() == "function_declarator" {
        declarator
    } else {
        find_descendant_by_kind(declarator, "function_declarator")
            .ok_or_else(|| parse_err("function_definition missing function_declarator"))?
    };
    let name_node = function_name_node(function_declarator)
        .ok_or_else(|| parse_err("function_declarator missing name"))?;
    let params_node = function_declarator
        .child_by_field_name("parameters")
        .ok_or_else(|| parse_err("function_declarator missing parameters"))?;
    let body_node = node
        .child_by_field_name("body")
        .ok_or_else(|| parse_err("function_definition missing body"))?;

    let mut params = Vec::new();
    for i in 0..params_node.named_child_count() {
        let child = params_node.named_child(i).unwrap();
        if child.kind() == "parameter_declaration" {
            if let Some(name) = child
                .child_by_field_name("declarator")
                .and_then(declarator_name_node)
            {
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
    if node.kind() != "compound_statement" {
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
        "declaration" => parse_var_decl(source, node),
        "expression_statement" => {
            let Some(inner) = node.named_child(0) else {
                return Ok(Stmt::Raw(node_text(source, node).to_string()));
            };
            if inner.kind() == "assignment_expression" {
                parse_assignment_stmt(source, node, inner)
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
                cond: parse_condition(source, cond)?,
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
        "for_range_loop" => parse_range_for_stmt(source, node),
        _ => Ok(Stmt::Raw(node_text(source, node).to_string())),
    }
}

fn parse_var_decl(source: &str, node: tree_sitter::Node) -> Result<Stmt> {
    let init_declarators = named_children_of_kind(node, "init_declarator");
    if init_declarators.len() > 1 {
        return Ok(Stmt::Raw(node_text(source, node).to_string()));
    }

    let (declarator, init) = if let Some(init_declarator) = init_declarators.first() {
        let declarator = init_declarator
            .child_by_field_name("declarator")
            .ok_or_else(|| parse_err("init_declarator missing declarator"))?;
        let init = init_declarator
            .child_by_field_name("value")
            .map(|value| parse_expr(source, value))
            .transpose()?;
        (declarator, init)
    } else {
        let declarators = declaration_declarators(node);
        if declarators.len() != 1 {
            return Ok(Stmt::Raw(node_text(source, node).to_string()));
        }
        (declarators[0], None)
    };

    if declarator.kind() != "identifier" {
        return Ok(Stmt::Raw(node_text(source, node).to_string()));
    }
    Ok(Stmt::VarDecl(VarDecl {
        name: node_text(source, declarator).to_string(),
        type_hint: node
            .child_by_field_name("type")
            .map(|ty| TypeHint(node_text(source, ty).to_string())),
        init,
    }))
}

fn parse_assignment_stmt(
    source: &str,
    statement: tree_sitter::Node,
    node: tree_sitter::Node,
) -> Result<Stmt> {
    let op_node = node
        .child_by_field_name("operator")
        .ok_or_else(|| parse_err("assignment missing operator"))?;
    if node_text(source, op_node) != "=" {
        return Ok(Stmt::Raw(node_text(source, statement).to_string()));
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
        .or_else(|| first_named_child_after_condition(node))
        .ok_or_else(|| parse_err("if missing consequence"))?;
    let else_block = node
        .child_by_field_name("alternative")
        .map(|alternative| parse_else_alternative(source, alternative))
        .transpose()?;

    Ok(Stmt::If {
        cond: parse_condition(source, cond)?,
        then_block: parse_block(source, consequence)?,
        else_block,
    })
}

fn parse_else_alternative(source: &str, node: tree_sitter::Node) -> Result<Block> {
    if node.kind() == "if_statement" {
        return parse_if_stmt(source, node).map(|stmt| Block(vec![stmt]));
    }
    if node.kind() == "else_clause" {
        if let Some(child) = node.named_child(0) {
            if child.kind() == "if_statement" {
                return parse_if_stmt(source, child).map(|stmt| Block(vec![stmt]));
            }
            return parse_block(source, child);
        }
    }
    parse_block(source, node)
}

fn parse_for_stmt(source: &str, node: tree_sitter::Node) -> Result<Stmt> {
    let Some(init) = node.child_by_field_name("initializer") else {
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

    let init = if init.kind() == "declaration" {
        parse_var_decl(source, init)?
    } else if init.kind() == "assignment_expression" {
        parse_assignment_stmt(source, init, init)?
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

fn parse_range_for_stmt(source: &str, node: tree_sitter::Node) -> Result<Stmt> {
    let name = node
        .child_by_field_name("declarator")
        .and_then(declarator_name_node)
        .ok_or_else(|| parse_err("range for missing declarator"))?;
    let iter = node
        .child_by_field_name("right")
        .ok_or_else(|| parse_err("range for missing iterator"))?;
    let body = node
        .child_by_field_name("body")
        .ok_or_else(|| parse_err("range for missing body"))?;

    Ok(Stmt::For {
        kind: ForKind::ForEach {
            var: node_text(source, name).to_string(),
            iter: parse_expr(source, iter)?,
        },
        body: parse_block(source, body)?,
    })
}

fn parse_condition(source: &str, node: tree_sitter::Node) -> Result<Expr> {
    if node.kind() == "condition_clause" {
        let inner = node
            .named_child(0)
            .ok_or_else(|| parse_err("condition_clause empty"))?;
        parse_expr(source, inner)
    } else {
        parse_expr(source, node)
    }
}

fn parse_expr(source: &str, node: tree_sitter::Node) -> Result<Expr> {
    match node.kind() {
        "identifier" => Ok(Expr::Ident(node_text(source, node).to_string())),
        "number_literal" => {
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
        "nullptr" | "nullptr_literal" => Ok(Expr::Literal(Literal::None)),
        "binary_expression" => parse_binary_expr(source, node),
        "unary_expression" => {
            let op_text = find_anon_operator(source, node)
                .ok_or_else(|| parse_err("unary_expression missing operator"))?;
            let op = parse_un_op(op_text)?;
            let operand = node
                .child_by_field_name("argument")
                .or_else(|| node.child_by_field_name("operand"))
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
        "call_expression" => parse_call_expr(source, node),
        "subscript_expression" => {
            let obj = node
                .child_by_field_name("argument")
                .ok_or_else(|| parse_err("subscript_expression missing argument"))?;
            let args = node
                .child_by_field_name("indices")
                .or_else(|| named_child_by_kind(node, "subscript_argument_list"))
                .ok_or_else(|| parse_err("subscript_expression missing indices"))?;
            let index = args
                .named_child(0)
                .ok_or_else(|| parse_err("subscript_argument_list empty"))?;
            Ok(Expr::Index {
                obj: Box::new(parse_expr(source, obj)?),
                index: Box::new(parse_expr(source, index)?),
            })
        }
        "field_expression" => {
            let object = node
                .child_by_field_name("argument")
                .ok_or_else(|| parse_err("field_expression missing argument"))?;
            let field = node
                .child_by_field_name("field")
                .ok_or_else(|| parse_err("field_expression missing field"))?;
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

fn parse_call_expr(source: &str, node: tree_sitter::Node) -> Result<Expr> {
    let callee_node = node
        .child_by_field_name("function")
        .ok_or_else(|| parse_err("call_expression missing function"))?;
    let args_node = node
        .child_by_field_name("arguments")
        .ok_or_else(|| parse_err("call_expression missing arguments"))?;
    let mut args = Vec::new();
    for i in 0..args_node.named_child_count() {
        args.push(parse_expr(source, args_node.named_child(i).unwrap())?);
    }

    Ok(Expr::Call {
        callee: Box::new(parse_expr(source, callee_node)?),
        args,
    })
}

fn function_name_node(node: tree_sitter::Node) -> Option<tree_sitter::Node> {
    let declarator = node.child_by_field_name("declarator")?;
    declarator_name_node(declarator)
}

fn declarator_name_node(node: tree_sitter::Node) -> Option<tree_sitter::Node> {
    match node.kind() {
        "identifier" | "field_identifier" | "qualified_identifier" => Some(node),
        _ => {
            for field in ["declarator", "name", "field"] {
                if let Some(child) = node.child_by_field_name(field) {
                    if let Some(name) = declarator_name_node(child) {
                        return Some(name);
                    }
                }
            }
            for i in 0..node.named_child_count() {
                if let Some(name) = declarator_name_node(node.named_child(i).unwrap()) {
                    return Some(name);
                }
            }
            None
        }
    }
}

fn declaration_declarators(node: tree_sitter::Node) -> Vec<tree_sitter::Node> {
    let mut declarators = Vec::new();
    for i in 0..node.named_child_count() {
        let child = node.named_child(i).unwrap();
        if child.kind() != "primitive_type"
            && child.kind() != "template_type"
            && child.kind() != "type_identifier"
        {
            declarators.push(child);
        }
    }
    declarators
}

fn first_named_child_after_condition(node: tree_sitter::Node) -> Option<tree_sitter::Node> {
    let mut seen_condition = false;
    for i in 0..node.named_child_count() {
        let child = node.named_child(i).unwrap();
        if child.kind() == "condition_clause" {
            seen_condition = true;
            continue;
        }
        if seen_condition {
            return Some(child);
        }
    }
    None
}

fn find_descendant_by_kind<'a>(
    node: tree_sitter::Node<'a>,
    kind: &str,
) -> Option<tree_sitter::Node<'a>> {
    if node.kind() == kind {
        return Some(node);
    }
    for i in 0..node.named_child_count() {
        if let Some(found) = find_descendant_by_kind(node.named_child(i).unwrap(), kind) {
            return Some(found);
        }
    }
    None
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::ir::{ForKind, Item, Stmt};

    #[test]
    fn parses_cpp_function_shape() {
        let source = r#"
int answer(int x) {
    return x;
}
"#;

        let module = CppParser::new().parse(source).unwrap();
        assert_eq!(module.source_language, SourceLang::Cpp);
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
    fn parses_cpp_uninitialized_declaration_as_vardecl() {
        let source = r#"
int answer(int x) {
    int result;
    return x;
}
"#;

        let module = CppParser::new().parse(source).unwrap();
        let Item::Function(function) = &module.items[0] else {
            panic!("expected function");
        };
        let Stmt::VarDecl(var_decl) = &function.body.0[0] else {
            panic!("expected variable declaration");
        };

        assert_eq!(var_decl.name, "result");
        assert!(var_decl.init.is_none());
    }

    #[test]
    fn parses_cpp_pointer_declaration_as_raw() {
        let source = r#"
int answer(int x) {
    int *p;
    return x;
}
"#;

        let module = CppParser::new().parse(source).unwrap();
        let Item::Function(function) = &module.items[0] else {
            panic!("expected function");
        };

        assert!(matches!(function.body.0[0], Stmt::Raw(ref text) if text == "int *p;"));
    }

    #[test]
    fn parses_cpp_initialized_reference_declaration_as_raw() {
        let source = r#"
int answer(int x) {
    int &r = x;
    return x;
}
"#;

        let module = CppParser::new().parse(source).unwrap();
        let Item::Function(function) = &module.items[0] else {
            panic!("expected function");
        };

        assert!(matches!(function.body.0[0], Stmt::Raw(ref text) if text == "int &r = x;"));
    }

    #[test]
    fn parses_cpp_multi_uninitialized_declaration_as_raw() {
        let source = r#"
int answer(int x) {
    int a, b;
    return x;
}
"#;

        let module = CppParser::new().parse(source).unwrap();
        let Item::Function(function) = &module.items[0] else {
            panic!("expected function");
        };

        assert!(matches!(function.body.0[0], Stmt::Raw(ref text) if text == "int a, b;"));
    }

    #[test]
    fn parses_cpp_empty_statement_without_parse_error() {
        let source = r#"
int answer(int x) {
    ;
    return x;
}
"#;

        let module = CppParser::new().parse(source).unwrap();
        let Item::Function(function) = &module.items[0] else {
            panic!("expected function");
        };

        assert!(matches!(function.body.0[0], Stmt::Raw(ref text) if text == ";"));
        assert!(matches!(function.body.0[1], Stmt::Return(_)));
    }

    #[test]
    fn parses_cpp_else_if_as_nested_if_in_else_block() {
        let source = r#"
int f(int x) {
    if (x == 1) { return 1; }
    else if (x == 2) { return 2; }
    else { return 3; }
}
"#;

        let module = CppParser::new().parse(source).unwrap();
        let Item::Function(function) = &module.items[0] else {
            panic!("expected function");
        };
        let Stmt::If { else_block, .. } = &function.body.0[0] else {
            panic!("expected if statement");
        };
        let Some(else_block) = else_block else {
            panic!("expected else block");
        };
        assert_eq!(else_block.0.len(), 1);
        assert!(matches!(else_block.0[0], Stmt::If { .. }));
    }

    #[test]
    fn parses_cpp_binary_search_control_flow() {
        let source = r#"
int binary_search(vector<int>& nums, int target) {
    int left = 0;
    int right = nums.size() - 1;
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
"#;

        let module = CppParser::new().parse(source).unwrap();
        let Item::Function(function) = &module.items[0] else {
            panic!("expected function");
        };

        assert!(matches!(function.body.0[0], Stmt::VarDecl(_)));
        assert!(matches!(function.body.0[2], Stmt::While { .. }));
    }

    #[test]
    fn parses_cpp_cstyle_for_loop() {
        let source = r#"
int first_even(vector<int>& nums) {
    for (int i = 0; i < nums.size(); i++) {
        if (nums[i] % 2 == 0) {
            return nums[i];
        }
    }
    return -1;
}
"#;

        let module = CppParser::new().parse(source).unwrap();
        let Item::Function(function) = &module.items[0] else {
            panic!("expected function");
        };
        let Stmt::For { kind, .. } = &function.body.0[0] else {
            panic!("expected for loop");
        };
        let ForKind::CStyle { init, cond, step } = kind else {
            panic!("expected C-style for loop");
        };
        let Stmt::VarDecl(init_decl) = init.as_ref() else {
            panic!("expected variable declaration init");
        };
        assert_eq!(init_decl.name, "i");
        assert_eq!(init_decl.init, Some(Expr::Literal(Literal::Int(0))));
        assert!(matches!(
            cond,
            Expr::Binary {
                op: BinOp::Lt,
                lhs,
                rhs,
            } if matches!(lhs.as_ref(), Expr::Ident(name) if name == "i")
                && matches!(rhs.as_ref(), Expr::Call { callee, args }
                    if args.is_empty()
                        && matches!(callee.as_ref(), Expr::Field { obj, name }
                            if name == "size"
                                && matches!(obj.as_ref(), Expr::Ident(obj_name) if obj_name == "nums")))
        ));
        assert_eq!(step, &Expr::Raw("i++".to_string()));
    }

    #[test]
    fn parses_cpp_range_for_loop() {
        let source = r#"
int total(vector<int>& nums) {
    int total = 0;
    for (int value : nums) {
        total = total + value;
    }
    return total;
}
"#;

        let module = CppParser::new().parse(source).unwrap();
        let Item::Function(function) = &module.items[0] else {
            panic!("expected function");
        };
        let Stmt::For { kind, .. } = &function.body.0[1] else {
            panic!("expected for loop");
        };
        assert!(matches!(kind, ForKind::ForEach { .. }));
    }
}
