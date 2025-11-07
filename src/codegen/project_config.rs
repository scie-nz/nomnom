//! Project configuration schema for nomnom.yaml
//!
//! This module defines the structure for project-level configuration that
//! drives code generation.

use serde::{Deserialize, Serialize};
use std::path::Path;
use crate::codegen::fs_utils;

/// Top-level project configuration from nomnom.yaml
#[derive(Debug, Clone, Deserialize, Serialize)]
pub struct ProjectConfig {
    pub project: ProjectMetadata,
    #[serde(default)]
    pub infrastructure: Vec<InfrastructureSpec>,
    #[serde(default)]
    pub codegen: CodegenOptions,
    #[serde(default)]
    pub dependencies: Vec<Dependency>,
    #[serde(default)]
    pub directories: Directories,
}

/// Project metadata
#[derive(Debug, Clone, Deserialize, Serialize)]
pub struct ProjectMetadata {
    pub name: String,
    pub version: String,
    pub description: String,
    #[serde(default)]
    pub python: PythonConfig,
    #[serde(default)]
    pub rust: RustConfig,
}

/// Python package configuration
#[derive(Debug, Clone, Default, Deserialize, Serialize)]
pub struct PythonConfig {
    pub module_name: String,
    pub package_name: String,
    pub min_version: String,
}

/// Rust crate configuration
#[derive(Debug, Clone, Default, Deserialize, Serialize)]
pub struct RustConfig {
    pub crate_name: String,
    pub lib_name: String,
    pub edition: String,
}

/// Infrastructure specification
#[derive(Debug, Clone, Deserialize, Serialize)]
pub struct InfrastructureSpec {
    pub r#type: String,
    #[serde(default)]
    pub enabled: bool,
    #[serde(flatten)]
    pub options: serde_yaml::Value,
}

/// Code generation options
#[derive(Debug, Clone, Default, Deserialize, Serialize)]
pub struct CodegenOptions {
    #[serde(default)]
    pub entities: bool,
    #[serde(default)]
    pub pyo3_bindings: bool,
    #[serde(default)]
    pub diesel: Option<DieselConfig>,
    #[serde(default)]
    pub python_wrappers: bool,
}

/// Diesel ORM configuration
#[derive(Debug, Clone, Deserialize, Serialize)]
pub struct DieselConfig {
    pub enabled: bool,
    pub database_url: String,
}

/// Rust dependency specification
#[derive(Debug, Clone, Deserialize, Serialize)]
pub struct Dependency {
    pub name: String,
    #[serde(default)]
    pub path: Option<String>,
    #[serde(default)]
    pub version: Option<String>,
    #[serde(default)]
    pub features: Vec<String>,
}

/// Directory configuration
#[derive(Debug, Clone, Default, Deserialize, Serialize)]
pub struct Directories {
    #[serde(default = "default_entities_dir")]
    pub entities: String,
    #[serde(default = "default_transforms_dir")]
    pub transforms: String,
    #[serde(default = "default_output_dir")]
    pub output: String,
}

fn default_entities_dir() -> String {
    "config/entities".to_string()
}

fn default_transforms_dir() -> String {
    "config/transforms".to_string()
}

fn default_output_dir() -> String {
    ".build".to_string()
}

impl ProjectConfig {
    /// Load project configuration from nomnom.yaml
    pub fn from_file<P: AsRef<Path>>(path: P) -> Result<Self, String> {
        let contents = std::fs::read_to_string(path.as_ref())
            .map_err(|e| format!("Failed to read nomnom.yaml: {}", e))?;

        serde_yaml::from_str(&contents)
            .map_err(|e| format!("Failed to parse nomnom.yaml: {}", e))
    }

    /// Get the Python transform module name (e.g., "data_processor.transforms")
    pub fn python_transform_module(&self) -> String {
        format!("{}.transforms", self.project.python.module_name)
    }

    /// Get the transform registry type path (e.g., "crate::transform_registry::TransformRegistry")
    pub fn transform_registry_type(&self) -> String {
        "crate::transform_registry::TransformRegistry".to_string()
    }
}

