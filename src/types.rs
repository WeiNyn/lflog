use serde::{Deserialize, Serialize};

/// Represents the type of a field extracted from log patterns.
/// Used for type hints that determine Arrow column types.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub enum FieldType {
    String,
    Int,
    Float,
    DateTime,
    Enum,
    Json,
}
