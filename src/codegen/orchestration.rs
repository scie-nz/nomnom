//! High-level orchestration API for code generation.
//!
//! This module provides a unified interface for generating all code artifacts
//! (Rust structs, Python bindings, Diesel schema, etc.) from YAML entity configs.

use std::error::Error;
use std::fs;
use std::io::Write;
use std::path::Path;

use crate::codegen::{
    EntityDef, RustCodegenConfig, PyO3Config, load_entities,
    generate_rust_code, generate_pyo3_bindings, generate_python_core_mapping,
    fs_utils,
};

/// Configuration for code generation orchestration.
///
/// Specifies input config directory and all output file paths.
#[derive(Debug, Clone)]
pub struct GenerationConfig {
    /// Directory containing YAML entity configurations
    pub config_dir: String,

    /// Output path for generated Rust code (e.g., "src/generated.rs")
    pub rust_output: String,

    /// Output path for generated PyO3 bindings (e.g., "src/generated_bindings.rs")
    pub pyo3_bindings_output: String,

    /// Optional: Output path for Diesel schema (e.g., "src/schema.rs")
    pub diesel_schema_output: Option<String>,

    /// Optional: Output path for Diesel models (e.g., "src/models/mod.rs")
    pub diesel_models_output: Option<String>,

    /// Optional: Output path for Diesel operations (e.g., "src/db/generated_operations.rs")
    pub diesel_operations_output: Option<String>,

    /// Optional: Output path for Diesel PyO3 persistence (e.g., "src/python/generated_persistence.rs")
    pub diesel_pyo3_output: Option<String>,

    /// Optional: Output path for Python core mapping (e.g., "../query/permanent/_generated.py")
    pub python_mapping_output: Option<String>,

    /// Python module name for imports (e.g., "data_processor._rust")
    pub python_module_name: String,

    /// Transform registry type for generated code (e.g., "crate::transform_registry::TransformRegistry")
    pub transform_registry_type: String,

    /// Additional header content to prepend to generated Rust code (e.g., helper functions)
    pub additional_rust_header: Option<String>,

    /// Optional: Output path for generated transform_registry.rs (e.g., "src/transform_registry.rs")
    pub transform_registry_output: Option<String>,

    /// Optional: Python transforms module name for PyTransformRegistry (e.g., "data_processor.transforms")
    /// Only used if transform_registry_output is specified
    pub python_transforms_module: Option<String>,

    /// Optional: Output path for generated python_bindings.rs (e.g., "src/python_bindings.rs")
    pub python_bindings_output: Option<String>,

    /// Optional: List of transform functions to register in Python module (e.g., ["build_segment_index", "extract_from_hl7_segment"])
    /// These should be #[pyfunction] annotated functions from the transforms module
    pub transform_functions: Option<Vec<String>>,

    /// Optional: Output path for generated lib.rs (e.g., "src/lib.rs")
    pub lib_rs_output: Option<String>,

    /// Optional: Dependency exports for lib.rs generation
    /// Maps dependency name to list of items to re-export
    /// Example: [("hl7utils", vec!["entity", "Segment", "FieldPath"])]
    pub dependency_exports: Option<Vec<(String, Vec<String>)>>,

    /// Optional: Python dependency imports for __init__.py generation
    /// Maps (dependency_name, module_path) to list of items to import
    /// Example: [("hl7utils", "hl7utils.transforms", vec!["TransformRegistry", "get_registry"])]
    pub python_dependency_imports: Option<Vec<(String, String, Vec<String>)>>,

    /// Optional: Output path for Python _rust shim module (e.g., "src/data_processor/_rust.py")
    /// This generates a Python file that re-exports all entities from the _rust extension module
    pub python_rust_shim_output: Option<String>,

    /// Optional: Output path for Python package __init__.py (e.g., "data_processor/__init__.py")
    /// This generates the main package __init__.py that re-exports from _rust extension module
    pub python_package_init_output: Option<String>,

    /// Optional: Rust transform definitions from nomnom.yaml
    /// Maps transform name to transform definition (args, return_type, code)
    pub rust_transforms: Option<std::collections::HashMap<String, crate::codegen::project_config::RustTransformDef>>,
}

