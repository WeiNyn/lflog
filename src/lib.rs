//! lflog - Log file querying with DataFusion and regex patterns.
//!
//! This crate provides tools for parsing log files using regex patterns with
//! macro expansions and querying them using SQL via DataFusion.

pub mod datafusion;
pub mod macros;
pub mod scanner;
pub mod types;

// Re-export commonly used items for convenience
pub use datafusion::LogTableProvider;
pub use scanner::Scanner;
pub use types::FieldType;
