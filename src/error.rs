#![allow(missing_docs)]
//! Project-wide error types for lflog.
//!
//! This module defines a `thiserror`-based `Error` enum and a `Result<T>` alias
//! that are intended to be used across the crate.

use thiserror::Error;

/// Project-level error enum.
#[derive(Debug, Error)]
pub enum Error {
    #[error("I/O error: {0}")]
    Io(#[from] std::io::Error),

    #[error("UTF-8 error: {0}")]
    Utf8(#[from] std::str::Utf8Error),

    #[error("Regex error: {0}")]
    Regex(#[from] regex::Error),

    #[error("Glob pattern error: {0}")]
    GlobPattern(#[from] glob::PatternError),

    #[error("Glob error: {0}")]
    Glob(#[from] glob::GlobError),

    #[error("TOML parse error: {0}")]
    Toml(#[from] toml::de::Error),

    #[error("DataFusion error: {0}")]
    DataFusion(#[from] datafusion_common::DataFusionError),

    #[error("Arrow error: {0}")]
    Arrow(#[from] datafusion::arrow::error::ArrowError),

    #[error("Rustyline error: {0}")]
    Readline(#[from] rustyline::error::ReadlineError),

    // Domain-specific errors
    #[error("Macro parse error: {0}")]
    MacroParse(String),

    #[error("Macro expansion error: {0}")]
    Macro(String),

    #[error("Configuration error: {0}")]
    Config(String),

    #[error("No files found for path: {0}")]
    NoFiles(String),

    #[error("{0}")]
    Other(String),
}

/// Convenience result alias using the crate's `Error`.
pub type Result<T> = std::result::Result<T, Error>;

impl Error {
    /// Convenience constructor for `Other` variant.
    pub fn other<S: Into<String>>(s: S) -> Self {
        Error::Other(s.into())
    }
}