/// Generate all code artifacts from entity configurations.
///
/// This is the main entry point for build scripts. It orchestrates:
/// 1. Loading entity definitions from YAML
/// 2. Generating Rust entity code
/// 3. Generating PyO3 Python bindings
/// 4. Generating Diesel schema/models/operations (if configured)
/// 5. Generating Python core mapping (if configured)
///
/// # Example
///
/// ```rust,no_run
/// use nomnom::codegen::{GenerationConfig, generate_all_from_config};
///
/// fn main() {
///     let config = GenerationConfig {
///         config_dir: "../../../config/entities".to_string(),
///         rust_output: "src/generated.rs".to_string(),
///         pyo3_bindings_output: "src/generated_bindings.rs".to_string(),
///         diesel_schema_output: Some("src/schema.rs".to_string()),
///         diesel_models_output: Some("src/models/mod.rs".to_string()),
///         diesel_operations_output: Some("src/db/generated_operations.rs".to_string()),
///         diesel_pyo3_output: Some("src/python/generated_persistence.rs".to_string()),
///         python_mapping_output: Some("../query/permanent/_generated.py".to_string()),
///         python_module_name: "my_app._rust".to_string(),
///         transform_registry_type: "crate::transform_registry::TransformRegistry".to_string(),
///         additional_rust_header: Some("fn my_helper() {}\n".to_string()),
///     };
///
///     generate_all_from_config(&config).expect("Code generation failed");
/// }
/// ```
pub fn generate_all_from_config(config: &GenerationConfig) -> Result<(), Box<dyn Error>> {
    println!("cargo:rerun-if-changed={}", config.config_dir);

    // 1. Load all entities from YAML configs
    let all_entities = load_entities(&config.config_dir)?;
    println!("Loaded {} entities from {}:", all_entities.len(), config.config_dir);
    for entity in &all_entities {
        println!("  - {} (source: {:?})", entity.name, entity.source_type);
    }

    // 2. Generate Rust entity code
    println!("Generating Rust structs for {} entities", all_entities.len());
    let rust_config = RustCodegenConfig {
        transform_registry_type: Some(config.transform_registry_type.clone()),
    };

    let mut rust_output = fs_utils::create_file(&config.rust_output)?;

    // Write header with imports (format-agnostic)
    writeln!(rust_output, "// Auto-generated from YAML entity specifications\n")?;
    writeln!(rust_output, "use nomnom::{{Entity, FieldValue, EntityError, Context, IntoOptionString}};")?;
    writeln!(rust_output, "use serde::{{Serialize, Deserialize}};")?;
    writeln!(rust_output, "use std::collections::HashMap;")?;
    writeln!(rust_output, "use pyo3::prelude::*;")?;
    writeln!(rust_output, "use sha1::{{Sha1, Digest}};")?;
    writeln!(rust_output, "use regex::Regex;\n")?;

    // Add any additional header content (e.g., helper functions)
    if let Some(header) = &config.additional_rust_header {
        writeln!(rust_output, "{}", header)?;
    }

    // Generate transform functions (if provided)
    if let Some(ref transforms) = config.rust_transforms {
        use crate::codegen::rust_codegen::generate_transform_functions;
        generate_transform_functions(&mut rust_output, transforms, &all_entities)?;
    }

    generate_rust_code(&mut rust_output, &all_entities, &rust_config)?;

    println!("cargo:rerun-if-changed={}", config.rust_output);
    println!("✓ Rust entity generation completed");

    // 3. Generate PyO3 Python bindings
    println!("Generating PyO3 bindings");
    let pyo3_config = PyO3Config {
        transform_registry_type: config.transform_registry_type.clone(),
        field_value_type: "crate::entity::FieldValue".to_string(),
        generate_database_constructors: false,
        additional_imports: vec![],
    };

    let mut pyo3_output = fs_utils::create_file(&config.pyo3_bindings_output)?;
    generate_pyo3_bindings(&mut pyo3_output, &all_entities, &pyo3_config)?;

    println!("cargo:rerun-if-changed={}", config.pyo3_bindings_output);
    println!("✓ PyO3 bindings generation completed");

    // 4. Generate Diesel artifacts (if configured)
    if config.diesel_schema_output.is_some()
        || config.diesel_models_output.is_some()
        || config.diesel_operations_output.is_some()
        || config.diesel_pyo3_output.is_some()
    {
        println!("Generating Diesel artifacts");
        use crate::codegen::diesel;

        if let Some(schema_path) = &config.diesel_schema_output {
            diesel::generate_schema(&all_entities, Path::new(schema_path), &config.config_dir)?;
            println!("  ✓ Diesel schema: {}", schema_path);
        }

        if let Some(models_path) = &config.diesel_models_output {
            diesel::generate_models(&all_entities, Path::new(models_path), &config.config_dir)?;
            println!("  ✓ Diesel models: {}", models_path);
        }

        if let Some(operations_path) = &config.diesel_operations_output {
            diesel::generate_operations(&all_entities, Path::new(operations_path), &config.config_dir)?;
            println!("  ✓ Diesel operations: {}", operations_path);
        }

        if let Some(pyo3_path) = &config.diesel_pyo3_output {
            diesel::generate_pyo3_persistence(&all_entities, Path::new(pyo3_path), &config.config_dir)?;
            println!("  ✓ Diesel PyO3 persistence: {}", pyo3_path);
        }

        println!("✓ Diesel generation completed");
    }

    // 5. Generate Python core mapping (if configured)
    if let Some(mapping_path) = &config.python_mapping_output {
        println!("Generating Python core mapping");

        // Detect permanent entities (those with persistence config)
        let permanent_entities = detect_permanent_entities(&config.config_dir)?;
        println!("Found {} permanent entities", permanent_entities.len());

        generate_python_core_mapping(
            &permanent_entities,
            mapping_path,
            &config.python_module_name,
        )?;

        println!("✓ Python core mapping generated at {}", mapping_path);
    }

    // 6. Generate transform_registry.rs wrapper (if configured)
    if let Some(registry_path) = &config.transform_registry_output {
        println!("Generating transform_registry.rs wrapper");

        let transforms_module = config.python_transforms_module.as_deref()
            .unwrap_or(&config.python_module_name);

        let registry_code = generate_transform_registry_wrapper(transforms_module);

        let mut file = fs_utils::create_file(registry_path)?;
        file.write_all(registry_code.as_bytes())?;

        println!("✓ Transform registry wrapper generated at {}", registry_path);
    }

    // 7. Generate python_bindings.rs (if configured)
    if let Some(bindings_path) = &config.python_bindings_output {
        println!("Generating python_bindings.rs");

        // Extract module name from python_module_name (e.g., "data_processor._rust" -> "_rust")
        let module_name = config.python_module_name.split('.').last()
            .unwrap_or("_rust");

        let transform_funcs = config.transform_functions.as_deref().unwrap_or(&[]);

        let bindings_code = generate_python_bindings(module_name, transform_funcs);

        let mut file = fs_utils::create_file(bindings_path)?;
        file.write_all(bindings_code.as_bytes())?;

        println!("✓ Python bindings generated at {}", bindings_path);
    }

    // 8. Generate lib.rs (if configured)
    if let Some(lib_path) = &config.lib_rs_output {
        println!("Generating lib.rs");

        let lib_code = generate_lib_rs_full(config);

        let mut file = fs_utils::create_file(lib_path)?;
        file.write_all(lib_code.as_bytes())?;

        println!("✓ lib.rs generated at {}", lib_path);
    }

    // 9. Generate Python _rust shim module (if configured)
    if let Some(shim_path) = &config.python_rust_shim_output {
        println!("Generating Python _rust shim module");

        // Extract the extension module name from python_module_name (e.g., "data_processor._rust" -> "_rust")
        let extension_module = config.python_module_name.split('.').last()
            .unwrap_or("_rust");

        let shim_code = generate_python_rust_shim(&all_entities, extension_module);

        let mut file = fs_utils::create_file(shim_path)?;
        file.write_all(shim_code.as_bytes())?;

        println!("✓ Python _rust shim generated at {}", shim_path);
    }

    // 10. Generate Python package __init__.py (if configured)
    if let Some(init_path) = &config.python_package_init_output {
        println!("Generating Python package __init__.py");

        // Extract package name from python_module_name (e.g., "data_processor._rust" -> "data_processor")
        let package_name = config.python_module_name.split('.').next()
            .unwrap_or("data_processor");

        let extension_module = config.python_module_name.split('.').last()
            .unwrap_or("_rust");

        let init_code = generate_python_package_init(package_name, extension_module);

        let mut file = fs_utils::create_file(init_path)?;
        file.write_all(init_code.as_bytes())?;

        println!("✓ Python package __init__.py generated at {}", init_path);
    }

    println!("✓ All code generation completed successfully");
    Ok(())
}

