//! Transform registry system for registering and calling transformation functions.
//!
//! This module provides a plugin architecture that allows domain-specific transformation
//! functions to be registered and called during entity field extraction.

use std::collections::HashMap;
use std::fmt;
use serde_json::Value;

/// Error type for transform operations
#[derive(Debug, Clone)]
pub enum TransformError {
    NotFound(String),
    InvalidArgs(String),
    ExecutionError(String),
}

impl fmt::Display for TransformError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            TransformError::NotFound(name) => write!(f, "Transform not found: {}", name),
            TransformError::InvalidArgs(msg) => write!(f, "Invalid arguments: {}", msg),
            TransformError::ExecutionError(msg) => write!(f, "Execution error: {}", msg),
        }
    }
}

impl std::error::Error for TransformError {}

/// Trait for transformation functions
///
/// Transform functions can return any JSON Value (String, Number, Bool, Array, Null).
/// This allows transforms to return lists (for repeated fields) or other complex types.
pub trait TransformFn: Send + Sync {
    /// Execute the transformation with given arguments
    ///
    /// # Returns
    ///
    /// * `Ok(Value::String(s))` - String value
    /// * `Ok(Value::Array(list))` - List of values (for repeated fields)
    /// * `Ok(Value::Null)` - No value / null
    /// * `Err(TransformError)` - Execution failed
    fn execute(&self, args: &HashMap<String, Value>) -> Result<Value, TransformError>;
}

/// Simple function-based implementation of TransformFn
impl<F> TransformFn for F
where
    F: Fn(&HashMap<String, Value>) -> Result<Value, TransformError> + Send + Sync,
{
    fn execute(&self, args: &HashMap<String, Value>) -> Result<Value, TransformError> {
        self(args)
    }
}

/// Registry for storing and calling transformation functions
pub struct TransformRegistry {
    transforms: HashMap<String, Box<dyn TransformFn>>,
}

impl TransformRegistry {
    /// Create a new empty transform registry
    pub fn new() -> Self {
        Self {
            transforms: HashMap::new(),
        }
    }

    /// Register a transformation function
    ///
    /// # Example
    ///
    /// ```ignore
    /// use nomnom::TransformRegistry;
    ///
    /// let mut registry = TransformRegistry::new();
    /// registry.register("uppercase", Box::new(|args| {
    ///     let text = args.get("text")
    ///         .and_then(|v| v.as_str())
    ///         .ok_or_else(|| TransformError::InvalidArgs("Missing 'text'".to_string()))?;
    ///     Ok(Some(text.to_uppercase()))
    /// }));
    /// ```
    pub fn register(&mut self, name: impl Into<String>, func: Box<dyn TransformFn>) {
        self.transforms.insert(name.into(), func);
    }

    /// Call a registered transformation function
    ///
    /// # Arguments
    ///
    /// * `name` - Name of the registered transform
    /// * `args` - Arguments to pass to the transform
    ///
    /// # Returns
    ///
    /// * `Ok(Value)` - Transform succeeded, returns JSON Value (String, Array, Null, etc.)
    /// * `Err(TransformError)` - Transform failed
    pub fn call(
        &self,
        name: &str,
        args: &HashMap<String, Value>,
    ) -> Result<Value, TransformError> {
        let transform = self
            .transforms
            .get(name)
            .ok_or_else(|| TransformError::NotFound(name.to_string()))?;

        transform.execute(args)
    }

    /// Check if a transform is registered
    pub fn has_transform(&self, name: &str) -> bool {
        self.transforms.contains_key(name)
    }

    /// Get list of all registered transform names
    pub fn list_transforms(&self) -> Vec<String> {
        self.transforms.keys().cloned().collect()
    }
}

impl Default for TransformRegistry {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_register_and_call_transform() {
        let mut registry = TransformRegistry::new();

        registry.register("uppercase", Box::new(|args: &HashMap<String, Value>| {
            let text = args
                .get("text")
                .and_then(|v| v.as_str())
                .ok_or_else(|| TransformError::InvalidArgs("Missing 'text'".to_string()))?;
            Ok(Value::String(text.to_uppercase()))
        }) as Box<dyn TransformFn>);

        let mut args = HashMap::new();
        args.insert("text".to_string(), Value::String("hello".to_string()));

        let result = registry.call("uppercase", &args).unwrap();
        assert_eq!(result, Value::String("HELLO".to_string()));
    }

    #[test]
    fn test_transform_not_found() {
        let registry = TransformRegistry::new();

        let args = HashMap::new();
        let result = registry.call("nonexistent", &args);

        assert!(matches!(result, Err(TransformError::NotFound(_))));
    }

    #[test]
    fn test_has_transform() {
        let mut registry = TransformRegistry::new();

        registry.register("test_fn", Box::new(|_args: &HashMap<String, Value>| {
            Ok(Value::String("test".to_string()))
        }) as Box<dyn TransformFn>);

        assert!(registry.has_transform("test_fn"));
        assert!(!registry.has_transform("other_fn"));
    }
}
