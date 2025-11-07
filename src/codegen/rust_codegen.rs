//! Rust entity code generation from entity definitions.
//!
//! This module generates Rust struct and impl code for entities based on
//! YAML configurations. It provides generic scaffolding that domain-specific
//! libraries can extend.

use crate::codegen::types::{EntityDef, FieldDef, ComputedFrom};
use crate::codegen::utils::to_snake_case;
use crate::codegen::project_config::RustTransformDef;
use std::io::Write;
use std::collections::{HashMap, HashSet};

/// Configuration for Rust code generation
#[derive(Debug, Clone)]
pub struct RustCodegenConfig {
    /// Transform registry type path (e.g., "crate::transform_registry::TransformRegistry")
    /// If None, entities with computed_from fields will cause an error
    pub transform_registry_type: Option<String>,
}

impl Default for RustCodegenConfig {
    fn default() -> Self {
        Self {
            transform_registry_type: None,
        }
    }
}

/// Check if entity has any fields with computed_from
fn entity_has_computed_fields(entity: &EntityDef) -> bool {
    entity.fields.iter().any(|f| f.computed_from.is_some())
}

// ============================================================================
// Transform Function Generation
// ============================================================================

/// Generate transform functions section for generated.rs
///
/// This generates static functions from transform definitions, plus stubs
/// for any transforms that are called but not provided.
///
/// # Arguments
///
/// * `writer` - Output writer
/// * `transforms` - Transform definitions from nomnom.yaml
/// * `entities` - Entity definitions (to find all called transforms)
///
/// # Returns
///
/// Set of transform names that were generated
pub fn generate_transform_functions<W: Write>(
    writer: &mut W,
    transforms: &HashMap<String, RustTransformDef>,
    entities: &[EntityDef],
) -> Result<HashSet<String>, std::io::Error> {
    writeln!(writer, "// ============================================================================")?;
    writeln!(writer, "// Auto-generated Transform Functions")?;
    writeln!(writer, "// Generated from nomnom.yaml transforms section")?;
    writeln!(writer, "// ============================================================================\n")?;

    let mut generated = HashSet::new();

    // Generate provided transforms
    for (name, def) in transforms {
        generate_single_transform(writer, name, def)?;
        writeln!(writer)?;
        generated.insert(name.clone());
    }

    // TODO: Generate stubs for missing transforms
    // This requires analyzing actual usage patterns to determine correct signatures
    // For now, users must provide all required transforms in nomnom.yaml

    // Find all transforms used by entities
    let required_transforms = collect_required_transforms(entities);
    let missing_transforms: Vec<_> = required_transforms.iter()
        .filter(|t| !generated.contains(*t))
        .collect();

    if !missing_transforms.is_empty() {
        writeln!(writer, "// ============================================================================")?;
        writeln!(writer, "// MISSING TRANSFORMS - Add these to nomnom.yaml transforms.rust section:")?;
        for transform_name in &missing_transforms {
            writeln!(writer, "//   - {}", transform_name)?;
        }
        writeln!(writer, "// ============================================================================\n")?;
    }

    if !generated.is_empty() {
        writeln!(writer, "// ============================================================================\n")?;
    }

    Ok(generated)
}

/// Generate a single transform function from definition
fn generate_single_transform<W: Write>(
    writer: &mut W,
    name: &str,
    def: &RustTransformDef,
) -> Result<(), std::io::Error> {
    // Documentation
    if let Some(doc) = &def.doc {
        writeln!(writer, "/// {}", doc)?;
        writeln!(writer, "///")?;
    } else {
        writeln!(writer, "/// Transform function: {}", name)?;
        writeln!(writer, "///")?;
    }

    // Parameter documentation
    if !def.args.is_empty() {
        writeln!(writer, "/// # Arguments")?;
        writeln!(writer, "///")?;
        for arg in &def.args {
            writeln!(writer, "/// * `{}` - {}", arg.name, arg.arg_type)?;
        }
        writeln!(writer, "///")?;
    }

    writeln!(writer, "/// # Returns")?;
    writeln!(writer, "///")?;
    writeln!(writer, "/// {}", def.return_type)?;

    // Function signature
    write!(writer, "pub fn {}(", name)?;
    for (i, arg) in def.args.iter().enumerate() {
        if i > 0 {
            write!(writer, ", ")?;
        }
        write!(writer, "{}: {}", arg.name, arg.arg_type)?;
    }
    writeln!(writer, ") -> {} {{", def.return_type)?;

    // Add imports if specified
    if !def.imports.is_empty() {
        for import in &def.imports {
            writeln!(writer, "    use {};", import)?;
        }
        writeln!(writer)?;
    }

    // Function body (indent each line)
    for line in def.code.lines() {
        if line.trim().is_empty() {
            writeln!(writer)?;
        } else {
            writeln!(writer, "    {}", line)?;
        }
    }

    writeln!(writer, "}}")?;

    Ok(())
}

