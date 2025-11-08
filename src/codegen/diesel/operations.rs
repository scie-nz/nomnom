//! Diesel get-or-create operations generation from entity YAML configurations.

use std::io::Write;
use std::path::Path;
use std::error::Error;
use crate::codegen::fs_utils;
use serde::Deserialize;
use std::collections::HashMap;

#[derive(Deserialize)]
struct DatabaseConfig {
    conformant_table: String,
    #[serde(default)]
    unicity_fields: Vec<String>,
}

#[derive(Deserialize)]
struct PrimaryKeyConfig {
    name: String,
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

/// Generate Diesel get-or-create database operations.
///
/// Creates database operation functions that implement idempotent get-or-create
/// logic based on entity unicity fields.
///
/// # Arguments
///
/// * `entities` - Slice of entity definitions
/// * `output_path` - Path to output file (e.g., "src/db/generated_operations.rs")
/// * `config_dir` - Path to YAML config directory
pub fn generate_operations(
    entities: &[crate::codegen::EntityDef],
    output_path: &Path,
    config_dir: &str,
) -> Result<(), Box<dyn Error>> {
    let mut output = fs_utils::create_file(output_path)?;

    writeln!(output, "//! Auto-generated GetOrCreate implementations\n")?;
    writeln!(output, "use diesel::prelude::*;")?;
    writeln!(output, "use diesel::result::Error as DieselError;")?;
    writeln!(output, "use diesel::pg::PgConnection;")?;
    writeln!(output, "use crate::models::*;")?;
    writeln!(output, "use crate::schema::*;")?;
    writeln!(output, "use crate::db::operations::GetOrCreate;\n")?;

    // For each entity with persistence, generate GetOrCreate implementation
    for entity in entities {
        let yaml_path = format!("{}/{}.yaml", config_dir, entity.name.to_lowercase());
        if let Ok(yaml_content) = std::fs::read_to_string(&yaml_path) {
            if let Ok(yaml) = serde_yaml::from_str::<EntityWrapper>(&yaml_content) {
                if let Some(persistence) = yaml.entity.persistence {
                    if let Some(db_config) = persistence.database {
                        let entity_name = &entity.name;
                        let table_name = &db_config.conformant_table;

                        writeln!(output, "// ============================================================================")?;
                        writeln!(output, "// {} - get_or_create implementation", entity_name)?;
                        writeln!(output, "// ============================================================================\n")?;

                        writeln!(output, "impl GetOrCreate for {} {{", entity_name)?;
                        writeln!(output, "    fn get_or_create(")?;
                        writeln!(output, "        conn: &mut PgConnection,")?;
                        writeln!(output, "        instance: &Self,")?;
                        writeln!(output, "    ) -> Result<Self, DieselError> {{")?;
                        writeln!(output, "        use crate::schema::{}::dsl::*;", table_name)?;
                        writeln!(output, "")?;

                        // Determine primary key for ordering (for auto-generated IDs)
                        let pk_field = if let Some(ref pk_config) = persistence.primary_key {
                            Some((&pk_config.name, pk_config.autogenerate))
                        } else {
                            None
                        };

                        // Build the query based on unicity fields
                        if db_config.unicity_fields.is_empty() {
                            writeln!(output, "        // No unicity fields - always insert")?;
                        } else {
                            // Find nullable status for each unicity field
                            let field_nullable: HashMap<String, bool> = persistence
                                .field_overrides
                                .iter()
                                .map(|f| (f.name.clone(), f.nullable))
                                .collect();

                            writeln!(output, "        // Check if exists by unicity fields")?;
                            writeln!(output, "        let mut query = {}.into_boxed();", table_name)?;
                            writeln!(output, "")?;

                            for field_name in &db_config.unicity_fields {
                                let is_nullable = field_nullable.get(field_name).copied().unwrap_or(false);

                                if is_nullable {
                                    writeln!(output, "        query = match &instance.{} {{", field_name)?;
                                    writeln!(output, "            Some(val) => query.filter({}.eq(val)),", field_name)?;
                                    writeln!(output, "            None => query.filter({}.is_null()),", field_name)?;
                                    writeln!(output, "        }};")?;
                                } else {
                                    writeln!(output, "        query = query.filter({}.eq(&instance.{}));", field_name, field_name)?;
                                }
                            }

                            writeln!(output, "")?;
                            writeln!(output, "        let existing = query.first::<{}>(conn).optional()?;", entity_name)?;
                            writeln!(output, "")?;
                            writeln!(output, "        match existing {{")?;
                            writeln!(output, "            Some(found) => Ok(found),")?;
                            writeln!(output, "            None => {{")?;
                        }

                        // Insert logic
                        writeln!(output, "                diesel::insert_into({})", table_name)?;
                        writeln!(output, "                    .values(instance)")?;
                        writeln!(output, "                    .execute(conn)?;")?;
                        writeln!(output, "")?;

                        // Return logic - query back if PK is auto-generated, otherwise clone
                        if let Some((pk_name, is_autogen)) = pk_field {
                            if is_autogen {
                                writeln!(output, "                // Query back to get auto-generated {}", pk_name)?;
                                writeln!(output, "                {}", table_name)?;
                                writeln!(output, "                    .order({}.desc())", pk_name)?;
                                writeln!(output, "                    .first::<{}>(conn)", entity_name)?;
                            } else {
                                writeln!(output, "                Ok(instance.clone())")?;
                            }
                        } else {
                            writeln!(output, "                Ok(instance.clone())")?;
                        }

                        if !db_config.unicity_fields.is_empty() {
                            writeln!(output, "            }}")?;
                            writeln!(output, "        }}")?;
                        }

                        writeln!(output, "    }}")?;
                        writeln!(output, "")?;

                        // unicity_fields method
                        writeln!(output, "    fn unicity_fields() -> Vec<&'static str> {{")?;
                        write!(output, "        vec![")?;
                        for (i, field) in db_config.unicity_fields.iter().enumerate() {
                            if i > 0 {
                                write!(output, ", ")?;
                            }
                            write!(output, "\"{}\"", field)?;
                        }
                        writeln!(output, "]")?;
                        writeln!(output, "    }}")?;
                        writeln!(output, "}}\n")?;
                    }
                }
            }
        }
    }

    println!("cargo:rerun-if-changed={}", output_path.display());
    Ok(())
}
