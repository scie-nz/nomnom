//! Code generation for transforms defined in YAML.
//!
//! Generates Rust code for inline transforms and PyO3 bindings.

use crate::runtime::transforms::{TransformDef, TransformLanguage, Implementation, Parameter};
use std::fmt::Write;

/// Generate Rust code for a transform.
///
/// # Arguments
/// * `transform` - Transform definition
///
/// # Returns
/// Generated Rust code as string
///
/// # Example
/// ```ignore
/// let rust_code = generate_rust_transform(&transform)?;
/// // Outputs:
/// // pub fn parse_date(date_string: &str) -> Option<String> {
/// //     // inline code here
/// // }
/// ```
pub fn generate_rust_transform(transform: &TransformDef) -> Result<String, String> {
    if transform.language != TransformLanguage::Rust {
        return Err(format!(
            "Cannot generate Rust code for {} transform '{}'",
            match transform.language {
                TransformLanguage::Python => "Python",
                _ => "unknown",
            },
            transform.name
        ));
    }

    let mut code = String::new();

    // Add documentation comment
    if let Some(doc) = &transform.doc {
        writeln!(code, "/// {}", doc).unwrap();
        writeln!(code, "///").unwrap();
    }

    // Add parameter documentation
    if !transform.parameters.is_empty() {
        writeln!(code, "/// # Arguments").unwrap();
        for param in &transform.parameters {
            write!(code, "/// * `{}` - ", param.name).unwrap();
            if let Some(doc) = &param.doc {
                writeln!(code, "{}", doc).unwrap();
            } else {
                writeln!(code, "{}", param.param_type).unwrap();
            }
        }
        writeln!(code, "///").unwrap();
    }

    // Add return documentation
    if let Some(doc) = &transform.returns.doc {
        writeln!(code, "/// # Returns").unwrap();
        writeln!(code, "/// {}", doc).unwrap();
    }

    // Generate function signature
    write!(code, "pub fn {}(", transform.name).unwrap();

    // Parameters
    for (i, param) in transform.parameters.iter().enumerate() {
        if i > 0 {
            write!(code, ", ").unwrap();
        }

        // Convert type to Rust reference type for strings
        let rust_type = convert_to_rust_type(&param.param_type);
        write!(code, "{}: {}", param.name, rust_type).unwrap();
    }

    // Return type
    let return_type = &transform.returns.return_type;
    writeln!(code, ") -> {} {{", return_type).unwrap();

    // Function body
    match &transform.implementation {
        Implementation::Builtin => {
            writeln!(code, "    // Builtin implementation - provided by runtime").unwrap();
            writeln!(code, "    unimplemented!(\"Builtin transform '{}' must be implemented in runtime\")", transform.name).unwrap();
        }
        Implementation::Inline { code: inline_code } => {
            // Add inline code with proper indentation
            for line in inline_code.lines() {
                if line.trim().is_empty() {
                    writeln!(code).unwrap();
                } else {
                    writeln!(code, "    {}", line).unwrap();
                }
            }
        }
        Implementation::Reference { steps, return_var } => {
            writeln!(code, "    // Transform chain implementation").unwrap();
            writeln!(code, "    // TODO: Generate code for transform chain").unwrap();
            writeln!(code, "    // Steps: {:?}", steps).unwrap();
            writeln!(code, "    // Return: {}", return_var).unwrap();
            writeln!(code, "    unimplemented!(\"Transform chains not yet implemented\")").unwrap();
        }
    }

    writeln!(code, "}}").unwrap();

    Ok(code)
}

