//! Integration tests for nomnom runtime and transform modules

use nomnom::runtime::{ExtractionContext, TransformRegistry, TransformDef, TransformLanguage, Implementation, Parameter, ReturnType};
use nomnom::codegen::{generate_rust_transform, generate_pyo3_binding, generate_python_transform};

#[test]
fn test_extraction_context_basic() {
    let mut ctx = ExtractionContext::empty();

    ctx.set_context_field("filename".to_string(), "test.csv".to_string());
    assert_eq!(ctx.get_context_field("filename"), Some(&"test.csv".to_string()));

    ctx.set_entity("MPI".to_string(), serde_json::json!({"lastname": "Smith"}));
    assert!(ctx.has_entity("MPI"));
    assert_eq!(ctx.entity_count(), 1);
}

#[test]
fn test_transform_registry() {
    let registry = TransformRegistry::new();

    assert_eq!(registry.count(), 0);
    assert!(!registry.has_transform("nonexistent"));
}

#[test]
fn test_generate_rust_transform() {
    let transform = TransformDef {
        name: "uppercase".to_string(),
        language: TransformLanguage::Rust,
        doc: Some("Convert string to uppercase".to_string()),
        parameters: vec![
            Parameter {
                name: "input".to_string(),
                param_type: "String".to_string(),
                default: None,
                doc: Some("Input string".to_string()),
            }
        ],
        returns: ReturnType {
            return_type: "String".to_string(),
            doc: Some("Uppercase string".to_string()),
        },
        implementation: Implementation::Inline {
            code: "input.to_uppercase()".to_string(),
        },
        tests: vec![],
    };

    let code = generate_rust_transform(&transform).unwrap();

    assert!(code.contains("pub fn uppercase"));
    assert!(code.contains("input: &str"));
    assert!(code.contains("-> String"));
    assert!(code.contains("input.to_uppercase()"));
}

#[test]
fn test_generate_pyo3_binding() {
    let transform = TransformDef {
        name: "test_func".to_string(),
        language: TransformLanguage::Rust,
        doc: Some("Test function".to_string()),
        parameters: vec![
            Parameter {
                name: "value".to_string(),
                param_type: "String".to_string(),
                default: None,
                doc: None,
            }
        ],
        returns: ReturnType {
            return_type: "Option<String>".to_string(),
            doc: None,
        },
        implementation: Implementation::Builtin,
        tests: vec![],
    };

    let code = generate_pyo3_binding(&transform).unwrap();

    assert!(code.contains("#[pyfunction]"));
    assert!(code.contains("pub fn test_func"));
    assert!(code.contains("PyResult<Option<String>>"));
}

#[test]
fn test_generate_python_transform() {
    let transform = TransformDef {
        name: "compute_value".to_string(),
        language: TransformLanguage::Python,
        doc: Some("Compute a value".to_string()),
        parameters: vec![
            Parameter {
                name: "x".to_string(),
                param_type: "int".to_string(),
                default: Some("0".to_string()),
                doc: None,
            }
        ],
        returns: ReturnType {
            return_type: "int".to_string(),
            doc: None,
        },
        implementation: Implementation::Inline {
            code: "return x * 2".to_string(),
        },
        tests: vec![],
    };

    let code = generate_python_transform(&transform).unwrap();

    assert!(code.contains("def compute_value(x=0):"));
    assert!(code.contains("\"\"\"Compute a value\"\"\""));
    assert!(code.contains("return x * 2"));
}
