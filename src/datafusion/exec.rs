//! LogTableExec execution plan implementation.

use anyhow::Error;
use datafusion::arrow::datatypes::SchemaRef;
use datafusion::arrow::record_batch::RecordBatch;
use datafusion::execution::SendableRecordBatchStream;
use datafusion::physical_expr::{EquivalenceProperties, Partitioning};
use datafusion::physical_plan::execution_plan::{Boundedness, EmissionType};
use datafusion::physical_plan::memory::MemoryStream;
use datafusion::physical_plan::{DisplayAs, ExecutionPlan, PlanProperties};
use datafusion_common::Result;
use memmap2::Mmap;
use rayon::iter::{IntoParallelIterator, ParallelIterator};
use std::fs::File;
use std::io::BufRead;
use std::sync::Arc;

use crate::Scanner;
use crate::datafusion::builder::FieldsBuilder;
use crate::datafusion::provider::LogTableProvider;
use crate::types::FieldType;

/// Physical execution plan for reading log files.
#[derive(Debug)]
pub struct LogTableExec {
    provider: LogTableProvider,
    projected_schema: SchemaRef,
    plan_properties: PlanProperties,
}

impl LogTableExec {
    /// Create a new LogTableExec with optional column projections.
    pub fn new(
        projections: Option<&Vec<usize>>,
        schema: SchemaRef,
        provider: LogTableProvider,
    ) -> Self {
        let projected_schema = projections
            .map(|p| {
                let fields: Vec<_> = p.iter().map(|i| schema.field(*i).clone()).collect();
                SchemaRef::new(datafusion::arrow::datatypes::Schema::new(fields))
            })
            .unwrap_or_else(|| schema.clone());

        let plan_properties = PlanProperties::new(
            EquivalenceProperties::new(projected_schema.clone()),
            Partitioning::UnknownPartitioning(1),
            EmissionType::Final,
            Boundedness::Bounded,
        );

        Self {
            provider,
            projected_schema,
            plan_properties,
        }
    }
}

impl DisplayAs for LogTableExec {
    fn fmt_as(
        &self,
        _t: datafusion::physical_plan::DisplayFormatType,
        _f: &mut std::fmt::Formatter,
    ) -> std::fmt::Result {
        write!(_f, "LogTableExec")
    }
}

impl ExecutionPlan for LogTableExec {
    fn name(&self) -> &str {
        "LogTableExec"
    }

    fn as_any(&self) -> &dyn std::any::Any {
        self
    }

    fn properties(&self) -> &PlanProperties {
        &self.plan_properties
    }

    fn children(&self) -> Vec<&Arc<dyn ExecutionPlan>> {
        vec![]
    }

    fn with_new_children(
        self: Arc<Self>,
        _children: Vec<Arc<dyn ExecutionPlan>>,
    ) -> Result<Arc<dyn ExecutionPlan>> {
        Ok(self)
    }

    fn execute(
        &self,
        _partition: usize,
        _context: Arc<datafusion::execution::TaskContext>,
    ) -> Result<SendableRecordBatchStream> {
        // Get field types in the same order as field_names, defaulting to String
        let default_string = FieldType::String;

        let projected_fields = self.projected_schema.fields().iter().collect::<Vec<_>>();
        let field_names: Vec<&str> = projected_fields
            .iter()
            .map(|f| f.name().as_str())
            .collect::<Vec<_>>();
        let field_types: Vec<&FieldType> = projected_fields
            .iter()
            .map(|f| {
                self.provider
                    .scanner
                    .type_hints
                    .get(f.name())
                    .unwrap_or(&default_string)
            })
            .collect();

        let partitions = parse(
            &self.provider.file_path,
            &self.provider.scanner,
            &field_names,
            &field_types,
            self.projected_schema.clone(),
            None, // Use default thread count
        )
        .map_err(|e| datafusion_common::DataFusionError::External(e.into()))?;

        Ok(Box::pin(MemoryStream::try_new(
            partitions,
            self.schema(),
            None,
        )?))
    }

    fn schema(&self) -> SchemaRef {
        self.projected_schema.clone()
    }
}

