//! lflog CLI - Query log files with SQL.
//!
//! Usage: lflog [OPTIONS] <LOG_FILE>
//!
//! Config file resolution order:
//! 1. --config <path> if provided
//! 2. LFLOG_CONFIG environment variable
//! 3. ~/.config/lflog/config.toml (default)

use clap::Parser;
use lflog::error::{Error, Result};
use std::io::{Write, stdout};
use std::path::PathBuf;

use lflog::{LfLog, QueryOptions};

/// Query log files with SQL using regex patterns.
#[derive(Parser)]
#[command(name = "lflog")]
#[command(version, about = "Query log files with SQL")]
struct Cli {
    /// Log file to query.
    log_file: String,

    /// Path to config file (TOML).
    /// Default: ~/.config/lflog/config.toml or LFLOG_CONFIG env var.
    #[arg(short, long)]
    config: Option<String>,

    /// Profile name from config.
    #[arg(short, long)]
    profile: Option<String>,

    /// Override pattern (or use without profile).
    #[arg(long)]
    pattern: Option<String>,

    /// Custom table name (default: "log").
    #[arg(short, long, default_value = "log")]
    table: String,

    /// SQL query to execute (omit for interactive mode).
    #[arg(short, long)]
    query: Option<String>,

    /// Whether to add file path column (default: false).
    #[arg(short = 'f', long, default_value = "false")]
    add_file_path: bool,

    /// Whether to add raw log line column (default: false).
    #[arg(short = 'r', long, default_value = "false")]
    add_raw: bool,

    /// Number of threads to use for processing (default: 8).
    #[arg(short, long, default_value = "8")]
    num_threads: Option<u32>,
}

/// Resolve config file path from CLI, env var, or default.
fn resolve_config_path(cli_config: Option<String>) -> Option<PathBuf> {
    // 1. CLI argument takes priority
    if let Some(path) = cli_config {
        return Some(PathBuf::from(path));
    }

    // 2. Environment variable
    if let Ok(path) = std::env::var("LFLOG_CONFIG") {
        return Some(PathBuf::from(path));
    }

    // 3. Default path: ~/.config/lflog/config.toml
    if let Some(home) = dirs::home_dir() {
        let default_path = home.join(".config").join("lflog").join("config.toml");
        if default_path.exists() {
            return Some(default_path);
        }
    }

    None
}

/// Run interactive REPL mode.
async fn run_repl(lflog: &LfLog) -> Result<()> {
    let mut rl = rustyline::DefaultEditor::new()?;

    println!("lflog interactive mode. Type SQL queries, '.exit' to quit.");
    println!();

    loop {
        let readline = rl.readline("lflog> ");
        match readline {
            Ok(line) => {
                let line = line.trim();
                if line.is_empty() {
                    continue;
                }
                if line == ".exit" || line == ".quit" || line == "exit" || line == "quit" {
                    break;
                }
                if line.starts_with('.') {
                    println!("Unknown command: {}", line);
                    println!("Commands: .exit, .quit");
                    continue;
                }

                let _ = rl.add_history_entry(line);

                match lflog.query_and_show(line).await {
                    Ok(()) => {}
                    Err(e) => {
                        eprintln!("Error: {}", e);
                    }
                }
                println!();
            }
            Err(rustyline::error::ReadlineError::Interrupted) => {
                println!("^C");
                continue;
            }
            Err(rustyline::error::ReadlineError::Eof) => {
                println!("Bye!");
                break;
            }
            Err(err) => {
                eprintln!("Error: {:?}", err);
                break;
            }
        }
        stdout().flush()?;
    }

    Ok(())
}

#[tokio::main]
async fn main() -> Result<()> {
    // Initialize logging
    let _ = env_logger::builder().is_test(false).try_init();

    let cli = Cli::parse();

    // Resolve config file path
    let config_path = resolve_config_path(cli.config);

    // Initialize LfLog with or without config
    let lflog = if let Some(ref path) = config_path {
        LfLog::from_config(path.to_str().unwrap())?
    } else {
        // No config file - must use inline pattern
        if cli.pattern.is_none() {
            return Err(Error::Config(
                "No config file found. Either:\n\
                 - Create ~/.config/lflog/config.toml\n\
                 - Set LFLOG_CONFIG environment variable\n\
                 - Use --config <path>\n\
                 - Use --pattern <regex> without a config file"
                    .into(),
            ));
        }
        LfLog::new()
    };

    // Build query options
    let options = QueryOptions::new(&cli.log_file).with_table_name(&cli.table);

    let options = if let Some(profile) = cli.profile {
        options.with_profile(profile)
    } else {
        options
    };

    let options = if let Some(pattern) = cli.pattern {
        options.with_pattern(pattern)
    } else {
        options
    };

    let options = options
        .with_add_file_path(cli.add_file_path)
        .with_add_raw(cli.add_raw)
        .with_num_threads(cli.num_threads);

    // Register the log file
    lflog.register(options)?;

    // Execute query or start REPL
    if let Some(sql) = cli.query {
        lflog.query_and_show(&sql).await?;
    } else {
        run_repl(&lflog).await?;
    }

    Ok(())
}
