/// Generate extract() functions for entity extraction
///
/// Each entity gets a synchronous extract function that:
/// - Takes source entity parameters (as references)
/// - Uses transform functions to extract fields
/// - Returns Result<EntityStruct, AppError>

use crate::codegen::{EntityDef, FieldDef, ComputedFrom};
use crate::codegen::types::FieldSource;
use crate::codegen::dependency_graph::DependencyGraph;
use std::error::Error;
use std::io::Write;
use std::path::Path;

/// Generate extract functions module
pub fn generate_extract_functions_file(
    entities: &[EntityDef],
    dependency_graph: &DependencyGraph,
    output_dir: &Path,
) -> Result<(), Box<dyn Error>> {
    let extract_file = output_dir.join("src/extract.rs");
    let mut output = std::fs::File::create(&extract_file)?;

    writeln!(output, "// Auto-generated entity extraction functions")?;
    writeln!(output, "// Each entity is extracted once from its source entities\n")?;

    writeln!(output, "use crate::entities::*;")?;
    writeln!(output, "use crate::error::AppError;")?;
    writeln!(output, "use crate::transforms::*;")?;
    writeln!(output, "use crate::parsers;\n")?;

    // Generate extract function for each non-root, non-abstract entity
    for entity in entities {
        if entity.is_root() || entity.is_abstract {
            continue;
        }

        generate_extract_function(&mut output, entity, entities, dependency_graph)?;
        writeln!(output)?;
    }

    Ok(())
}

