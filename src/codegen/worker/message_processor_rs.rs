/// Generate message_processor.rs
///
/// This handles routing parsed messages to the correct entity processor

use crate::codegen::EntityDef;
use std::path::Path;
use std::error::Error;
use std::io::Write;

/// Convert field names to snake_case for SQL/Rust
fn to_snake_case(s: &str) -> String {
    let stripped = if s.starts_with("f_") {
        &s[2..]
    } else {
        s
    };

    if stripped.contains('_') || stripped.chars().all(|c| !c.is_uppercase()) {
        return s.to_string();
    }

    let mut result = String::new();
    let mut prev_lowercase = false;

    for (i, ch) in s.chars().enumerate() {
        if ch.is_uppercase() {
            if i > 0 && (prev_lowercase || s.chars().nth(i + 1).map_or(false, |c| c.is_lowercase())) {
                result.push('_');
            }
            result.push(ch.to_lowercase().next().unwrap());
            prev_lowercase = false;
        } else {
            result.push(ch);
            prev_lowercase = ch.is_lowercase();
        }
    }
    result
}

pub fn generate_message_processor_rs(
    entities: &[EntityDef],
    output_dir: &Path,
) -> Result<(), Box<dyn Error>> {
    let file = output_dir.join("src/message_processor.rs");
    let mut output = std::fs::File::create(&file)?;

    writeln!(output, "// Auto-generated message processor")?;
    writeln!(output, "// Routes parsed messages to entity processors\n")?;

    writeln!(output, "use crate::parsers::ParsedMessage;")?;
    writeln!(output, "use crate::database::DbConnection;")?;
    writeln!(output, "use crate::error::AppError;")?;
    writeln!(output, "use crate::entity_processors;")?;
    writeln!(output, "use async_nats::jetstream::Context;\n")?;

    writeln!(output, "/// Process all derived entities from parsed message")?;
    writeln!(output, "pub async fn process_derived_entities(")?;
    writeln!(output, "    parsed: &ParsedMessage,")?;
    writeln!(output, "    conn: &mut DbConnection,")?;
    writeln!(output, "    jetstream: &Context,")?;
    writeln!(output, ") -> Result<(), AppError> {{")?;

    // Find derived entities (non-root entities)
    let derived_entities: Vec<_> = entities.iter()
        .filter(|e| e.source_type != "root")
        .collect();

    if derived_entities.is_empty() {
        writeln!(output, "    // No derived entities to process")?;
        writeln!(output, "    Ok(())")?;
        writeln!(output, "}}")?;
        return Ok(());
    }

    writeln!(output, "    // Process each derived entity")?;
    writeln!(output, "    // Errors are logged but don't stop processing of other entities\n")?;

    for entity in derived_entities {
        let processor_name = format!("process_{}", to_snake_case(&entity.name));
        writeln!(output, "    // Process {}", entity.name)?;
        writeln!(output, "    if let Err(e) = entity_processors::{}(parsed, conn, jetstream).await {{", processor_name)?;
        writeln!(output, "        tracing::warn!(\"Failed to process {}: {{:?}}\", e);", entity.name)?;
        writeln!(output, "    }}\n")?;
    }

    writeln!(output, "    Ok(())")?;
    writeln!(output, "}}")?;

    Ok(())
}