/// Generate a stub for a missing transform
fn generate_transform_stub<W: Write>(
    writer: &mut W,
    name: &str,
) -> Result<(), std::io::Error> {
    writeln!(writer, "/// STUB: Transform '{}' not implemented", name)?;
    writeln!(writer, "///")?;
    writeln!(writer, "/// This transform is called by generated entities but no implementation")?;
    writeln!(writer, "/// was provided in nomnom.yaml transforms section.")?;
    writeln!(writer, "///")?;
    writeln!(writer, "/// To implement this transform, add it to nomnom.yaml:")?;
    writeln!(writer, "///")?;
    writeln!(writer, "/// ```yaml")?;
    writeln!(writer, "/// transforms:")?;
    writeln!(writer, "///   rust:")?;
    writeln!(writer, "///     {}:", name)?;
    writeln!(writer, "///       args:")?;
    writeln!(writer, "///         - name: arg1")?;
    writeln!(writer, "///           type: \"&str\"")?;
    writeln!(writer, "///       return_type: \"Result<String, String>\"")?;
    writeln!(writer, "///       code: |")?;
    writeln!(writer, "///         // Your implementation here")?;
    writeln!(writer, "///         Ok(\"result\".to_string())")?;
    writeln!(writer, "/// ```")?;
    writeln!(writer, "pub fn {}(args: &std::collections::HashMap<String, serde_json::Value>) -> Result<serde_json::Value, String> {{", name)?;
    writeln!(writer, "    Err(format!(")?;
    writeln!(writer, "        \"Transform '{}' is not implemented. \\", name)?;
    writeln!(writer, "         Add it to nomnom.yaml transforms.rust section. \\", )?;
    writeln!(writer, "         See generated code for example.\"")?;
    writeln!(writer, "    ))")?;
    writeln!(writer, "}}")?;

    Ok(())
}

/// Collect all transform names used by entities
fn collect_required_transforms(entities: &[EntityDef]) -> HashSet<String> {
    let mut transforms = HashSet::new();

    for entity in entities {
        for field in &entity.fields {
            if let Some(ref computed) = field.computed_from {
                // Skip copy_field - it's not a real transform, just direct field copy
                if computed.transform != "copy_field" {
                    transforms.insert(computed.transform.clone());
                }
            }
        }
    }

    transforms
}

/// Generate Rust code for all entities
///
/// Writes generated Rust code to the provided writer.
///
/// # Arguments
///
/// * `writer` - Output writer for generated code
/// * `entities` - Vector of entity definitions to generate
/// * `config` - Code generation configuration (transform registry, etc.)
///
/// # Example
///
/// ```ignore
/// use nomnom::codegen::{load_entities, generate_rust_code, RustCodegenConfig};
/// use std::fs::File;
///
/// let entities = load_entities("config/entities").unwrap();
/// let mut output = File::create("src/generated.rs").unwrap();
/// let config = RustCodegenConfig {
///     transform_registry_type: Some("crate::transform_registry::TransformRegistry".to_string()),
/// };
/// generate_rust_code(&mut output, &entities, &config).unwrap();
/// ```
pub fn generate_rust_code<W: Write>(
    writer: &mut W,
    entities: &[EntityDef],
    config: &RustCodegenConfig,
) -> Result<(), std::io::Error> {
    // NOTE: Header with imports should be generated by caller (e.g., build.rs)
    // to allow domain-specific imports and configuration

    // Generate each entity
    for entity in entities {
        generate_entity(writer, entity, config)?;
    }

    Ok(())
}

