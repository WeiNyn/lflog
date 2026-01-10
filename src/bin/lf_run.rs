use anyhow::Result;
use datafusion::prelude::SessionContext;
use std::sync::Arc;
use std::time::Instant;

use lflog::profile::Scanner;
use lflog::table_provider::LogTableProvider;

#[tokio::main]
async fn main() -> Result<()> {
    // initialize optional logging
    let _ = env_logger::builder().is_test(false).try_init();

    // CLI arguments:
    // argv[1] = file path (optional, default matches tests)
    // argv[2] = regex pattern (optional, default matches tests)
    // argv[3] = limit (optional, default = 10)
    let mut args = std::env::args().skip(1);
    let file_path = args
        .next()
        .unwrap_or_else(|| "loghub/Apache/Apache_2k.log".to_string());
    let pattern = args.next().unwrap_or_else(|| {
        r"^\[(?P<time>\w{3} \w{3} \d{1,2} \d{2}:\d{2}:\d{2} \d{4})\] \[(?P<level>[^\]]+)\] (?P<message>.*)$"
            .to_string()
    });
    let limit: usize = args
        .next()
        .and_then(|s| s.parse::<usize>().ok())
        .unwrap_or(10);

    println!("file: {}", file_path);
    println!("pattern: {}", pattern);
    println!("limit: {}", limit);

    // Build scanner and table provider
    let scanner = Scanner::new(pattern.clone());
    let log_table = LogTableProvider::new(scanner, file_path.clone());

    // Create DataFusion context and register the table
    let ctx = SessionContext::new();
    ctx.register_table("log", Arc::new(log_table))?;

    // Prepare SQL with LIMIT so df.show() doesn't print everything unless intended
    let sql = format!(
        "SELECT time, level, message FROM log WHERE level = 'error' LIMIT {}",
        limit
    );

    // Time the planning (building the logical/physical plan)
    let plan_start = Instant::now();
    let df = ctx.sql(&sql).await?;
    let plan_elapsed = plan_start.elapsed();

    // Time execution using df.show() as requested (this will print the rows)
    let exec_start = Instant::now();
    df.show().await?;
    let exec_elapsed = exec_start.elapsed();

    println!();
    println!("--- benchmark ---");
    println!("sql: {}", sql);
    println!(
        "planning time: {:.3} ms",
        plan_elapsed.as_secs_f64() * 1000.0
    );
    println!(
        "execution time (df.show): {:.3} ms",
        exec_elapsed.as_secs_f64() * 1000.0
    );
    println!("note: row count is not collected when using df.show()");

    Ok(())
}