/// Generate PyO3 binding for a Rust transform.
///
/// # Arguments
/// * `transform` - Transform definition (must be Rust language)
///
/// # Returns
/// Generated PyO3 binding code
///
/// # Example
/// ```ignore
/// let pyo3_code = generate_pyo3_binding(&transform)?;
/// // Outputs:
/// // #[pyfunction]
/// // pub fn parse_date(date_string: &str) -> PyResult<Option<String>> {
/// //     Ok(parse_date_impl(date_string))
/// // }
/// ```
pub fn generate_pyo3_binding(transform: &TransformDef) -> Result<String, String> {
    if transform.language != TransformLanguage::Rust {
        return Err(format!(
            "Cannot generate PyO3 binding for non-Rust transform '{}'",
            transform.name
        ));
    }

    let mut code = String::new();

    // Add documentation
    if let Some(doc) = &transform.doc {
        writeln!(code, "/// {}", doc).unwrap();
    }

    // PyO3 function attribute
    writeln!(code, "#[pyfunction]").unwrap();

    // Function signature for Python
    write!(code, "pub fn {}(", transform.name).unwrap();

    // Parameters - convert types for Python interop
    for (i, param) in transform.parameters.iter().enumerate() {
        if i > 0 {
            write!(code, ", ").unwrap();
        }

        let py_type = convert_to_python_type(&param.param_type);
        write!(code, "{}: {}", param.name, py_type).unwrap();
    }

    // Return type wrapped in PyResult
    let py_return_type = convert_to_python_return_type(&transform.returns.return_type);
    writeln!(code, ") -> PyResult<{}> {{", py_return_type).unwrap();

    // Call the Rust implementation
    write!(code, "    Ok({}(", transform.name).unwrap();
    for (i, param) in transform.parameters.iter().enumerate() {
        if i > 0 {
            write!(code, ", ").unwrap();
        }
        write!(code, "{}", param.name).unwrap();
    }
    writeln!(code, "))").unwrap();
    writeln!(code, "}}").unwrap();

    Ok(code)
}

/// Generate Python code for a Python inline transform.
///
/// # Arguments
/// * `transform` - Transform definition (must be Python language)
///
/// # Returns
/// Generated Python function code
pub fn generate_python_transform(transform: &TransformDef) -> Result<String, String> {
    if transform.language != TransformLanguage::Python {
        return Err(format!(
            "Cannot generate Python code for non-Python transform '{}'",
            transform.name
        ));
    }

    let mut code = String::new();

    // Function definition
    write!(code, "def {}(", transform.name).unwrap();

    // Parameters
    for (i, param) in transform.parameters.iter().enumerate() {
        if i > 0 {
            write!(code, ", ").unwrap();
        }

        write!(code, "{}", param.name).unwrap();

        // Add default value if specified
        if let Some(default) = &param.default {
            write!(code, "={}", default).unwrap();
        }
    }

    writeln!(code, "):").unwrap();

    // Docstring
    if let Some(doc) = &transform.doc {
        writeln!(code, "    \"\"\"{}\"\"\"", doc).unwrap();
    }

    // Function body
    match &transform.implementation {
        Implementation::Inline { code: inline_code } => {
            for line in inline_code.lines() {
                if line.trim().is_empty() {
                    writeln!(code).unwrap();
                } else {
                    writeln!(code, "    {}", line).unwrap();
                }
            }
        }
        _ => {
            writeln!(code, "    raise NotImplementedError(\"Only inline Python transforms supported\")").unwrap();
        }
    }

    Ok(code)
}

/// Generate unit test code for a transform.
///
/// # Arguments
/// * `transform` - Transform definition with tests
///
/// # Returns
/// Generated Rust test code
pub fn generate_transform_tests(transform: &TransformDef) -> Result<String, String> {
    if transform.tests.is_empty() {
        return Ok(String::new());
    }

    let mut code = String::new();

    writeln!(code, "#[cfg(test)]").unwrap();
    writeln!(code, "mod tests {{").unwrap();
    writeln!(code, "    use super::*;").unwrap();
    writeln!(code).unwrap();

    for test in &transform.tests {
        writeln!(code, "    #[test]").unwrap();
        writeln!(code, "    fn {}() {{", test.name).unwrap();

        // Generate test call
        write!(code, "        let result = {}(", transform.name).unwrap();

        // TODO: Generate proper test arguments from test.args
        // For now, just add a placeholder
        writeln!(code, "/* test args */);").unwrap();

        // Generate assertion
        writeln!(code, "        // Expected: {:?}", test.expected).unwrap();
        writeln!(code, "        // assert_eq!(result, expected);").unwrap();
        writeln!(code, "        todo!(\"Implement test assertion\")").unwrap();

        writeln!(code, "    }}").unwrap();
        writeln!(code).unwrap();
    }

    writeln!(code, "}}").unwrap();

    Ok(code)
}

