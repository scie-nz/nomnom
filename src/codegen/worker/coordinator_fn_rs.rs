/// Generate coordinator function for dependency-based entity processing
///
/// The coordinator processes entities level by level based on dependency graph

use crate::codegen::EntityDef;
use crate::codegen::dependency_graph::DependencyGraph;
use crate::codegen::worker::DatabaseType;
use std::collections::HashMap;
use std::error::Error;
use std::io::Write;
use std::path::Path;

/// Generate coordinator module
pub fn generate_coordinator_file(
    entities: &[EntityDef],
    dependency_graph: &DependencyGraph,
    output_dir: &Path,
    _db_type: DatabaseType,
) -> Result<(), Box<dyn Error>> {
    let file_path = output_dir.join("src/coordinator.rs");
    let mut output = std::fs::File::create(&file_path)?;

    writeln!(output, "// Auto-generated coordinator for dependency-based processing")?;
    writeln!(output, "// Processes entities level by level based on dependencies\n")?;

    writeln!(output, "use crate::entities::*;")?;
    writeln!(output, "use crate::extract::*;")?;
    writeln!(output, "use crate::persist_publish::*;")?;
    writeln!(output, "use crate::parsers;")?;
    writeln!(output, "use crate::database::DbConnection;")?;
    writeln!(output, "use crate::error::AppError;")?;
    writeln!(output, "use async_nats::jetstream;\n")?;

    // Generate coordinator function
    generate_coordinator_function(&mut output, entities, dependency_graph)?;

    Ok(())
}

/// Generate the main coordinator function
fn generate_coordinator_function(
    output: &mut std::fs::File,
    entities: &[EntityDef],
    dependency_graph: &DependencyGraph,
) -> Result<(), Box<dyn Error>> {
    writeln!(output, "/// Process a message using dependency-based entity extraction")?;
    writeln!(output, "pub async fn process_message(")?;
    writeln!(output, "    root_message: &parsers::Hl7v2MessageFileMessage,")?;
    writeln!(output, "    _raw_json: &serde_json::Value,")?;
    writeln!(output, "    conn: &mut DbConnection,")?;
    writeln!(output, "    jetstream: &jetstream::Context,")?;
    writeln!(output, ") -> Result<(), AppError> {{")?;

    // Generate entity storage maps for each level
    writeln!(output, "    // Storage for extracted entities")?;
    for level_idx in 0..dependency_graph.num_levels() {
        if let Some(level_entities) = dependency_graph.get_level(level_idx) {
            for entity_name in level_entities {
                let entity = entities.iter()
                    .find(|e| &e.name == entity_name)
                    .ok_or(format!("Entity {} not found", entity_name))?;

                if entity.is_root() || entity.is_abstract {
                    continue;
                }

                // Determine if this entity is repeated
                let is_repeated = entity.repetition.as_ref()
                    .map(|r| r.to_lowercase() == "repeated")
                    .unwrap_or(false);

                if is_repeated {
                    writeln!(output, "    let mut {}_entities: Vec<{}> = Vec::new();",
                        to_snake_case(entity_name),
                        entity_name
                    )?;
                } else {
                    writeln!(output, "    let mut {}_entity: Option<{}> = None;",
                        to_snake_case(entity_name),
                        entity_name
                    )?;
                }
            }
        }
    }

    writeln!(output)?;
    writeln!(output, "    // Process entities level by level")?;

    // Process each level
    for level_idx in 0..dependency_graph.num_levels() {
        if let Some(level_entities) = dependency_graph.get_level(level_idx) {
            writeln!(output, "\n    // Level {}: {}", level_idx, level_entities.join(", "))?;

            for entity_name in level_entities {
                generate_entity_processing(output, entity_name, entities, dependency_graph)?;
            }
        }
    }

    writeln!(output)?;
    writeln!(output, "    Ok(())")?;
    writeln!(output, "}}")?;

    Ok(())
}

