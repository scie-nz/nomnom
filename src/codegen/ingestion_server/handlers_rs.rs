/// Generate handlers.rs with API endpoint implementations

use crate::codegen::EntityDef;
use std::path::Path;
use std::error::Error;
use std::io::Write;

pub fn generate_handlers_rs(
    entities: &[EntityDef],
    output_dir: &Path,
) -> Result<(), Box<dyn Error>> {
    let handlers_file = output_dir.join("src/handlers.rs");
    let mut output = std::fs::File::create(&handlers_file)?;

    writeln!(output, "// Auto-generated request handlers")?;
    writeln!(output)?;
    writeln!(output, "use axum::{{")?;
    writeln!(output, "    extract::State,")?;
    writeln!(output, "    http::StatusCode,")?;
    writeln!(output, "    response::IntoResponse,")?;
    writeln!(output, "    Json,")?;
    writeln!(output, "}};")?;
    writeln!(output, "use diesel::prelude::*;")?;
    writeln!(output, "use crate::{{")?;
    writeln!(output, "    database::DbPool,")?;
    writeln!(output, "    models::{{IngestResponse, BatchResponse, HealthResponse}},")?;
    writeln!(output, "    parsers::{{MessageParser, ParsedMessage}},")?;
    writeln!(output, "    error::AppError,")?;
    writeln!(output, "}};\n")?;

    // Generate ingest_message handler
    generate_ingest_message_handler(&mut output, entities)?;

    // Generate ingest_batch handler
    generate_ingest_batch_handler(&mut output, entities)?;

    // Generate health_check handler
    generate_health_check_handler(&mut output, entities)?;

    // Generate stats handler
    generate_stats_handler(&mut output)?;

    Ok(())
}

fn generate_ingest_message_handler(
    output: &mut std::fs::File,
    entities: &[EntityDef],
) -> Result<(), Box<dyn Error>> {
    writeln!(output, "/// Ingest a single message")?;
    writeln!(output, "#[utoipa::path(")?;
    writeln!(output, "    post,")?;
    writeln!(output, "    path = \"/ingest/message\",")?;
    writeln!(output, "    request_body = String,")?;
    writeln!(output, "    responses(")?;
    writeln!(output, "        (status = 200, description = \"Message ingested successfully\", body = IngestResponse),")?;
    writeln!(output, "        (status = 400, description = \"Invalid message format\")")?;
    writeln!(output, "    )")?;
    writeln!(output, ")]")?;
    writeln!(output, "pub async fn ingest_message(")?;
    writeln!(output, "    State(pool): State<DbPool>,")?;
    writeln!(output, "    body: String,")?;
    writeln!(output, ") -> Result<Json<IngestResponse>, AppError> {{")?;
    writeln!(output, "    let start = std::time::Instant::now();\n")?;

    writeln!(output, "    // Parse JSON message")?;
    writeln!(output, "    let (entity_name, parsed) = MessageParser::parse_json(&body)?;\n")?;

    writeln!(output, "    // Get database connection")?;
    writeln!(output, "    let mut conn = pool.get()?;\n")?;

    writeln!(output, "    // Insert into database based on entity type")?;
    writeln!(output, "    let id = match parsed {{")?;

    for entity in entities {
        // Only include root entities that are persistent
        if !entity.is_root() || !entity.is_persistent() || entity.is_abstract {
            continue;
        }
        if entity.source_type.to_lowercase() == "reference" {
            continue;
        }

        let table_name = if let Some(ref db_config) = entity.get_database_config() {
            &db_config.conformant_table
        } else {
            continue;
        };

        writeln!(output, "        ParsedMessage::{}(msg) => {{", entity.name)?;
        writeln!(output, "            // TODO: Map {}Message to actual Diesel model and insert", entity.name)?;
        writeln!(output, "            // For now, just return a placeholder ID")?;
        writeln!(output, "            1")?;
        writeln!(output, "        }}")?;
    }

    writeln!(output, "    }};\n")?;

    writeln!(output, "    Ok(Json(IngestResponse {{")?;
    writeln!(output, "        status: \"success\".to_string(),")?;
    writeln!(output, "        entity: entity_name,")?;
    writeln!(output, "        id,")?;
    writeln!(output, "        timestamp: chrono::Utc::now(),")?;
    writeln!(output, "        duration_ms: start.elapsed().as_millis() as u64,")?;
    writeln!(output, "    }}))")?;
    writeln!(output, "}}\n")?;

    Ok(())
}

