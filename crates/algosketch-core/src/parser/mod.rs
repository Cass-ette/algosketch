use crate::error::Result;
use crate::ir::Module;
use crate::SourceLang;

pub(crate) mod common;
pub mod cpp;
pub mod java;
pub mod python;

pub use cpp::CppParser;
pub use java::JavaParser;
pub use python::PythonParser;

pub trait LanguageParser {
    fn language(&self) -> SourceLang;
    fn parse(&self, source: &str) -> Result<Module>;
}
