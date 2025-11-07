//! Python bridge for calling Python transformation functions from Rust.
//!
//! This module is only available when the `python-bridge` feature is enabled.
//!
//! # Feature Gate
//!
//! ```toml
//! [dependencies]
//! nomnom = { version = "0.1", features = ["python-bridge"] }
//! ```

use crate::transform_registry::{TransformError, TransformFn};
use pyo3::prelude::*;
use pyo3::types::{PyDict, PyList, PyModule};
use serde_json::Value;
use std::collections::HashMap;
use std::sync::Mutex;

/// Python transform function wrapper
///
/// Calls Python functions from Rust transform registry.
/// Supports both single-value and list-valued transforms.
pub struct PythonTransform {
    module_name: String,
    function_name: String,
    /// Cached Python interpreter state
    /// We use Mutex for thread-safe access
    py: Mutex<()>,
}

impl PythonTransform {
    /// Create a new Python transform
    ///
    /// # Arguments
    ///
    /// * `module_name` - Python module path (e.g., "myapp.transforms")
    /// * `function_name` - Function name within the module
    pub fn new(module_name: impl Into<String>, function_name: impl Into<String>) -> Self {
        Self {
            module_name: module_name.into(),
            function_name: function_name.into(),
            py: Mutex::new(()),
        }
    }

    /// Call the Python function with given arguments
    fn call_python(&self, args: &HashMap<String, Value>) -> Result<Value, TransformError> {
        let _guard = self.py.lock().unwrap();

        Python::with_gil(|py| {
            // Import the module
            let module = PyModule::import(py, self.module_name.as_str()).map_err(|e| {
                TransformError::ExecutionError(format!(
                    "Failed to import module '{}': {}",
                    self.module_name, e
                ))
            })?;

            // Get the function
            let func = module.getattr(&*self.function_name).map_err(|e| {
                TransformError::ExecutionError(format!(
                    "Failed to get function '{}': {}",
                    self.function_name, e
                ))
            })?;

            // Convert args to Python dict
            let py_args = PyDict::new(py);
            for (key, value) in args {
                let py_value = json_value_to_py(py, value)?;
                py_args.set_item(key, py_value).map_err(|e| {
                    TransformError::ExecutionError(format!("Failed to set arg '{}': {}", key, e))
                })?;
            }

            // Call the function
            let result = func.call((), Some(py_args)).map_err(|e| {
                TransformError::ExecutionError(format!("Python function call failed: {}", e))
            })?;

            // Convert result back to Rust Value
            py_to_json_value(py, result)
        })
    }
}

impl TransformFn for PythonTransform {
    fn execute(&self, args: &HashMap<String, Value>) -> Result<Value, TransformError> {
        self.call_python(args)
    }
}

/// Convert serde_json::Value to PyObject
fn json_value_to_py(py: Python, value: &Value) -> Result<PyObject, TransformError> {
    match value {
        Value::Null => Ok(py.None()),
        Value::Bool(b) => Ok(b.into_py(py)),
        Value::Number(n) => {
            if let Some(i) = n.as_i64() {
                Ok(i.into_py(py))
            } else if let Some(f) = n.as_f64() {
                Ok(f.into_py(py))
            } else {
                Err(TransformError::InvalidArgs(format!(
                    "Unsupported number: {}",
                    n
                )))
            }
        }
        Value::String(s) => Ok(s.into_py(py)),
        Value::Array(arr) => {
            let py_list: Result<Vec<PyObject>, TransformError> =
                arr.iter().map(|v| json_value_to_py(py, v)).collect();
            Ok(py_list?.into_py(py))
        }
        Value::Object(obj) => {
            let py_dict = PyDict::new(py);
            for (k, v) in obj {
                let py_value = json_value_to_py(py, v)?;
                py_dict
                    .set_item(k, py_value)
                    .map_err(|e| TransformError::ExecutionError(e.to_string()))?;
            }
            Ok(py_dict.into())
        }
    }
}

/// Convert PyObject to serde_json::Value
///
/// Handles None, String, List (for repeated fields), and other types
fn py_to_json_value(py: Python, obj: &PyAny) -> Result<Value, TransformError> {
    // Handle None
    if obj.is_none() {
        return Ok(Value::Null);
    }

    // Try to extract as list first (for repeated fields)
    if let Ok(list) = obj.downcast::<PyList>() {
        let values: Result<Vec<String>, _> = list.iter()
            .map(|item| {
                if item.is_none() {
                    Ok(String::new())
                } else {
                    item.extract::<String>()
                }
            })
            .collect();

        match values {
            Ok(strings) => {
                let json_values: Vec<Value> = strings.into_iter()
                    .map(Value::String)
                    .collect();
                return Ok(Value::Array(json_values));
            }
            Err(_) => {
                // List extraction failed, continue to try other types
            }
        }
    }

    // Try to extract as string
    if let Ok(s) = obj.extract::<String>() {
        return Ok(Value::String(s));
    }

    // Try to extract as bool
    if let Ok(b) = obj.extract::<bool>() {
        return Ok(Value::Bool(b));
    }

    // Try to extract as i64
    if let Ok(i) = obj.extract::<i64>() {
        return Ok(Value::Number(i.into()));
    }

    // Try to extract as f64
    if let Ok(f) = obj.extract::<f64>() {
        if let Some(num) = serde_json::Number::from_f64(f) {
            return Ok(Value::Number(num));
        }
    }

    // Fallback: convert to string representation
    match obj.str() {
        Ok(s) => Ok(Value::String(s.to_string())),
        Err(e) => Err(TransformError::ExecutionError(format!(
            "Failed to convert Python object to JSON Value: {}",
            e
        )))
    }
}

