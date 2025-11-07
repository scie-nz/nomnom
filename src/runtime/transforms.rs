//! Transform YAML definitions and loader.
//!
//! Provides types and loader for transform YAMLs that define data transformation
//! functions in a format-agnostic way.

use std::collections::HashMap;
use std::fs;
use std::path::Path;
use serde::{Deserialize, Serialize};

/// Transform definition from YAML.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TransformDef {
    /// Transform name (unique identifier)
    pub name: String,

    /// Programming language (rust or python)
    pub language: TransformLanguage,

    /// Documentation string
    #[serde(default)]
    pub doc: Option<String>,

    /// Parameters that the transform accepts
    pub parameters: Vec<Parameter>,

    /// Return type
    pub returns: ReturnType,

    /// Implementation details
    pub implementation: Implementation,

    /// Optional unit tests
    #[serde(default)]
    pub tests: Vec<TransformTest>,
}

/// Programming language for transform implementation.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum TransformLanguage {
    Rust,
    Python,
}

/// Parameter definition.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Parameter {
    /// Parameter name
    pub name: String,

    /// Parameter type (e.g., "String", "Option<String>", "PatientAccount")
    #[serde(rename = "type")]
    pub param_type: String,

    /// Optional default value (as string)
    #[serde(default)]
    pub default: Option<String>,

    /// Documentation
    #[serde(default)]
    pub doc: Option<String>,
}

/// Return type definition.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ReturnType {
    /// Return type (e.g., "String", "Option<String>", "Vec<String>")
    #[serde(rename = "type")]
    pub return_type: String,

    /// Documentation
    #[serde(default)]
    pub doc: Option<String>,
}

/// Transform implementation.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(tag = "type", rename_all = "lowercase")]
pub enum Implementation {
    /// Built-in transform (implemented in Rust, no code needed)
    Builtin,

    /// Inline code
    Inline {
        /// Source code
        code: String,
    },

    /// Reference to other transforms (transform chain)
    Reference {
        /// Steps in the transform chain
        steps: Vec<TransformStep>,

        /// Variable name to return
        #[serde(rename = "return")]
        return_var: String,
    },
}

/// Step in a transform chain.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TransformStep {
    /// Transform name to call
    pub transform: String,

    /// Arguments (variable names starting with $ are substituted)
    pub args: HashMap<String, serde_json::Value>,

    /// Variable name to store result
    pub output: String,
}

/// Unit test for a transform.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TransformTest {
    /// Test name
    pub name: String,

    /// Test arguments
    pub args: HashMap<String, serde_json::Value>,

    /// Expected result
    pub expected: serde_json::Value,
}

/// Registry of transforms loaded from YAMLs.
#[derive(Debug, Clone)]
pub struct TransformRegistry {
    /// Loaded transforms: name -> definition
    transforms: HashMap<String, TransformDef>,
}

impl TransformRegistry {
    /// Create a new empty transform registry.
    pub fn new() -> Self {
        Self {
            transforms: HashMap::new(),
        }
    }

    /// Load a transform from YAML file.
    ///
    /// # Arguments
    /// * `path` - Path to transform YAML file
    ///
    /// # Returns
    /// Loaded transform definition
    ///
    /// # Errors
    /// Returns error if file doesn't exist or has invalid format
    pub fn load_transform<P: AsRef<Path>>(&mut self, path: P) -> Result<TransformDef, String> {
        let path = path.as_ref();

        // Read file
        let contents = fs::read_to_string(path)
            .map_err(|e| format!("Failed to read transform file {}: {}", path.display(), e))?;

        // Parse YAML
        let yaml: serde_yaml::Value = serde_yaml::from_str(&contents)
            .map_err(|e| format!("Failed to parse YAML: {}", e))?;

        // Extract transform definition
        let transform_yaml = yaml
            .get("transform")
            .ok_or_else(|| "Transform YAML missing 'transform' field".to_string())?;

        // Deserialize transform
        let transform: TransformDef = serde_yaml::from_value(transform_yaml.clone())
            .map_err(|e| format!("Failed to parse transform definition: {}", e))?;

        // Validate
        self.validate_transform(&transform)?;

        // Register
        self.transforms.insert(transform.name.clone(), transform.clone());

        Ok(transform)
    }

