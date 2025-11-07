//! Transform registry trait and types.
//!
//! Defines the interface that transform registries must implement for use
//! with nomnom-generated code.

use std::collections::HashMap;
use serde_json::Value;

/// Transform registry trait for field extraction.
///
/// Generated entity code calls `registry.call_transform(name, args)` to execute
/// transform functions. Implementations must provide this method.
///
/// # Example
///
/// ```rust
/// use nomnom::runtime::TransformRegistry;
/// use std::collections::HashMap;
/// use serde_json::Value;
///
/// pub struct MyRegistry;
///
/// impl TransformRegistry for MyRegistry {
///     fn call_transform(
///         &self,
///         name: &str,
///         args: &HashMap<String, Value>,
///     ) -> Result<Value, String> {
///         match name {
///             "uppercase" => {
///                 let input = args.get("input")
///                     .and_then(|v| v.as_str())
///                     .ok_or("Missing 'input' argument")?;
///                 Ok(Value::String(input.to_uppercase()))
///             }
///             _ => Err(format!("Unknown transform: {}", name))
///         }
///     }
/// }
/// ```
pub trait TransformRegistry {
    /// Call a transform function by name with arguments.
    ///
    /// # Arguments
    ///
    /// * `name` - Transform function name (e.g., "extract_from_hl7_segment")
    /// * `args` - Arguments as JSON Value HashMap
    ///
    /// # Returns
    ///
    /// * `Ok(Value)` - Transform succeeded, returns JSON Value (String, Array, Null, etc.)
    /// * `Err(String)` - Transform failed with error message
    fn call_transform(
        &self,
        name: &str,
        args: &HashMap<String, Value>,
    ) -> Result<Value, String>;
}
