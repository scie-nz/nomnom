//! Diesel model generation from entity YAML configurations.

use std::io::Write;
use std::path::Path;
use std::error::Error;
use crate::codegen::fs_utils;
use serde::Deserialize;

#[derive(Deserialize)]
struct DatabaseConfig {
    conformant_table: String,
    conformant_id_column: String,
    #[serde(default)]
    autogenerate_conformant_id: bool,
}

#[derive(Deserialize)]
struct PrimaryKeyConfig {
    name: String,
    #[serde(rename = "type")]
    key_type: String,
    #[serde(default)]
    autogenerate: bool,
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
    writeln!(output, "use bigdecimal::{{BigDecimal, FromPrimitive}};")?;
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

                        // Generate New* struct for insertion
                        // This excludes auto-generated primary key fields
                        writeln!(output, "#[derive(Debug, Clone, Insertable)]")?;
                        writeln!(output, "#[diesel(table_name = {})]", db_config.conformant_table)?;
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
                            // Skip the conformant ID if it's auto-generated
                            if db_config.autogenerate_conformant_id {
                                if field.name == db_config.conformant_id_column {
                                    continue;
                                }
                            }

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

    // Generate From trait implementations for Core -> NewX conversions
    writeln!(output, "\n// From trait implementations for Core -> New conversions\n")?;

    for entity in entities {
        let yaml_path = format!("{}/{}.yaml", config_dir, entity.name.to_lowercase());
        if let Ok(yaml_content) = std::fs::read_to_string(&yaml_path) {
            if let Ok(yaml) = serde_yaml::from_str::<EntityWrapper>(&yaml_content) {
                if let Some(persistence) = yaml.entity.persistence {
                    if let Some(db_config) = persistence.database {
                        let core_type = format!("crate::generated::{}Core", entity.name);
                        let new_type = format!("New{}", entity.name);

                        writeln!(output, "impl From<&{}> for {} {{", core_type, new_type)?;
                        writeln!(output, "    fn from(core: &{}) -> Self {{", core_type)?;
                        writeln!(output, "        Self {{")?;

                        // Handle primary key if not auto-generated
                        let has_autogen_pk = if let Some(ref pk_config) = persistence.primary_key {
                            let is_autogen = pk_config.key_type == "Integer";
                            if !is_autogen {
                                writeln!(output, "            {}: core.{}.clone(),", pk_config.name, pk_config.name)?;
                            }
                            is_autogen
                        } else {
                            false
                        };

                        // Convert each field
                        for field in &persistence.field_overrides {
                            // Skip the conformant ID if it's auto-generated
                            if db_config.autogenerate_conformant_id {
                                if field.name == db_config.conformant_id_column {
                                    continue;
                                }
                            }

                            match field.field_type.as_str() {
                                "Float" => {
                                    // Convert f64 to BigDecimal
                                    if field.nullable {
                                        writeln!(output, "            {}: core.{}.and_then(BigDecimal::from_f64),",
                                            field.name, field.name)?;
                                    } else {
                                        writeln!(output, "            {}: BigDecimal::from_f64(core.{}).unwrap_or_else(|| BigDecimal::from(0)),",
                                            field.name, field.name)?;
                                    }
                                },
                                "Integer" => {
                                    // Convert i64 to i32
                                    if field.nullable {
                                        writeln!(output, "            {}: core.{}.map(|v| v as i32),",
                                            field.name, field.name)?;
                                    } else {
                                        writeln!(output, "            {}: core.{} as i32,",
                                            field.name, field.name)?;
                                    }
                                },
                                _ => {
                                    // String, Boolean, DateTime
                                    if field.nullable {
                                        // Database field is nullable - direct clone
                                        writeln!(output, "            {}: core.{}.clone(),",
                                            field.name, field.name)?;
                                    } else {
                                        // Database field is non-nullable - unwrap Option or clone
                                        // Assume core field might be Option<String>, unwrap with default
                                        writeln!(output, "            {}: core.{}.clone().unwrap_or_default(),",
                                            field.name, field.name)?;
                                    }
                                }
                            }
                        }

                        writeln!(output, "        }}")?;
                        writeln!(output, "    }}")?;
                        writeln!(output, "}}\n")?;
                    }
                }
            }
        }
    }

    // Generate From implementations for derived entities that extend persistent entities
    // Example: From<&PrimaryCareProviderCore> for NewProvider
    writeln!(output, "\n// From trait implementations for derived entity cores -> parent model conversions\n")?;

