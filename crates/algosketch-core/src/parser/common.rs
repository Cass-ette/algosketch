use crate::error::{PseudoError, Result};
use crate::ir::{BinOp, UnOp};

pub(crate) fn node_text<'a>(source: &'a str, node: tree_sitter::Node<'_>) -> &'a str {
    &source[node.start_byte()..node.end_byte()]
}

pub(crate) fn parse_err(msg: impl Into<String>) -> PseudoError {
    PseudoError::Parse {
        file: "input".into(),
        message: msg.into(),
    }
}

pub(crate) fn named_child_by_kind<'a>(
    node: tree_sitter::Node<'a>,
    kind: &str,
) -> Option<tree_sitter::Node<'a>> {
    (0..node.child_count())
        .filter_map(|i| node.child(i))
        .find(|c| c.is_named() && c.kind() == kind)
}

pub(crate) fn named_children_of_kind<'a>(
    node: tree_sitter::Node<'a>,
    kind: &str,
) -> Vec<tree_sitter::Node<'a>> {
    let mut children = Vec::new();
    for i in 0..node.named_child_count() {
        let child = node.named_child(i).unwrap();
        if child.kind() == kind {
            children.push(child);
        }
    }
    children
}

pub(crate) fn find_anon_operator<'a>(
    source: &'a str,
    node: tree_sitter::Node<'_>,
) -> Option<&'a str> {
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

pub(crate) fn parse_bin_op(text: &str) -> Result<BinOp> {
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

pub(crate) fn parse_comparison_op(text: &str) -> Result<BinOp> {
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

pub(crate) fn parse_c_family_bin_op(text: &str) -> Option<BinOp> {
    match text {
        "==" => Some(BinOp::Eq),
        "!=" => Some(BinOp::Ne),
        "<" => Some(BinOp::Lt),
        "<=" => Some(BinOp::Le),
        ">" => Some(BinOp::Gt),
        ">=" => Some(BinOp::Ge),
        "/" => Some(BinOp::IntDiv),
        "+" => Some(BinOp::Add),
        "-" => Some(BinOp::Sub),
        "*" => Some(BinOp::Mul),
        "%" => Some(BinOp::Mod),
        "&&" => Some(BinOp::And),
        "||" => Some(BinOp::Or),
        "&" => Some(BinOp::BitAnd),
        "|" => Some(BinOp::BitOr),
        "^" => Some(BinOp::BitXor),
        "<<" => Some(BinOp::Shl),
        ">>" => Some(BinOp::Shr),
        _ => None,
    }
}

pub(crate) fn parse_un_op(text: &str) -> Result<UnOp> {
    match text {
        "-" => Ok(UnOp::Neg),
        "not" | "!" => Ok(UnOp::Not),
        "~" => Ok(UnOp::BitNot),
        _ => Err(parse_err(format!("unknown unary operator: {text}"))),
    }
}