/// Generate complete transform module with all transforms.
///
/// # Arguments
/// * `transforms` - List of transform definitions
/// * `module_name` - Name of the generated module
///
/// # Returns
/// Complete Rust module code
pub fn generate_transforms_module(
    transforms: &[&TransformDef],
    module_name: &str,
) -> Result<String, String> {
    let mut code = String::new();

    // Module header
    writeln!(code, "//! Auto-generated transform module").unwrap();
    writeln!(code, "//!").unwrap();
    writeln!(code, "//! This file is auto-generated from transform YAML definitions.").unwrap();
    writeln!(code, "//! DO NOT EDIT MANUALLY - changes will be overwritten.").unwrap();
    writeln!(code).unwrap();

    // Imports
    writeln!(code, "use pyo3::prelude::*;").unwrap();
    writeln!(code).unwrap();

    // Generate each transform
    for transform in transforms {
        if transform.language == TransformLanguage::Rust {
            // Generate Rust implementation
            let rust_code = generate_rust_transform(transform)?;
            writeln!(code, "{}", rust_code).unwrap();
            writeln!(code).unwrap();

            // Generate tests
            if !transform.tests.is_empty() {
                let test_code = generate_transform_tests(transform)?;
                writeln!(code, "{}", test_code).unwrap();
                writeln!(code).unwrap();
            }
        }
    }

    // Generate PyO3 module registration
    writeln!(code, "/// Register all transforms with PyO3 module").unwrap();
    writeln!(code, "#[pymodule]").unwrap();
    writeln!(code, "pub fn {}(_py: Python, m: &PyModule) -> PyResult<()> {{", module_name).unwrap();

    for transform in transforms {
        if transform.language == TransformLanguage::Rust {
            writeln!(code, "    m.add_function(wrap_pyfunction!({}, m)?)?;", transform.name).unwrap();
        }
    }

    writeln!(code, "    Ok(())").unwrap();
    writeln!(code, "}}").unwrap();

    Ok(code)
}

/// Convert YAML type to Rust parameter type.
fn convert_to_rust_type(yaml_type: &str) -> String {
    // Handle common type conversions
    match yaml_type {
        "String" => "&str".to_string(),
        "Option<String>" => "Option<&str>".to_string(),
        "usize" | "i32" | "i64" | "f64" | "bool" => yaml_type.to_string(),
        // For complex types, assume they're passed by reference
        _ => format!("&{}", yaml_type),
    }
}

/// Convert YAML type to Python-compatible type for PyO3.
fn convert_to_python_type(yaml_type: &str) -> String {
    match yaml_type {
        "String" => "&str".to_string(),
        "Option<String>" => "Option<&str>".to_string(),
        "usize" => "usize".to_string(),
        "i32" => "i32".to_string(),
        "i64" => "i64".to_string(),
        "f64" => "f64".to_string(),
        "bool" => "bool".to_string(),
        // Complex types passed by reference
        _ => format!("&{}", yaml_type),
    }
}

/// Convert Rust return type to Python-compatible return type.
fn convert_to_python_return_type(rust_type: &str) -> String {
    // Most types can be returned directly through PyO3
    // String needs to be owned for Python
    if rust_type == "Option<&str>" {
        "Option<String>".to_string()
    } else if rust_type == "&str" {
        "String".to_string()
    } else {
        rust_type.to_string()
    }
}

