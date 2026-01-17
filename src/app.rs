//! Application module for lflog.
//!
//! Provides high-level API for loading configuration and querying log files with SQL.

use anyhow::{Result, bail};
use datafusion::prelude::{DataFrame, SessionContext};
use std::sync::Arc;

use crate::datafusion::LogTableProvider;
use crate::macros::parser::Profiles;
use crate::scanner::Scanner;

/// Query options for registering a log file.
#[derive(Debug, Clone)]
pub struct QueryOptions {
    /// Path to the log file to query.
    pub log_file: String,
    /// Profile name from config (optional).
    pub profile_name: Option<String>,
    /// Override pattern (optional). If provided, overrides the profile's pattern.
    pub pattern_override: Option<String>,
    /// Table name for SQL queries (default: "log").
    pub table_name: String,
    /// Add file path to the schema (default: false).
    pub add_file_path: bool,
    /// Add raw log line to the schema (default: false).
    pub add_raw: bool,
    /// Number of threads
    pub num_threads: Option<usize>,
}

impl Default for QueryOptions {
    fn default() -> Self {
        Self {
            log_file: String::new(),
            profile_name: None,
            pattern_override: None,
            table_name: "log".to_string(),
            add_file_path: false,
            add_raw: false,
            num_threads: None,
        }
    }
}

impl QueryOptions {
    /// Create new QueryOptions with a log file path.
    pub fn new(log_file: impl Into<String>) -> Self {
        Self {
            log_file: log_file.into(),
            ..Default::default()
        }
    }

    /// Set the profile name.
    pub fn with_profile(mut self, profile: impl Into<String>) -> Self {
        self.profile_name = Some(profile.into());
        self
    }

    /// Set the pattern override.
    pub fn with_pattern(mut self, pattern: impl Into<String>) -> Self {
        self.pattern_override = Some(pattern.into());
        self
    }

    /// Set the table name.
    pub fn with_table_name(mut self, name: impl Into<String>) -> Self {
        self.table_name = name.into();
        self
    }

    /// Set whether to add file path column.
    pub fn with_add_file_path(mut self, add_file_path: bool) -> Self {
        self.add_file_path = add_file_path;
        self
    }

    /// Set whether to add raw log line column.
    pub fn with_add_raw(mut self, add_raw: bool) -> Self {
        self.add_raw = add_raw;
        self
    }

    /// Set the number of threads to use for processing.
    pub fn with_num_threads(mut self, num_threads: Option<u32>) -> Self {
        self.num_threads = num_threads.map(|n| n as usize);
        self
    }
}

/// Main application struct for querying log files.
pub struct LfLog {
    ctx: SessionContext,
    profiles: Option<Profiles>,
}

impl LfLog {
    /// Initialize from a TOML config file.
    pub fn from_config(config_path: &str) -> Result<Self> {
        let profiles = Profiles::from_file(config_path)?;
        Ok(Self::from_profiles(profiles))
    }

    /// Initialize from a Profiles struct.
    pub fn from_profiles(profiles: Profiles) -> Self {
        Self {
            ctx: SessionContext::new(),
            profiles: Some(profiles),
        }
    }

    /// Initialize with no profiles (for inline pattern usage only).
    pub fn new() -> Self {
        Self {
            ctx: SessionContext::new(),
            profiles: None,
        }
    }

    /// Register a log file for querying.
    ///
    /// The pattern is determined in the following order:
    /// 1. `pattern_override` if provided
    /// 2. Profile's pattern if `profile_name` is provided
    /// 3. Error if neither is provided
    pub fn register(&self, options: QueryOptions) -> Result<()> {
        // Determine the pattern to use
        let (pattern, custom_macros) = if let Some(ref override_pattern) = options.pattern_override
        {
            // Use override pattern with profile's macros if available
            let macros = if let (Some(profiles), Some(profile_name)) =
                (&self.profiles, &options.profile_name)
            {
                profiles
                    .get_profile(profile_name)
                    .map(|p| p.custom_macros.clone())
            } else {
                self.profiles
                    .as_ref()
                    .map(|profiles| profiles.custom_macros.clone())
            };
            (override_pattern.clone(), macros)
        } else if let Some(ref profile_name) = options.profile_name {
            // Use profile's pattern
            let profiles = self
                .profiles
                .as_ref()
                .ok_or_else(|| anyhow::anyhow!("No profiles loaded, cannot use --profile"))?;
            let profile = profiles
                .get_profile(profile_name)
                .ok_or_else(|| anyhow::anyhow!("Profile '{}' not found", profile_name))?;
            (profile.pattern.clone(), Some(profile.custom_macros.clone()))
        } else {
            bail!("Either --profile or --pattern must be provided");
        };

        // Create scanner with the pattern and custom macros
        let scanner = if let Some(macros) = custom_macros {
            Scanner::with_custom_macros(pattern, Some(&macros))
        } else {
            Scanner::new(pattern)
        };

        // Create table provider and register it
        let table = LogTableProvider::new(
            scanner,
            options.log_file,
            options.add_file_path,
            options.add_raw,
            options.num_threads,
        );
        self.ctx
            .register_table(&options.table_name, Arc::new(table))?;

        Ok(())
    }

    /// Execute a SQL query and return results as a DataFrame.
    pub async fn query(&self, sql: &str) -> Result<DataFrame> {
        let df = self.ctx.sql(sql).await?;
        Ok(df)
    }

    /// Execute a SQL query and print results to stdout.
    pub async fn query_and_show(&self, sql: &str) -> Result<()> {
        let df = self.ctx.sql(sql).await?;
        df.show().await?;
        Ok(())
    }

    /// Get the underlying SessionContext for advanced usage.
    pub fn context(&self) -> &SessionContext {
        &self.ctx
    }
}

impl Default for LfLog {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn test_lflog_with_inline_pattern() {
        let lflog = LfLog::new();

        let options = QueryOptions::new("loghub/Apache/Apache_2k.log")
            .with_pattern(r#"^\[{{time:datetime("%a %b %d %H:%M:%S %Y")}}\] \[{{level:var_name}}\] {{message:any}}$"#)
            .with_table_name("log");

        lflog.register(options).unwrap();

        // Select all columns to avoid projection issues in LogTableExec
        let df = lflog.query("SELECT * FROM log LIMIT 5").await.unwrap();
        let batches = df.collect().await.unwrap();
        assert!(!batches.is_empty());
    }
}
