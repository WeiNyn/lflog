use serde::{Deserialize, Serialize};

/// Represents the type of a field extracted from log patterns.
/// Used for type hints that determine Arrow column types.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub enum FieldType {
    String,
    Int,
    Float,
    DateTime(DateTime),
    Enum,
    Json,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct DateTime {
    pub formats: Option<Vec<String>>,
}

impl DateTime {
    pub fn new(formats: Option<Vec<String>>) -> Self {
        DateTime { formats }
    }

    pub fn parse(&self, value: &str) -> Option<i64> {
        if let Some(formats) = &self.formats {
            for format in formats {
                if let Ok(parsed) = chrono::NaiveDateTime::parse_from_str(value, format) {
                    return Some(parsed.and_utc().timestamp_micros());
                }
            }
        }
        None
    }
}