/// Simplified build configuration for build.rs usage
///
/// This is a simpler schema specifically designed for build.rs scripts that need
/// to generate code from YAML entity definitions.
#[derive(Debug, Clone, Deserialize, Serialize)]
pub struct BuildConfig {
    pub project: BuildProjectInfo,
    pub rust: Option<RustPackageConfig>,
    pub python: Option<PythonPackageConfig>,
    pub dependencies: Option<Vec<DependencyConfig>>,
    pub paths: BuildPathsConfig,
    pub transforms: Option<BuildTransformsConfig>,
    pub helpers: Option<Vec<BuildHelperConfig>>,
}

/// Build project information
#[derive(Debug, Clone, Deserialize, Serialize)]
pub struct BuildProjectInfo {
    pub name: String,
    pub module_name: String,
    #[serde(default = "default_version")]
    pub version: String,
    #[serde(default)]
    pub description: String,
    #[serde(default)]
    pub authors: Vec<String>,
    #[serde(default)]
    pub license: String,
    #[serde(default)]
    pub repository: String,
}

fn default_version() -> String {
    "0.1.0".to_string()
}

fn default_edition() -> String {
    "2021".to_string()
}

fn default_min_python_version() -> String {
    "3.8".to_string()
}

/// Rust package configuration
#[derive(Debug, Clone, Deserialize, Serialize)]
pub struct RustPackageConfig {
    pub crate_name: String,
    #[serde(default = "default_edition")]
    pub edition: String,
}

/// Python package configuration
#[derive(Debug, Clone, Deserialize, Serialize)]
pub struct PythonPackageConfig {
    pub package_name: String,
    #[serde(default = "default_min_python_version")]
    pub min_version: String,
}

/// Dependency configuration
#[derive(Debug, Clone, Deserialize, Serialize)]
pub struct DependencyConfig {
    pub name: String,
    pub path: Option<String>,
    pub version: Option<String>,
    #[serde(default)]
    pub features: Vec<String>,

    /// Optional list of items to re-export from this dependency (legacy, use rust_exports)
    /// Example: exports: ["entity", "Segment", "FieldPath"]
    #[serde(default)]
    pub exports: Option<Vec<String>>,

    /// Rust exports - items to re-export in generated Rust lib.rs
    /// Example: rust_exports: ["entity", "Segment", "FieldPath"]
    #[serde(default)]
    pub rust_exports: Option<Vec<String>>,

    /// Python exports - items to import in generated Python __init__.py
    /// Example: python_exports: ["TransformRegistry", "get_registry"]
    #[serde(default)]
    pub python_exports: Option<Vec<String>>,

    /// Python module path for imports (when python_exports is specified)
    /// Example: python_module: "hl7utils.transforms"
    pub python_module: Option<String>,
}

/// Build paths configuration
#[derive(Debug, Clone, Deserialize, Serialize)]
pub struct BuildPathsConfig {
    #[serde(default = "default_source_root")]
    pub source_root: Option<String>,
    pub config_dir: String,
    pub outputs: BuildOutputsConfig,
}

fn default_source_root() -> Option<String> {
    None
}

/// Build output paths
#[derive(Debug, Clone, Deserialize, Serialize)]
pub struct BuildOutputsConfig {
    pub rust_entities: String,
    pub pyo3_bindings: String,
    pub transform_registry: Option<String>,
    pub python_bindings: Option<String>,
    pub lib_rs: Option<String>,
    pub diesel_schema: Option<String>,
    pub diesel_models: Option<String>,
    pub diesel_operations: Option<String>,
    pub diesel_pyo3: Option<String>,
    pub python_mapping: Option<String>,
    pub python_rust_shim: Option<String>,
    pub python_package_init: Option<String>,
}

/// Build transforms configuration
#[derive(Debug, Clone, Deserialize, Serialize)]
pub struct BuildTransformsConfig {
    /// Optional registry type (only for PyO3 mode with Python transforms)
    #[serde(default)]
    pub registry_type: Option<String>,

    /// Optional Python module (only for PyO3 mode)
    #[serde(default)]
    pub python_module: Option<String>,

    /// Legacy functions list (deprecated, kept for backward compat)
    #[serde(default)]
    pub functions: Vec<String>,

    /// Rust transform definitions with inline code
    #[serde(default)]
    pub rust: std::collections::HashMap<String, RustTransformDef>,

    /// PyO3-wrapped transforms (require PyO3 feature, not available in standalone)
    #[serde(default)]
    pub pyo3_functions: Vec<String>,
}

