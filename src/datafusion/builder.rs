//! Arrow array builder for log fields.

use datafusion::arrow::array::{
    ArrayBuilder, ArrayRef, Float64Builder, Int32Builder, StringBuilder,
};

use crate::types::FieldType;

/// Builds Arrow arrays from parsed log field values.
pub struct FieldsBuilder {
    builders: Vec<Box<dyn ArrayBuilder>>,
}

impl FieldsBuilder {
    /// Create a new FieldsBuilder with the appropriate builder for each field type.
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

    /// Push a row of values into the builders.
    ///
    /// Accepts string slices (`&str`) to avoid intermediate allocations.
    /// For Int and Float types, parsing errors result in null values.
    pub fn push(&mut self, field_types: &[&FieldType], values: &[&str]) {
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

    /// Finish building and return the Arrow arrays.
    pub fn finish(&mut self) -> Vec<ArrayRef> {
        self.builders.iter_mut().map(|b| b.finish()).collect()
    }
}
