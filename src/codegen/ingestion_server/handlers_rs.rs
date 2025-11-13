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
    writeln!(output, "    extract::{{State, Path}},")?;
    writeln!(output, "    http::StatusCode,")?;
    writeln!(output, "    response::IntoResponse,")?;
    writeln!(output, "    Json,")?;
    writeln!(output, "}};")?;
    writeln!(output, "use diesel::prelude::*;")?;
    writeln!(output, "use std::sync::Arc;")?;
    writeln!(output, "use uuid::Uuid;")?;
    writeln!(output, "use crate::{{")?;
    writeln!(output, "    database::DbPool,")?;
    writeln!(output, "    models::{{IngestRequest, IngestResponse, BatchResponse, HealthResponse}},")?;
    writeln!(output, "    parsers::{{MessageParser, ParsedMessage}},")?;
    writeln!(output, "    error::AppError,")?;
    writeln!(output, "    nats_client::NatsClient,")?;
    writeln!(output, "    message_envelope::{{MessageEnvelope, IngestionResponse, IngestionStatus}},")?;
    writeln!(output, "}};\n")?;
    writeln!(output, "/// Application state shared across handlers")?;
    writeln!(output, "#[derive(Clone)]")?;
    writeln!(output, "pub struct AppState {{")?;
    writeln!(output, "    pub nats: NatsClient,")?;
    writeln!(output, "    pub db_pool: DbPool,")?;
    writeln!(output, "}}\n")?;

    // Generate ingest_message handler
    generate_ingest_message_handler(&mut output, entities)?;

    // Generate ingest_batch handler
    generate_ingest_batch_handler(&mut output, entities)?;

    // Generate health_check handler
    generate_health_check_handler(&mut output, entities)?;

    // Generate ready_check handler
    generate_ready_check_handler(&mut output)?;

    // Generate stats handler
    generate_stats_handler(&mut output)?;

    // Generate status check handler
    generate_status_check_handler(&mut output)?;

    Ok(())
}