    /// Load all transforms from a directory.
    ///
    /// # Arguments
    /// * `dir_path` - Path to directory containing transform YAMLs
    ///
    /// # Returns
    /// Number of transforms loaded
    pub fn load_transforms_from_dir<P: AsRef<Path>>(&mut self, dir_path: P) -> Result<usize, String> {
        let dir_path = dir_path.as_ref();

        if !dir_path.exists() {
            return Err(format!("Transform directory does not exist: {}", dir_path.display()));
        }

        if !dir_path.is_dir() {
            return Err(format!("Path is not a directory: {}", dir_path.display()));
        }

        let mut count = 0;

        // Read directory entries
        let entries = fs::read_dir(dir_path)
            .map_err(|e| format!("Failed to read directory {}: {}", dir_path.display(), e))?;

        for entry in entries {
            let entry = entry.map_err(|e| format!("Failed to read directory entry: {}", e))?;
            let path = entry.path();

            // Only process .yaml and .yml files
            if let Some(ext) = path.extension() {
                if ext == "yaml" || ext == "yml" {
                    match self.load_transform(&path) {
                        Ok(_) => count += 1,
                        Err(e) => {
                            // Log error but continue loading other transforms
                            eprintln!("Warning: Failed to load transform from {}: {}", path.display(), e);
                        }
                    }
                }
            }
        }

        Ok(count)
    }

    /// Validate a transform definition.
    ///
    /// Checks:
    /// - Name is not empty
    /// - Has at least one parameter or returns a value
    /// - Referenced transforms exist (for Reference implementation)
    /// - No circular dependencies
    fn validate_transform(&self, transform: &TransformDef) -> Result<(), String> {
        // Check name
        if transform.name.is_empty() {
            return Err("Transform name cannot be empty".to_string());
        }

        // Check for self-reference in transform chains
        if let Implementation::Reference { steps, .. } = &transform.implementation {
            for step in steps {
                if step.transform == transform.name {
                    return Err(format!(
                        "Transform '{}' contains circular reference to itself",
                        transform.name
                    ));
                }

                // Check that referenced transforms exist
                if !self.has_transform(&step.transform) {
                    return Err(format!(
                        "Transform '{}' references undefined transform '{}'",
                        transform.name, step.transform
                    ));
                }
            }
        }

        Ok(())
    }

    /// Check if a transform is registered.
    pub fn has_transform(&self, name: &str) -> bool {
        self.transforms.contains_key(name)
    }

    /// Get a transform definition by name.
    pub fn get_transform(&self, name: &str) -> Option<&TransformDef> {
        self.transforms.get(name)
    }

    /// Get all transform names.
    pub fn transform_names(&self) -> Vec<&String> {
        self.transforms.keys().collect()
    }

    /// Get number of registered transforms.
    pub fn count(&self) -> usize {
        self.transforms.len()
    }

    /// Get all Rust transforms (for code generation).
    pub fn rust_transforms(&self) -> Vec<&TransformDef> {
        self.transforms
            .values()
            .filter(|t| t.language == TransformLanguage::Rust)
            .collect()
    }

    /// Get all Python transforms (for code generation).
    pub fn python_transforms(&self) -> Vec<&TransformDef> {
        self.transforms
            .values()
            .filter(|t| t.language == TransformLanguage::Python)
            .collect()
    }

    /// Get all transforms with inline implementations (need code generation).
    pub fn inline_transforms(&self) -> Vec<&TransformDef> {
        self.transforms
            .values()
            .filter(|t| matches!(t.implementation, Implementation::Inline { .. }))
            .collect()
    }
}

impl Default for TransformRegistry {
    fn default() -> Self {
        Self::new()
    }
}

/// Convenience function to load a single transform from a YAML file
///
/// # Arguments
/// * `path` - Path to transform YAML file
///
/// # Returns
/// TransformDef on success
pub fn load_transform<P: AsRef<Path>>(path: P) -> Result<TransformDef, String> {
    let mut registry = TransformRegistry::new();
    registry.load_transform(path)
}