/// Generate code for a single entity
fn generate_entity<W: Write>(
    writer: &mut W,
    entity: &EntityDef,
    config: &RustCodegenConfig,
) -> Result<(), std::io::Error> {
    // Determine core name (add "Core" suffix for consistency)
    // Entity names in YAML are already PascalCase, so use them as-is
    let core_name = format!("{}Core", entity.name);

    // Generate struct definition
    generate_struct(writer, entity, &core_name)?;

    // Generate impl block based on entity type (skip for abstract entities)
    if !entity.is_abstract {
        if entity.is_root() {
            generate_root_impl(writer, entity, &core_name, config)?;
        } else if entity.is_derived() {
            if entity.repeated_for.is_some() {
                generate_repeated_impl(writer, entity, &core_name, config)?;
            } else {
                generate_derived_impl(writer, entity, &core_name, config)?;
            }
        }
    }

    Ok(())
}

/// Generate struct definition
fn generate_struct<W: Write>(
    writer: &mut W,
    entity: &EntityDef,
    struct_name: &str,
) -> Result<(), std::io::Error> {
    // Documentation
    if let Some(ref doc) = entity.doc {
        if !doc.is_empty() {
            for line in doc.lines() {
                writeln!(writer, "/// {}", line)?;
            }
        }
    }

    // Derive macros
    writeln!(writer, "#[derive(Debug, Clone, Serialize, Deserialize)]")?;
    writeln!(writer, "pub struct {} {{", struct_name)?;

    // Fields
    for field in &entity.fields {
        // Field documentation
        if let Some(ref doc) = field.doc {
            if !doc.is_empty() {
                for line in doc.lines() {
                    writeln!(writer, "    /// {}", line)?;
                }
            }
        }

        let rust_type = map_field_type(&field.field_type, field.nullable);
        writeln!(writer, "    pub {}: {},", field.name, rust_type)?;
    }

    writeln!(writer, "}}\n")?;

    Ok(())
}

/// Generate impl block for root entities
fn generate_root_impl<W: Write>(
    writer: &mut W,
    entity: &EntityDef,
    struct_name: &str,
    config: &RustCodegenConfig,
) -> Result<(), std::io::Error> {
    writeln!(writer, "impl {} {{", struct_name)?;

    // from_string constructor signature
    writeln!(writer, "    /// Create root entity from raw string input")?;
    writeln!(writer, "    ///")?;
    writeln!(writer, "    /// # Arguments")?;
    writeln!(writer, "    ///")?;
    writeln!(writer, "    /// * `raw_input` - Raw string input to parse")?;
    writeln!(writer, "    pub fn from_string(")?;
    writeln!(writer, "        raw_input: &str,")?;
    writeln!(writer, "    ) -> Result<Self, String> {{")?;

    // Generate field extraction for each field
    for field in &entity.fields {
        if let Some(ref computed) = field.computed_from {
            // Field computed via transform
            generate_field_extraction(writer, field, computed, "        ")?;
        } else if field.root_source.is_some() {
            // Field sourced from raw_input
            writeln!(writer, "        // Field '{}' from root source", field.name)?;
            writeln!(writer, "        let {} = raw_input.to_string();", field.name)?;
        }
    }

    // Build and return struct
    writeln!(writer)?;
    writeln!(writer, "        Ok(Self {{")?;
    for field in &entity.fields {
        writeln!(writer, "            {},", field.name)?;
    }
    writeln!(writer, "        }})")?;
    writeln!(writer, "    }}\n")?;

    // Serialization methods
    generate_serialization_methods(writer, entity)?;

    writeln!(writer, "}}\n")?;

    Ok(())
}