fn parse(
    file: &str,
    scanner: &Scanner,
    field_names: &[&str],
    field_types: &[&FieldType],
    schema: SchemaRef,
    thread_count: Option<usize>,
) -> Result<Vec<RecordBatch>, Error> {
    let file = File::open(file)?;

    let mmap = unsafe { Mmap::map(&file)? };

    // If thread_count is not provided, use LFLOGTHREADS environment variable
    let thread_count = thread_count.or_else(|| {
        std::env::var("LFLOGTHREADS")
            .ok()
            .and_then(|s| s.parse().ok())
    });

    let chunk_count = thread_count
        .unwrap_or_else(rayon::current_num_threads)
        .clamp(1, rayon::current_num_threads());
    let total_size = mmap.len();
    let chunk_size = total_size / chunk_count;

    let partitions: Result<Vec<RecordBatch>, Error> = (0..chunk_count)
        .into_par_iter()
        .map(|i| {
            let mut fields_builder = FieldsBuilder::new(field_types);

            let start = i * chunk_size;

            // Find actual chunk boundaries at newline positions
            let actual_start = if i == 0 {
                0
            } else {
                // Start after the newline that ends the previous chunk's last line
                find_next_newline(&mmap, start, total_size).unwrap_or(total_size)
            };

            let actual_end = if i == chunk_count - 1 {
                // Last chunk goes to the end of the file
                total_size
            } else {
                // Find the newline at or after the nominal end position
                let nominal_end = (i + 1) * chunk_size;
                find_next_newline(&mmap, nominal_end, total_size).unwrap_or(total_size)
            };

            if actual_start >= actual_end {
                // Empty chunk, return empty batch
                let columns = fields_builder.finish();
                return RecordBatch::try_new(schema.clone(), columns)
                    .map_err(|e| Error::msg(e.to_string()));
            }

            let section = &mmap[actual_start..actual_end];

            section.lines().map_while(Result::ok).for_each(|line| {
                let values = scanner.scan_with(&line, field_names);
                if let Some(values) = values {
                    fields_builder.push(field_types, &values);
                };
            });

            let columns = fields_builder.finish();
            RecordBatch::try_new(schema.clone(), columns).map_err(|e| Error::msg(e.to_string()))
        })
        .collect();

    partitions
}

