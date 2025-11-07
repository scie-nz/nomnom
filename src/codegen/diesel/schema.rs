//! Diesel schema generation from entity YAML configurations.

use std::fs;
use std::io::Write;
use std::path::Path;
use std::error::Error;
use crate::codegen::fs_utils;
use serde::Deserialize;

#[derive(Deserialize)]
struct DatabaseConfig {
    #[serde(default)]
    legacy_table: Option<String>,
    conformant_table: String,
    conformant_id_column: String,
    #[serde(default)]
    unicity_fields: Vec<String>,
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
    args: Vec<serde_yaml::Value>,
    #[serde(default)]
    nullable: bool,
    #[serde(default)]
    primary_key: bool,
    #[serde(default)]
    index: bool,
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

/// Generate Diesel schema.rs from entity persistence configurations.
///
/// Creates a `schema.rs` file with Diesel `table!` macros for each entity
/// that has a persistence configuration in its YAML.
///
/// # Arguments
///
/// * `entities` - Slice of entity definitions
/// * `output_path` - Path to output file (e.g., "src/schema.rs")
/// * `config_dir` - Path to YAML config directory (e.g., "../../../config/entities")
pub fn generate_schema(
    entities: &[crate::codegen::EntityDef],
    output_path: &Path,
    config_dir: &str,
) -> Result<(), Box<dyn Error>> {
    let mut output = fs_utils::create_file(output_path)?;

    writeln!(output, "// @generated automatically by Diesel CLI.\n")?;

    // For each entity with persistence, read its YAML to get database config
    for entity in entities {
        // Read the entity YAML file to get persistence section
        let yaml_path = format!("{}/{}.yaml", config_dir, entity.name.to_lowercase());
        if let Ok(yaml_content) = std::fs::read_to_string(&yaml_path) {
            if let Ok(yaml) = serde_yaml::from_str::<EntityWrapper>(&yaml_content) {
                if let Some(persistence) = yaml.entity.persistence {
                    if let Some(db_config) = persistence.database {
                        // Generate table! macro for conformant table
                        writeln!(output, "\ndiesel::table! {{")?;
                        writeln!(output, "    {} (", db_config.conformant_table)?;

                        // Find primary key field - check primary_key section first, then field_overrides
                        let pk_field = if let Some(ref pk_config) = persistence.primary_key {
                            &pk_config.name
                        } else {
                            persistence.field_overrides.iter()
                                .find(|f| f.primary_key)
                                .map(|f| &f.name)
                                .unwrap_or(&db_config.conformant_id_column)
                        };

                        writeln!(output, "        {}", pk_field)?;
                        writeln!(output, "    ) {{")?;

                        // If primary_key section exists, output it first
                        if let Some(ref pk_config) = persistence.primary_key {
                            let diesel_type = match pk_config.key_type.as_str() {
                                "Integer" => "Integer",
                                "String" => "Varchar",
                                _ => "Integer",
                            };
                            writeln!(output, "        {} -> {},", pk_config.name, diesel_type)?;
                        }

                        // Generate columns from field_overrides
                        for field in &persistence.field_overrides {
                            let diesel_type = match field.field_type.as_str() {
                                "String" => {
                                    if let Some(_len) = field.args.first() {
                                        format!("Varchar")
                                    } else {
                                        "Text".to_string()
                                    }
                                },
                                "Integer" => "Integer".to_string(),
                                "Float" => "Float".to_string(),
                                "Boolean" => "Bool".to_string(),
                                "DateTime" => "Timestamp".to_string(),
                                _ => "Text".to_string(),
                            };

                            let type_spec = if field.nullable {
                                format!("Nullable<{}>", diesel_type)
                            } else {
                                diesel_type.clone()
                            };

                            writeln!(output, "        {} -> {},", field.name, type_spec)?;
                        }

                        writeln!(output, "    }}")?;
                        writeln!(output, "}}")?;
                    }
                }
            }
        }
    }

    println!("cargo:rerun-if-changed={}", output_path.display());
    Ok(())
}
