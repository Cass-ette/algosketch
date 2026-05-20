use thiserror::Error;

/// Top-level error type for the algosketch pipeline.
#[derive(Error, Debug)]
pub enum PseudoError {
    #[error("unsupported language: {0}")]
    UnsupportedLanguage(String),

    #[error("cannot infer source language; pass --source-lang")]
    UnknownLanguage,

    #[error("io error: {0}")]
    Io(#[from] std::io::Error),

    #[error("parse error in {file}: {message}")]
    Parse { file: String, message: String },

    #[error("internal error: {0}")]
    Internal(String),
}

pub type Result<T> = std::result::Result<T, PseudoError>;