/// Generate a single extract function for an entity
fn generate_extract_function(
    output: &mut std::fs::File,
    entity: &EntityDef,
    all_entities: &[EntityDef],
    dependency_graph: &DependencyGraph,
) -> Result<(), Box<dyn Error>> {
    // Get source entities from dependency graph
    let source_entities = if let Some(node) = dependency_graph.nodes.get(&entity.name) {
        &node.depends_on
    } else {
        return Err(format!("Entity {} not found in dependency graph", entity.name).into());
    };

    // Generate doc comment
    if let Some(ref doc) = entity.doc {
        let sanitized = sanitize_doc_string(doc);
        if !sanitized.is_empty() {
            writeln!(output, "/** {} */", sanitized.replace('\n', " "))?;
        }
    }

    // Generate function signature
    write!(output, "pub fn extract_{}", to_snake_case(&entity.name))?;

    // Generate parameters
    write!(output, "(")?;

    // For entities with repeated_for, the parameter is the individual segment
    if let Some(ref repeated_for) = entity.repeated_for {
        let param_name = &repeated_for.each_known_as;
        writeln!(output, "\n    {}: &String", param_name)?;
    } else if !source_entities.is_empty() {
        // For non-repeating entities, parameters are source entities
        // Get source entity specs to check for ancillary flag
        let source_specs = entity.get_source_entity_specs();

        let params: Vec<String> = source_entities
            .iter()
            .map(|src| {
                // Check if source entity is a root entity or repeated
                let src_entity = all_entities.iter().find(|e| &e.name == src);
                let is_root = src_entity.map(|e| e.is_root()).unwrap_or(false);
                let is_repeated = src_entity.map(|e| e.repetition.as_ref()
                    .map(|r| r.to_lowercase() == "repeated")
                    .unwrap_or(false))
                    .unwrap_or(false);

                // Check if source is ancillary
                // source_specs is HashMap<alias, (entity_name, is_ancillary)>
                let is_ancillary = source_specs.values()
                    .any(|(entity_name, ancillary)| entity_name == src && *ancillary);

                let type_name = if is_root {
                    format!("parsers::{}Message", src)
                } else {
                    src.to_string()
                };

                // Repeated dependencies and ancillary dependencies are optional
                if is_repeated || is_ancillary {
                    format!("{}: Option<&{}>", to_snake_case(src), type_name)
                } else {
                    format!("{}: &{}", to_snake_case(src), type_name)
                }
            })
            .collect();
        write!(output, "\n    {}", params.join(",\n    "))?;
        writeln!(output)?;
    }
    // If no parameters needed (no dependencies), function takes no arguments
    writeln!(output, ") -> Result<Option<{}>, AppError> {{", entity.name)?;

    // For repeated_for entities, wrap the segment parameter in Option for transform compatibility
    if let Some(ref repeated_for) = entity.repeated_for {
        let param_name = &repeated_for.each_known_as;
        writeln!(output, "    // Wrap segment in Option for transform compatibility")?;
        writeln!(output, "    let {}_opt = Some({}.clone());", param_name, param_name)?;
        writeln!(output)?;
    }

    // Build map of which sources are optional (ancillary or repeated)
    let source_specs = entity.get_source_entity_specs();
    let mut optional_sources = std::collections::HashSet::new();
    for src in source_entities {
        let src_entity = all_entities.iter().find(|e| &e.name == src);
        let is_repeated = src_entity.map(|e| e.repetition.as_ref()
            .map(|r| r.to_lowercase() == "repeated")
            .unwrap_or(false))
            .unwrap_or(false);

        // Check if this entity name is marked as ancillary in source_specs
        // source_specs is HashMap<alias, (entity_name, is_ancillary)>
        let is_ancillary = source_specs.values()
            .any(|(entity_name, ancillary)| entity_name == src && *ancillary);

        if is_repeated || is_ancillary {
            optional_sources.insert(src.clone());
        }
    }

    // First pass: identify fields that need local variables (have self-references in their dependencies)
    let mut fields_needing_locals = std::collections::HashSet::new();
    for field in &entity.fields {
        if let Some(ref computed) = field.computed_from {
            // Check if any field depends on this field
            for other_field in &entity.fields {
                if let Some(ref other_computed) = other_field.computed_from {
                    for source in &other_computed.sources {
                        if source.source_name() == "self" && source.field_name() == Some(&field.name) {
                            fields_needing_locals.insert(field.name.clone());
                        }
                    }
                }
            }
        }
    }

    // Generate local variables for fields that are referenced by other fields
    for field in &entity.fields {
        if fields_needing_locals.contains(&field.name) {
            let field_code = generate_field_extraction(field, entity, source_entities, all_entities, &optional_sources)?;
            writeln!(output, "    let {} = {};", field.name, field_code)?;
        }
    }

    if !fields_needing_locals.is_empty() {
        writeln!(output)?;
    }

    // Generate struct initialization
    writeln!(output, "    let entity = {} {{", entity.name)?;

    for field in &entity.fields {
        if fields_needing_locals.contains(&field.name) {
            // Clone the local variable to avoid move errors
            writeln!(output, "        {}: {}.clone(),", field.name, field.name)?;
        } else {
            // Generate inline
            let field_code = generate_field_extraction(field, entity, source_entities, all_entities, &optional_sources)?;
            writeln!(output, "        {}: {},", field.name, field_code)?;
        }
    }

    writeln!(output, "    }};")?;

    // Generate existence check
    writeln!(output)?;
    if let Some(ref minimal_existence) = entity.minimal_existence {
        // Use explicit minimal_existence constraint
        generate_minimal_existence_check(output, entity, minimal_existence)?;
    } else {
        // No minimal_existence - check all fields from non-ancillary sources
        generate_default_existence_check(output, entity)?;
    }

    writeln!(output)?;
    writeln!(output, "    Ok(Some(entity))")?;
    writeln!(output, "}}")?;

    Ok(())
}

/// Generate code for extracting a single field
fn generate_field_extraction(
    field: &FieldDef,
    entity: &EntityDef,
    source_entities: &[String],
    all_entities: &[EntityDef],
    optional_sources: &std::collections::HashSet<String>,
) -> Result<String, Box<dyn Error>> {
    if let Some(ref computed) = field.computed_from {
        generate_computed_field(field, computed, entity, source_entities, all_entities, optional_sources)
    } else {
        // Field has no computed_from - should not happen for derived entities
        Ok("None".to_string())
    }
}