/// Detect entities with persistence configuration (permanent entities).
///
/// Scans YAML files for entities that have a `persistence` section,
/// indicating they should be persisted to a database.
fn detect_permanent_entities(config_dir: &str) -> Result<Vec<EntityDef>, Box<dyn Error>> {
    use serde::Deserialize;

    #[derive(Deserialize)]
    struct EntityWithPersistence {
        name: String,
        persistence: Option<serde_yaml::Value>,
    }

    #[derive(Deserialize)]
    struct EntityYaml {
        entity: Option<EntityWithPersistence>,
    }

    let entities_dir = Path::new(config_dir);
    let mut permanent_entities = Vec::new();

    if entities_dir.exists() && entities_dir.is_dir() {
        for entry in fs::read_dir(entities_dir)? {
            let entry = entry?;
            let path = entry.path();

            // Only process .yaml and .yml files
            if let Some(ext) = path.extension() {
                if (ext == "yaml" || ext == "yml") && path.file_name().unwrap() != "README.md" {
                    let yaml_content = fs::read_to_string(&path)?;
                    let yaml: EntityYaml = serde_yaml::from_str(&yaml_content)?;

                    // Only process entities with persistence section
                    if let Some(entity) = yaml.entity {
                        if entity.persistence.is_some() {
                            println!("  - {}", entity.name);
                            permanent_entities.push(EntityDef {
                                name: entity.name,
                                source_type: "Permanent".to_string(),
                                repetition: None,
                                parent: None,
                                parents: vec![],
                                repeated_for: None,
                                fields: vec![],
                                doc: None,
                                persistence: None,  // Will be loaded properly by load_entities
                                database: None,
                                derivation: None,
                                is_abstract: false,
                                extends: None,
                                abstract_implementations: None,
                                serialization: vec![],
                                prefix: None,
                            });
                        }
                    }
                }
            }
        }
    }

    Ok(permanent_entities)
}

