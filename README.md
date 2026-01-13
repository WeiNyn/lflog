# lflog

Query log files with SQL using DataFusion and regex pattern macros.

## Features

- 🔍 **SQL Queries** - Query log files using familiar SQL syntax via [DataFusion](https://datafusion.apache.org/)
- 🧩 **Pattern Macros** - Use intuitive macros like `{{timestamp:datetime("%Y-%m-%d")}}` instead of raw regex
- 📊 **Type Inference** - Automatic schema generation with proper types (Int32, Float64, String)
- ⚡ **Fast** - Leverages DataFusion's optimized query engine

## Quick Start

```bash
cargo run --bin lf_run -- <log_file> <pattern> [limit]
```

### Example

```bash
# Query Apache logs
cargo run --bin lf_run -- loghub/Apache/Apache_2k.log \
  '^\[{{time:datetime("%a %b %d %H:%M:%S %Y")}}\] \[{{level:var_name}}\] {{message:any}}$'
```

Output:
```
+--------------------------+--------+-----------------------------------------+
| time                     | level  | message                                 |
+--------------------------+--------+-----------------------------------------+
| Sun Dec 04 04:47:44 2005 | error  | mod_jk child workerEnv in error state 6 |
| Sun Dec 04 04:51:18 2005 | error  | mod_jk child workerEnv in error state 6 |
...
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

You can also use raw regex with named capture groups:

```regex
^(?P<ip>\d+\.\d+\.\d+\.\d+) - (?P<method>\w+)
```

## Library Usage

```rust
use lflog::{Scanner, LogTableProvider};
use datafusion::prelude::SessionContext;

#[tokio::main]
async fn main() {
    let pattern = r#"^\[{{time:datetime("%a %b %d %H:%M:%S %Y")}}\] \[{{level:var_name}}\] {{message:any}}$"#;
    let scanner = Scanner::new(pattern.to_string());
    let table = LogTableProvider::new(scanner, "access.log".to_string());

    let ctx = SessionContext::new();
    ctx.register_table("logs", Arc::new(table)).unwrap();

    let df = ctx.sql("SELECT * FROM logs WHERE level = 'error'").await.unwrap();
    df.show().await.unwrap();
}
```

## Project Structure

```
src/
├── lib.rs              # Public API
├── types.rs            # FieldType enum
├── scanner.rs          # Pattern matching
├── macros/             # Macro expansion
│   ├── parser.rs
│   └── expander.rs
└── datafusion/         # DataFusion integration
    ├── builder.rs
    ├── provider.rs
    └── exec.rs
```

## License

MIT
