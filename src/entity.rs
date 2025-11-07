//! Core entity trait and types for format-agnostic data entities.
//!
//! This module provides the fundamental abstractions for defining entities
//! that can be derived from any structured data format (CSV, JSON, XML, etc.).

use serde::{Serialize, Deserialize};
use std::collections::HashMap;
use std::fmt;

/// Represents different types of field values in an entity
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(untagged)]
pub enum FieldValue {
    String(String),
    Int(i64),
    Float(f64),
    Bool(bool),
    List(Vec<FieldValue>),
    Null,
}

impl fmt::Display for FieldValue {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            FieldValue::String(s) => write!(f, "{}", s),
            FieldValue::Int(i) => write!(f, "{}", i),
            FieldValue::Float(fl) => write!(f, "{}", fl),
            FieldValue::Bool(b) => write!(f, "{}", b),
            FieldValue::List(l) => write!(f, "{:?}", l),
            FieldValue::Null => write!(f, "null"),
        }
    }
}

/// Error type for entity operations
#[derive(Debug, Clone)]
pub enum EntityError {
    ParseError(String),
    ValidationError(String),
    TransformError(String),
    SourceTypeMismatch {
        expected: &'static str,
        actual: String,
    },
    RequiredFieldMissing {
        field: String,
    },
    ExtractionFailed {
        field: String,
        reason: String,
    },
    InvalidFieldValue(String),
    ContextFieldMissing {
        field: String,
    },
}

impl fmt::Display for EntityError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            EntityError::ParseError(msg) => write!(f, "Parse error: {}", msg),
            EntityError::ValidationError(msg) => write!(f, "Validation error: {}", msg),
            EntityError::TransformError(msg) => write!(f, "Transform error: {}", msg),
            EntityError::SourceTypeMismatch { expected, actual } => {
                write!(f, "Source type mismatch: expected {}, got {}", expected, actual)
            }
            EntityError::RequiredFieldMissing { field } => {
                write!(f, "Required field '{}' is missing or empty", field)
            }
            EntityError::ExtractionFailed { field, reason } => {
                write!(f, "Failed to extract field '{}': {}", field, reason)
            }
            EntityError::InvalidFieldValue(msg) => {
                write!(f, "Invalid field value: {}", msg)
            }
            EntityError::ContextFieldMissing { field } => {
                write!(f, "Context field '{}' not found in context", field)
            }
        }
    }
}

impl std::error::Error for EntityError {}

/// Core trait for all entities in the nomnom framework.
///
/// This trait is format-agnostic and can be implemented for entities
/// derived from any structured data source.
///
/// # Example
///
/// ```ignore
/// use nomnom::Entity;
/// use serde::Serialize;
/// use std::collections::HashMap;
///
/// #[derive(Serialize)]
/// struct CsvRow {
///     first_name: String,
///     last_name: String,
/// }
///
/// impl Entity for CsvRow {
///     const NAME: &'static str = "CsvRow";
/// }
/// ```
pub trait Entity: Serialize + Sized {
    /// The name of this entity type
    const NAME: &'static str;

    /// Convert entity to a dictionary representation
    fn to_dict(&self) -> HashMap<String, FieldValue> {
        // Default implementation using serde_json
        let json_value = serde_json::to_value(self)
            .expect("Failed to serialize entity");

        if let serde_json::Value::Object(map) = json_value {
            map.into_iter()
                .map(|(k, v)| (k, json_value_to_field_value(v)))
                .collect()
        } else {
            HashMap::new()
        }
    }

    /// Convert entity to JSON string
    fn to_json(&self) -> Result<String, serde_json::Error> {
        serde_json::to_string(self)
    }

    /// Convert entity to pretty-printed JSON string
    fn to_json_pretty(&self) -> Result<String, serde_json::Error> {
        serde_json::to_string_pretty(self)
    }

    /// Convert entity to NDJSON line (newline-delimited JSON)
    fn to_ndjson_line(&self) -> Result<String, serde_json::Error> {
        let json = self.to_json()?;
        Ok(format!("{}\n", json))
    }
}

/// Helper function to convert serde_json::Value to FieldValue
fn json_value_to_field_value(value: serde_json::Value) -> FieldValue {
    match value {
        serde_json::Value::String(s) => FieldValue::String(s),
        serde_json::Value::Number(n) => {
            if let Some(i) = n.as_i64() {
                FieldValue::Int(i)
            } else if let Some(f) = n.as_f64() {
                FieldValue::Float(f)
            } else {
                FieldValue::Null
            }
        }
        serde_json::Value::Bool(b) => FieldValue::Bool(b),
        serde_json::Value::Array(arr) => {
            FieldValue::List(arr.into_iter().map(json_value_to_field_value).collect())
        }
        serde_json::Value::Null => FieldValue::Null,
        serde_json::Value::Object(_) => {
            // For nested objects, serialize to string
            FieldValue::String(value.to_string())
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[derive(Serialize)]
    struct TestEntity {
        name: String,
        age: i32,
    }

    impl Entity for TestEntity {
        const NAME: &'static str = "TestEntity";
    }

    #[test]
    fn test_entity_to_json() {
        let entity = TestEntity {
            name: "Alice".to_string(),
            age: 30,
        };

        let json = entity.to_json().unwrap();
        assert!(json.contains("Alice"));
        assert!(json.contains("30"));
    }

    #[test]
    fn test_entity_to_dict() {
        let entity = TestEntity {
            name: "Bob".to_string(),
            age: 25,
        };

        let dict = entity.to_dict();
        assert_eq!(dict.get("name"), Some(&FieldValue::String("Bob".to_string())));
        assert_eq!(dict.get("age"), Some(&FieldValue::Int(25)));
    }
}

/// Context for additional fields not in the data source.
///
/// Provides a key-value store for fields that come from external sources
/// (e.g., filename, batch ID, facility code).
#[derive(Debug, Clone, Default)]
pub struct Context {
    values: HashMap<String, String>,
}

impl Context {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn with_value(mut self, key: impl Into<String>, value: impl Into<String>) -> Self {
        self.values.insert(key.into(), value.into());
        self
    }

    pub fn get(&self, key: &str) -> Option<&str> {
        self.values.get(key).map(|s| s.as_str())
    }

    pub fn from_hashmap(values: HashMap<String, String>) -> Self {
        Self { values }
    }
}

/// Trait to convert field values to Option<String> for transforms.
pub trait IntoOptionString {
    fn into_option_string(self) -> Option<String>;
}

impl IntoOptionString for String {
    fn into_option_string(self) -> Option<String> {
        Some(self)
    }
}

impl IntoOptionString for Option<String> {
    fn into_option_string(self) -> Option<String> {
        self
    }
}