/// Generate code for a computed field based on its transform
fn generate_computed_field(
    field: &FieldDef,
    computed: &ComputedFrom,
    entity: &EntityDef,
    source_entities: &[String],
    all_entities: &[EntityDef],
    optional_sources: &std::collections::HashSet<String>,
) -> Result<String, Box<dyn Error>> {
    let transform = &computed.transform;

    match transform.as_str() {
        "copy_field" => {
            // Copy a field from a source entity
            // Format: source.field.clone()
            if let Some(first_source) = computed.sources.first() {
                let source_name = first_source.source_name();
                let source_field = first_source.field_name().ok_or("Missing field name for copy_field")?;

                if source_name == "self" {
                    // Self-reference - should not occur in extract function
                    return Err("Self-reference in extract function".into());
                }

                let source_var = to_snake_case(source_name);

                // Resolve alias to actual entity name
                // source_name might be an alias like "diagnosis", need to find actual entity name "Diagnosis"
                let actual_entity_name = entity.derivation.as_ref()
                    .and_then(|d| d.source_entities.as_ref())
                    .and_then(|se| {
                        if let serde_yaml::Value::Mapping(map) = se {
                            map.get(&serde_yaml::Value::String(source_name.to_string()))
                                .and_then(|v| {
                                    match v {
                                        // Simple string format: facility: Facility
                                        serde_yaml::Value::String(s) => Some(s.as_str()),
                                        // Detailed object format: facility: {entity: Facility, ancillary: true}
                                        serde_yaml::Value::Mapping(obj) => {
                                            obj.get(&serde_yaml::Value::String("entity".to_string()))
                                                .and_then(|e| e.as_str())
                                        }
                                        _ => None
                                    }
                                })
                        } else {
                            None
                        }
                    })
                    .unwrap_or(source_name);

                // Check if source entity is optional (repeated or ancillary)
                let is_optional_source = optional_sources.contains(actual_entity_name);
                let source_entity = all_entities.iter().find(|e| &e.name == actual_entity_name);

                if is_optional_source {
                    // Source is optional - use .and_then() or .map()
                    let field_is_nullable = source_entity
                        .and_then(|e| e.fields.iter().find(|f| f.name == source_field))
                        .map(|f| f.nullable)
                        .unwrap_or(true);

                    if field_is_nullable {
                        // Field is Option<String>, flatten with and_then
                        Ok(format!("{}.and_then(|e| e.{}.clone())", source_var, source_field))
                    } else {
                        // Field is String, map to Option<String>
                        Ok(format!("{}.map(|e| e.{}.clone())", source_var, source_field))
                    }
                } else {
                    // Source is required - normal field access
                    Ok(format!("{}.{}.clone()", source_var, source_field))
                }
            } else {
                Ok("None".to_string())
            }
        }

        "constant_value" => {
            // Return a constant value
            if let Some(args) = &computed.args {
                if let serde_yaml::Value::Mapping(map) = args {
                    if let Some(value) = map.get(&serde_yaml::Value::String("value".to_string())) {
                        // Convert YAML value to string
                        let value_str = match value {
                            serde_yaml::Value::String(s) => s.clone(),
                            serde_yaml::Value::Number(n) => n.to_string(),
                            serde_yaml::Value::Bool(b) => b.to_string(),
                            _ => value.as_str().unwrap_or("").to_string(),
                        };
                        // Generate Option<String> with the constant value
                        Ok(format!("Some(\"{}\".to_string())", value_str))
                    } else {
                        Ok("None".to_string())
                    }
                } else {
                    Ok("None".to_string())
                }
            } else {
                Ok("None".to_string())
            }
        }

        "coalesce" => {
            // Coalesce: return first non-None value using .or_else() chain
            if computed.sources.is_empty() {
                return Ok("None".to_string());
            }

            if computed.sources.len() == 1 {
                // Single source - just clone
                let source = &computed.sources[0];
                let source_name = source.source_name();
                if let Some(field_name) = source.field_name() {
                    let source_var = to_snake_case(source_name);

                    // Resolve alias to actual entity name
                    let actual_entity_name = entity.derivation.as_ref()
                        .and_then(|d| d.source_entities.as_ref())
                        .and_then(|se| {
                            if let serde_yaml::Value::Mapping(map) = se {
                                map.get(&serde_yaml::Value::String(source_name.to_string()))
                                    .and_then(|v| v.as_str())
                            } else {
                                None
                            }
                        })
                        .unwrap_or(source_name);

                    // Check if source entity is repeated (optional parameter)
                    let source_entity = all_entities.iter().find(|e| &e.name == actual_entity_name);
                    let is_repeated_source = source_entity.map(|e| e.repetition.as_ref()
                        .map(|r| r.to_lowercase() == "repeated")
                        .unwrap_or(false))
                        .unwrap_or(false);

                    if is_repeated_source {
                        // Source is optional - use .and_then() or .map()
                        let field_is_nullable = source_entity
                            .and_then(|e| e.fields.iter().find(|f| f.name == field_name))
                            .map(|f| f.nullable)
                            .unwrap_or(true);

                        if field_is_nullable {
                            return Ok(format!("{}.and_then(|e| e.{}.clone())", source_var, field_name));
                        } else {
                            return Ok(format!("{}.map(|e| e.{}.clone())", source_var, field_name));
                        }
                    } else {
                        return Ok(format!("{}.{}.clone()", source_var, field_name));
                    }
                }
            }

            // Multiple sources - build .or_else() chain
            let mut result = String::new();
            for (i, source) in computed.sources.iter().enumerate() {
                let source_name = source.source_name();
                if let Some(field_name) = source.field_name() {
                    let source_var = to_snake_case(source_name);

                    // Resolve alias to actual entity name
                    let actual_entity_name = entity.derivation.as_ref()
                        .and_then(|d| d.source_entities.as_ref())
                        .and_then(|se| {
                            if let serde_yaml::Value::Mapping(map) = se {
                                map.get(&serde_yaml::Value::String(source_name.to_string()))
                                    .and_then(|v| v.as_str())
                            } else {
                                None
                            }
                        })
                        .unwrap_or(source_name);

                    // Check if source entity is repeated (optional parameter)
                    let source_entity = all_entities.iter().find(|e| &e.name == actual_entity_name);
                    let is_repeated_source = source_entity.map(|e| e.repetition.as_ref()
                        .map(|r| r.to_lowercase() == "repeated")
                        .unwrap_or(false))
                        .unwrap_or(false);

                    let field_access = if is_repeated_source {
                        // Source is optional - use .and_then() or .map()
                        let field_is_nullable = source_entity
                            .and_then(|e| e.fields.iter().find(|f| f.name == field_name))
                            .map(|f| f.nullable)
                            .unwrap_or(true);

                        if field_is_nullable {
                            format!("{}.and_then(|e| e.{}.clone())", source_var, field_name)
                        } else {
                            format!("{}.map(|e| e.{}.clone())", source_var, field_name)
                        }
                    } else {
                        format!("{}.{}.clone()", source_var, field_name)
                    };

                    if i == 0 {
                        result = field_access;
                    } else {
                        result = format!("{}.or_else(|| {})", result, field_access);
                    }
                }
            }
            Ok(result)
        }

        "copy_field_conditional" => {
            // Conditional field selection based on condition
            if computed.sources.len() != 2 {
                return Err("copy_field_conditional requires exactly 2 sources".into());
            }

            let condition = computed.condition.as_ref()
                .ok_or("copy_field_conditional requires a condition")?;

            // Get condition field
            let cond_source = condition.field.source_name();
            let cond_field = condition.field.field_name().ok_or("Condition field missing")?;
            let cond_var = if cond_source == "self" {
                cond_field.to_string()
            } else {
                format!("{}.{}", to_snake_case(cond_source), cond_field)
            };

            // Get true and false sources
            let true_source = &computed.sources[0];
            let false_source = &computed.sources[1];

            let true_expr = if let Some(true_field) = true_source.field_name() {
                let true_src = true_source.source_name();
                if true_src == "self" {
                    format!("{}.clone()", true_field)
                } else {
                    format!("{}.{}.clone()", to_snake_case(true_src), true_field)
                }
            } else {
                "None".to_string()
            };

            let false_expr = if let Some(false_field) = false_source.field_name() {
                let false_src = false_source.source_name();
                if false_src == "self" {
                    format!("{}.clone()", false_field)
                } else {
                    format!("{}.{}.clone()", to_snake_case(false_src), false_field)
                }
            } else {
                "None".to_string()
            };

            // Generate inline conditional
            Ok(format!("if {}.as_deref() == Some(\"{}\") {{ {} }} else {{ {} }}",
                cond_var, condition.equals, true_expr, false_expr))
        }

        _ => {
            // Custom transform function
            // Call the transform with parameters from source entities
            generate_transform_call(field, computed, entity, source_entities, all_entities, optional_sources)
        }
    }
}