fn generate_ingest_message_handler(
    output: &mut std::fs::File,
    entities: &[EntityDef],
) -> Result<(), Box<dyn Error>> {
    writeln!(output, "/// Ingest a single message (async via NATS)")?;
    writeln!(output, "#[utoipa::path(")?;
    writeln!(output, "    post,")?;
    writeln!(output, "    path = \"/ingest/message\",")?;
    writeln!(output, "    request_body = IngestRequest,")?;
    writeln!(output, "    responses(")?;
    writeln!(output, "        (status = 202, description = \"Message accepted for processing\", body = IngestionResponse),")?;
    writeln!(output, "        (status = 400, description = \"Invalid message format\")")?;
    writeln!(output, "    )")?;
    writeln!(output, ")]")?;
    writeln!(output, "pub async fn ingest_message(")?;
    writeln!(output, "    State(state): State<Arc<AppState>>,")?;
    writeln!(output, "    Json(request): Json<IngestRequest>,")?;
    writeln!(output, ") -> Result<(StatusCode, Json<IngestionResponse>), AppError> {{")?;
    writeln!(output, "    eprintln!(\"[INGESTION-SERVER] Received message (base64 {{}} bytes)\", request.body_base64.len());\n")?;

    writeln!(output, "    // Decode base64 body")?;
    writeln!(output, "    let body_bytes = base64::decode(&request.body_base64)")?;
    writeln!(output, "        .map_err(|e| {{")?;
    writeln!(output, "            eprintln!(\"[INGESTION-SERVER] Base64 decode error: {{}}\", e);")?;
    writeln!(output, "            AppError::ValidationError(format!(\"Invalid base64: {{}}\", e))")?;
    writeln!(output, "        }})?;")?;
    writeln!(output, "    let body = String::from_utf8(body_bytes)")?;
    writeln!(output, "        .map_err(|e| {{")?;
    writeln!(output, "            eprintln!(\"[INGESTION-SERVER] UTF-8 decode error: {{}}\", e);")?;
    writeln!(output, "            AppError::ValidationError(format!(\"Invalid UTF-8: {{}}\", e))")?;
    writeln!(output, "        }})?;\n")?;

    writeln!(output, "    eprintln!(\"[INGESTION-SERVER] Decoded message ({{}} bytes)\", body.len());\n")?;

    writeln!(output, "    // Optionally validate JSON format (but don't parse entities yet)")?;
    writeln!(output, "    let json_value: serde_json::Value = serde_json::from_str(&body)")?;
    writeln!(output, "        .map_err(|e| {{")?;
    writeln!(output, "            eprintln!(\"[INGESTION-SERVER] JSON parse error: {{}}\", e);")?;
    writeln!(output, "            eprintln!(\"[INGESTION-SERVER] Raw body: {{}}\", body);")?;
    writeln!(output, "            AppError::ValidationError(format!(\"Invalid JSON: {{}}\", e))")?;
    writeln!(output, "        }})?;\n")?;

    writeln!(output, "    // Pretty-print the JSON for debugging")?;
    writeln!(output, "    if let Ok(pretty) = serde_json::to_string_pretty(&json_value) {{")?;
    writeln!(output, "        eprintln!(\"[INGESTION-SERVER] Parsed JSON:\\n{{}}\", pretty);")?;
    writeln!(output, "    }}\n")?;

    writeln!(output, "    // Use entity_type from request or extract from JSON")?;
    writeln!(output, "    let entity_type = request.entity_type.or_else(|| {{")?;
    writeln!(output, "        json_value.get(\"entity_type\")")?;
    writeln!(output, "            .and_then(|t| t.as_str())")?;
    writeln!(output, "            .map(String::from)")?;
    writeln!(output, "    }});\n")?;

    writeln!(output, "    if let Some(ref et) = entity_type {{")?;
    writeln!(output, "        eprintln!(\"[INGESTION-SERVER] Entity type hint: {{}}\", et);")?;
    writeln!(output, "    }} else {{")?;
    writeln!(output, "        eprintln!(\"[INGESTION-SERVER] No entity_type hint provided\");")?;
    writeln!(output, "    }}\n")?;

    writeln!(output, "    // Create message envelope")?;
    writeln!(output, "    let envelope = MessageEnvelope::new(body, entity_type.clone());\n")?;

    writeln!(output, "    // Publish to NATS JetStream")?;
    writeln!(output, "    eprintln!(\"[INGESTION-SERVER] Publishing message {{}} to NATS\", envelope.message_id);")?;
    writeln!(output, "    state.nats.publish_message(&envelope).await")?;
    writeln!(output, "        .map_err(|e| {{")?;
    writeln!(output, "            eprintln!(\"[INGESTION-SERVER] NATS publish failed: {{}}\", e);")?;
    writeln!(output, "            AppError::InternalError(format!(\"NATS publish failed: {{}}\", e))")?;
    writeln!(output, "        }})?;\n")?;

    writeln!(output, "    eprintln!(\"[INGESTION-SERVER] Message {{}} successfully queued\", envelope.message_id);\n")?;

    writeln!(output, "    // Record message status in database")?;
    writeln!(output, "    let mut conn = state.db_pool.get()?;")?;
    writeln!(output, "    diesel::sql_query(")?;
    writeln!(output, "        r#\"INSERT INTO message_status (message_id, entity_type, status, received_at)")?;
    writeln!(output, "           VALUES ($1, $2, $3, $4)\"#")?;
    writeln!(output, "    )")?;
    writeln!(output, "    .bind::<diesel::sql_types::Uuid, _>(&envelope.message_id)")?;
    writeln!(output, "    .bind::<diesel::sql_types::Text, _>(entity_type.as_deref().unwrap_or(\"unknown\"))")?;
    writeln!(output, "    .bind::<diesel::sql_types::Text, _>(\"accepted\")")?;
    writeln!(output, "    .bind::<diesel::sql_types::Timestamp, _>(&envelope.received_at.naive_utc())")?;
    writeln!(output, "    .execute(&mut conn)")?;
    writeln!(output, "    .ok(); // Ignore errors - status tracking is optional\n")?;

    writeln!(output, "    tracing::info!(\"Message {{}} queued for processing\", envelope.message_id);\n")?;

    writeln!(output, "    Ok((")?;
    writeln!(output, "        StatusCode::ACCEPTED,")?;
    writeln!(output, "        Json(IngestionResponse {{")?;
    writeln!(output, "            message_id: envelope.message_id.to_string(),")?;
    writeln!(output, "            status: IngestionStatus::Accepted,")?;
    writeln!(output, "            timestamp: envelope.received_at,")?;
    writeln!(output, "        }})")?;
    writeln!(output, "    ))")?;
    writeln!(output, "}}\n")?;

    Ok(())
}

