# AGENTS.md

This document provides guidelines for agentic coding agents working on the lflog codebase.

## Project Overview

lflog is a high-performance SQL query engine for application logs, built in Rust using DataFusion. It uses a profile DSL (regex + metadata) to parse and query log files efficiently.

**Tech Stack**: Rust (core), Python (profile tooling, CLI), DataFusion (SQL engine)

## Build Commands

### Standard Build
```bash
cargo build                    # Debug build
cargo build --release          # Optimized release build
```

### Linting
```bash
cargo clippy -- -D warnings    # Lint with warnings as errors
cargo clippy                   # Standard clippy check
```

### Testing
```bash
cargo test                     # Run all tests
cargo test -- --nocapture      # Run tests with stdout/stderr visible
cargo test <test_name>         # Run specific test by name
cargo test --lib               # Run only library unit tests
cargo test --bin lf_run        # Run binary tests only
```

**Run single test** (most common pattern):
```bash
cargo test test_scanner_integration
cargo test test_log_table_provider
```

## Code Style Guidelines

### Imports

Organize: std/core → external crates → internal modules (`crate::`)
```rust
use regex::Regex;
use std::collections::HashMap;
use async_trait::async_trait;
use datafusion::arrow::datatypes::DataType;
use crate::scanner::Scanner;
use crate::types::FieldType;
```

### Naming Conventions

- **Structs/Enums/Traits**: PascalCase (`LogTableProvider`, `FieldType`)
- **Functions/Methods**: snake_case (`scan`, `create_physical_plan`, `expand_macros`)
- **Constants**: SCREAMING_SNAKE_CASE
- **Fields**: snake_case (`field_name`, `file_path`)
- **Modules**: snake_case
- **Type Parameters**: Single uppercase letter (`T`, `E`)

### Types & Errors

- Use `anyhow::Result<T>` or `datafusion::common::Result<T>` for error propagation
- Use `Option<T>` for nullable values
- Use `bail!("error message")` for early returns with errors
- Use `?` operator for propagating errors

```rust
use anyhow::{Result, bail};
fn parse_config(path: &str) -> Result<Config> {
    if !path.ends_with(".yaml") {
        bail!("config file must be .yaml");
    }
    Ok(Config {})
}
```

### Async Code

- Use `#[async_trait]` for trait implementations
- Use `#[tokio::test]` for async test functions
- Use `async fn` for async functions
- Use `await?` for propagating async errors

```rust
#[async_trait]
impl TableProvider for LogTableProvider {
    async fn scan(&self, _state: &dyn Session, ...) -> Result<Arc<dyn ExecutionPlan>> {
        self.create_physical_plan(projection, self.schema())
    }
}
```

### Structs & Derives

Common derives: `Debug, Clone, PartialEq, Eq, Serialize, Deserialize`
```rust
#[derive(Debug, Clone)]
pub struct Scanner {
    regex: Regex,
    pub field_names: Vec<String>,
}
```

### Documentation

- Use `//!` for module-level documentation at top of files
- Use `///` for public API documentation on items
```rust
//! Log line scanner using compiled regex patterns.

/// Scans log lines using a compiled regex pattern with named capture groups.
pub struct Scanner { }
```

### Tests

- Place tests in `#[cfg(test)]` modules at bottom of files
- Use `#[test]` for sync tests, `#[tokio::test]` for async tests
- Prefix test functions with `test_`
```rust
#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn test_scanner_integration() { }
}
```

### File Organization

- `src/lib.rs`: Module exports with re-exports for convenience
- `src/bin/`: Binary entry points
- One logical module per file when possible
- Tests co-located with implementation

### Performance Considerations

- Prefer zero-copy references (`&str`, `&[u8]`) when possible
- Use `String::with_capacity()` and `Vec::with_capacity()` for known-size collections
- Use `Arc` for shared ownership across threads

### Git Workflow

- Build and test before committing: `cargo build && cargo test`
- Run clippy: `cargo clippy -- -D warnings`
- Keep commits focused and atomic

## Dependencies

Key external crates (verify before adding):
- `anyhow` - Error handling
- `serde` - Serialization
- `tokio` - Async runtime
- `datafusion` - SQL query engine
- `regex` - Pattern matching
- `rayon` - Parallel processing
- `async-trait` - Async trait support
- `memmap2` - Memory mapped files

Check `Cargo.toml` for current dependencies and versions before adding new ones.

## Notes

- Rust edition: 2024
- Primary binaries: `src/bin/lflog.rs`, `src/bin/lf_run.rs`
- Log data for testing: `loghub/` directory
- No existing rustfmt.toml or clippy.toml - use default tooling