/// Rust transform definition from nomnom.yaml
#[derive(Debug, Clone, Deserialize, Serialize)]
pub struct RustTransformDef {
    /// Function parameters
    pub args: Vec<TransformArg>,

    /// Return type (e.g., "Result<String, String>")
    pub return_type: String,

    /// Inline Rust code for the function body
    pub code: String,

    /// Optional documentation
    #[serde(default)]
    pub doc: Option<String>,

    /// Optional imports needed by this transform (e.g., ["regex::Regex", "sha1::{Digest, Sha1}"])
    #[serde(default)]
    pub imports: Vec<String>,
}

/// Transform function argument
#[derive(Debug, Clone, Deserialize, Serialize)]
pub struct TransformArg {
    /// Argument name
    pub name: String,

    /// Rust type (e.g., "&str", "usize", "Option<String>")
    #[serde(rename = "type")]
    pub arg_type: String,
}

/// Build helper configuration
#[derive(Debug, Clone, Deserialize, Serialize)]
pub struct BuildHelperConfig {
    pub name: String,
    pub path: String,
    #[serde(default)]
    pub description: String,
}

impl BuildConfig {
    /// Load build configuration from YAML file
    pub fn from_file<P: AsRef<Path>>(path: P) -> Result<Self, String> {
        let contents = std::fs::read_to_string(path.as_ref())
            .map_err(|e| format!("Failed to read nomnom.yaml: {}", e))?;

        serde_yaml::from_str(&contents)
            .map_err(|e| format!("Failed to parse nomnom.yaml: {}", e))
    }

    /// Convert to GenerationConfig for code generation
    pub fn to_generation_config(&self) -> Result<super::GenerationConfig, String> {
        // If helpers are specified, load them from files
        // Otherwise, use empty string (functions should come from imported crates like hl7utils)
        let additional_rust_header = if let Some(helpers) = &self.helpers {
            let mut content = String::new();
            for helper in helpers {
                let helper_content = std::fs::read_to_string(&helper.path)
                    .map_err(|e| format!("Failed to read helper file {}: {}", helper.path, e))?;
                content.push_str(&helper_content);
                content.push('\n');
            }
            content
        } else {
            String::new()
        };

        // Extract Rust dependency exports (use rust_exports if present, fallback to exports for backward compat)
        let dependency_exports = self.dependencies.as_ref().and_then(|deps| {
            let exports: Vec<(String, Vec<String>)> = deps
                .iter()
                .filter_map(|dep| {
                    let rust_exports = dep.rust_exports.as_ref()
                        .or(dep.exports.as_ref());  // Fallback to legacy exports field

                    rust_exports.map(|exports| {
                        (dep.name.clone(), exports.clone())
                    })
                })
                .collect();

            if exports.is_empty() {
                None
            } else {
                Some(exports)
            }
        });

        // Extract Python dependency imports
        let python_dependency_imports = self.dependencies.as_ref().and_then(|deps| {
            let imports: Vec<(String, String, Vec<String>)> = deps
                .iter()
                .filter_map(|dep| {
                    // Only include if python_exports and python_module are specified
                    if let (Some(exports), Some(module)) = (&dep.python_exports, &dep.python_module) {
                        Some((dep.name.clone(), module.clone(), exports.clone()))
                    } else {
                        None
                    }
                })
                .collect();

            if imports.is_empty() {
                None
            } else {
                Some(imports)
            }
        });

        Ok(super::GenerationConfig {
            config_dir: self.paths.config_dir.clone(),
            rust_output: self.paths.outputs.rust_entities.clone(),
            pyo3_bindings_output: self.paths.outputs.pyo3_bindings.clone(),
            diesel_schema_output: self.paths.outputs.diesel_schema.clone(),
            diesel_models_output: self.paths.outputs.diesel_models.clone(),
            diesel_operations_output: self.paths.outputs.diesel_operations.clone(),
            diesel_pyo3_output: self.paths.outputs.diesel_pyo3.clone(),
            python_mapping_output: self.paths.outputs.python_mapping.clone(),
            python_module_name: self.project.module_name.clone(),
            transform_registry_type: self
                .transforms
                .as_ref()
                .and_then(|t| t.registry_type.clone())
                .unwrap_or_else(|| "crate::transform_registry::TransformRegistry".to_string()),
            additional_rust_header: Some(additional_rust_header),
            transform_registry_output: self.paths.outputs.transform_registry.clone(),
            python_transforms_module: self.transforms.as_ref().and_then(|t| t.python_module.clone()),
            python_bindings_output: self.paths.outputs.python_bindings.clone(),
            transform_functions: self.transforms.as_ref().map(|t| t.functions.clone()),
            lib_rs_output: self.paths.outputs.lib_rs.clone(),
            dependency_exports,
            python_dependency_imports,
            python_rust_shim_output: self.paths.outputs.python_rust_shim.clone(),
            python_package_init_output: self.paths.outputs.python_package_init.clone(),
            rust_transforms: self.transforms.as_ref().map(|t| t.rust.clone()),
        })
    }