fn generate_ingest_batch_handler(
    output: &mut std::fs::File,
    _entities: &[EntityDef],
) -> Result<(), Box<dyn Error>> {
    writeln!(output, "/// Ingest a batch of messages")?;
    writeln!(output, "#[utoipa::path(")?;
    writeln!(output, "    post,")?;
    writeln!(output, "    path = \"/ingest/batch\",")?;
    writeln!(output, "    request_body = String,")?;
    writeln!(output, "    responses(")?;
    writeln!(output, "        (status = 200, description = \"Batch processed\", body = BatchResponse)")?;
    writeln!(output, "    )")?;
    writeln!(output, ")]")?;
    writeln!(output, "pub async fn ingest_batch(")?;
    writeln!(output, "    State(pool): State<DbPool>,")?;
    writeln!(output, "    body: String,")?;
    writeln!(output, ") -> Result<Json<BatchResponse>, AppError> {{")?;
    writeln!(output, "    let start = std::time::Instant::now();")?;
    writeln!(output, "    let lines: Vec<&str> = body.lines().collect();\n")?;

    writeln!(output, "    let mut processed = 0;")?;
    writeln!(output, "    let mut inserted = 0;")?;
    writeln!(output, "    let mut failed = 0;")?;
    writeln!(output, "    let mut errors = Vec::new();\n")?;

    writeln!(output, "    let mut conn = pool.get()?;\n")?;

    writeln!(output, "    for (line_num, line) in lines.iter().enumerate() {{")?;
    writeln!(output, "        processed += 1;\n")?;

    writeln!(output, "        match MessageParser::parse_json(line) {{")?;
    writeln!(output, "            Ok((_entity_name, _parsed)) => {{")?;
    writeln!(output, "                // TODO: Insert parsed message into database")?;
    writeln!(output, "                inserted += 1;")?;
    writeln!(output, "            }}")?;
    writeln!(output, "            Err(e) => {{")?;
    writeln!(output, "                failed += 1;")?;
    writeln!(output, "                errors.push(format!(\"Line {{}}: {{:?}}\", line_num + 1, e));")?;
    writeln!(output, "            }}")?;
    writeln!(output, "        }}")?;
    writeln!(output, "    }}\n")?;

    writeln!(output, "    Ok(Json(BatchResponse {{")?;
    writeln!(output, "        status: if failed == 0 {{ \"success\" }} else {{ \"partial\" }}.to_string(),")?;
    writeln!(output, "        processed,")?;
    writeln!(output, "        inserted,")?;
    writeln!(output, "        failed,")?;
    writeln!(output, "        errors,")?;
    writeln!(output, "        duration_ms: start.elapsed().as_millis() as u64,")?;
    writeln!(output, "    }}))")?;
    writeln!(output, "}}\n")?;

    Ok(())
}

fn generate_health_check_handler(
    output: &mut std::fs::File,
    entities: &[EntityDef],
) -> Result<(), Box<dyn Error>> {
    writeln!(output, "/// Health check endpoint")?;
    writeln!(output, "#[utoipa::path(")?;
    writeln!(output, "    get,")?;
    writeln!(output, "    path = \"/health\",")?;
    writeln!(output, "    responses(")?;
    writeln!(output, "        (status = 200, description = \"Service is healthy\", body = HealthResponse)")?;
    writeln!(output, "    )")?;
    writeln!(output, ")]")?;
    writeln!(output, "pub async fn health_check(")?;
    writeln!(output, "    State(pool): State<DbPool>,")?;
    writeln!(output, ") -> Result<Json<HealthResponse>, AppError> {{")?;
    writeln!(output, "    // Test database connection")?;
    writeln!(output, "    let mut conn = pool.get()?;")?;
    writeln!(output, "    diesel::sql_query(\"SELECT 1\").execute(&mut conn)?;\n")?;

    writeln!(output, "    let entities = vec![")?;
    for entity in entities {
        // Only include root entities that are persistent
        if !entity.is_root() || !entity.is_persistent() || entity.is_abstract {
            continue;
        }
        if entity.source_type.to_lowercase() == "reference" {
            continue;
        }
        writeln!(output, "        \"{}\".to_string(),", entity.name)?;
    }
    writeln!(output, "    ];\n")?;

    writeln!(output, "    Ok(Json(HealthResponse {{")?;
    writeln!(output, "        status: \"healthy\".to_string(),")?;
    writeln!(output, "        database: \"connected\".to_string(),")?;
    writeln!(output, "        entities,")?;
    writeln!(output, "        version: env!(\"CARGO_PKG_VERSION\").to_string(),")?;
    writeln!(output, "    }}))")?;
    writeln!(output, "}}\n")?;

    Ok(())
}

fn generate_stats_handler(
    output: &mut std::fs::File,
) -> Result<(), Box<dyn Error>> {
    writeln!(output, "/// Statistics endpoint")?;
    writeln!(output, "pub async fn stats() -> impl IntoResponse {{")?;
    writeln!(output, "    // TODO: Implement metrics collection")?;
    writeln!(output, "    Json(serde_json::json!({{")?;
    writeln!(output, "        \"total_messages_processed\": 0,")?;
    writeln!(output, "        \"uptime_seconds\": 0,")?;
    writeln!(output, "    }}))")?;
    writeln!(output, "}}")?;

    Ok(())
}
