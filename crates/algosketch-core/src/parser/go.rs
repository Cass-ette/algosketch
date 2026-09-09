use crate::diagnostics::RawDiagnostics;
use crate::error::{PseudoError, Result};
use crate::ir::*;
use crate::parser::common::{node_text, parse_err, record_raw_item};
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
                "package_clause" | "import_declaration" => continue,
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
    let body_node = node
        .child_by_field_name("body")
        .ok_or_else(|| parse_err("function missing body"))?;

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

/// Stub for Task 2: bodies stay empty until statement parsing lands in Task 3.
/// Go `block` nodes wrap their statements in a single `statement_list` child
/// (unlike java/cpp blocks), which Task 3's implementation must descend through.
fn parse_block(
    _source: &str,
    _node: tree_sitter::Node,
    _diag: &mut RawDiagnostics,
) -> Result<Block> {
    Ok(Block(vec![]))
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
}
