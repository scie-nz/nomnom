/// Generate entities.rs with struct definitions for all entities
///
/// Each entity gets a struct with all its fields for efficient reuse

use crate::codegen::EntityDef;
use std::error::Error;
use std::io::Write;
use std::path::Path;

/// Generate entities.rs file with all entity struct definitions
pub fn generate_entities_file(
    entities: &[EntityDef],
    output_dir: &Path,
) -> Result<(), Box<dyn Error>> {
    let entities_file = output_dir.join("src/entities.rs");
    let mut output = std::fs::File::create(&entities_file)?;

    writeln!(output, "// Auto-generated entity struct definitions")?;
    writeln!(output, "// Each entity is extracted once and reused by dependents\n")?;

    writeln!(output, "use chrono::NaiveDate;")?;
    writeln!(output, "use rust_decimal::Decimal;\n")?;

    // Generate struct for each entity (both transient and persistent)
    for entity in entities {
        // Skip root entities (they come from parsers)
        if entity.is_root() {
            continue;
        }

        // Skip abstract entities
        if entity.is_abstract {
            continue;
        }

        generate_entity_struct(&mut output, entity)?;
        writeln!(output)?;
    }

    Ok(())
}

/// Generate a single entity struct definition
fn generate_entity_struct(
    output: &mut std::fs::File,
    entity: &EntityDef,
) -> Result<(), Box<dyn Error>> {
    // Generate doc comment using multiline syntax
    if let Some(ref doc) = entity.doc {
        let sanitized = sanitize_doc_string(doc);
        if !sanitized.is_empty() {
            writeln!(output, "/** {} */", sanitized.replace('\n', " "))?;
        }
    }
    writeln!(output, "#[derive(Debug, Clone)]")?;
    writeln!(output, "pub struct {} {{", entity.name)?;

    // Generate fields
    for field in &entity.fields {
        // Add field doc using multiline syntax
        if let Some(ref doc) = field.doc {
            let sanitized = sanitize_doc_string(doc);
            if !sanitized.is_empty() {
                writeln!(output, "    /** {} */", sanitized.replace('\n', " "))?;
            }
        }

        let field_type = map_field_type(&field.field_type);

        if field.nullable {
            writeln!(output, "    pub {}: Option<{}>,", field.name, field_type)?;
        } else {
            writeln!(output, "    pub {}: {},", field.name, field_type)?;
        }
    }

    writeln!(output, "}}")?;

    Ok(())
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

/// Map entity field type to Rust type
fn map_field_type(field_type: &str) -> &str {
    match field_type.to_lowercase().as_str() {
        "string" => "String",
        "integer" | "int" => "i32",
        "bigint" | "long" => "i64",
        "double" | "float" => "f64",
        "boolean" | "bool" => "bool",
        "date" => "NaiveDate",
        "decimal" | "numeric" => "Decimal",
        "vec<string>" | "list[string]" => "Vec<String>",
        _ => "String", // Default to String
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_field_type_mapping() {
        assert_eq!(map_field_type("String"), "String");
        assert_eq!(map_field_type("Integer"), "i32");
        assert_eq!(map_field_type("BigInt"), "i64");
        assert_eq!(map_field_type("Date"), "NaiveDate");
        assert_eq!(map_field_type("Decimal"), "Decimal");
    }
}