/// Generate impl block for derived entities
fn generate_derived_impl<W: Write>(
    writer: &mut W,
    entity: &EntityDef,
    struct_name: &str,
    config: &RustCodegenConfig,
) -> Result<(), std::io::Error> {
    writeln!(writer, "impl {} {{", struct_name)?;

    // Get parent information
    let parents = entity.get_parents();

    // Generate constructor signature - always use from_sources for consistency
    if !parents.is_empty() {
        // Derived entity: from_sources (whether single or multiple parents)
        writeln!(writer, "    /// Create entity from source entities")?;
        writeln!(writer, "    ///")?;
        writeln!(writer, "    /// # Arguments")?;
        writeln!(writer, "    ///")?;
        for parent_name in &parents {
            let param_name = to_snake_case(parent_name);
            writeln!(writer, "    /// * `{}` - Source {} entity", param_name, parent_name)?;
        }

        // Generate parameters - parent entities only (no registry)
        write!(writer, "    pub fn from_sources(")?;
        for (i, parent_name) in parents.iter().enumerate() {
            let param_name = to_snake_case(parent_name);
            let parent_type = format!("{}Core", parent_name);

            if i > 0 {
                write!(writer, ", ")?;
            }
            write!(writer, "{}: &{}", param_name, parent_type)?;
        }
        writeln!(writer)?;
        writeln!(writer, "    ) -> Result<Self, String> {{")?;

        // Generate field extraction for each field with computed_from
        for field in &entity.fields {
            if let Some(ref computed) = field.computed_from {
                generate_field_extraction(writer, field, computed, "        ")?;
            }
        }

        // Build and return struct
        writeln!(writer)?;
        writeln!(writer, "        Ok(Self {{")?;
        for field in &entity.fields {
            writeln!(writer, "            {},", field.name)?;
        }
        writeln!(writer, "        }})")?;
        writeln!(writer, "    }}\n")?;
    }

    // Serialization methods
    generate_serialization_methods(writer, entity)?;

    writeln!(writer, "}}\n")?;

    Ok(())
}

/// Generate impl block for repeated entities (repeated_for pattern)
fn generate_repeated_impl<W: Write>(
    writer: &mut W,
    entity: &EntityDef,
    struct_name: &str,
    config: &RustCodegenConfig,
) -> Result<(), std::io::Error> {
    writeln!(writer, "impl {} {{", struct_name)?;

    if let Some(ref repeated_for) = entity.repeated_for {
        let parent_type = format!("{}Core", repeated_for.entity);
        let parent_param = to_snake_case(&repeated_for.entity);
        let list_field = &repeated_for.field;
        let item_var = &repeated_for.each_known_as;

        writeln!(writer, "    /// Create entity instances from repeated parent data")?;
        writeln!(writer, "    ///")?;
        writeln!(writer, "    /// # Arguments")?;
        writeln!(writer, "    ///")?;
        writeln!(writer, "    /// * `{}` - Parent {} entity", parent_param, repeated_for.entity)?;
        writeln!(writer, "    ///")?;
        writeln!(writer, "    /// # Returns")?;
        writeln!(writer, "    ///")?;
        writeln!(writer, "    /// Vector of {} instances, one per item in parent.{}", entity.name, list_field)?;
        writeln!(writer, "    pub fn from_parent_repeated(")?;
        writeln!(writer, "        {}: &{}", parent_param, parent_type)?;
        writeln!(writer, "    ) -> Result<Vec<Self>, String> {{")?;
        writeln!(writer, "        let mut instances = Vec::new();")?;
        writeln!(writer)?;
        writeln!(writer, "        // Iterate over parent.{}", list_field)?;
        writeln!(writer, "        for {} in &{}.{} {{", item_var, parent_param, list_field)?;

        // Generate field extraction for each field (indent by 12 spaces)
        for field in &entity.fields {
            if let Some(ref computed) = field.computed_from {
                generate_field_extraction(writer, field, computed, "            ")?;
            }
        }

        // Build instance and add to vector
        writeln!(writer)?;
        writeln!(writer, "            instances.push(Self {{")?;
        for field in &entity.fields {
            writeln!(writer, "                {},", field.name)?;
        }
        writeln!(writer, "            }});")?;
        writeln!(writer, "        }}")?;
        writeln!(writer)?;
        writeln!(writer, "        Ok(instances)")?;
        writeln!(writer, "    }}\n")?;
    }

    // Serialization methods
    generate_serialization_methods(writer, entity)?;

    writeln!(writer, "}}\n")?;

    Ok(())
}