/// Generate Python transforms module with all Python transforms
///
/// # Arguments
/// * `transforms` - List of transform definitions (will filter for Python only)
///
/// # Returns
/// Complete Python module code with all Python transform functions
pub fn generate_python_transforms(transforms: &[&TransformDef]) -> String {
    let mut code = String::new();

    // Module header
    writeln!(code, "\"\"\"Auto-generated Python transform functions.\"\"\"").unwrap();
    writeln!(code, "# This file is auto-generated from transform YAML definitions.").unwrap();
    writeln!(code, "# DO NOT EDIT MANUALLY - changes will be overwritten.").unwrap();
    writeln!(code).unwrap();

    // Imports
    writeln!(code, "from typing import Optional, List, Dict, Any").unwrap();
    writeln!(code).unwrap();

    // Generate each Python transform
    for transform in transforms {
        if transform.language == TransformLanguage::Python {
            match generate_python_transform(transform) {
                Ok(func_code) => {
                    writeln!(code, "{}", func_code).unwrap();
                    writeln!(code).unwrap();
                }
                Err(e) => {
                    writeln!(code, "# Error generating transform '{}': {}", transform.name, e).unwrap();
                }
            }
        }
    }

    code
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::runtime::transforms::{ReturnType, Implementation};

    fn create_test_transform(name: &str, language: TransformLanguage) -> TransformDef {
        TransformDef {
            name: name.to_string(),
            language,
            doc: Some("Test transform".to_string()),
            parameters: vec![
                Parameter {
                    name: "input".to_string(),
                    param_type: "String".to_string(),
                    default: None,
                    doc: Some("Input string".to_string()),
                }
            ],
            returns: ReturnType {
                return_type: "Option<String>".to_string(),
                doc: Some("Processed output".to_string()),
            },
            implementation: Implementation::Inline {
                code: "Some(input.to_uppercase())".to_string(),
            },
            tests: vec![],
        }
    }

    #[test]
    fn test_generate_rust_transform() {
        let transform = create_test_transform("test_transform", TransformLanguage::Rust);
        let code = generate_rust_transform(&transform).unwrap();

        assert!(code.contains("pub fn test_transform"));
        assert!(code.contains("input: &str"));
        assert!(code.contains("-> Option<String>"));
        assert!(code.contains("Some(input.to_uppercase())"));
    }

    #[test]
    fn test_generate_pyo3_binding() {
        let transform = create_test_transform("test_transform", TransformLanguage::Rust);
        let code = generate_pyo3_binding(&transform).unwrap();

        assert!(code.contains("#[pyfunction]"));
        assert!(code.contains("pub fn test_transform"));
        assert!(code.contains("PyResult<Option<String>>"));
    }

    #[test]
    fn test_generate_python_transform() {
        let transform = create_test_transform("test_transform", TransformLanguage::Python);
        let code = generate_python_transform(&transform).unwrap();

        assert!(code.contains("def test_transform(input):"));
        assert!(code.contains("\"\"\"Test transform\"\"\""));
        assert!(code.contains("Some(input.to_uppercase())"));
    }

    #[test]
    fn test_cannot_generate_rust_for_python() {
        let transform = create_test_transform("test_transform", TransformLanguage::Python);
        let result = generate_rust_transform(&transform);

        assert!(result.is_err());
        assert!(result.unwrap_err().contains("Cannot generate Rust code"));
    }

    #[test]
    fn test_cannot_generate_python_for_rust() {
        let transform = create_test_transform("test_transform", TransformLanguage::Rust);
        let result = generate_python_transform(&transform);

        assert!(result.is_err());
        assert!(result.unwrap_err().contains("Cannot generate Python code"));
    }

    #[test]
    fn test_type_conversions() {
        assert_eq!(convert_to_rust_type("String"), "&str");
        assert_eq!(convert_to_rust_type("Option<String>"), "Option<&str>");
        assert_eq!(convert_to_rust_type("usize"), "usize");
        assert_eq!(convert_to_rust_type("CustomType"), "&CustomType");
    }

    #[test]
    fn test_generate_transforms_module() {
        let transform1 = create_test_transform("transform1", TransformLanguage::Rust);
        let transform2 = create_test_transform("transform2", TransformLanguage::Rust);
        let transforms = vec![&transform1, &transform2];

        let code = generate_transforms_module(&transforms, "my_transforms").unwrap();

        assert!(code.contains("pub fn transform1"));
        assert!(code.contains("pub fn transform2"));
        assert!(code.contains("#[pymodule]"));
        assert!(code.contains("pub fn my_transforms"));
        assert!(code.contains("m.add_function(wrap_pyfunction!(transform1"));
        assert!(code.contains("m.add_function(wrap_pyfunction!(transform2"));
    }
}