/// Generate a call to a custom transform function
fn generate_transform_call(
    field: &FieldDef,
    computed: &ComputedFrom,
    entity: &EntityDef,
    source_entities: &[String],
    all_entities: &[EntityDef],
    optional_sources: &std::collections::HashSet<String>,
) -> Result<String, Box<dyn Error>> {
    let transform_name = &computed.transform;

    // Build arguments for the transform function
    let mut args = Vec::new();

    // First, add arguments from sources (entity fields)
    for source in &computed.sources {
        let source_name = source.source_name();

        if source_name == "self" {
            // Self-reference - for fields computed from other fields of same entity
            // These need to be handled differently - we'll pass references to fields
            if let Some(field_name) = source.field_name() {
                args.push(format!("&{}", field_name));
            }
        } else {
            if let Some(field_name) = source.field_name() {
                // Entity.field format - source_name is entity, field_name is field
                let source_var = to_snake_case(source_name);

                // Resolve alias to actual entity name
                let actual_entity_name = entity.derivation.as_ref()
                    .and_then(|d| d.source_entities.as_ref())
                    .and_then(|se| {
                        if let serde_yaml::Value::Mapping(map) = se {
                            map.get(&serde_yaml::Value::String(source_name.to_string()))
                                .and_then(|v| v.as_str())
                        } else {
                            None
                        }
                    })
                    .unwrap_or(source_name);

                // Check if source entity is repeated (and thus optional parameter)
                let source_entity = all_entities.iter().find(|e| &e.name == actual_entity_name);
                let is_repeated_source = source_entity.map(|e| e.repetition.as_ref()
                    .map(|r| r.to_lowercase() == "repeated")
                    .unwrap_or(false))
                    .unwrap_or(false);

                if is_repeated_source {
                    // Source is optional - use .and_then() to access field
                    // Check if field itself is nullable
                    let field_is_nullable = source_entity
                        .and_then(|e| e.fields.iter().find(|f| f.name == field_name))
                        .map(|f| f.nullable)
                        .unwrap_or(true);

                    if field_is_nullable {
                        // Field is Option<String>, flatten the nested Option
                        args.push(format!("&{}.and_then(|e| e.{}.clone())", source_var, field_name));
                    } else {
                        // Field is String, map to Option<String>
                        args.push(format!("&{}.map(|e| e.{}.clone())", source_var, field_name));
                    }
                } else {
                    // Source is required - normal field access
                    // Check if the source field is nullable
                    let should_wrap = source_entity
                        .and_then(|e| e.fields.iter().find(|f| f.name == field_name))
                        .map(|f| {
                            let is_list = f.field_type.starts_with("List[") || f.field_type.starts_with("Vec<");
                            // Wrap if field is non-nullable and not a list
                            !f.nullable && !is_list
                        })
                        .unwrap_or(false);

                    // Wrap non-nullable fields in Some() for transforms that expect Option
                    if should_wrap {
                        args.push(format!("&Some({}.{}.clone())", source_var, field_name));
                    } else {
                        args.push(format!("&{}.{}", source_var, field_name));
                    }
                }
            } else {
                // Just field name - source_name is actually the field name
                let field_name = source_name;

                // First check if this is a direct parameter (e.g., "segment" for repeating entities)
                let snake_field = to_snake_case(field_name);
                if source_entities.iter().any(|se| to_snake_case(se) == snake_field) {
                    // This is a direct parameter
                    // For repeated_for entities, use the _opt wrapper for transforms
                    if entity.repeated_for.is_some() {
                        args.push(format!("&{}_opt", snake_field));
                    } else {
                        args.push(format!("&{}", snake_field));
                    }
                } else {
                    // Find which source entity has this field
                    let mut found = false;

                    for src_entity_name in source_entities {
                        // Look up the source entity definition
                        if let Some(src_entity) = all_entities.iter().find(|e| &e.name == src_entity_name) {
                            // Check if this entity has the field
                            if let Some(src_field) = src_entity.fields.iter().find(|f| f.name == field_name) {
                                let source_var = to_snake_case(src_entity_name);

                                // Check if source entity is repeated (optional parameter)
                                let is_repeated_source = src_entity.repetition.as_ref()
                                    .map(|r| r.to_lowercase() == "repeated")
                                    .unwrap_or(false);

                                if is_repeated_source {
                                    // Source is optional - use .and_then() or .map()
                                    if src_field.nullable {
                                        // Field is Option<String>, flatten the nested Option
                                        args.push(format!("&{}.and_then(|e| e.{}.clone())", source_var, field_name));
                                    } else {
                                        // Field is String, map to Option<String>
                                        args.push(format!("&{}.map(|e| e.{}.clone())", source_var, field_name));
                                    }
                                } else {
                                    // Source is required - normal field access
                                    let is_list = src_field.field_type.starts_with("List[") || src_field.field_type.starts_with("Vec<");
                                    let should_wrap = !src_field.nullable && !is_list;

                                    // Wrap non-nullable fields in Some() for transforms that expect Option
                                    if should_wrap {
                                        args.push(format!("&Some({}.{}.clone())", source_var, field_name));
                                    } else {
                                        args.push(format!("&{}.{}", source_var, field_name));
                                    }
                                }
                                found = true;
                                break;
                            }
                        }
                    }

                    if !found {
                        // Field not found - might be a root entity parameter
                        // Try using it as a direct parameter
                        // For repeated_for entities, use the _opt wrapper for transforms
                        if entity.repeated_for.is_some() {
                            args.push(format!("&{}_opt", snake_field));
                        } else {
                            args.push(format!("&{}", snake_field));
                        }
                    }
                }
            }
        }
    }

    // Then, add arguments from args (literal values)
    if let Some(yaml_args) = &computed.args {
        if let serde_yaml::Value::Mapping(map) = yaml_args {
            // Process all args from the map
            for (_key, value) in map {
                let value_str = match value {
                    serde_yaml::Value::String(s) => format!("\"{}\"", s),
                    serde_yaml::Value::Number(n) => n.to_string(),
                    serde_yaml::Value::Bool(b) => b.to_string(),
                    _ => continue,
                };
                args.push(value_str);
            }
        }
    }

    // Call transform function
    let call = if args.is_empty() {
        format!("{}()", transform_name)
    } else {
        format!("{}({})", transform_name, args.join(", "))
    };

    // Handle Result unwrapping based on field type
    if field.field_type.starts_with("List[") || field.field_type.starts_with("Vec<") {
        // List/Vec types: unwrap with empty vec as default
        Ok(format!("{}.unwrap_or_else(|_| Vec::new())", call))
    } else if field.nullable {
        // Nullable Option fields: unwrap with None as default
        Ok(format!("{}.unwrap_or(None)", call))
    } else {
        // Non-nullable fields: just return the Result (will be unwrapped at call site)
        Ok(call)
    }
}