/// Python Transform Registry
///
/// Provides a registry-style interface for calling Python transform functions.
/// This wraps a Python module's TRANSFORM_REGISTRY dictionary.
#[derive(Clone)]
pub struct PyTransformRegistry {
    /// Python transforms module name (e.g., "myapp.transforms")
    transforms_module_name: String,
}

impl PyTransformRegistry {
    /// Create a new registry for a Python transforms module
    ///
    /// # Arguments
    ///
    /// * `transforms_module_name` - Python module path (e.g., "data_processor.transforms")
    pub fn new(transforms_module_name: impl Into<String>) -> Self {
        Self {
            transforms_module_name: transforms_module_name.into(),
        }
    }

    /// Call a transform function with keyword arguments
    ///
    /// # Arguments
    ///
    /// * `py` - Python GIL token
    /// * `name` - Transform function name
    /// * `kwargs` - Keyword arguments (String keys, Option<String> values)
    ///
    /// # Returns
    ///
    /// * `Ok(Some(String))` - Transform returned a string value
    /// * `Ok(None)` - Transform returned None
    /// * `Err(PyErr)` - Transform failed
    pub fn call_transform(
        &self,
        py: Python,
        name: &str,
        kwargs: HashMap<String, Option<String>>,
    ) -> PyResult<Option<String>> {
        let transforms_module = PyModule::import(py, self.transforms_module_name.as_str())?;
        let registry = transforms_module.getattr("TRANSFORM_REGISTRY")?;

        match registry.call_method1("get", (name,)) {
            Ok(func) => {
                if func.is_none() {
                    return Err(pyo3::exceptions::PyKeyError::new_err(format!(
                        "Transform '{}' not found in registry",
                        name
                    )));
                }

                // Convert kwargs to PyDict
                let py_kwargs = PyDict::new(py);
                for (key, value) in kwargs {
                    py_kwargs.set_item(key, value)?;
                }

                // Call function with kwargs
                let result = func.call((), Some(py_kwargs))?;

                // Convert result
                if result.is_none() {
                    Ok(None)
                } else {
                    Ok(Some(result.extract::<String>()?))
                }
            }
            Err(e) => Err(e),
        }
    }

    /// Call a list-valued transform function
    ///
    /// Some transforms return lists (e.g., extract_segments returning multiple segment strings).
    ///
    /// # Returns
    ///
    /// * `Ok(Vec<String>)` - Transform returned a list (may be empty)
    /// * `Err(PyErr)` - Transform failed or doesn't return a list
    pub fn call_transform_list(
        &self,
        py: Python,
        name: &str,
        kwargs: HashMap<String, Option<String>>,
    ) -> PyResult<Vec<String>> {
        let transforms_module = PyModule::import(py, self.transforms_module_name.as_str())?;
        let registry = transforms_module.getattr("TRANSFORM_REGISTRY")?;

        let func = registry.call_method1("get", (name,))?;
        if func.is_none() {
            return Err(pyo3::exceptions::PyKeyError::new_err(format!(
                "Transform '{}' not found in registry",
                name
            )));
        }

        // Convert kwargs to PyDict
        let py_kwargs = PyDict::new(py);
        for (key, value) in kwargs {
            py_kwargs.set_item(key, value)?;
        }

        // Call function
        let result = func.call((), Some(py_kwargs))?;

        // Try to extract as list
        if let Ok(list) = result.downcast::<PyList>() {
            let strings: PyResult<Vec<String>> = list.iter()
                .map(|item| {
                    if item.is_none() {
                        Ok(String::new())
                    } else {
                        item.extract::<String>()
                    }
                })
                .collect();
            strings
        } else {
            Err(pyo3::exceptions::PyTypeError::new_err(format!(
                "Transform '{}' did not return a list",
                name
            )))
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_python_transform_creation() {
        let transform = PythonTransform::new("mymodule", "myfunction");
        assert_eq!(transform.module_name, "mymodule");
        assert_eq!(transform.function_name, "myfunction");
    }

    #[test]
    fn test_py_transform_registry_creation() {
        let registry = PyTransformRegistry::new("myapp.transforms");
        assert_eq!(registry.transforms_module_name, "myapp.transforms");
    }

    // Note: Full integration tests with actual Python functions would require
    // a Python environment with the test modules available
}