/// Generate transform_registry.rs wrapper that delegates to nomnom::PyTransformRegistry.
///
/// This generates a thin wrapper around nomnom's PyTransformRegistry with a configured
/// Python transforms module name.
///
/// # Arguments
/// * `python_transforms_module` - Python module containing TRANSFORM_REGISTRY (e.g., "data_processor.transforms")
///
/// # Returns
/// Generated transform_registry.rs content
fn generate_transform_registry_wrapper(python_transforms_module: &str) -> String {
    format!(r#"//! Transform registry for entity field extraction.
//!
//! This file is auto-generated by nomnom.
//! DO NOT EDIT MANUALLY - changes will be overwritten.
//!
//! This module provides a TransformRegistry that implements the interface expected
//! by nomnom-generated code, wrapping PyTransformRegistry for Python transform calls.

use pyo3::prelude::*;
use std::collections::HashMap;
use serde_json::Value;

/// Transform Registry
///
/// Provides transform function calls that automatically acquire Python GIL.
/// This implements the interface expected by nomnom's generated code.
pub struct TransformRegistry;

impl TransformRegistry {{
    /// Create a new transform registry
    pub fn new() -> Self {{
        Self
    }}

    /// Call a transform function
    ///
    /// This method automatically acquires the Python GIL and delegates to PyTransformRegistry.
    ///
    /// # Arguments
    ///
    /// * `name` - Transform function name
    /// * `args` - Arguments as JSON Value HashMap
    ///
    /// # Returns
    ///
    /// * `Ok(Value)` - Transform succeeded, returns JSON Value (String, Array, Null, etc.)
    /// * `Err(String)` - Transform failed (error message)
    ///
    /// # Note
    ///
    /// This implements the transform registry interface expected by nomnom-generated code.
    /// It delegates to nomnom::PyTransformRegistry with the configured Python module.
    pub fn call_transform(
        &self,
        name: &str,
        args: &HashMap<String, Value>,
    ) -> Result<Value, String> {{
        Python::with_gil(|py| {{
            // Use nomnom's PyTransformRegistry with configured Python transforms module
            let py_registry = nomnom::PyTransformRegistry::new("{}");

            // Convert JSON Value args to Option<String> kwargs for Python
            let mut kwargs = HashMap::new();

            for (key, value) in args {{
                let opt_string = match value {{
                    Value::String(s) => Some(s.clone()),
                    Value::Null => None,
                    Value::Number(n) => Some(n.to_string()),
                    Value::Bool(b) => Some(b.to_string()),
                    _ => Some(value.to_string()),
                }};
                kwargs.insert(key.clone(), opt_string);
            }}

            // Try calling as a list-valued transform first (for transforms like extract_segments)
            match py_registry.call_transform_list(py, name, kwargs.clone()) {{
                Ok(list_result) => {{
                    // Transform succeeded and returned a list (empty or not)
                    let values: Vec<Value> = list_result.into_iter()
                        .map(Value::String)
                        .collect();
                    Ok(Value::Array(values))
                }}
                Err(_list_err) => {{
                    // List extraction failed - try as string-valued transform
                    let result = py_registry.call_transform(py, name, kwargs)
                        .map_err(|e| format!("Python transform '{{}}' failed: {{}}", name, e))?;

                    // Convert Option<String> to JSON Value
                    match result {{
                        Some(s) => Ok(Value::String(s)),
                        None => Ok(Value::Null),
                    }}
                }}
            }}
        }})
    }}
}}

impl Default for TransformRegistry {{
    fn default() -> Self {{
        Self::new()
    }}
}}

// Implement the nomnom trait for the PyO3 TransformRegistry
impl nomnom::runtime::transform_registry::TransformRegistry for TransformRegistry {{
    fn call_transform(
        &self,
        name: &str,
        args: &HashMap<String, Value>,
    ) -> Result<Value, String> {{
        // Delegate to the struct's method
        self.call_transform(name, args)
    }}
}}
"#, python_transforms_module)
}

/// Generate python_bindings.rs that registers entities and transform functions.
///
/// This generates the PyO3 module definition that registers all entities and functions.
///
/// # Arguments
/// * `module_name` - Python module name (e.g., "_rust")
/// * `transform_functions` - List of transform function names to register
///
/// # Returns
/// Generated python_bindings.rs content
fn generate_python_bindings(module_name: &str, transform_functions: &[String]) -> String {
    let mut code = String::new();

    // Header
    code.push_str("//! Python bindings for entities using PyO3.\n");
    code.push_str("//!\n");
    code.push_str("//! This file is auto-generated by nomnom.\n");
    code.push_str("//! DO NOT EDIT MANUALLY - changes will be overwritten.\n");
    code.push_str("//!\n");
    code.push_str("//! This module registers all entities and transform functions for Python.\n\n");

    // Allow non-local definitions
    code.push_str("// Allow non-local definitions from PyO3 macro expansion (expected in Rust 1.80+)\n");
    code.push_str("#![allow(non_local_definitions)]\n\n");

    // Imports
    code.push_str("use pyo3::prelude::*;\n\n");

    // Include generated bindings
    code.push_str("// Include auto-generated Python bindings for entities\n");
    code.push_str("// This file is generated by build.rs from YAML entity specifications\n");
    code.push_str("include!(\"generated_bindings.rs\");\n\n");

    // Python module definition
    code.push_str("/// Python module definition\n");
    code.push_str(&format!("/// Note: The function name must match the lib name in Cargo.toml (lib.name = \"{}\")\n", module_name));
    code.push_str("#[pymodule]\n");
    code.push_str(&format!("fn {}(_py: Python, m: &PyModule) -> PyResult<()> {{\n", module_name));

    // Register all entities
    code.push_str("    // Register all entities (transient + permanent cores)\n");
    code.push_str("    // This is auto-generated by build.rs from YAML entity definitions\n");
    code.push_str("    register_all_entities(m)?;\n\n");

    // Register Database class
    code.push_str("    // Register Database class for persistence\n");
    code.push_str("    m.add_class::<crate::python::PyDatabase>()?;\n\n");

    // Register persistence functions
    code.push_str("    // Register get_or_create persistence functions\n");
    code.push_str("    crate::python::register_persistence_functions(m)?;\n\n");

    // Register transform functions
    if !transform_functions.is_empty() {
        code.push_str("    // Register transform functions from hl7utils crate\n");
        code.push_str("    // These are the Rust implementations of transforms that are also exposed to Python\n");
        for func_name in transform_functions {
            code.push_str(&format!("    m.add_function(wrap_pyfunction!(hl7utils::transforms::{}, m)?)?;\n", func_name));
        }
        code.push_str("\n");
    }

    code.push_str("    Ok(())\n");
    code.push_str("}\n");

    code
}

/// Generate lib.rs that declares all modules and re-exports.
///
/// This generates the main library file with module declarations and re-exports.
///
/// # Returns
/// Generated lib.rs content
fn generate_lib_rs_full(config: &GenerationConfig) -> String {
    let mut code = String::new();

    // Header with documentation
    code.push_str("//! Auto-generated library module.\n");
    code.push_str("//!\n");
    code.push_str("//! This file is auto-generated by nomnom.\n");
    code.push_str("//! DO NOT EDIT MANUALLY - changes will be overwritten.\n");
    code.push_str("//!\n");
    code.push_str("//! High-performance data transformation library with:\n");
    code.push_str("//! - Fast, safe data extraction\n");
    code.push_str("//! - Automatic code generation from YAML specifications\n");
    code.push_str("//! - Python bindings via PyO3\n\n");

    // Transform registry module declaration
    code.push_str("pub mod transform_registry;\n\n");

    // Re-export PyTransformRegistry from nomnom
    code.push_str("// Re-export PyTransformRegistry from nomnom\n");
    code.push_str("pub use nomnom::PyTransformRegistry;\n\n");

    // Re-export infrastructure from dependencies (data-driven from YAML)
    if let Some(dep_exports) = &config.dependency_exports {
        for (dep_name, exports) in dep_exports {
            if exports.is_empty() {
                continue;
            }
            code.push_str(&format!("// Re-export from {}\n", dep_name));
            code.push_str(&format!("pub use {}::{{", dep_name));

            // Handle single vs multiple exports
            if exports.len() == 1 {
                code.push_str(&exports[0]);
            } else {
                code.push('\n');
                for (i, export) in exports.iter().enumerate() {
                    code.push_str(&format!("    {}", export));
                    if i < exports.len() - 1 {
                        code.push_str(",\n");
                    } else {
                        code.push('\n');
                    }
                }
            }
            code.push_str("};\n\n");
        }
    }

    // Diesel ORM modules
    code.push_str("// Diesel ORM (database persistence)\n");
    code.push_str("pub mod schema;\n");
    code.push_str("pub mod models;\n");
    code.push_str("pub mod db {\n");
    code.push_str("    //! Database connection management and operations\n");
    code.push_str("    //!\n");
    code.push_str("    //! This module re-exports nomnom's Diesel runtime infrastructure and provides\n");
    code.push_str("    //! generated database operations from YAML.\n\n");
    code.push_str("    // Re-export nomnom's generic Diesel infrastructure\n");
    code.push_str("    pub use nomnom::diesel_runtime::{\n");
    code.push_str("        Database, DatabaseConfig, Pool, PooledConnection,\n");
    code.push_str("        GetOrCreate, BulkInsert,\n");
    code.push_str("    };\n\n");
    code.push_str("    // Database operations module\n");
    code.push_str("    pub mod operations {\n");
    code.push_str("        //! Database operations (get_or_create, bulk insert, etc.)\n");
    code.push_str("        //!\n");
    code.push_str("        //! This module re-exports nomnom's generic operation traits.\n");
    code.push_str("        //! Entity-specific implementations are auto-generated in generated_operations.rs.\n\n");
    code.push_str("        // Re-export nomnom's generic traits\n");
    code.push_str("        pub use nomnom::diesel_runtime::{GetOrCreate, BulkInsert};\n\n");
    code.push_str("        // GetOrCreate implementations are auto-generated in src/db/generated_operations.rs\n");
    code.push_str("        // This is generated by build.rs from entity YAML persistence configs\n");
    code.push_str("    }\n\n");
    code.push_str("    pub mod generated_operations;\n");
    code.push_str("}\n\n");

    // Python bindings modules (conditional based on YAML config)
    if config.python_bindings_output.is_some() {
        code.push_str("// Python bindings for Rust transforms\n");
        code.push_str("pub mod python_bindings;\n");
    }
    code.push_str("pub mod python {\n");
    code.push_str("    //! PyO3 bindings for database operations\n\n");
    code.push_str("    pub mod database {\n");
    code.push_str("        //! PyO3 wrapper for Database connection pool\n");
    code.push_str("        //!\n");
    code.push_str("        //! This module re-exports nomnom's PyDatabase implementation.\n\n");
    code.push_str("        // Re-export nomnom's generic PyDatabase (requires python-bridge feature)\n");
    code.push_str("        pub use nomnom::diesel_runtime::PyDatabase;\n");
    code.push_str("    }\n\n");
    code.push_str("    pub mod generated_persistence;\n\n");
    code.push_str("    pub use database::PyDatabase;\n");
    code.push_str("    pub use generated_persistence::*;\n");
    code.push_str("}\n\n");

    // Generated entities
    code.push_str("// Generated entities (included at compile time)\n");
    code.push_str("// This file is generated by build.rs\n");
    code.push_str("pub mod generated;\n");
    code.push_str("pub use generated::*;\n");

    code
}

/// Generate Python _rust shim module that re-exports all entities from the extension module.
///
/// This creates a Python file that imports everything from the _rust extension module
/// installed to site-packages by maturin, making it accessible via the package namespace.
///
/// # Arguments
///
/// * `entities` - List of all entities to include in exports
/// * `extension_module` - Name of the extension module (e.g., "_rust")
///
/// # Returns
///
/// Python code as a String
fn generate_python_rust_shim(entities: &[EntityDef], extension_module: &str) -> String {
    let mut code = String::new();

    // Header
    code.push_str("\"\"\"\n");
    code.push_str("Python shim for _rust extension module.\n\n");
    code.push_str("This file re-exports all entities from the _rust extension module\n");
    code.push_str("installed to site-packages by maturin.\n\n");
    code.push_str("Auto-generated by nomnom - DO NOT EDIT MANUALLY\n");
    code.push_str("\"\"\"\n\n");

    // Import everything from extension module
    code.push_str(&format!("# Import everything from the {} extension module\n", extension_module));
    code.push_str(&format!("from {} import *\n\n", extension_module));

    // Explicitly re-export commonly used classes for IDE autocomplete
    code.push_str("# Explicitly re-export commonly used classes for IDE autocomplete\n");
    code.push_str(&format!("from {} import (\n", extension_module));

    // Group entities by type (exclude abstract entities)
    let root_entities: Vec<_> = entities.iter()
        .filter(|e| e.source_type == "root" && !e.is_abstract)
        .collect();
    let derived_entities: Vec<_> = entities.iter()
        .filter(|e| e.source_type == "derived" && !e.is_abstract)
        .collect();

    // Root entities
    if !root_entities.is_empty() {
        code.push_str("    # Root entities\n");
        for entity in root_entities {
            code.push_str(&format!("    {},\n", entity.name));
        }
        code.push_str("\n");
    }

    // Derived entities (cores)
    if !derived_entities.is_empty() {
        code.push_str("    # Derived entities (cores)\n");
        for entity in derived_entities {
            code.push_str(&format!("    {}Core,\n", entity.name));
        }
        code.push_str("\n");
    }

    // Always include Database
    code.push_str("    # Database\n");
    code.push_str("    Database,\n");

    code.push_str(")\n");

    code
}

/// Generate Python package __init__.py
///
/// Creates the main package __init__.py that re-exports everything from the _rust extension module.
/// This allows users to import directly from the package: `from data_processor import Diagnosis`
fn generate_python_package_init(package_name: &str, extension_module: &str) -> String {
    let mut code = String::new();

    // Header
    code.push_str(&format!("\"\"\"\n"));
    code.push_str(&format!("{} - Data transformation library\n\n", package_name));
    code.push_str("High-performance data transformation with Rust core.\n");
    code.push_str("All entities are provided by the Rust extension module.\n\n");
    code.push_str("Auto-generated by nomnom - DO NOT EDIT MANUALLY\n");
    code.push_str("\"\"\"\n\n");

    // Re-export everything from _rust extension
    code.push_str(&format!("# Re-export everything from the Rust extension module\n"));
    code.push_str(&format!("from {}.{} import *\n\n", package_name, extension_module));

    // Version
    code.push_str("__version__ = \"0.1.0\"\n");

    code
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_generation_config_creation() {
        let config = GenerationConfig {
            config_dir: "config/entities".to_string(),
            rust_output: "src/generated.rs".to_string(),
            pyo3_bindings_output: "src/generated_bindings.rs".to_string(),
            diesel_schema_output: None,
            diesel_models_output: None,
            diesel_operations_output: None,
            diesel_pyo3_output: None,
            python_mapping_output: None,
            python_module_name: "test._rust".to_string(),
            transform_registry_type: "crate::registry::Registry".to_string(),
            additional_rust_header: None,
            transform_registry_output: None,
            python_transforms_module: None,
            python_bindings_output: None,
            transform_functions: None,
            lib_rs_output: None,
            dependency_exports: None,
            python_dependency_imports: None,
            python_rust_shim_output: None,
            python_package_init_output: None,
            rust_transforms: None,
        };

        assert_eq!(config.config_dir, "config/entities");
        assert_eq!(config.python_module_name, "test._rust");
    }

    #[test]
    fn test_generate_transform_registry_wrapper() {
        let code = generate_transform_registry_wrapper("myapp.transforms");

        // Check that it includes the module name
        assert!(code.contains("myapp.transforms"));

        // Check that it uses nomnom::PyTransformRegistry
        assert!(code.contains("nomnom::PyTransformRegistry"));

        // Check that it has the required methods
        assert!(code.contains("pub fn new()"));
        assert!(code.contains("pub fn call_transform"));

        // Check that it's marked as auto-generated
        assert!(code.contains("auto-generated by nomnom"));
        assert!(code.contains("DO NOT EDIT MANUALLY"));
    }

    #[test]
    fn test_generate_python_bindings() {
        let funcs = vec!["build_segment_index".to_string(), "extract_from_hl7_segment".to_string()];
        let code = generate_python_bindings("_rust", &funcs);

        // Check that it includes the module name
        assert!(code.contains("fn _rust(_py: Python, m: &PyModule)"));

        // Check that it registers entities
        assert!(code.contains("register_all_entities(m)?"));

        // Check that it registers Database
        assert!(code.contains("PyDatabase"));

        // Check that it registers persistence functions
        assert!(code.contains("register_persistence_functions(m)?"));

        // Check that it registers transform functions
        assert!(code.contains("build_segment_index"));
        assert!(code.contains("extract_from_hl7_segment"));
        assert!(code.contains("hl7utils::transforms::"));

        // Check that it's marked as auto-generated
        assert!(code.contains("auto-generated by nomnom"));
        assert!(code.contains("DO NOT EDIT MANUALLY"));
    }

    #[test]
    fn test_generate_lib_rs_full() {
        // Create minimal GenerationConfig with dependency exports
        let config = GenerationConfig {
            config_dir: "config/entities".to_string(),
            rust_output: "src/generated.rs".to_string(),
            pyo3_bindings_output: "src/generated_bindings.rs".to_string(),
            diesel_schema_output: None,
            diesel_models_output: None,
            diesel_operations_output: None,
            diesel_pyo3_output: None,
            python_mapping_output: None,
            python_module_name: "test._rust".to_string(),
            transform_registry_type: "crate::transform_registry::TransformRegistry".to_string(),
            additional_rust_header: None,
            transform_registry_output: None,
            python_transforms_module: None,
            python_bindings_output: None,
            transform_functions: None,
            lib_rs_output: Some("src/lib.rs".to_string()),
            dependency_exports: Some(vec![
                ("hl7utils".to_string(), vec!["entity".to_string(), "Segment".to_string()]),
            ]),
            python_dependency_imports: None,
            python_rust_shim_output: None,
            python_package_init_output: None,
            rust_transforms: None,
        };

        let code = generate_lib_rs_full(&config);

        // Check module declarations
        assert!(code.contains("pub mod transform_registry;"));
        assert!(code.contains("pub mod schema;"));
        assert!(code.contains("pub mod models;"));
        assert!(code.contains("pub mod db {"));  // Inline module, not separate file
        // python_bindings is not included since python_bindings_output is None
        assert!(!code.contains("pub mod python_bindings;"));
        assert!(code.contains("pub mod python {"));  // Inline module
        assert!(code.contains("pub mod generated;"));

        // Check re-exports (now data-driven from dependency_exports)
        assert!(code.contains("pub use nomnom::PyTransformRegistry;"));
        assert!(code.contains("pub use hl7utils::{"));
        assert!(code.contains("entity"));
        assert!(code.contains("Segment"));
        assert!(code.contains("pub use generated::*;"));

        // Check that it's marked as auto-generated
        assert!(code.contains("auto-generated by nomnom"));
        assert!(code.contains("DO NOT EDIT MANUALLY"));
    }
}
