//! Diesel model generation from entity YAML configurations.

use std::io::Write;
use std::path::Path;
use std::error::Error;
use crate::codegen::fs_utils;
use serde::Deserialize;

#[derive(Deserialize)]
struct DatabaseConfig {
    conformant_table: String,
}

#[derive(Deserialize)]
struct PrimaryKeyConfig {
    name: String,
    #[serde(rename = "type")]
    key_type: String,
}

#[derive(Deserialize)]
struct PersistenceConfig {
    database: Option<DatabaseConfig>,
    #[serde(default)]
    primary_key: Option<PrimaryKeyConfig>,
    #[serde(default)]
    field_overrides: Vec<FieldOverride>,
}

#[derive(Deserialize)]
struct FieldOverride {
    name: String,
    #[serde(rename = "type")]
    field_type: String,
    #[serde(default)]
    nullable: bool,
}

#[derive(Deserialize)]
struct EntityYaml {
    #[serde(default)]
    persistence: Option<PersistenceConfig>,
}

#[derive(Deserialize)]
struct EntityWrapper {
    entity: EntityYaml,
}

/// Generate Diesel model structs from entity field definitions.
///
/// Creates model structs annotated with Diesel's `#[derive(Queryable, Insertable)]`
/// for database operations.
///
/// # Arguments
///
/// * `entities` - Slice of entity definitions
/// * `output_path` - Path to output file (e.g., "src/models/mod.rs")
/// * `config_dir` - Path to YAML config directory
pub fn generate_models(
    entities: &[crate::codegen::EntityDef],
    output_path: &Path,
    config_dir: &str,
) -> Result<(), Box<dyn Error>> {
    let mut output = fs_utils::create_file(output_path)?;

    writeln!(output, "//! Diesel models generated from entity YAML configs\n")?;
    writeln!(output, "use diesel::prelude::*;")?;
    writeln!(output, "use serde::{{Serialize, Deserialize}};")?;
    writeln!(output, "use bigdecimal::BigDecimal;")?;
    writeln!(output, "use crate::schema::*;\n")?;

    // For each entity with persistence, generate a model struct
    for entity in entities {
        let yaml_path = format!("{}/{}.yaml", config_dir, entity.name.to_lowercase());
        if let Ok(yaml_content) = std::fs::read_to_string(&yaml_path) {
            if let Ok(yaml) = serde_yaml::from_str::<EntityWrapper>(&yaml_content) {
                if let Some(persistence) = yaml.entity.persistence {
                    if let Some(db_config) = persistence.database {
                        // Generate main struct (Queryable only - for reading from DB)
                        writeln!(output, "#[derive(Debug, Clone, Queryable, Serialize, Deserialize)]")?;
                        writeln!(output, "#[diesel(table_name = {})]", db_config.conformant_table)?;
                        writeln!(output, "pub struct {} {{", entity.name)?;

                        // If primary_key section exists, output it first
                        let has_autogen_pk = if let Some(ref pk_config) = persistence.primary_key {
                            let rust_type = match pk_config.key_type.as_str() {
                                "Integer" => "i32",
                                "String" => "String",
                                _ => "i32",
                            };
                            writeln!(output, "    pub {}: {},", pk_config.name, rust_type)?;
                            pk_config.key_type == "Integer"  // Assume Integer PKs are auto-generated
                        } else {
                            false
                        };

                        // Generate fields from field_overrides
                        for field in &persistence.field_overrides {
                            let rust_type = match field.field_type.as_str() {
                                "String" => "String",
                                "Integer" => "i32",
                                "Float" => "BigDecimal",
                                "Boolean" => "bool",
                                "DateTime" => "chrono::NaiveDateTime",
                                _ => "String",
                            };

                            let final_type = if field.nullable {
                                format!("Option<{}>", rust_type)
                            } else {
                                rust_type.to_string()
                            };

                            writeln!(output, "    pub {}: {},", field.name, final_type)?;
                        }

                        writeln!(output, "}}\n")?;

                        // Generate New* struct (for reference - not used with column-based insertion)
                        // This excludes auto-generated primary key fields
                        // NOTE: We don't derive Insertable to avoid f64 issues - use column-based insertion instead
                        writeln!(output, "#[derive(Debug, Clone)]")?;
                        writeln!(output, "pub struct New{} {{", entity.name)?;

                        // Skip primary key if it's auto-generated
                        // Otherwise include it
                        if let Some(ref pk_config) = persistence.primary_key {
                            if !has_autogen_pk {
                                let rust_type = match pk_config.key_type.as_str() {
                                    "Integer" => "i32",
                                    "String" => "String",
                                    _ => "i32",
                                };
                                writeln!(output, "    pub {}: {},", pk_config.name, rust_type)?;
                            }
                        }

                        // Generate fields from field_overrides
                        for field in &persistence.field_overrides {
                            let rust_type = match field.field_type.as_str() {
                                "String" => "String",
                                "Integer" => "i32",
                                "Float" => "BigDecimal",
                                "Boolean" => "bool",
                                "DateTime" => "chrono::NaiveDateTime",
                                _ => "String",
                            };

                            let final_type = if field.nullable {
                                format!("Option<{}>", rust_type)
                            } else {
                                rust_type.to_string()
                            };

                            writeln!(output, "    pub {}: {},", field.name, final_type)?;
                        }

                        writeln!(output, "}}\n")?;
                    }
                }
            }
        }
    }

    println!("cargo:rerun-if-changed={}", output_path.display());
    Ok(())
}
