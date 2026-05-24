//! algosketch-core
//!
//! Core library for parsing real source code into a language-neutral IR
//! and rendering pseudocode + natural-language explanations.

pub mod diagnostics;
pub mod error;
pub mod ir;
pub mod parser;
pub mod renderer;

pub use error::{PseudoError, Result};

/// Source language tag carried by every parsed module.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum SourceLang {
    Python,
    Java,
    Cpp,
}

impl SourceLang {
    /// Guess source language from a file extension (without the dot).
    pub fn from_extension(ext: &str) -> Option<Self> {
        match ext.to_ascii_lowercase().as_str() {
            "py" => Some(Self::Python),
            "java" => Some(Self::Java),
            "cpp" | "cc" | "cxx" | "hpp" | "h" => Some(Self::Cpp),
            _ => None,
        }
    }

    pub fn as_str(self) -> &'static str {
        match self {
            Self::Python => "python",
            Self::Java => "java",
            Self::Cpp => "cpp",
        }
    }
}

/// Natural language for explanation output.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum NaturalLang {
    Zh,
    En,
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn natural_lang_enum_exists() {
        let zh = NaturalLang::Zh;
        let en = NaturalLang::En;
        assert_ne!(zh, en);
    }
}
