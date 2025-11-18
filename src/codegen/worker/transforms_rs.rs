/// Generate transforms.rs with helper functions for derived entity field extraction

use std::path::Path;
use std::error::Error;
use std::io::Write;
use std::collections::HashMap;
use crate::codegen::project_config::{RustTransformDef, TestExpectation};

pub fn generate_transforms_rs(
    output_dir: &Path,
    transforms: Option<&HashMap<String, RustTransformDef>>,
) -> Result<(), Box<dyn Error>> {
    let transforms_file = output_dir.join("src/transforms.rs");
    let mut output = std::fs::File::create(&transforms_file)?;

    writeln!(output, "// Auto-generated transform functions")?;
    writeln!(output, "// Generated from nomnom.yaml transforms configuration\n")?;

    // Generate custom transforms from nomnom.yaml
    if let Some(transforms) = transforms {
        generate_custom_transforms(&mut output, transforms)?;
    } else {
        // If no transforms provided, generate an empty module
        writeln!(output, "// No custom transforms defined in nomnom.yaml")?;
    }

    Ok(())
}

/// Generate custom transform functions from nomnom.yaml
fn generate_custom_transforms(
    output: &mut std::fs::File,
    transforms: &HashMap<String, RustTransformDef>,
) -> Result<(), Box<dyn Error>> {
    writeln!(output, "// Custom transform functions from nomnom.yaml\n")?;

    // Collect all unique imports
    let mut all_imports = std::collections::HashSet::new();
    for transform in transforms.values() {
        for import in &transform.imports {
            all_imports.insert(import.clone());
        }
    }

    // Generate imports
    let has_imports = !all_imports.is_empty();
    for import in all_imports {
        writeln!(output, "use {};", import)?;
    }
    if has_imports {
        writeln!(output)?;
    }

    // Generate each transform function
    for (name, transform) in transforms {
        // Generate documentation
        if let Some(ref doc) = transform.doc {
            writeln!(output, "/// {}", doc)?;
        }

        // Generate function signature
        write!(output, "pub fn {}(", name)?;
        for (i, arg) in transform.args.iter().enumerate() {
            if i > 0 {
                write!(output, ", ")?;
            }
            write!(output, "{}: {}", arg.name, arg.arg_type)?;
        }
        writeln!(output, ") -> {} {{", transform.return_type)?;

        // Generate function body (indent each line)
        for line in transform.code.lines() {
            writeln!(output, "    {}", line)?;
        }

        writeln!(output, "}}\n")?;
    }

    // Generate tests
    generate_transform_tests(output, transforms)?;

    Ok(())
}

/// Generate test module for transforms with test definitions
fn generate_transform_tests(
    output: &mut std::fs::File,
    transforms: &HashMap<String, RustTransformDef>,
) -> Result<(), Box<dyn Error>> {
    // Collect all tests from all transforms
    let mut has_tests = false;
    for transform in transforms.values() {
        if !transform.tests.is_empty() {
            has_tests = true;
            break;
        }
    }

    if !has_tests {
        return Ok(());
    }

    // Generate test module
    writeln!(output, "#[cfg(test)]")?;
    writeln!(output, "mod transform_tests {{")?;
    writeln!(output, "    use super::*;\n")?;

    // Generate each test
    for (transform_name, transform) in transforms {
        for test in &transform.tests {
            // Generate test function
            writeln!(output, "    #[test]")?;
            writeln!(output, "    fn {}() {{", test.name)?;
            writeln!(output, "        // {}", test.description)?;

            // Generate input variable declarations
            for arg in &transform.args {
                if let Some(value) = test.inputs.get(&arg.name) {
                    let rust_value = yaml_value_to_rust(value, &arg.arg_type);
                    writeln!(output, "        let {} = {};", arg.name, rust_value)?;
                }
            }

            // Generate function call
            write!(output, "        let result = {}(", transform_name)?;
            for (i, arg) in transform.args.iter().enumerate() {
                if i > 0 {
                    write!(output, ", ")?;
                }
                // Add & prefix if the arg type starts with &
                if arg.arg_type.starts_with('&') {
                    write!(output, "&{}", arg.name)?;
                } else {
                    write!(output, "{}", arg.name)?;
                }
            }
            writeln!(output, ");")?;
            writeln!(output)?;

            // Generate assertion based on expectation
            match &test.expected {
                TestExpectation::Ok { ok } => {
                    // Extract the success type from Result<T, E>
                    let success_type = extract_result_ok_type(&transform.return_type);
                    let expected_value = yaml_value_to_rust(ok, &success_type);
                    writeln!(output, "        assert!(result.is_ok(), \"Expected Ok, got Err: {{:?}}\", result);")?;
                    writeln!(output, "        assert_eq!(result.unwrap(), {});", expected_value)?;
                }
                TestExpectation::Err { err } => {
                    writeln!(output, "        assert!(result.is_err(), \"Expected Err, got Ok: {{:?}}\", result);")?;
                    writeln!(output, "        let err_msg = result.unwrap_err();")?;
                    writeln!(output, "        assert!(err_msg.contains(\"{}\"),", err)?;
                    writeln!(output, "                \"Error message should contain '{}', got: {{}}\", err_msg);", err)?;
                }
            }

            writeln!(output, "    }}\n")?;
        }
    }

    writeln!(output, "}}")?;

    Ok(())
}

/// Extract the OK type from a Result<T, E> type signature
fn extract_result_ok_type(return_type: &str) -> String {
    // Handle Result<T, E> - extract T
    let trimmed = return_type.trim();
    if trimmed.starts_with("Result<") && trimmed.ends_with('>') {
        // Remove "Result<" and ">"
        let inner = &trimmed[7..trimmed.len()-1];
        // Find the comma that separates T from E (need to handle nested types)
        if let Some(comma_pos) = find_top_level_comma(inner) {
            return inner[..comma_pos].trim().to_string();
        }
    }
    return_type.to_string()
}

/// Find the position of the top-level comma in a type signature (ignoring nested commas)
fn find_top_level_comma(s: &str) -> Option<usize> {
    let mut depth = 0;
    for (i, ch) in s.chars().enumerate() {
        match ch {
            '<' => depth += 1,
            '>' => depth -= 1,
            ',' if depth == 0 => return Some(i),
            _ => {}
        }
    }
    None
}

/// Convert YAML value to Rust literal based on the parameter type
fn yaml_value_to_rust(value: &serde_yaml::Value, param_type: &str) -> String {
    // Strip & and whitespace from param_type for analysis
    let clean_type = param_type.trim().trim_start_matches('&').trim();

    match value {
        serde_yaml::Value::Null => {
            if clean_type.starts_with("Option") {
                "None".to_string()
            } else {
                "None".to_string()  // Default to None for null values
            }
        }
        serde_yaml::Value::String(s) => {
            if clean_type.starts_with("Option") {
                format!("Some(\"{}\".to_string())", s)
            } else if clean_type == "str" || clean_type.starts_with("&str") {
                format!("\"{}\"", s)
            } else {
                format!("\"{}\".to_string()", s)
            }
        }
        serde_yaml::Value::Number(n) => n.to_string(),
        serde_yaml::Value::Bool(b) => b.to_string(),
        _ => "/* unsupported value type */".to_string(),
    }
}
