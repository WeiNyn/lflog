use async_trait::async_trait;
use datafusion::arrow::datatypes::{DataType, Field, Schema, SchemaRef};
use datafusion::arrow::record_batch::RecordBatch;
use datafusion::catalog::{Session, TableProvider};
use datafusion::common::Result;
use datafusion::physical_expr::EquivalenceProperties;
use datafusion::physical_plan::execution_plan::{Boundedness, EmissionType};
use datafusion::physical_plan::memory::MemoryStream;
use datafusion::physical_plan::{DisplayAs, ExecutionPlan, Partitioning, PlanProperties};
use datafusion_common::project_schema;
use datafusion_expr::{Expr, TableType};
use std::any::Any;
use std::sync::Arc;

use std::{
    fs::File,
    io::{BufRead, BufReader},
};

use datafusion::arrow::array::{
    ArrayBuilder, ArrayRef, Float64Builder, Int32Builder, StringBuilder,
};

use crate::profile::{FieldType, Scanner};

pub struct FieldsBuilder {
    builders: Vec<Box<dyn ArrayBuilder>>,
}

impl FieldsBuilder {
    pub fn new(fields: &[&FieldType]) -> Self {
        let builders = fields
            .iter()
            .map(|field| match field {
                FieldType::String => Box::new(StringBuilder::new()) as Box<dyn ArrayBuilder>,
                FieldType::Int => Box::new(Int32Builder::new()) as Box<dyn ArrayBuilder>,
                FieldType::Float => Box::new(Float64Builder::new()) as Box<dyn ArrayBuilder>,
                FieldType::DateTime => Box::new(StringBuilder::new()) as Box<dyn ArrayBuilder>,
                FieldType::Enum => Box::new(StringBuilder::new()) as Box<dyn ArrayBuilder>,
                FieldType::Json => Box::new(StringBuilder::new()) as Box<dyn ArrayBuilder>,
            })
            .collect();
        Self { builders }
    }

    pub fn push(&mut self, field_types: &[&FieldType], values: &[String]) {
        for ((builder, field_type), value) in self.builders.iter_mut().zip(field_types).zip(values)
        {
            match field_type {
                FieldType::String | FieldType::DateTime | FieldType::Enum | FieldType::Json => {
                    builder
                        .as_any_mut()
                        .downcast_mut::<StringBuilder>()
                        .unwrap()
                        .append_value(value);
                }
                FieldType::Int => {
                    let int_builder = builder.as_any_mut().downcast_mut::<Int32Builder>().unwrap();
                    match value.parse::<i32>() {
                        Ok(i) => int_builder.append_value(i),
                        Err(_) => int_builder.append_null(),
                    }
                }
                FieldType::Float => {
                    let float_builder = builder
                        .as_any_mut()
                        .downcast_mut::<Float64Builder>()
                        .unwrap();
                    match value.parse::<f64>() {
                        Ok(f) => float_builder.append_value(f),
                        Err(_) => float_builder.append_null(),
                    }
                }
            }
        }
    }

    pub fn finish(&mut self) -> Vec<ArrayRef> {
        self.builders
            .iter_mut()
            .map(|builder| builder.finish())
            .collect()
    }
}

#[derive(Debug, Clone)]
pub struct LogTableProvider {
    scanner: Scanner,
    file_path: String,
}

impl LogTableProvider {
    pub fn new(scanner: Scanner, file_path: String) -> Self {
        Self { scanner, file_path }
    }

    pub async fn create_physical_plan(
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

#[derive(Debug)]
pub struct LogTableExec {
    provider: LogTableProvider,
    projected_schema: SchemaRef,
    properties: PlanProperties,
}

impl LogTableExec {
    pub fn new(
        projections: Option<&Vec<usize>>,
        schema: SchemaRef,
        provider: LogTableProvider,
    ) -> Self {
        let projected_schema = project_schema(&schema, projections).unwrap();
        let properties = PlanProperties::new(
            EquivalenceProperties::new(projected_schema.clone()),
            Partitioning::UnknownPartitioning(10),
            EmissionType::Incremental,
            Boundedness::Bounded,
        );
        Self {
            provider,
            projected_schema,
            properties,
        }
    }
}

impl DisplayAs for LogTableExec {
    fn fmt_as(
        &self,
        _t: datafusion::physical_plan::DisplayFormatType,
        _f: &mut std::fmt::Formatter,
    ) -> std::fmt::Result {
        todo!()
    }
}

impl ExecutionPlan for LogTableExec {
    fn name(&self) -> &str {
        "LogTableExec"
    }

    fn as_any(&self) -> &dyn std::any::Any {
        self
    }

    fn properties(&self) -> &datafusion::physical_plan::PlanProperties {
        &self.properties
    }

    fn children(&self) -> Vec<&std::sync::Arc<dyn ExecutionPlan>> {
        Vec::new()
    }

    fn with_new_children(
        self: std::sync::Arc<Self>,
        _children: Vec<std::sync::Arc<dyn ExecutionPlan>>,
    ) -> datafusion_common::Result<std::sync::Arc<dyn ExecutionPlan>> {
        Ok(self)
    }

    fn execute(
        &self,
        _partition: usize,
        _context: std::sync::Arc<datafusion::execution::TaskContext>,
    ) -> datafusion_common::Result<datafusion::execution::SendableRecordBatchStream> {
        // Get field types in the same order as field_names, defaulting to String
        let default_string = FieldType::String;
        let field_types: Vec<&FieldType> = self
            .provider
            .scanner
            .field_names
            .iter()
            .map(|name| {
                self.provider
                    .scanner
                    .type_hints
                    .get(name)
                    .unwrap_or(&default_string)
            })
            .collect();
        let mut fields_builder = FieldsBuilder::new(&field_types);

        let file = File::open(self.provider.file_path.clone()).unwrap();
        let reader = BufReader::new(file);

        for line in reader.lines() {
            let line = line?;
            let fields = self.provider.scanner.scan(&line);

            if let Some(fields) = fields {
                fields_builder.push(&field_types, &fields);
            }
        }

        let columns = fields_builder.finish();

        let partitions = RecordBatch::try_new(self.projected_schema.clone(), columns).unwrap();

        Ok(Box::pin(MemoryStream::try_new(
            vec![partitions],
            self.schema(),
            None,
        )?))
    }
}

#[async_trait]
impl TableProvider for LogTableProvider {
    #[doc = " Returns the table provider as [`Any`] so that it can be"]
    #[doc = " downcast to a specific implementation."]
    fn as_any(&self) -> &dyn Any {
        self
    }

    #[doc = " Get a reference to the schema for this table"]
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

    #[doc = " Get the type of this table for metadata/catalog purposes."]
    fn table_type(&self) -> TableType {
        TableType::Base
    }

    async fn scan(
        &self,
        _state: &dyn Session,
        projection: Option<&Vec<usize>>,
        // filters and limit can be used here to inject some push-down operations if needed
        _filters: &[Expr],
        _limit: Option<usize>,
    ) -> Result<Arc<dyn ExecutionPlan>> {
        return self.create_physical_plan(projection, self.schema()).await;
    }
}

#[cfg(test)]
pub mod tests {
    use super::*;
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
}
