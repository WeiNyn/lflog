use anyhow::Result;
use datafusion::prelude::SessionContext;
use std::sync::Arc;
use std::time::Instant;

use lflog::{LogTableProvider, Scanner};

#[tokio::main]
async fn main() -> Result<()> {
    let args: Vec<String> = std::env::args().collect();
    if args.len() < 2 {
        eprintln!("Usage: {} <file_path> [query]", args[0]);
        return Ok(());
    }

    let file_path = &args[1];
    let query = if args.len() > 2 {
        &args[2]
    } else {
        "SELECT count(*) FROM log WHERE level = 'error'"
    };

    let pattern = r"^\[(?P<time>\w{3} \w{3} \d{1,2} \d{2}:\d{2}:\d{2} \d{4})\] \[(?P<level>[^\]]+)\] (?P<message>.*)$";

    println!("Benchmarking file: {}", file_path);
    println!("Query: {}", query);

    let scanner = Scanner::new(pattern.to_string());
    let log_table = LogTableProvider::new(scanner, file_path.clone());

    let ctx = SessionContext::new();
    ctx.register_table("log", Arc::new(log_table))?;

    let start = Instant::now();
    let df = ctx.sql(query).await?;
    let batches = df.collect().await?;
    let duration = start.elapsed();

    let row_count: usize = batches.iter().map(|b| b.num_rows()).sum();

    println!("Result row count: {}", row_count);
    println!("Time elapsed: {:.2?}", duration);

    Ok(())
}