/// Generate serialization methods (to_json, to_dict, etc.)
fn generate_serialization_methods<W: Write>(
    writer: &mut W,
    entity: &EntityDef,
) -> Result<(), std::io::Error> {
    // to_dict - convert entity to HashMap
    writeln!(writer, "    /// Convert entity to dictionary/map")?;
    writeln!(writer, "    pub fn to_dict(&self) -> HashMap<String, serde_json::Value> {{")?;
    writeln!(writer, "        let mut map = HashMap::new();")?;
    for field in &entity.fields {
        writeln!(writer, "        map.insert(\"{}\".to_string(), serde_json::to_value(&self.{}).unwrap_or(serde_json::Value::Null));", field.name, field.name)?;
    }
    writeln!(writer, "        map")?;
    writeln!(writer, "    }}\n")?;

    // to_json
    writeln!(writer, "    /// Serialize entity to JSON string")?;
    writeln!(writer, "    pub fn to_json(&self) -> Result<String, serde_json::Error> {{")?;
    writeln!(writer, "        serde_json::to_string(self)")?;
    writeln!(writer, "    }}\n")?;

    // to_json_pretty
    writeln!(writer, "    /// Serialize entity to pretty-printed JSON string")?;
    writeln!(writer, "    pub fn to_json_pretty(&self) -> Result<String, serde_json::Error> {{")?;
    writeln!(writer, "        serde_json::to_string_pretty(self)")?;
    writeln!(writer, "    }}\n")?;

    // to_ndjson_line
    writeln!(writer, "    /// Serialize entity to NDJSON line (newline-delimited JSON)")?;
    writeln!(writer, "    pub fn to_ndjson_line(&self) -> Result<String, serde_json::Error> {{")?;
    writeln!(writer, "        let json = self.to_json()?;")?;
    writeln!(writer, "        Ok(format!(\"{{}}\\n\", json))")?;
    writeln!(writer, "    }}")?;

    Ok(())
}

