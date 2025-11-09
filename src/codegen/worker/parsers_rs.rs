/// Generate parsers.rs from entity YAML definitions

use crate::codegen::EntityDef;
use std::path::Path;
use std::error::Error;
use std::io::Write;

pub fn generate_parsers_rs(
    entities: &[EntityDef],
    output_dir: &Path,
) -> Result<(), Box<dyn Error>> {
    let parsers_file = output_dir.join("src/parsers.rs");
    let mut output = std::fs::File::create(&parsers_file)?;

    writeln!(output, "// Auto-generated message parsers")?;
    writeln!(output, "// Generated from entity definitions\n")?;

    writeln!(output, "use crate::error::AppError;")?;
    writeln!(output, "use chrono::NaiveDate;")?;
    writeln!(output, "use rust_decimal::Decimal;")?;
    writeln!(output, "use std::str::FromStr;\n")?;

    // Generate ParsedMessage enum (only for root entities)
    writeln!(output, "#[derive(Debug)]")?;
    writeln!(output, "pub enum ParsedMessage {{")?;
    for entity in entities {
        // Only include root entities that are persistent
        if !entity.is_root() || !entity.is_persistent() || entity.is_abstract {
            continue;
        }
        if entity.source_type.to_lowercase() == "reference" {
            continue;
        }
        writeln!(output, "    {}({}Message),", entity.name, entity.name)?;
    }
    writeln!(output, "}}\n")?;

    // Generate message structs for each root entity
    for entity in entities {
        // Only include root entities that are persistent
        if !entity.is_root() || !entity.is_persistent() || entity.is_abstract {
            continue;
        }
        if entity.source_type.to_lowercase() == "reference" {
            continue;
        }

        generate_message_struct(&mut output, entity)?;
    }

    // Generate MessageParser impl
    writeln!(output, "pub struct MessageParser;\n")?;
    writeln!(output, "impl MessageParser {{")?;

    // Main parse_line function
    generate_parse_line_function(&mut output, entities)?;

    // Individual parser functions (only for root entities)
    for entity in entities {
        // Only include root entities that are persistent
        if !entity.is_root() || !entity.is_persistent() || entity.is_abstract {
            continue;
        }
        if entity.source_type.to_lowercase() == "reference" {
            continue;
        }

        generate_entity_parser(&mut output, entity)?;
    }

    writeln!(output, "}}")?;

    Ok(())
}

fn generate_message_struct(
    output: &mut std::fs::File,
    entity: &EntityDef,
) -> Result<(), Box<dyn Error>> {
    writeln!(output, "#[derive(Debug)]")?;
    writeln!(output, "pub struct {}Message {{", entity.name)?;

    if let Some(ref persistence) = entity.persistence {
        for field in &persistence.field_overrides {
            let field_type_str = field.field_type.as_deref().unwrap_or("String");
            let base_type = map_field_type(field_type_str);
            let rust_type = if field.nullable.unwrap_or(false) {
                format!("Option<{}>", base_type)
            } else {
                base_type
            };
            writeln!(output, "    pub {}: {},", field.name, rust_type)?;
        }
    }

    writeln!(output, "}}\n")?;

    Ok(())
}

fn generate_parse_line_function(
    output: &mut std::fs::File,
    entities: &[EntityDef],
) -> Result<(), Box<dyn Error>> {
    writeln!(output, "    /// Parse JSON message and return entity name + parsed data")?;
    writeln!(output, "    pub fn parse_json(json_str: &str) -> Result<(String, ParsedMessage), AppError> {{")?;
    writeln!(output, "        use serde_json::Value;")?;
    writeln!(output, "        let json_str = json_str.trim();\n")?;

    writeln!(output, "        if json_str.is_empty() {{")?;
    writeln!(output, "            return Err(AppError::EmptyMessage);")?;
    writeln!(output, "        }}\n")?;

    writeln!(output, "        // Parse JSON")?;
    writeln!(output, "        let value: Value = serde_json::from_str(json_str)")?;
    writeln!(output, "            .map_err(|e| AppError::InvalidFormat(format!(\"Invalid JSON: {{}}\", e)))?;\n")?;

    writeln!(output, "        let obj = value.as_object()")?;
    writeln!(output, "            .ok_or_else(|| AppError::InvalidFormat(\"Expected JSON object\".to_string()))?;\n")?;

    // Generate entity detection logic - try to parse as each root entity type
    writeln!(output, "        // Try to parse as each known root entity type")?;

    for entity in entities {
        // Only include root entities that are persistent
        if !entity.is_root() || !entity.is_persistent() || entity.is_abstract {
            continue;
        }
        if entity.source_type.to_lowercase() == "reference" {
            continue;
        }

        writeln!(output, "        if let Ok(msg) = Self::parse_{}(obj) {{", entity.name.to_lowercase())?;
        writeln!(output, "            return Ok((\"{}\".to_string(), ParsedMessage::{}(msg)));", entity.name, entity.name)?;
        writeln!(output, "        }}")?;
    }

    writeln!(output, "\n        Err(AppError::InvalidFormat(\"Could not parse as any known entity type\".to_string()))")?;
    writeln!(output, "    }}\n")?;

    Ok(())
}