fn generate_ingest_batch_handler(
    output: &mut std::fs::File,
    _entities: &[EntityDef],
) -> Result<(), Box<dyn Error>> {
    writeln!(output, "/// Ingest a batch of messages (async via NATS)")?;
    writeln!(output, "#[utoipa::path(")?;
    writeln!(output, "    post,")?;
    writeln!(output, "    path = \"/ingest/batch\",")?;
    writeln!(output, "    request_body = String,")?;
    writeln!(output, "    responses(")?;
    writeln!(output, "        (status = 202, description = \"Batch accepted for processing\", body = BatchResponse)")?;
    writeln!(output, "    )")?;
    writeln!(output, ")]")?;
    writeln!(output, "pub async fn ingest_batch(")?;
    writeln!(output, "    State(state): State<Arc<AppState>>,")?;
    writeln!(output, "    body: String,")?;
    writeln!(output, ") -> Result<(StatusCode, Json<BatchResponse>), AppError> {{")?;
    writeln!(output, "    let start = std::time::Instant::now();")?;
    writeln!(output, "    let lines: Vec<&str> = body.lines().collect();\n")?;

    writeln!(output, "    let mut processed = 0;")?;
    writeln!(output, "    let mut inserted = 0;")?;
    writeln!(output, "    let mut failed = 0;")?;
    writeln!(output, "    let mut errors = Vec::new();\n")?;

    writeln!(output, "    for (line_num, line) in lines.iter().enumerate() {{")?;
    writeln!(output, "        processed += 1;\n")?;

    writeln!(output, "        // Validate JSON format")?;
    writeln!(output, "        match serde_json::from_str::<serde_json::Value>(line) {{")?;
    writeln!(output, "            Ok(_) => {{")?;
    writeln!(output, "                // Create envelope and publish to NATS")?;
    writeln!(output, "                let envelope = MessageEnvelope::new(line.to_string(), None);")?;
    writeln!(output, "                match state.nats.publish_message(&envelope).await {{")?;
    writeln!(output, "                    Ok(_) => inserted += 1,")?;
    writeln!(output, "                    Err(e) => {{")?;
    writeln!(output, "                        failed += 1;")?;
    writeln!(output, "                        errors.push(format!(\"Line {{}}: NATS error: {{}}\", line_num + 1, e));")?;
    writeln!(output, "                    }}")?;
    writeln!(output, "                }}")?;
    writeln!(output, "            }}")?;
    writeln!(output, "            Err(e) => {{")?;
    writeln!(output, "                failed += 1;")?;
    writeln!(output, "                errors.push(format!(\"Line {{}}: Invalid JSON: {{}}\", line_num + 1, e));")?;
    writeln!(output, "            }}")?;
    writeln!(output, "        }}")?;
    writeln!(output, "    }}\n")?;

    writeln!(output, "    Ok((")?;
    writeln!(output, "        StatusCode::ACCEPTED,")?;
    writeln!(output, "        Json(BatchResponse {{")?;
    writeln!(output, "            status: if failed == 0 {{ \"success\" }} else {{ \"partial\" }}.to_string(),")?;
    writeln!(output, "            processed,")?;
    writeln!(output, "            inserted,")?;
    writeln!(output, "            failed,")?;
    writeln!(output, "            errors,")?;
    writeln!(output, "            duration_ms: start.elapsed().as_millis() as u64,")?;
    writeln!(output, "        }})")?;
    writeln!(output, "    ))")?;
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
    writeln!(output, "    State(state): State<Arc<AppState>>,")?;
    writeln!(output, ") -> Result<Json<HealthResponse>, AppError> {{")?;
    writeln!(output, "    // Test database connection")?;
    writeln!(output, "    let mut conn = state.db_pool.get()?;")?;
    writeln!(output, "    diesel::sql_query(\"SELECT 1\").execute(&mut conn)?;\n")?;

    writeln!(output, "    let entities = vec![")?;
    for entity in entities {
        // Only include root entities (transient or persistent)
        if !entity.is_root() || entity.is_abstract {
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

fn generate_ready_check_handler(
    output: &mut std::fs::File,
) -> Result<(), Box<dyn Error>> {
    writeln!(output, "/// Readiness check endpoint (lightweight)")?;
    writeln!(output, "#[utoipa::path(")?;
    writeln!(output, "    get,")?;
    writeln!(output, "    path = \"/ready\",")?;
    writeln!(output, "    responses(")?;
    writeln!(output, "        (status = 200, description = \"Service is ready to accept traffic\")")?;
    writeln!(output, "    )")?;
    writeln!(output, ")]")?;
    writeln!(output, "pub async fn ready_check() -> impl IntoResponse {{")?;
    writeln!(output, "    // Lightweight check - just verify the app is running")?;
    writeln!(output, "    // Don't check database here (that's for /health)")?;
    writeln!(output, "    Json(serde_json::json!({{")?;
    writeln!(output, "        \"ready\": true,")?;
    writeln!(output, "        \"timestamp\": chrono::Utc::now().to_rfc3339()")?;
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
    writeln!(output, "}}\n")?;

    Ok(())
}

fn generate_status_check_handler(
    output: &mut std::fs::File,
) -> Result<(), Box<dyn Error>> {
    writeln!(output, "/// Check message processing status by ID")?;
    writeln!(output, "#[utoipa::path(")?;
    writeln!(output, "    get,")?;
    writeln!(output, "    path = \"/ingest/status/{{message_id}}\",")?;
    writeln!(output, "    responses(")?;
    writeln!(output, "        (status = 200, description = \"Message status retrieved\"),")?;
    writeln!(output, "        (status = 404, description = \"Message not found\")")?;
    writeln!(output, "    )")?;
    writeln!(output, ")]")?;
    writeln!(output, "pub async fn check_status(")?;
    writeln!(output, "    State(state): State<Arc<AppState>>,")?;
    writeln!(output, "    Path(message_id): Path<String>,")?;
    writeln!(output, ") -> Result<Json<serde_json::Value>, AppError> {{")?;
    writeln!(output, "    // Parse message_id as UUID")?;
    writeln!(output, "    let uuid = Uuid::parse_str(&message_id)")?;
    writeln!(output, "        .map_err(|_| AppError::ValidationError(\"Invalid UUID format\".to_string()))?;\n")?;

    writeln!(output, "    // Query message_status table")?;
    writeln!(output, "    let mut conn = state.db_pool.get()?;\n")?;

    writeln!(output, "    #[derive(diesel::QueryableByName)]")?;
    writeln!(output, "    struct MessageStatus {{")?;
    writeln!(output, "        #[diesel(sql_type = diesel::sql_types::Text)]")?;
    writeln!(output, "        entity_type: String,")?;
    writeln!(output, "        #[diesel(sql_type = diesel::sql_types::Text)]")?;
    writeln!(output, "        status: String,")?;
    writeln!(output, "        #[diesel(sql_type = diesel::sql_types::Timestamp)]")?;
    writeln!(output, "        received_at: chrono::NaiveDateTime,")?;
    writeln!(output, "        #[diesel(sql_type = diesel::sql_types::Nullable<diesel::sql_types::Timestamp>)]")?;
    writeln!(output, "        processed_at: Option<chrono::NaiveDateTime>,")?;
    writeln!(output, "        #[diesel(sql_type = diesel::sql_types::Integer)]")?;
    writeln!(output, "        retry_count: i32,")?;
    writeln!(output, "        #[diesel(sql_type = diesel::sql_types::Nullable<diesel::sql_types::Text>)]")?;
    writeln!(output, "        error_message: Option<String>,")?;
    writeln!(output, "    }}\n")?;

    writeln!(output, "    let result: Option<MessageStatus> = diesel::sql_query(")?;
    writeln!(output, "        r#\"SELECT entity_type, status, received_at, processed_at, retry_count, error_message")?;
    writeln!(output, "           FROM message_status WHERE message_id = $1\"#")?;
    writeln!(output, "    )")?;
    writeln!(output, "    .bind::<diesel::sql_types::Uuid, _>(&uuid)")?;
    writeln!(output, "    .get_result(&mut conn)")?;
    writeln!(output, "    .optional()?;\n")?;

    writeln!(output, "    match result {{")?;
    writeln!(output, "        Some(status) => {{")?;
    writeln!(output, "            Ok(Json(serde_json::json!({{")?;
    writeln!(output, "                \"message_id\": message_id,")?;
    writeln!(output, "                \"entity_type\": status.entity_type,")?;
    writeln!(output, "                \"status\": status.status,")?;
    writeln!(output, "                \"received_at\": status.received_at.and_utc().to_rfc3339(),")?;
    writeln!(output, "                \"processed_at\": status.processed_at.map(|t| t.and_utc().to_rfc3339()),")?;
    writeln!(output, "                \"retry_count\": status.retry_count,")?;
    writeln!(output, "                \"error_message\": status.error_message,")?;
    writeln!(output, "            }})))")?;
    writeln!(output, "        }}")?;
    writeln!(output, "        None => {{")?;
    writeln!(output, "            Err(AppError::ValidationError(\"Message not found\".to_string()))")?;
    writeln!(output, "        }}")?;
    writeln!(output, "    }}")?;
    writeln!(output, "}}")?;

    Ok(())
}