/// Sanitize doc strings by replacing problematic characters
fn sanitize_doc_string(doc: &str) -> String {
    doc.replace("→", "->")
       .replace("←", "<-")
       .replace("⇒", "=>")
       .replace("⇐", "<=")
       .chars()
       .filter(|c| c.is_ascii() || c.is_whitespace())
       .collect()
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

/// Generate existence check using explicit minimal_existence constraint
fn generate_minimal_existence_check(
    output: &mut std::fs::File,
    entity: &EntityDef,
    minimal_existence: &crate::codegen::types::MinimalExistence,
) -> Result<(), Box<dyn Error>> {
    let fields = minimal_existence.fields()
        .expect("Validated in MinimalExistence::validate");

    writeln!(output, "    // Minimal existence constraint")?;

    if minimal_existence.is_require_any() {
        // OR logic: at least one must be present
        writeln!(output, "    // Require at least ONE of: {}", fields.join(", "))?;

        let checks: Vec<String> = fields.iter()
            .map(|field_name| {
                let field = entity.fields.iter()
                    .find(|f| &f.name == field_name)
                    .expect("Validated in MinimalExistence::validate");
                generate_field_nonempty_check(field)
            })
            .collect();

        writeln!(output, "    if !({}) {{", checks.join(" || "))?;
        writeln!(output, "        return Ok(None); // Minimal existence constraint not met")?;
        writeln!(output, "    }}")?;

    } else {
        // AND logic: all must be present
        writeln!(output, "    // Require ALL of: {}", fields.join(", "))?;

        let checks: Vec<String> = fields.iter()
            .map(|field_name| {
                let field = entity.fields.iter()
                    .find(|f| &f.name == field_name)
                    .expect("Validated in MinimalExistence::validate");
                generate_field_nonempty_check(field)
            })
            .collect();

        writeln!(output, "    if !({}) {{", checks.join(" && "))?;
        writeln!(output, "        return Ok(None); // Minimal existence constraint not met")?;
        writeln!(output, "    }}")?;
    }

    Ok(())
}

/// Generate default existence check - check all fields from core (non-ancillary) sources
fn generate_default_existence_check(
    output: &mut std::fs::File,
    entity: &EntityDef,
) -> Result<(), Box<dyn Error>> {
    // Get source entity specs to identify ancillary sources
    let source_specs = entity.get_source_entity_specs();

    // Collect fields from non-ancillary sources
    let fields_to_check: Vec<&FieldDef> = entity.fields.iter()
        .filter(|f| {
            if let Some(ref computed) = f.computed_from {
                // Check if field comes from a core (non-ancillary) source
                for source in &computed.sources {
                    if source.source_name() == "self" {
                        continue;
                    }

                    if let Some((_entity_name, is_ancillary)) = source_specs.get(source.source_name()) {
                        if !is_ancillary {
                            return true; // Field from core source
                        }
                    }
                }
            }
            false
        })
        .collect();

    if fields_to_check.is_empty() {
        return Ok(()); // No fields to check
    }

    writeln!(output, "    // Check if all core source fields are empty")?;
    writeln!(output, "    // (no minimal_existence specified, checking all non-ancillary fields)")?;

    let checks: Vec<String> = fields_to_check.iter()
        .map(|f| generate_field_nonempty_check(f))
        .collect();

    writeln!(output, "    if !({}) {{", checks.join(" || "))?;
    writeln!(output, "        return Ok(None); // No data extracted")?;
    writeln!(output, "    }}")?;

    Ok(())
}

/// Generate code to check if a field is non-empty
fn generate_field_nonempty_check(field: &FieldDef) -> String {
    let is_list = field.field_type.starts_with("List[")
        || field.field_type.starts_with("Vec<");

    if is_list {
        format!("!entity.{}.is_empty()", field.name)
    } else if field.nullable {
        format!(
            "entity.{}.as_ref().map(|s: &String| !s.is_empty()).unwrap_or(false)",
            field.name
        )
    } else {
        format!("!entity.{}.is_empty()", field.name)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_snake_case_conversion() {
        assert_eq!(to_snake_case("PatientVisit"), "patient_visit");
        assert_eq!(to_snake_case("Anesthesiologist"), "anesthesiologist");
        assert_eq!(to_snake_case("MPI"), "m_p_i");
        assert_eq!(to_snake_case("Hl7v2Message"), "hl7v2_message");
    }
}