/// Generate processing code for a single entity
fn generate_entity_processing(
    output: &mut std::fs::File,
    entity_name: &str,
    entities: &[EntityDef],
    dependency_graph: &DependencyGraph,
) -> Result<(), Box<dyn Error>> {
    let entity = entities.iter()
        .find(|e| &e.name == entity_name)
        .ok_or(format!("Entity {} not found", entity_name))?;

    if entity.is_root() || entity.is_abstract {
        return Ok(());
    }

    let is_repeated = entity.repetition.as_ref()
        .map(|r| r.to_lowercase() == "repeated")
        .unwrap_or(false);

    let is_persistent = entity.is_persistent(entities);

    writeln!(output, "\n    // Process {} ({})",
        entity_name,
        if is_repeated { "repeated" } else { "singleton" }
    )?;

    // Get dependencies
    let deps = dependency_graph.nodes.get(entity_name)
        .map(|node| &node.depends_on)
        .ok_or(format!("Entity {} not in dependency graph", entity_name))?;

    if is_repeated {
        // Check if entity has repeated_for - if so, loop over segments
        let entity_def = entities.iter().find(|e| &e.name == entity_name)
            .ok_or(format!("Entity {} not found", entity_name))?;

        if let Some(ref repeated_for) = entity_def.repeated_for {
            // This entity repeats for each segment in a parent field
            let parent_entity = &repeated_for.entity;
            let field_name = &repeated_for.field;
            let each_name = &repeated_for.each_known_as;

            writeln!(output, "    // Process {} (repeated for each {} segment)", entity_name, field_name)?;
            writeln!(output, "    if let Some(ref {}_entity_val) = {}_entity {{",
                to_snake_case(parent_entity), to_snake_case(parent_entity))?;
            writeln!(output, "        for {} in &{}_entity_val.{} {{",
                each_name, to_snake_case(parent_entity), field_name)?;
            writeln!(output, "            let entity_opt = extract_{}({});", to_snake_case(entity_name), each_name)?;
            writeln!(output, "            if let Ok(Some(entity)) = entity_opt {{")?;
            writeln!(output, "                {}_entities.push(entity);", to_snake_case(entity_name))?;
            writeln!(output, "            }}")?;
            writeln!(output, "        }}")?;
            writeln!(output, "    }}")?;
        } else {
            // Regular repeated entity (depends on other entities, not repeated_for)
            // Check if this entity depends on any repeated entities - if so, loop over them
            let repeated_deps: Vec<&String> = deps.iter()
                .filter(|dep| {
                    entities.iter()
                        .find(|e| &e.name == *dep)
                        .map(|e| e.repetition.as_ref()
                            .map(|r| r.to_lowercase() == "repeated")
                            .unwrap_or(false))
                        .unwrap_or(false)
                })
                .collect();

            if !repeated_deps.is_empty() {
                // Loop over the first repeated dependency (typically the main parent)
                let loop_dep = repeated_deps[0];
                writeln!(output, "    // Process {} (loop over {} entities)", entity_name, loop_dep)?;

                // Generate existence checks
                // Only check: 1) singleton dependencies exist, 2) loop dependency non-empty
                // Other repeated dependencies are optional (union pattern)
                let mut checks: Vec<String> = Vec::new();
                for dep in deps {
                    let dep_entity = entities.iter().find(|e| &e.name == dep);

                    // Skip root entities (they always exist)
                    if let Some(de) = dep_entity {
                        if de.is_root() {
                            continue;
                        }
                    }

                    let is_dep_repeated = dep_entity.map(|e| e.repetition.as_ref()
                        .map(|r| r.to_lowercase() == "repeated")
                        .unwrap_or(false))
                        .unwrap_or(false);

                    if is_dep_repeated {
                        // Only check that the loop dependency is non-empty
                        if dep == loop_dep {
                            checks.push(format!("!{}_entities.is_empty()", to_snake_case(dep)));
                        }
                        // Other repeated dependencies are optional (don't check)
                    } else {
                        // Check that singleton dependencies exist
                        checks.push(format!("{}_entity.is_some()", to_snake_case(dep)));
                    }
                }

                writeln!(output, "    if {} {{", checks.join(" && "))?;
                writeln!(output, "        for {}_item in &{}_entities {{",
                    to_snake_case(loop_dep), to_snake_case(loop_dep))?;

                // Generate extract call inside the loop
                write!(output, "            let entity = extract_{}(", to_snake_case(entity_name))?;
                let params: Vec<String> = deps.iter()
                    .map(|dep| {
                        let dep_entity = entities.iter().find(|e| &e.name == dep);

                        // Check if this is a root entity
                        if let Some(de) = dep_entity {
                            if de.is_root() {
                                return "root_message".to_string();
                            }
                        }

                        let is_dep_repeated = dep_entity.map(|e| e.repetition.as_ref()
                            .map(|r| r.to_lowercase() == "repeated")
                            .unwrap_or(false))
                            .unwrap_or(false);

                        if is_dep_repeated {
                            // Use the loop variable if this is the dependency we're looping over
                            if dep == loop_dep {
                                format!("Some({}_item)", to_snake_case(dep))
                            } else {
                                // Use first() for other repeated dependencies (returns Option)
                                format!("{}_entities.first()", to_snake_case(dep))
                            }
                        } else {
                            format!("&{}_entity.as_ref().unwrap()", to_snake_case(dep))
                        }
                    })
                    .collect();
                writeln!(output, "{})?;", params.join(", "))?;

                writeln!(output, "            if let Some(entity) = entity {{")?;
                writeln!(output, "                {}_entities.push(entity);", to_snake_case(entity_name))?;
                writeln!(output, "            }}")?;
                writeln!(output, "        }}")?;
                writeln!(output, "    }}")?;
            } else {
                // No repeated dependencies - process as singleton
                writeln!(output, "    // Process {} (no repeated dependencies)", entity_name)?;

                // Generate dependency existence checks
                let mut checks: Vec<String> = Vec::new();
                for dep in deps {
                    let dep_entity = entities.iter().find(|e| &e.name == dep);

                    // Skip root entities (they always exist)
                    if let Some(de) = dep_entity {
                        if de.is_root() {
                            continue;
                        }
                    }

                    checks.push(format!("{}_entity.is_some()", to_snake_case(dep)));
                }

                if !checks.is_empty() {
                    writeln!(output, "    if {} {{", checks.join(" && "))?;
                }

                // Generate extract call
                write!(output, "    let entity = extract_{}(", to_snake_case(entity_name))?;
                let params: Vec<String> = deps.iter()
                    .map(|dep| {
                        let dep_entity = entities.iter().find(|e| &e.name == dep);

                        // Check if this is a root entity
                        if let Some(de) = dep_entity {
                            if de.is_root() {
                                return "root_message".to_string();
                            }
                        }

                        format!("&{}_entity.as_ref().unwrap()", to_snake_case(dep))
                    })
                    .collect();
                writeln!(output, "{})?;", params.join(", "))?;

                writeln!(output, "    if let Some(entity) = entity {{")?;
                writeln!(output, "        {}_entities.push(entity);", to_snake_case(entity_name))?;
                writeln!(output, "    }}")?;

                if !checks.is_empty() {
                    writeln!(output, "    }}")?;
                }
            }
        }

    } else {
        // Singleton entity

        // Check which sources are ancillary
        let source_specs = entity.get_source_entity_specs();

        // Generate dependency existence checks (skip ancillary dependencies)
        let mut checks: Vec<String> = Vec::new();
        for dep in deps {
            let dep_entity = entities.iter().find(|e| &e.name == dep);

            // Skip root entities (they always exist)
            if let Some(de) = dep_entity {
                if de.is_root() {
                    continue;
                }
            }

            // Check if this dependency is ancillary
            let is_ancillary = source_specs.values()
                .any(|(entity_name, ancillary)| entity_name == dep && *ancillary);

            let is_dep_repeated = dep_entity.map(|e| e.repetition.as_ref()
                .map(|r| r.to_lowercase() == "repeated")
                .unwrap_or(false))
                .unwrap_or(false);

            if is_dep_repeated {
                checks.push(format!("!{}_entities.is_empty()", to_snake_case(dep)));
            } else if !is_ancillary {
                // Only check existence for non-ancillary singleton dependencies
                checks.push(format!("{}_entity.is_some()", to_snake_case(dep)));
            }
        }

        if !checks.is_empty() {
            writeln!(output, "    if {} {{", checks.join(" && "))?;
        }

        write!(output, "    {}_entity = extract_{}(",
            to_snake_case(entity_name),
            to_snake_case(entity_name)
        )?;

        let params: Vec<String> = deps.iter()
            .map(|dep| {
                let dep_entity = entities.iter().find(|e| &e.name == dep);

                // Check if this is a root entity
                if let Some(de) = dep_entity {
                    if de.is_root() {
                        // Root entities use the root_message parameter
                        return "root_message".to_string();
                    }
                }

                // Check if this dependency is ancillary
                let is_ancillary = source_specs.values()
                    .any(|(entity_name, ancillary)| entity_name == dep && *ancillary);

                let is_dep_repeated = dep_entity.map(|e| e.repetition.as_ref()
                    .map(|r| r.to_lowercase() == "repeated")
                    .unwrap_or(false))
                    .unwrap_or(false);

                if is_dep_repeated {
                    format!("&{}_entities.first().unwrap()", to_snake_case(dep))
                } else if is_ancillary {
                    // Ancillary dependencies are Option<&Entity>
                    format!("{}_entity.as_ref().map(|e| e as &_)", to_snake_case(dep))
                } else {
                    format!("&{}_entity.as_ref().unwrap()", to_snake_case(dep))
                }
            })
            .collect();
        writeln!(output, "{})?;", params.join(", "))?;

        if !checks.is_empty() {
            writeln!(output, "    }}")?;
        }
    }

    // Generate persist/publish call
    if is_repeated {
        writeln!(output, "    for entity in &{}_entities {{", to_snake_case(entity_name))?;
        if is_persistent {
            writeln!(output, "        persist_{}(entity, conn).await?;", to_snake_case(entity_name))?;
        } else {
            writeln!(output, "        publish_{}(entity, jetstream).await?;", to_snake_case(entity_name))?;
        }
        writeln!(output, "    }}")?;
    } else {
        writeln!(output, "    if let Some(ref entity) = {}_entity {{", to_snake_case(entity_name))?;
        if is_persistent {
            writeln!(output, "        persist_{}(entity, conn).await?;", to_snake_case(entity_name))?;
        } else {
            writeln!(output, "        publish_{}(entity, jetstream).await?;", to_snake_case(entity_name))?;
        }
        writeln!(output, "    }}")?;
    }

    Ok(())
}

/// Convert CamelCase to snake_case
fn to_snake_case(s: &str) -> String {
    let mut result = String::new();
    let mut prev_is_upper = false;

    for (i, ch) in s.chars().enumerate() {
        if ch.is_uppercase() {
            if i > 0 && !prev_is_upper {
                result.push('_');
            }
            result.push(ch.to_lowercase().next().unwrap());
            prev_is_upper = true;
        } else {
            result.push(ch);
            prev_is_upper = false;
        }
    }

    result
}
