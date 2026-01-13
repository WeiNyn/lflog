//! DataFusion integration for log file querying.
//!
//! Provides a `TableProvider` implementation that allows querying log files
//! with SQL using DataFusion.

mod builder;
mod exec;
mod provider;

pub use builder::FieldsBuilder;
pub use exec::LogTableExec;
pub use provider::LogTableProvider;