/// Helper to find the index of the next newline character
fn find_next_newline(mmap: &[u8], start: usize, end: usize) -> Option<usize> {
    mmap[start..end]
        .iter()
        .position(|&b| b == b'\n')
        .map(|pos| start + pos + 1) // +1 to include the newline itself
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::datafusion::LogTableProvider;
    use crate::scanner::Scanner;
    use datafusion::arrow::datatypes::DataType;
    use datafusion::prelude::SessionContext;

    #[tokio::test]
    async fn test_log_table_provider() {
        let ctx = SessionContext::new();

        let pattern = r"^\[(?P<time>\w{3} \w{3} \d{1,2} \d{2}:\d{2}:\d{2} \d{4})\] \[(?P<level>[^\]]+)\] (?P<message>.*)$";
        let scanner = Scanner::new(pattern.to_string());
        let log_table = LogTableProvider {
            scanner,
            file_path: String::from("loghub/Apache/Apache_2k.log"),
        };

        let _ = ctx.register_table("log", Arc::new(log_table));
        let df = ctx.sql("SELECT * FROM log").await.unwrap();
        let _ = df.show_limit(10).await;
    }

    #[tokio::test]
    async fn test_log_table_provider_with_macros() {
        let ctx = SessionContext::new();

        let pattern = r#"^\[{{time:datetime("%a %b %d %H:%M:%S %Y")}}\] \[{{level:var_name}}\] {{message:any}}$"#;
        let scanner = Scanner::new(pattern.to_string());
        let log_table = LogTableProvider {
            scanner,
            file_path: String::from("loghub/Apache/Apache_2k.log"),
        };

        let _ = ctx.register_table("log_mac", Arc::new(log_table));
        let df = ctx
            .sql("SELECT time, level, message FROM log_mac LIMIT 5")
            .await
            .unwrap();
        let _ = df.show_limit(5).await;
    }

    #[tokio::test]
    async fn test_log_table_provider_with_int_fields() {
        let ctx = SessionContext::new();

        // Pattern for jk2_init() messages: "jk2_init() Found child 6725 in scoreboard slot 10"
        let pattern = r#"^\[{{time:datetime("%a %b %d %H:%M:%S %Y")}}\] \[{{level:var_name}}\] jk2_init\(\) Found child {{child_pid:number}} in scoreboard slot {{slot:number}}$"#;
        let scanner = Scanner::new(pattern.to_string());
        let log_table = LogTableProvider {
            scanner,
            file_path: String::from("loghub/Apache/Apache_2k.log"),
        };

        let _ = ctx.register_table("log_int", Arc::new(log_table));

        // Test that child_pid and slot are Int32 columns
        let df = ctx
            .sql("SELECT time, level, child_pid, slot FROM log_int LIMIT 10")
            .await
            .unwrap();

        let results = df.collect().await.unwrap();
        assert!(
            !results.is_empty(),
            "Should have matched some jk2_init() lines"
        );

        // Verify schema has Int32 columns
        let schema = results[0].schema();
        let child_pid_field = schema.field_with_name("child_pid").unwrap();
        let slot_field = schema.field_with_name("slot").unwrap();
        assert_eq!(child_pid_field.data_type(), &DataType::Int32);
        assert_eq!(slot_field.data_type(), &DataType::Int32);
    }

    /// Tests that column projection works correctly when only a subset of columns is selected.
    ///
    /// This test verifies the fix for the projection bug where selecting fewer columns than
    /// defined in the pattern caused a schema mismatch. The fix ensures that:
    /// 1. Only the projected columns are scanned and returned
    /// 2. The schema matches the selected columns
    /// 3. The record batch has the correct number of columns
    #[tokio::test]
    async fn test_log_table_projection() {
        let ctx = SessionContext::new();

        // Pattern with 3 fields: time, level, message
        let pattern = r#"^\[{{time:datetime("%a %b %d %H:%M:%S %Y")}}\] \[{{level:var_name}}\] {{message:any}}$"#;
        let scanner = Scanner::new(pattern.to_string());

        // Verify scanner has 3 fields
        assert_eq!(scanner.field_names.len(), 3);
        assert_eq!(scanner.field_names, vec!["time", "level", "message"]);

        let log_table = LogTableProvider {
            scanner,
            file_path: String::from("loghub/Apache/Apache_2k.log"),
        };

        let _ = ctx.register_table("log_proj", Arc::new(log_table));

        // Test 1: Select only 2 columns out of 3
        let df = ctx
            .sql("SELECT time, level FROM log_proj LIMIT 5")
            .await
            .unwrap();
        let results = df.collect().await.unwrap();

        assert!(!results.is_empty(), "Should have results");
        let schema = results[0].schema();
        assert_eq!(
            schema.fields().len(),
            2,
            "Should have exactly 2 columns in projection"
        );
        assert_eq!(schema.field(0).name(), "time");
        assert_eq!(schema.field(1).name(), "level");

        // Test 2: Select only 1 column
        let df = ctx
            .sql("SELECT message FROM log_proj LIMIT 5")
            .await
            .unwrap();
        let results = df.collect().await.unwrap();

        assert!(!results.is_empty(), "Should have results");
        let schema = results[0].schema();
        assert_eq!(
            schema.fields().len(),
            1,
            "Should have exactly 1 column in projection"
        );
        assert_eq!(schema.field(0).name(), "message");

        // Test 3: Select columns in different order than pattern definition
        let df = ctx
            .sql("SELECT message, time FROM log_proj LIMIT 5")
            .await
            .unwrap();
        let results = df.collect().await.unwrap();

        assert!(!results.is_empty(), "Should have results");
        let schema = results[0].schema();
        assert_eq!(schema.fields().len(), 2);
        assert_eq!(schema.field(0).name(), "message");
        assert_eq!(schema.field(1).name(), "time");
    }

    /// Tests that parsing works correctly with very small files (fewer lines than thread count).
    /// This verifies that empty chunks are handled gracefully.
    #[tokio::test]
    async fn test_log_table_small_file() {
        use std::io::Write;
        use tempfile::NamedTempFile;

        // Create a small temp file with just 3 lines
        let mut temp_file = NamedTempFile::new().unwrap();
        writeln!(temp_file, "[Sun Dec 04 04:47:44 2005] [notice] Line 1").unwrap();
        writeln!(temp_file, "[Sun Dec 04 04:47:45 2005] [error] Line 2").unwrap();
        writeln!(temp_file, "[Sun Dec 04 04:47:46 2005] [notice] Line 3").unwrap();
        temp_file.flush().unwrap();

        let ctx = SessionContext::new();

        let pattern = r#"^\[{{time:datetime("%a %b %d %H:%M:%S %Y")}}\] \[{{level:var_name}}\] {{message:any}}$"#;
        let scanner = Scanner::new(pattern.to_string());
        let log_table = LogTableProvider {
            scanner,
            file_path: temp_file.path().to_string_lossy().to_string(),
        };

        let _ = ctx.register_table("log_small", Arc::new(log_table));
        let df = ctx
            .sql("SELECT time, level, message FROM log_small")
            .await
            .unwrap();
        let results = df.collect().await.unwrap();

        // Count total rows across all batches
        let total_rows: usize = results.iter().map(|b| b.num_rows()).sum();
        assert_eq!(total_rows, 3, "Should have exactly 3 rows");
    }

    /// Tests that proper error handling occurs when the log file doesn't exist.
    #[tokio::test]
    async fn test_log_table_file_not_found() {
        let ctx = SessionContext::new();

        let pattern = r#"^\[{{time:datetime("%a %b %d %H:%M:%S %Y")}}\] \[{{level:var_name}}\] {{message:any}}$"#;
        let scanner = Scanner::new(pattern.to_string());
        let log_table = LogTableProvider {
            scanner,
            file_path: String::from("/nonexistent/path/to/file.log"),
        };

        let _ = ctx.register_table("log_missing", Arc::new(log_table));
        let df = ctx.sql("SELECT * FROM log_missing").await.unwrap();
        let result = df.collect().await;

        // Should return an error, not panic
        assert!(result.is_err(), "Should return error for missing file");
    }
}
