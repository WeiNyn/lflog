//! LogTableProvider implementation for DataFusion.

use async_trait::async_trait;
use datafusion::arrow::datatypes::{DataType, Field, Schema, SchemaRef};
use datafusion::catalog::{Session, TableProvider};
use datafusion::common::Result;
use datafusion::logical_expr::{Expr, TableType};
use datafusion::physical_plan::ExecutionPlan;
use std::any::Any;
use std::sync::Arc;

use crate::datafusion::exec::LogTableExec;
use crate::scanner::Scanner;
use crate::types::FieldType;

/// A DataFusion TableProvider that reads and parses log files.
#[derive(Debug, Clone)]
pub struct LogTableProvider {
    pub scanner: Scanner,
    pub file_path: String,
}

impl LogTableProvider {
    /// Create a new LogTableProvider.
    pub fn new(scanner: Scanner, file_path: String) -> Self {
        Self { scanner, file_path }
    }

    /// Create a physical execution plan with optional projections.
    pub fn create_physical_plan(
        &self,
        projections: Option<&Vec<usize>>,
        schema: SchemaRef,
    ) -> Result<Arc<dyn ExecutionPlan>> {
        Ok(Arc::new(LogTableExec::new(
            projections,
            schema,
            self.clone(),
        )))
    }
}

#[async_trait]
impl TableProvider for LogTableProvider {
    fn as_any(&self) -> &dyn Any {
        self
    }

    /// Generate schema dynamically from the scanner's field names and type hints.
    fn schema(&self) -> SchemaRef {
        let fields: Vec<Field> = self
            .scanner
            .field_names
            .iter()
            .map(|name| {
                let data_type = match self.scanner.type_hints.get(name) {
                    Some(FieldType::Int) => DataType::Int32,
                    Some(FieldType::Float) => DataType::Float64,
                    _ => DataType::Utf8,
                };
                Field::new(name, data_type, true)
            })
            .collect();
        SchemaRef::new(Schema::new(fields))
    }

    fn table_type(&self) -> TableType {
        TableType::Base
    }

    async fn scan(
        &self,
        _state: &dyn Session,
        projection: Option<&Vec<usize>>,
        _filters: &[Expr],
        _limit: Option<usize>,
    ) -> Result<Arc<dyn ExecutionPlan>> {
        self.create_physical_plan(projection, self.schema())
    }
}