fn generate_entity_parser(
    output: &mut std::fs::File,
    entity: &EntityDef,
) -> Result<(), Box<dyn Error>> {
    writeln!(output, "    fn parse_{}(obj: &serde_json::Map<String, serde_json::Value>) -> Result<{}Message, AppError> {{",
        entity.name.to_lowercase(), entity.name)?;

    if let Some(ref persistence) = entity.persistence {
        writeln!(output, "        Ok({}Message {{", entity.name)?;

        for field in &persistence.field_overrides {
            let field_type_str = field.field_type.as_deref().unwrap_or("String");
            let is_nullable = field.nullable.unwrap_or(false);
            let parse_expr = generate_json_parse_expression(field_type_str, &field.name, is_nullable);
            writeln!(output, "            {}: {},", field.name, parse_expr)?;
        }

        writeln!(output, "        }})")?;
    }

    writeln!(output, "    }}\n")?;

    Ok(())
}

fn map_field_type(field_type: &str) -> String {
    match field_type {
        "i32" | "i64" => field_type.to_string(),
        "Integer" => "i32".to_string(),
        "Float" => "f64".to_string(),
        "String" => "String".to_string(),
        "Option<String>" => "Option<String>".to_string(),
        "NaiveDate" => "NaiveDate".to_string(),
        "Decimal" => "Decimal".to_string(),
        _ => "String".to_string(), // Default to String
    }
}

fn generate_json_parse_expression(field_type: &str, field_name: &str, nullable: bool) -> String {
    if nullable {
        // For nullable fields, return Option<T>
        match field_type {
            "i32" | "Integer" => format!(
                "obj.get(\"{}\").and_then(|v| if v.is_null() {{ None }} else {{ v.as_i64().map(|x| x as i32) }})",
                field_name
            ),
            "i64" => format!(
                "obj.get(\"{}\").and_then(|v| if v.is_null() {{ None }} else {{ v.as_i64() }})",
                field_name
            ),
            "Float" => format!(
                "obj.get(\"{}\").and_then(|v| if v.is_null() {{ None }} else {{ v.as_f64() }})",
                field_name
            ),
            "String" => format!(
                "obj.get(\"{}\").and_then(|v| if v.is_null() {{ None }} else {{ v.as_str().map(|s| s.to_string()) }})",
                field_name
            ),
            _ => format!(
                "obj.get(\"{}\").and_then(|v| if v.is_null() {{ None }} else {{ v.as_str().map(|s| s.to_string()) }})",
                field_name
            ),
        }
    } else {
        // For required fields, return T with error handling
        match field_type {
            "i32" | "Integer" => format!(
                "obj.get(\"{}\").and_then(|v| v.as_i64()).map(|v| v as i32).ok_or_else(|| AppError::InvalidField(\"{}\".to_string()))?",
                field_name, field_name
            ),
            "i64" => format!(
                "obj.get(\"{}\").and_then(|v| v.as_i64()).ok_or_else(|| AppError::InvalidField(\"{}\".to_string()))?",
                field_name, field_name
            ),
            "Float" => format!(
                "obj.get(\"{}\").and_then(|v| v.as_f64()).ok_or_else(|| AppError::InvalidField(\"{}\".to_string()))?",
                field_name, field_name
            ),
            "String" => format!(
                "obj.get(\"{}\").and_then(|v| v.as_str()).map(|s| s.to_string()).ok_or_else(|| AppError::InvalidField(\"{}\".to_string()))?",
                field_name, field_name
            ),
            _ => format!(
                "obj.get(\"{}\").and_then(|v| v.as_str()).map(|s| s.to_string()).ok_or_else(|| AppError::InvalidField(\"{}\".to_string()))?",
                field_name, field_name
            ),
        }
    }
}
