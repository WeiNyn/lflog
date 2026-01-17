# lflog

Query log files with SQL using DataFusion and regex pattern macros.

## Features

- 🔍 **SQL Queries** - Query log files using familiar SQL syntax via [DataFusion](https://datafusion.apache.org/)
- 🧩 **Pattern Macros** - Use intuitive macros like `{{timestamp:datetime("%Y-%m-%d")}}` instead of raw regex
- 📊 **Type Inference** - Automatic schema generation with proper types (Int32, Float64, String)
- ⚡ **Fast** - Leverages DataFusion's optimized query engine with parallel processing
- 📁 **Glob Patterns** - Query multiple files at once with patterns like `logs/*.log`
- 🏷️ **Metadata Columns** - Access file path (`__FILE__`) and raw log lines (`__RAW__`)
- 📝 **Config Profiles** - Define reusable log profiles in TOML config files
- 💻 **Interactive REPL** - Query logs interactively with command history

## Installation

```bash
cargo build --release
```

## CLI Usage

```bash
lflog <log_file> [OPTIONS]
```

### Options

| Option | Description |
|--------|-------------|
| `-c, --config <path>` | Config file (default: `~/.config/lflog/config.toml` or `LFLOG_CONFIG` env) |
| `-p, --profile <name>` | Use profile from config |
| `--pattern <regex>` | Inline pattern (overrides profile) |
| `-t, --table <name>` | Table name for SQL (default: `log`) |
| `-q, --query <sql>` | Execute SQL query (omit for interactive mode) |
| `-f, --add-file-path` | Add `__FILE__` column with source file path |
| `-r, --add-raw` | Add `__RAW__` column with raw log line |
| `-n, --num-threads <N>` | Number of threads (default: 8, or `LFLOGTHREADS` env) |

### Examples

```bash
# Query with inline pattern
lflog loghub/Apache/Apache_2k.log \
  --pattern '^\[{{time:any}}\] \[{{level:var_name}}\] {{message:any}}$' \
  --query "SELECT * FROM log WHERE level = 'error' LIMIT 10"

# Query multiple files with glob pattern
lflog 'logs/*.log' \
  --pattern '{{ts:datetime}} [{{level:var_name}}] {{msg:any}}' \
  --query "SELECT * FROM log"

# Include file path and raw line in results
lflog 'logs/*.log' --pattern '...' \
  --add-file-path --add-raw \
  --query 'SELECT level, "__FILE__", "__RAW__" FROM log'

# Query with config profile
lflog /var/log/apache.log --profile apache --query "SELECT * FROM log LIMIT 5"

# Interactive REPL mode
lflog server.log --pattern '{{ts:datetime}} [{{level:var_name}}] {{msg:any}}'
> SELECT * FROM log WHERE level = 'error'
> SELECT level, COUNT(*) FROM log GROUP BY level
> .exit
```

## Demos (Loghub)

`lflog` includes a comprehensive set of demos using the [Loghub](https://github.com/logpai/loghub) dataset collection. These demos showcase how to query 16 different types of system logs (Android, Apache, Hadoop, HDFS, Linux, Spark, etc.).

To run a demo:

```bash
# 1. Go to the demo scripts directory
cd examples/loghub_demos/scripts

# 2. Run the demo for a specific dataset (e.g., Apache)
./run_demo.sh apache
```

See [examples/loghub_demos/README.md](examples/loghub_demos/README.md) for the full list of available datasets and more details.

## Config File

Create `~/.config/lflog/config.toml`:

```toml
# Global custom macros
[[custom_macros]]
name = "timestamp"
pattern = '\\d{4}-\\d{2}-\\d{2} \\d{2}:\\d{2}:\\d{2}'
type_hint = "DateTime"

# Apache log profile
[[profiles]]
name = "apache"
description = "Apache error log format"
pattern = '^\[{{time:datetime("%a %b %d %H:%M:%S %Y")}}\] \[{{level:var_name}}\] {{message:any}}$'

# Nginx access log profile
[[profiles]]
name = "nginx"
pattern = '{{ip:ip}} - - \[{{time:any}}\] "{{method:var_name}} {{path:any}}" {{status:number}} {{bytes:number}}'
```

## Pattern Macros

| Macro | Description | Type |
|-------|-------------|------|
| `{{field:number}}` | Integer (digits) | Int32 |
| `{{field:string}}` | Non-greedy string | String |
| `{{field:any}}` | Non-greedy match all | String |
| `{{field:var_name}}` | Identifier (`[A-Za-z_][A-Za-z0-9_]*`) | String |
| `{{field:datetime("%fmt")}}` | Datetime with strftime format | String |
| `{{field:enum(a,b,c)}}` | One of the listed values | String |
| `{{field:uuid}}` | UUID format | String |
| `{{field:ip}}` | IPv4 address | String |

You can also use raw regex with named capture groups:

```regex
^(?P<ip>\d+\.\d+\.\d+\.\d+) - (?P<method>\w+)
```

## Metadata Columns

When enabled, lflog adds special metadata columns to your query results:

| Column | Flag | Description |
|--------|------|-------------|
| `__FILE__` | `-f, --add-file-path` | Absolute path of the source log file |
| `__RAW__` | `-r, --add-raw` | The original, unparsed log line |

These are useful when querying multiple files or when you need to see the original log line alongside parsed fields:

```bash
# Find errors across all log files with their source
lflog 'logs/*.log' --pattern '...' --add-file-path \
  --query 'SELECT "__FILE__", level, message FROM log WHERE level = '\''error'\'''
```

> **Note**: Use double quotes around `__FILE__` and `__RAW__` in SQL to preserve case.

## Library Usage

```rust
use lflog::{LfLog, QueryOptions};

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    // With inline pattern
    let lflog = LfLog::new();
    
    lflog.register(
        QueryOptions::new("access.log")
            .with_pattern(r#"^\[{{time:any}}\] \[{{level:var_name}}\] {{message:any}}$"#)
    )?;
    
    lflog.query_and_show("SELECT * FROM log WHERE level = 'error'").await?;
    Ok(())
}
```

With glob patterns and metadata columns:

```rust
use lflog::{LfLog, QueryOptions};

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    let lflog = LfLog::new();
    
    // Query multiple files with metadata columns
    lflog.register(
        QueryOptions::new("logs/*.log")  // Glob pattern
            .with_pattern(r#"^\[{{time:any}}\] \[{{level:var_name}}\] {{message:any}}$"#)
            .with_add_file_path(true)    // Add __FILE__ column
            .with_add_raw(true)          // Add __RAW__ column
            .with_num_threads(Some(4))   // Use 4 threads
    )?;
    
    lflog.query_and_show(r#"SELECT level, "__FILE__" FROM log WHERE level = 'error'"#).await?;
    Ok(())
}
```

Or with config profiles:

```rust
use lflog::{LfLog, QueryOptions};

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    let lflog = LfLog::from_config("~/.config/lflog/config.toml")?;
    
    lflog.register(
        QueryOptions::new("/var/log/apache.log")
            .with_profile("apache")
    )?;
    
    let df = lflog.query("SELECT level, COUNT(*) FROM log GROUP BY level").await?;
    df.show().await?;
    Ok(())
}
```

## Project Structure

```
src/
├── lib.rs              # Public API
├── app.rs              # LfLog application struct
├── types.rs            # FieldType enum
├── scanner.rs          # Pattern matching
├── macros/             # Macro expansion
│   ├── parser.rs       # Config & macro parsing
│   └── expander.rs     # Macro to regex expansion
├── datafusion/         # DataFusion integration
│   ├── builder.rs
│   ├── provider.rs
│   └── exec.rs
└── bin/
    ├── lflog.rs        # Main CLI
    └── lf_run.rs       # Simple runner (deprecated)
```

## Performance

`lflog` is designed for high performance, leveraging **zero-copy parsing** and DataFusion's vectorized execution engine.

### Benchmarks

Parsing an Apache error log (168MB, 2 million lines):

| Query | Time | Throughput |
|-------|------|------------|
| `SELECT count(*) FROM log WHERE level = 'error'` | ~450ms | ~370 MB/s (4.4M lines/s) |
| `SELECT count(*) FROM log WHERE message LIKE '%error%'` | ~450ms | ~370 MB/s |

*Tested on Linux, single-threaded execution (default).*

### Optimizations

- **Zero-Copy Parsing**: Parses log lines directly from memory-mapped files without intermediate String allocations.
- **Pre-calculated Regex Indices**: Resolves capture group indices once at startup, avoiding repeated string lookups in the hot loop.
- **Parallel Execution**: Automatically partitions files for parallel processing (configurable via `LFLOGTHREADS`).

## License

MIT