/// Generate field extraction code
fn generate_field_extraction<W: Write>(
    writer: &mut W,
    field: &FieldDef,
    computed: &ComputedFrom,
    indent: &str,
) -> Result<(), std::io::Error> {
    writeln!(writer, "{}// Extract field: {}", indent, field.name)?;

    // Special case: copy_field transform with sources - direct field copy
    if computed.transform == "copy_field" && !computed.sources.is_empty() {
        let source = &computed.sources[0];

        // Check if this is a self-reference (source: self, field: other_field)
        if let crate::codegen::types::FieldSource::Parent { source: src, field: src_field, .. } = source {
            if src.to_lowercase() == "self" {
                // Self-reference: use the already-extracted local variable directly
                writeln!(writer, "{}let {} = {}.clone();",
                    indent, field.name, src_field)?;
                return Ok(());
            }
        }

        // Get the variable name
        // - Parent.source: entity TYPE -> convert to snake_case for variable name
        // - Direct: already a variable name
        let source_var = match source {
            crate::codegen::types::FieldSource::Parent { source, .. } => to_snake_case(source),
            crate::codegen::types::FieldSource::Direct(name) => name.clone(),
        };

        if let Some(source_field) = source.field_name() {
            // Direct copy from parent.field
            writeln!(writer, "{}let {} = {}.{}.clone();",
                indent, field.name, source_var, source_field)?;
            return Ok(());
        } else {
            // Direct source (no field) - shouldn't happen for copy_field
            return Err(std::io::Error::new(
                std::io::ErrorKind::InvalidInput,
                format!("copy_field transform for '{}' has no field name in source", field.name)
            ));
        }
    }

    // General case: call transform function directly
    // Build function call: transform_name(arg1, arg2, ...)

    let mut call_args = Vec::new();

    // Add sources as positional arguments
    for source in &computed.sources {
        // Check if this is a self-reference
        if let crate::codegen::types::FieldSource::Parent { source: src, field: src_field, .. } = source {
            if src.to_lowercase() == "self" {
                // Self-reference: use the already-extracted local variable directly
                call_args.push(format!("&{}", src_field));
                continue;
            }
        }

        // Get the variable name for the source
        // - Parent.source: entity TYPE (e.g., "Hl7v2Message") -> convert to snake_case for variable name
        // - Direct: already a variable name from repeated_for.each_known_as (e.g., "item_record")
        let source_var = match source {
            crate::codegen::types::FieldSource::Parent { source, .. } => to_snake_case(source),
            crate::codegen::types::FieldSource::Direct(name) => name.clone(),
        };

        if let Some(field_name) = source.field_name() {
            // Parent field reference: pass the field value as reference
            call_args.push(format!("&{}.{}", source_var, field_name));
        } else {
            // Direct source reference: wrap in Some() for compatibility with &Option<String> transforms
            // Both iterator variables (&String from Vec<String>) and self-references (String fields)
            // need to be wrapped when the transform expects &Option<String>
            call_args.push(format!("&Some({}.clone())", source_var));
        }
    }

    // Add additional arguments from 'args' field
    if let Some(ref args) = computed.args {
        match args {
            serde_yaml::Value::Mapping(map) => {
                // For mappings, each key-value becomes an argument
                // The order matters - we need to know the expected parameter order
                // For now, we'll add them in the order they appear in the YAML
                for (key, value) in map {
                    if let Some(_key_str) = key.as_str() {
                        // Convert YAML value to Rust literal
                        let rust_value = yaml_value_to_rust_literal(value);
                        call_args.push(rust_value);
                    }
                }
            }
            _ => {
                // Scalar value - add as single argument
                let rust_value = yaml_value_to_rust_literal(args);
                call_args.push(rust_value);
            }
        }
    }

    // Generate the function call
    let args_str = call_args.join(", ");
    writeln!(
        writer,
        "{}let {} = {}({})",
        indent, field.name, computed.transform, args_str
    )?;
    writeln!(
        writer,
        "{}    .map_err(|e| format!(\"Failed to extract '{}': {{}}\", e))?;",
        indent, field.name
    )?;

    Ok(())
}

/// Convert YAML value to Rust literal
fn yaml_value_to_rust_literal(value: &serde_yaml::Value) -> String {
    match value {
        serde_yaml::Value::String(s) => format!("\"{}\"", s.replace('\\', "\\\\").replace('"', "\\\"")),
        serde_yaml::Value::Number(n) => n.to_string(),
        serde_yaml::Value::Bool(b) => b.to_string(),
        serde_yaml::Value::Null => "None".to_string(),
        _ => panic!("Unsupported YAML value type for Rust literal conversion: {:?}", value),
    }
}

