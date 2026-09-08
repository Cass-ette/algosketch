use crate::diagnostics::RawDiagnostics;
use crate::error::{PseudoError, Result};
use crate::ir::Module;
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

        let diag = RawDiagnostics::default(); // no mut yet: nothing records until Task 2
        let items = Vec::new(); // statements land in Task 2–4
        Ok((
            Module {
                source_language: SourceLang::Go,
                items,
            },
            diag,
        ))
    }
}
