//! PyO3 persistence bindings generation for Diesel operations.

use std::io::Write;
use std::path::Path;
use std::error::Error;
use serde::Deserialize;

use crate::codegen::fs_utils;

#[derive(Deserialize)]
struct DatabaseConfig {
    #[serde(default)]
    autogenerate_conformant_id: bool,
}

#[derive(Deserialize)]
struct FieldOverride {
    name: String,
    #[serde(rename = "type")]
    field_type: String,
    #[serde(default)]
    nullable: bool,
    #[serde(default)]
    primary_key: bool,
}

#[derive(Deserialize)]
struct PrimaryKeyConfig {
    name: String,
    #[serde(default)]
    autogenerate: bool,
}

#[derive(Deserialize)]
struct PersistenceConfig {
    #[serde(default)]
    database: Option<DatabaseConfig>,
    #[serde(default)]
    field_overrides: Vec<FieldOverride>,
    #[serde(default)]
    primary_key: Option<PrimaryKeyConfig>,
}

#[derive(Deserialize)]
struct EntityYaml {
    name: String,
    #[serde(default)]
    persistence: Option<PersistenceConfig>,
}

#[derive(Deserialize)]
struct EntityWrapper {
    entity: EntityYaml,
}

/// Generate PyO3 persistence bindings for database operations.
///
/// Creates PyO3-annotated functions that expose Diesel operations to Python,
/// enabling Python code to call Rust database operations.
///
/// # Arguments
///
/// * `entities` - Slice of entity definitions
/// * `output_path` - Path to output file (e.g., "src/python/generated_persistence.rs")
/// * `config_dir` - Path to YAML config directory
pub fn generate_pyo3_persistence(
    entities: &[crate::codegen::EntityDef],
    output_path: &Path,
    config_dir: &str,
) -> Result<(), Box<dyn Error>> {
    let mut output = fs_utils::create_file(output_path)?;

    writeln!(output, "//! Auto-generated PyO3 get_or_create methods\n")?;
    writeln!(output, "use pyo3::prelude::*;")?;
    writeln!(output, "use crate::db::operations::GetOrCreate;")?;
    writeln!(output, "use crate::models::*;")?;
    writeln!(output, "use crate::python::PyDatabase;")?;
    writeln!(output, "use crate::generated::*;\n")?;

    let mut entity_names = Vec::new();

    // Read entity YAMLs from config/entities (where persistence sections are)
    for entity in entities {
        let yaml_path = format!("{}/{}.yaml", config_dir, entity.name.to_lowercase());
        if let Ok(yaml_content) = std::fs::read_to_string(&yaml_path) {
            if let Ok(yaml) = serde_yaml::from_str::<EntityWrapper>(&yaml_content) {
                if let Some(persistence) = yaml.entity.persistence {
                    if persistence.database.is_some() {
                        let entity_name = yaml.entity.name.clone();
                        let core_name = format!("{}Core", entity_name);
                        let func_name = entity_name.to_lowercase();

                        entity_names.push(func_name.clone());

                        writeln!(output, "/// Get or create {} in database", entity_name)?;
                        writeln!(output, "#[pyfunction]")?;
                        writeln!(output, "pub fn {}_get_or_create(", func_name)?;
                        writeln!(output, "    py: Python<'_>,")?;
                        writeln!(output, "    core: &PyAny,")?;
                        writeln!(output, "    database: &PyDatabase,")?;
                        writeln!(output, ") -> PyResult<PyObject> {{")?;

                        writeln!(output, "    // Convert Core to Diesel model")?;
                        writeln!(output, "    let diesel_model = {} {{", entity_name)?;

                        // Determine if we have an auto-generated primary key
                        let has_autogen_pk = persistence.database.as_ref()
                            .map_or(false, |db| db.autogenerate_conformant_id);

                        // Add auto-generated primary key with default value if needed (from primary_key section)
                        if let Some(ref pk_config) = persistence.primary_key {
                            if pk_config.autogenerate {
                                writeln!(output, "        {}: 0,  // Auto-generated, placeholder value", pk_config.name)?;
                            }
                        }

                        // Generate field mappings
                        for field in &persistence.field_overrides {
                            // Handle primary key fields from field_overrides
                            if field.primary_key {
                                // Only use placeholder for Integer auto-increment PKs
                                let is_db_autoinc = has_autogen_pk && field.field_type == "Integer";

                                if is_db_autoinc {
                                    // Database auto-increment - use placeholder
                                    writeln!(output, "        {}: 0,  // Database auto-increment, placeholder value", field.name)?;
                                } else {
                                    // Computed or manual primary key - extract from core
                                    if field.nullable {
                                        writeln!(output, "        {}: core.getattr(\"{}\")?.extract()?,", field.name, field.name)?;
                                    } else {
                                        writeln!(output, "        {}: core.getattr(\"{}\")?.extract::<Option<String>>()?", field.name, field.name)?;
                                        writeln!(output, "            .ok_or_else(|| PyErr::new::<pyo3::exceptions::PyValueError, _>(")?;
                                        writeln!(output, "                \"{} is required\"))?,", field.name)?;
                                    }
                                }
                                continue;
                            }

                            if field.nullable {
                                writeln!(output, "        {}: core.getattr(\"{}\")?.extract()?,", field.name, field.name)?;
                            } else {
                                writeln!(output, "        {}: core.getattr(\"{}\")?.extract::<Option<String>>()?", field.name, field.name)?;
                                writeln!(output, "            .ok_or_else(|| PyErr::new::<pyo3::exceptions::PyValueError, _>(")?;
                                writeln!(output, "                \"{} is required\"))?,", field.name)?;
                            }
                        }

                        writeln!(output, "    }};\n")?;

                        writeln!(output, "    // Get connection and perform get_or_create")?;
                        writeln!(output, "    let mut conn = database.get_connection()?;")?;
                        writeln!(output, "    let result = {}::get_or_create(&mut conn, &diesel_model)", entity_name)?;
                        writeln!(output, "        .map_err(|e| PyErr::new::<pyo3::exceptions::PyRuntimeError, _>(")?;
                        writeln!(output, "            format!(\"Database error: {{}}\", e)))?;\n")?;

                        writeln!(output, "    // Convert back to PyObject (Core class)")?;
                        writeln!(output, "    let core_class = py.import(\"data_processor._rust\")?.getattr(\"{}\")?;", core_name)?;
                        writeln!(output, "    let kwargs = pyo3::types::PyDict::new(py);")?;

                        // Include primary key in return value (from primary_key section)
                        if let Some(ref pk_config) = persistence.primary_key {
                            writeln!(output, "    kwargs.set_item(\"{}\", result.{})?;", pk_config.name, pk_config.name)?;
                        }

                        // Include all fields (including primary keys from field_overrides)
                        for field in &persistence.field_overrides {
                            if field.nullable {
                                writeln!(output, "    kwargs.set_item(\"{}\", result.{})?;", field.name, field.name)?;
                            } else {
                                writeln!(output, "    kwargs.set_item(\"{}\", Some(result.{}))?;", field.name, field.name)?;
                            }
                        }

                        writeln!(output, "    let instance = core_class.call((), Some(kwargs))?;")?;
                        writeln!(output, "    Ok(instance.to_object(py))")?;
                        writeln!(output, "}}\n")?;
                    }
                }
            }
        }
    }

    // Generate registration function
    writeln!(output, "/// Register all get_or_create functions with Python module")?;
    writeln!(output, "pub fn register_persistence_functions(m: &PyModule) -> PyResult<()> {{")?;
    for func_name in &entity_names {
        writeln!(output, "    m.add_function(wrap_pyfunction!({}_get_or_create, m)?)?;", func_name)?;
    }
    writeln!(output, "    Ok(())")?;
    writeln!(output, "}}")?;

    println!("cargo:rerun-if-changed={}", output_path.display());
    println!("cargo:rerun-if-changed={}/", config_dir);
    Ok(())
}