    /// Generate Cargo.toml content
    pub fn generate_cargo_toml(&self) -> String {
        let crate_name = self.rust.as_ref()
            .map(|r| r.crate_name.clone())
            .unwrap_or_else(|| format!("{}_rust", self.project.name));

        let edition = self.rust.as_ref()
            .map(|r| r.edition.clone())
            .unwrap_or_else(|| "2021".to_string());

        let python_module_name = self.project.module_name
            .split('.')
            .last()
            .unwrap_or("_rust");

        // Find nomnom dependency path
        let nomnom_path = self.dependencies.as_ref()
            .and_then(|deps| deps.iter().find(|d| d.name == "nomnom"))
            .and_then(|d| d.path.as_ref())
            .map(|p| p.clone())
            .unwrap_or_else(|| "../../nomnom".to_string());

        // Get lib.rs path from config, or default to "src/lib.rs"
        let lib_rs_path = self.paths.outputs.lib_rs.as_ref()
            .map(|p| p.as_str())
            .unwrap_or("src/lib.rs");

        let mut toml = format!(r#"[package]
name = "{}"
version = "{}"
edition = "{}"
description = "{}"
"#, crate_name, self.project.version, edition, self.project.description);

        if !self.project.authors.is_empty() {
            toml.push_str(&format!("authors = {:?}\n", self.project.authors));
        }

        if !self.project.license.is_empty() {
            toml.push_str(&format!("license = \"{}\"\n", self.project.license));
        }

        if !self.project.repository.is_empty() {
            toml.push_str(&format!("repository = \"{}\"\n", self.project.repository));
        }

        toml.push_str(&format!(r#"
[lib]
name = "{}"
path = "{}"
crate-type = ["cdylib", "rlib"]
doctest = false

[[bin]]
name = "hl7-parser"
path = "rust_build/src/bin/record_parser.rs"

[dependencies]
clap = {{ version = "4.4", features = ["derive"] }}
pyo3 = {{ version = "0.20", features = ["extension-module", "abi3-py38"] }}
serde = {{ version = "1.0", features = ["derive"] }}
serde_json = "1.0"
serde_yaml = "0.9"
once_cell = "1.19"
thiserror = "1.0"
sha1 = "0.10"
sha2 = "0.10"
regex = "1.10"

# Diesel ORM with connection pooling
diesel = {{ version = "2.1", features = ["mysql", "sqlite", "r2d2", "chrono"] }}
diesel_migrations = "2.1"
r2d2 = "0.8"
chrono = {{ version = "0.4", features = ["serde"] }}

# Nomnom entity framework (runtime with python-bridge feature)
nomnom = {{ path = "{}", features = ["python-bridge"] }}

"#, python_module_name, lib_rs_path, nomnom_path));

        // Add additional dependencies
        if let Some(deps) = &self.dependencies {
            for dep in deps {
                if dep.name == "nomnom" {
                    continue; // Already added
                }
                toml.push_str(&self.format_dependency(dep));
            }
        }

        toml.push_str(&format!(r#"
[build-dependencies]
serde = {{ version = "1.0", features = ["derive"] }}
serde_yaml = "0.9"
glob = "0.3"
nomnom = {{ path = "{}" }}
"#, nomnom_path));

        toml
    }

    /// Generate pyproject.toml content
    pub fn generate_pyproject_toml(&self) -> String {
        let python_package_name = self.python.as_ref()
            .map(|p| p.package_name.clone())
            .unwrap_or_else(|| self.project.name.replace("_", "-"));

        let min_version = self.python.as_ref()
            .map(|p| p.min_version.clone())
            .unwrap_or_else(|| "3.8".to_string());

        let python_module_name = self.project.module_name
            .split('.')
            .last()
            .unwrap_or("_rust");

        let mut toml = format!(r#"[build-system]
requires = ["maturin>=1.0,<2.0"]
build-backend = "maturin"

[project]
name = "{}"
version = "{}"
description = "{}"
readme = "README.md"
requires-python = ">={}"
"#, python_package_name, self.project.version, self.project.description, min_version);

        if !self.project.license.is_empty() {
            toml.push_str(&format!("license = {{text = \"{}\"}}\n", self.project.license));
        }

        if !self.project.authors.is_empty() {
            toml.push_str("authors = [\n");
            for author in &self.project.authors {
                toml.push_str(&format!("    {{name = \"{}\"}},\n", author));
            }
            toml.push_str("]\n");
        }

        toml.push_str(&format!(r#"keywords = ["data-transformation", "etl", "parsing", "nomnom"]
classifiers = [
    "Development Status :: 3 - Alpha",
    "Intended Audience :: Developers",
    "License :: OSI Approved :: MIT License",
    "Programming Language :: Python :: 3",
    "Programming Language :: Python :: 3.8",
    "Programming Language :: Python :: 3.9",
    "Programming Language :: Python :: 3.10",
    "Programming Language :: Python :: 3.11",
    "Programming Language :: Python :: 3.12",
    "Programming Language :: Rust",
    "Topic :: Software Development :: Libraries",
]

dependencies = [
    "PyYAML>=6.0.2",
    "pydantic>=2.0.0",
]

[project.optional-dependencies]
test = [
    "pytest>=8.0.0",
    "pytest-benchmark>=5.0.0",
]
dev = [
    "maturin>=1.0.0",
    "ruff>=0.1.0",
]

[tool.maturin]
module-name = "{}.{}"
features = ["pyo3/extension-module"]
include = ["config/**/*.yaml"]

[tool.pytest.ini_options]
minversion = "8.0"
testpaths = ["tests"]
python_files = ["test_*.py"]
addopts = [
    "--strict-markers",
    "-ra",
]
markers = [
    "filename_config: marks tests that use filename configuration (deselect with '-m \\\"not filename_config\\\"')",
]

[tool.ruff]
line-length = 100
target-version = "py38"
"#, self.project.name, python_module_name));

        toml
    }

    /// Generate README.md content
    pub fn generate_readme(&self) -> String {
        format!(r#"# {}

{}

## Auto-Generated Project

This project was automatically generated by [nomnom](https://github.com/scie-nz/ingestion/tree/main/nomnom),
a YAML-based code generation framework for data transformation libraries.

## Building

### Development Build

```bash
maturin develop
```

### Release Build

```bash
maturin build --release
```

## Testing

```bash
pytest tests/
```

## Generated From

This project was generated from YAML entity and transform definitions using:

```bash
nomnom build --config config --output .build --release
```

## Structure

- `src/lib.rs` - Main library entry point
- `src/generated.rs` - Auto-generated entity definitions
- `src/generated_bindings.rs` - Auto-generated PyO3 bindings
- `Cargo.toml` - Rust dependencies and build configuration
- `pyproject.toml` - Python packaging configuration

## License

{}
"#,
            self.project.name,
            self.project.description,
            if !self.project.license.is_empty() { &self.project.license } else { "MIT" }
        )
    }

    /// Format a dependency for Cargo.toml
    fn format_dependency(&self, dep: &DependencyConfig) -> String {
        let mut line = format!("{} = ", dep.name);

        if let Some(path) = &dep.path {
            line.push_str(&format!("{{ path = \"{}\"", path));
            if let Some(version) = &dep.version {
                line.push_str(&format!(", version = \"{}\"", version));
            }
            if !dep.features.is_empty() {
                line.push_str(&format!(", features = {:?}", dep.features));
            }
            line.push_str(" }");
        } else if let Some(version) = &dep.version {
            if dep.features.is_empty() {
                line.push_str(&format!("\"{}\"", version));
            } else {
                line.push_str(&format!("{{ version = \"{}\", features = {:?} }}", version, dep.features));
            }
        }

        line.push('\n');
        line
    }

    /// Write all build configuration files to the output directory
    pub fn write_build_configs<P: AsRef<Path>>(&self, output_dir: P) -> Result<(), String> {
        let output_dir = output_dir.as_ref();

        // Create output directory if needed
        std::fs::create_dir_all(output_dir)
            .map_err(|e| format!("Failed to create output directory: {}", e))?;

        // Generate Cargo.toml
        let cargo_toml = self.generate_cargo_toml();
        fs_utils::write_file(output_dir.join("Cargo.toml"), cargo_toml)
            .map_err(|e| format!("Failed to write Cargo.toml: {}", e))?;

        // Generate pyproject.toml
        let pyproject_toml = self.generate_pyproject_toml();
        fs_utils::write_file(output_dir.join("pyproject.toml"), pyproject_toml)
            .map_err(|e| format!("Failed to write pyproject.toml: {}", e))?;

        // Generate README.md
        let readme = self.generate_readme();
        fs_utils::write_file(output_dir.join("README.md"), readme)
            .map_err(|e| format!("Failed to write README.md: {}", e))?;

        Ok(())
    }

    /// Validate configuration
    pub fn validate(&self) -> Result<(), String> {
        // Check that required fields are present
        if self.project.name.is_empty() {
            return Err("project.name is required".to_string());
        }

        if self.project.module_name.is_empty() {
            return Err("project.module_name is required".to_string());
        }

        // Check that helper files exist
        if let Some(helpers) = &self.helpers {
            for helper in helpers {
                if !Path::new(&helper.path).exists() {
                    return Err(format!("Helper file not found: {}", helper.path));
                }
            }
        }

        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_parse_sample_config() {
        let yaml = r#"
project:
  name: data_processor
  version: 0.1.0
  description: HL7v2 message parsing
  python:
    module_name: data_processor
    package_name: hl7-ingestion
    min_version: "3.8"
  rust:
    crate_name: data_processor_rust
    lib_name: _rust
    edition: "2021"

infrastructure:
  - type: hl7_segment_parser
    enabled: true

codegen:
  entities: true
  pyo3_bindings: true

dependencies:
  - name: hl7utils
    path: ../hl7utils
    version: "0.1.0"

directories:
  entities: config/entities
  transforms: config/transforms
  output: .build
"#;

        let config: ProjectConfig = serde_yaml::from_str(yaml).unwrap();
        assert_eq!(config.project.name, "data_processor");
        assert_eq!(config.project.python.module_name, "data_processor");
        assert_eq!(config.dependencies.len(), 1);
        assert_eq!(config.dependencies[0].name, "hl7utils");
    }

    #[test]
    fn test_build_config_minimal() {
        let yaml = r#"
project:
  name: test_project
  module_name: test._rust

paths:
  config_dir: config/entities
  outputs:
    rust_entities: src/generated.rs
    pyo3_bindings: src/generated_bindings.rs
"#;

        let config: BuildConfig = serde_yaml::from_str(yaml).unwrap();
        assert_eq!(config.project.name, "test_project");
        assert_eq!(config.paths.config_dir, "config/entities");
        assert!(config.transforms.is_none());
        assert!(config.helpers.is_none());
    }

    #[test]
    fn test_build_config_full() {
        let yaml = r#"
project:
  name: data_processor
  module_name: data_processor._rust

paths:
  config_dir: config/entities
  outputs:
    rust_entities: src/generated.rs
    pyo3_bindings: src/generated_bindings.rs
    transform_registry: src/transform_registry.rs
    lib_rs: src/lib.rs

transforms:
  registry_type: crate::transform_registry::TransformRegistry
  python_module: data_processor.transforms
  functions:
    - build_segment_index
    - extract_from_hl7_segment

helpers:
  - name: format_datetime
    path: helpers/datetime.rs
    description: Format datetime strings
"#;

        let config: BuildConfig = serde_yaml::from_str(yaml).unwrap();
        assert_eq!(config.project.name, "data_processor");
        assert!(config.transforms.is_some());
        assert!(config.helpers.is_some());

        let transforms = config.transforms.unwrap();
        assert_eq!(transforms.functions.len(), 2);

        let helpers = config.helpers.unwrap();
        assert_eq!(helpers.len(), 1);
        assert_eq!(helpers[0].name, "format_datetime");
    }
}