/// Map field type to Rust type
fn map_field_type(field_type: &str, nullable: bool) -> String {
    let base_type = match field_type {
        "String" => "String",
        "Int" | "Integer" => "i64",
        "Float" | "Double" => "f64",
        "Bool" | "Boolean" => "bool",
        "DateTime" | "Date" => "String", // ISO8601 strings
        "List[String]" => "Vec<String>",
        _ => "String", // Default to String
    }.to_string();

    if nullable && field_type != "List[String]" {
        format!("Option<{}>", base_type)
    } else {
        base_type
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_map_field_type() {
        assert_eq!(map_field_type("String", false), "String");
        assert_eq!(map_field_type("String", true), "Option<String>");
        assert_eq!(map_field_type("Integer", false), "i64");
        assert_eq!(map_field_type("Integer", true), "Option<i64>");
        assert_eq!(map_field_type("List[String]", true), "Vec<String>");
    }

    #[test]
    fn test_generate_struct() {
        let entity = EntityDef {
            name: "TestEntity".to_string(),
            source_type: "root".to_string(),
            repetition: Some("singleton".to_string()),
            parent: None,
            parents: vec![],
            repeated_for: None,
            fields: vec![
                crate::codegen::types::FieldDef {
                    name: "field1".to_string(),
                    field_type: "String".to_string(),
                    nullable: false,
                    computed_from: None,
                    doc: Some("Test field".to_string()),
                    primary_key: false,
                    index: false,
                    extraction: None,
                    computed: None,
                    args: None,
                    constant: None,
                    segment_field: None,
                    filename_component: None,
                    derived_from: None,
                    root_source: None,
                },
            ],
            doc: Some("Test entity".to_string()),
            database: None,
            derivation: None,
            is_abstract: false,
            extends: None,
            abstract_implementations: None,
            serialization: vec![],
        };

        let mut output = Vec::new();
        generate_struct(&mut output, &entity, "TestEntityCore").unwrap();

        let generated = String::from_utf8(output).unwrap();
        assert!(generated.contains("pub struct TestEntityCore"));
        assert!(generated.contains("pub field1: String"));
        assert!(generated.contains("/// Test entity"));
        assert!(generated.contains("/// Test field"));
    }

    #[test]
    fn test_generate_root_entity() {
        let entity = EntityDef {
            name: "RootEntity".to_string(),
            source_type: "root".to_string(),
            repetition: Some("singleton".to_string()),
            parent: None,
            parents: vec![],
            repeated_for: None,
            fields: vec![],
            doc: None,
            database: None,
            derivation: None,
            is_abstract: false,
            extends: None,
            abstract_implementations: None,
            serialization: vec![],
        };

        let mut output = Vec::new();
        let config = RustCodegenConfig::default();
        generate_entity(&mut output, &entity, &config).unwrap();

        let generated = String::from_utf8(output).unwrap();
        assert!(generated.contains("pub struct RootEntityCore"));
        assert!(generated.contains("pub fn from_string"));
    }

    #[test]
    fn test_generate_derived_entity_single_parent() {
        let entity = EntityDef {
            name: "ChildEntity".to_string(),
            source_type: "derived".to_string(),
            repetition: Some("singleton".to_string()),
            parent: Some("ParentEntity".to_string()),
            parents: vec![],
            repeated_for: None,
            fields: vec![],
            doc: None,
            database: None,
            derivation: None,
            is_abstract: false,
            extends: None,
            abstract_implementations: None,
            serialization: vec![],
        };

        let mut output = Vec::new();
        let config = RustCodegenConfig::default();
        generate_entity(&mut output, &entity, &config).unwrap();

        let generated = String::from_utf8(output).unwrap();
        assert!(generated.contains("pub struct ChildEntityCore"));
        // Now uses from_sources for consistency (even with single parent)
        assert!(generated.contains("pub fn from_sources("));
        assert!(generated.contains("parent_entity: &ParentEntityCore"));
        // No transform registry in generated code when no computed_from fields
        assert!(generated.contains("registry: &()"));
    }

    #[test]
    fn test_generate_derived_entity_multi_parent() {
        let entity = EntityDef {
            name: "MultiEntity".to_string(),
            source_type: "derived".to_string(),
            repetition: Some("singleton".to_string()),
            parent: None,
            parents: vec![
                crate::codegen::types::ParentDef {
                    name: "Parent1".to_string(),
                    parent_type: "Parent1".to_string(),
                    source: "transient".to_string(),
                    doc: None,
                    same_segment_as: None,
                },
                crate::codegen::types::ParentDef {
                    name: "Parent2".to_string(),
                    parent_type: "Parent2".to_string(),
                    source: "transient".to_string(),
                    doc: None,
                    same_segment_as: None,
                },
            ],
            repeated_for: None,
            fields: vec![],
            doc: None,
            database: None,
            derivation: None,
            is_abstract: false,
            extends: None,
            abstract_implementations: None,
            serialization: vec![],
        };

        let mut output = Vec::new();
        let config = RustCodegenConfig::default();
        generate_entity(&mut output, &entity, &config).unwrap();

        let generated = String::from_utf8(output).unwrap();
        assert!(generated.contains("pub struct MultiEntityCore"));
        assert!(generated.contains("pub fn from_sources"));
        assert!(generated.contains("parent_1: &Parent1Core"));
        assert!(generated.contains("parent_2: &Parent2Core"));
    }
}