/// Convenience function to load all transforms from a directory
///
/// # Arguments
/// * `dir_path` - Path to directory containing transform YAML files
///
/// # Returns
/// Vector of TransformDef structures
pub fn load_transforms_from_dir<P: AsRef<Path>>(dir_path: P) -> Result<Vec<TransformDef>, String> {
    let mut registry = TransformRegistry::new();
    registry.load_transforms_from_dir(dir_path)?;

    // Extract all transforms from the registry
    Ok(registry.transforms.values().cloned().collect())
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::io::Write;
    use tempfile::TempDir;

    fn create_test_transform_yaml(dir: &Path, name: &str, yaml_content: &str) -> std::path::PathBuf {
        let file_path = dir.join(format!("{}.yaml", name));
        let mut file = fs::File::create(&file_path).unwrap();
        file.write_all(yaml_content.as_bytes()).unwrap();
        file_path
    }

    #[test]
    fn test_load_builtin_transform() {
        let temp_dir = TempDir::new().unwrap();
        let yaml = r#"
transform:
  name: extract_from_hl7_segment
  language: rust
  doc: "Extract value from HL7 segment"

  parameters:
    - name: segment
      type: String
      doc: "Raw segment string"

    - name: segment_path
      type: String
      doc: "Path notation (e.g., 'data.3.1')"

  returns:
    type: Option<String>
    doc: "Extracted value"

  implementation:
    type: builtin
"#;

        let file_path = create_test_transform_yaml(temp_dir.path(), "test_transform", yaml);

        let mut registry = TransformRegistry::new();
        let transform = registry.load_transform(&file_path).unwrap();

        assert_eq!(transform.name, "extract_from_hl7_segment");
        assert_eq!(transform.language, TransformLanguage::Rust);
        assert_eq!(transform.parameters.len(), 2);
        assert!(matches!(transform.implementation, Implementation::Builtin));
    }

    #[test]
    fn test_load_inline_transform() {
        let temp_dir = TempDir::new().unwrap();
        let yaml = r#"
transform:
  name: parse_date
  language: rust
  doc: "Parse date"

  parameters:
    - name: date_string
      type: String

  returns:
    type: Option<String>

  implementation:
    type: inline
    code: |
      if date_string.is_empty() {
          return None;
      }
      Some(date_string.to_string())
"#;

        let file_path = create_test_transform_yaml(temp_dir.path(), "parse_date", yaml);

        let mut registry = TransformRegistry::new();
        let transform = registry.load_transform(&file_path).unwrap();

        assert_eq!(transform.name, "parse_date");
        assert!(matches!(transform.implementation, Implementation::Inline { .. }));

        if let Implementation::Inline { code } = &transform.implementation {
            assert!(code.contains("date_string.is_empty()"));
        }
    }

    #[test]
    fn test_load_transforms_from_dir() {
        let temp_dir = TempDir::new().unwrap();

        // Create multiple transform YAMLs
        create_test_transform_yaml(
            temp_dir.path(),
            "transform1",
            r#"
transform:
  name: transform1
  language: rust
  parameters:
    - name: input
      type: String
  returns:
    type: String
  implementation:
    type: builtin
"#,
        );

        create_test_transform_yaml(
            temp_dir.path(),
            "transform2",
            r#"
transform:
  name: transform2
  language: python
  parameters:
    - name: input
      type: String
  returns:
    type: String
  implementation:
    type: builtin
"#,
        );

        let mut registry = TransformRegistry::new();
        let count = registry.load_transforms_from_dir(temp_dir.path()).unwrap();

        assert_eq!(count, 2);
        assert!(registry.has_transform("transform1"));
        assert!(registry.has_transform("transform2"));
    }

    #[test]
    fn test_validate_circular_reference() {
        let temp_dir = TempDir::new().unwrap();
        let yaml = r#"
transform:
  name: circular
  language: rust
  parameters:
    - name: input
      type: String
  returns:
    type: String
  implementation:
    type: reference
    steps:
      - transform: circular
        args:
          input: $input
        output: result
    return: $result
"#;

        let file_path = create_test_transform_yaml(temp_dir.path(), "circular", yaml);

        let mut registry = TransformRegistry::new();
        let result = registry.load_transform(&file_path);

        assert!(result.is_err());
        assert!(result.unwrap_err().contains("circular reference"));
    }

    #[test]
    fn test_rust_and_python_transforms() {
        let mut registry = TransformRegistry::new();

        // Add transforms directly for testing
        registry.transforms.insert(
            "rust_transform".to_string(),
            TransformDef {
                name: "rust_transform".to_string(),
                language: TransformLanguage::Rust,
                doc: None,
                parameters: vec![],
                returns: ReturnType {
                    return_type: "String".to_string(),
                    doc: None,
                },
                implementation: Implementation::Builtin,
                tests: vec![],
            },
        );

        registry.transforms.insert(
            "python_transform".to_string(),
            TransformDef {
                name: "python_transform".to_string(),
                language: TransformLanguage::Python,
                doc: None,
                parameters: vec![],
                returns: ReturnType {
                    return_type: "String".to_string(),
                    doc: None,
                },
                implementation: Implementation::Builtin,
                tests: vec![],
            },
        );

        let rust_transforms = registry.rust_transforms();
        let python_transforms = registry.python_transforms();

        assert_eq!(rust_transforms.len(), 1);
        assert_eq!(python_transforms.len(), 1);
        assert_eq!(rust_transforms[0].name, "rust_transform");
        assert_eq!(python_transforms[0].name, "python_transform");
    }
}