    for entity in entities {
        // Check if this entity extends another entity
        if let Some(ref parent_name) = entity.extends {
            // Load the derived entity's YAML to get its field list
            let derived_yaml_path = format!("{}/{}.yaml", config_dir, entity.name.to_lowercase());
            let derived_field_names: std::collections::HashSet<String> =
                if let Ok(derived_yaml_content) = std::fs::read_to_string(&derived_yaml_path) {
                    if let Ok(derived_yaml) = serde_yaml::from_str::<EntityWrapper>(&derived_yaml_content) {
                        // Collect all field names from the derived entity
                        entity.fields.iter().map(|f| f.name.clone()).collect()
                    } else {
                        std::collections::HashSet::new()
                    }
                } else {
                    std::collections::HashSet::new()
                };

            // Load the parent entity's YAML to check if it has persistence
            let parent_yaml_path = format!("{}/{}.yaml", config_dir, parent_name.to_lowercase());
            if let Ok(parent_yaml_content) = std::fs::read_to_string(&parent_yaml_path) {
                if let Ok(parent_yaml) = serde_yaml::from_str::<EntityWrapper>(&parent_yaml_content) {
                    if let Some(parent_persistence) = parent_yaml.entity.persistence {
                        if let Some(parent_db_config) = parent_persistence.database {
                            // Parent has persistence, generate From implementation
                            let derived_core_type = format!("crate::generated::{}Core", entity.name);
                            let parent_new_type = format!("New{}", parent_name);

                            writeln!(output, "impl From<&{}> for {} {{", derived_core_type, parent_new_type)?;
                            writeln!(output, "    fn from(core: &{}) -> Self {{", derived_core_type)?;
                            writeln!(output, "        Self {{")?;

                            // Handle primary key if not auto-generated
                            let has_autogen_pk = if let Some(ref pk_config) = parent_persistence.primary_key {
                                let is_autogen = pk_config.key_type == "Integer";
                                if !is_autogen {
                                    writeln!(output, "            {}: core.{}.clone(),", pk_config.name, pk_config.name)?;
                                }
                                is_autogen
                            } else {
                                false
                            };

                            // Convert each field from parent's persistence config
                            for field in &parent_persistence.field_overrides {
                                // Skip the conformant ID if it's auto-generated
                                if parent_db_config.autogenerate_conformant_id {
                                    if field.name == parent_db_config.conformant_id_column {
                                        continue;
                                    }
                                }

                                // Check if this field exists in the derived entity
                                let field_exists = derived_field_names.contains(&field.name);

                                if field_exists {
                                    // Field exists in derived entity - use it
                                    match field.field_type.as_str() {
                                        "Float" => {
                                            // Convert f64 to BigDecimal
                                            if field.nullable {
                                                writeln!(output, "            {}: core.{}.and_then(BigDecimal::from_f64),",
                                                    field.name, field.name)?;
                                            } else {
                                                writeln!(output, "            {}: BigDecimal::from_f64(core.{}).unwrap_or_else(|| BigDecimal::from(0)),",
                                                    field.name, field.name)?;
                                            }
                                        },
                                        "Integer" => {
                                            // Convert i64 to i32
                                            if field.nullable {
                                                writeln!(output, "            {}: core.{}.map(|v| v as i32),",
                                                    field.name, field.name)?;
                                            } else {
                                                writeln!(output, "            {}: core.{} as i32,",
                                                    field.name, field.name)?;
                                            }
                                        },
                                        _ => {
                                            // String, Boolean, DateTime
                                            if field.nullable {
                                                // Database field is nullable - direct clone
                                                writeln!(output, "            {}: core.{}.clone(),",
                                                    field.name, field.name)?;
                                            } else {
                                                // Database field is non-nullable - unwrap Option or clone
                                                // Assume core field might be Option<String>, unwrap with default
                                                writeln!(output, "            {}: core.{}.clone().unwrap_or_default(),",
                                                    field.name, field.name)?;
                                            }
                                        }
                                    }
                                } else {
                                    // Field doesn't exist in derived entity - use default value
                                    if field.nullable {
                                        writeln!(output, "            {}: None,", field.name)?;
                                    } else {
                                        match field.field_type.as_str() {
                                            "Integer" => writeln!(output, "            {}: 0,", field.name)?,
                                            "Float" => writeln!(output, "            {}: BigDecimal::from(0),", field.name)?,
                                            "Boolean" => writeln!(output, "            {}: false,", field.name)?,
                                            _ => writeln!(output, "            {}: String::new(),", field.name)?,
                                        }
                                    }
                                }
                            }

                            writeln!(output, "        }}")?;
                            writeln!(output, "    }}")?;
                            writeln!(output, "}}\n")?;
                        }
                    }
                }
            }
        }
    }

    println!("cargo:rerun-if-changed={}", output_path.display());
    Ok(())
}
