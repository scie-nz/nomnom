/// Generate main.rs with NATS consumer loop

use crate::codegen::EntityDef;
use super::WorkerConfig;
use std::path::Path;
use std::error::Error;
use std::io::Write;

/// Convert field names to snake_case for SQL/Rust
/// Handles both camelCase and already-snake_case inputs
fn to_snake_case(s: &str) -> String {
    // If already contains underscores (except f_ prefix), likely already snake_case
    let stripped = if s.starts_with("f_") {
        &s[2..]
    } else {
        s
    };

    // Check if already snake_case (has underscores or all lowercase)
    if stripped.contains('_') || stripped.chars().all(|c| !c.is_uppercase()) {
        return s.to_string();
    }

    // Convert camelCase to snake_case
    let mut result = String::new();
    let mut prev_lowercase = false;

    for (i, ch) in s.chars().enumerate() {
        if ch.is_uppercase() {
            // Add underscore before uppercase if:
            // 1. Not at start
            // 2. Previous char was lowercase
            // 3. OR next char is lowercase (handles "XMLParser" -> "xml_parser")
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

pub fn generate_main_rs(
    entities: &[EntityDef],
    output_dir: &Path,
    _config: &WorkerConfig,
) -> Result<(), Box<dyn Error>> {
    let main_file = output_dir.join("src/main.rs");
    let mut output = std::fs::File::create(&main_file)?;

    writeln!(output, "// Auto-generated NATS worker")?;
    writeln!(output, "// Generated from entity definitions\n")?;

    writeln!(output, "use async_nats::jetstream;")?;
    writeln!(output, "use diesel::prelude::*;")?;
    writeln!(output, "use diesel::sql_types::{{Text, Integer, BigInt, Double, Bool, Date, Numeric, Nullable}};")?;
    writeln!(output, "use futures::StreamExt;")?;
    writeln!(output, "use std::time::Duration;\n")?;

    writeln!(output, "mod parsers;")?;
    writeln!(output, "mod models;")?;
    writeln!(output, "mod database;")?;
    writeln!(output, "mod error;")?;
    writeln!(output, "mod transforms;")?;
    writeln!(output, "mod entities;")?;
    writeln!(output, "mod extract;")?;
    writeln!(output, "mod persist_publish;")?;
    writeln!(output, "mod coordinator;\n")?;

    writeln!(output, "use database::{{create_pool, ensure_tables, DbConnection}};")?;
    writeln!(output, "use parsers::{{MessageParser, ParsedMessage}};")?;
    writeln!(output, "use error::AppError;\n")?;

    // Add database-agnostic UUID type handling
    writeln!(output, "// Database-agnostic UUID type handling")?;
    writeln!(output, "#[cfg(feature = \"postgres\")]")?;
    writeln!(output, "type UuidSqlType = diesel::sql_types::Uuid;\n")?;
    writeln!(output, "#[cfg(feature = \"mysql\")]")?;
    writeln!(output, "type UuidSqlType = diesel::sql_types::Text;\n")?;

    // Add UUID conversion helper
    writeln!(output, "#[cfg(feature = \"postgres\")]")?;
    writeln!(output, "#[inline]")?;
    writeln!(output, "fn uuid_to_sql_value(uuid: &uuid::Uuid) -> &uuid::Uuid {{")?;
    writeln!(output, "    uuid")?;
    writeln!(output, "}}\n")?;
    writeln!(output, "#[cfg(feature = \"mysql\")]")?;
    writeln!(output, "#[inline]")?;
    writeln!(output, "fn uuid_to_sql_value(uuid: &uuid::Uuid) -> String {{")?;
    writeln!(output, "    uuid.to_string()")?;
    writeln!(output, "}}\n")?;

    writeln!(output, "/// Message envelope from NATS API")?;
    writeln!(output, "#[derive(Debug, serde::Deserialize)]")?;
    writeln!(output, "struct MessageEnvelope {{")?;
    writeln!(output, "    message_id: uuid::Uuid,")?;
    writeln!(output, "    body: String,")?;
    writeln!(output, "    entity_type: Option<String>,")?;
    writeln!(output, "    received_at: chrono::DateTime<chrono::Utc>,")?;
    writeln!(output, "    retry_count: u32,")?;
    writeln!(output, "    source: Option<String>,")?;
    writeln!(output, "}}\n")?;

    writeln!(output, "#[tokio::main]")?;
    writeln!(output, "async fn main() {{")?;
    writeln!(output, "    // Initialize tracing")?;
    writeln!(output, "    eprintln!(\"[WORKER] Initializing worker...\");")?;
    writeln!(output, "    tracing_subscriber::fmt::init();")?;
    writeln!(output, "    eprintln!(\"[WORKER] Tracing initialized\");\n")?;

    writeln!(output, "    // Load environment variables")?;
    writeln!(output, "    dotenv::dotenv().ok();")?;
    writeln!(output, "    eprintln!(\"[WORKER] Environment loaded\");\n")?;

    writeln!(output, "    // Get NATS configuration")?;
    writeln!(output, "    let nats_url = std::env::var(\"NATS_URL\")")?;
    writeln!(output, "        .unwrap_or_else(|_| \"nats://localhost:4222\".to_string());")?;
    writeln!(output, "    let stream_name = std::env::var(\"NATS_STREAM\")")?;
    writeln!(output, "        .unwrap_or_else(|_| \"MESSAGES\".to_string());")?;
    writeln!(output, "    let consumer_name = std::env::var(\"NATS_CONSUMER\")")?;
    writeln!(output, "        .unwrap_or_else(|_| \"workers\".to_string());\n")?;

    writeln!(output, "    // Get worker configuration")?;
    writeln!(output, "    let max_deliver = std::env::var(\"MAX_DELIVER\")")?;
    writeln!(output, "        .ok()")?;
    writeln!(output, "        .and_then(|s| s.parse::<i64>().ok())")?;
    writeln!(output, "        .unwrap_or(3);")?;
    writeln!(output, "    let batch_size = std::env::var(\"BATCH_SIZE\")")?;
    writeln!(output, "        .ok()")?;
    writeln!(output, "        .and_then(|s| s.parse::<usize>().ok())")?;
    writeln!(output, "        .unwrap_or(10);")?;
    writeln!(output, "    let poll_interval_ms = std::env::var(\"POLL_INTERVAL_MS\")")?;
    writeln!(output, "        .ok()")?;
    writeln!(output, "        .and_then(|s| s.parse::<u64>().ok())")?;
    writeln!(output, "        .unwrap_or(100);\n")?;

    writeln!(output, "    // Create database pool")?;
    writeln!(output, "    eprintln!(\"[WORKER] Creating database pool...\");")?;
    writeln!(output, "    let db_pool = create_pool()")?;
    writeln!(output, "        .expect(\"Failed to create database pool\");")?;
    writeln!(output, "    eprintln!(\"[WORKER] Database pool created\");\n")?;

    writeln!(output, "    // Ensure tables exist")?;
    writeln!(output, "    eprintln!(\"[WORKER] Ensuring database tables exist...\");")?;
    writeln!(output, "    {{")?;
    writeln!(output, "        let mut conn = db_pool.get()")?;
    writeln!(output, "            .expect(\"Failed to get database connection\");")?;
    writeln!(output, "        ensure_tables(&mut conn)")?;
    writeln!(output, "            .expect(\"Failed to ensure tables exist\");")?;
    writeln!(output, "    }}")?;
    writeln!(output, "    eprintln!(\"[WORKER] Database tables ready\");\n")?;

    writeln!(output, "    // Connect to NATS")?;
    writeln!(output, "    eprintln!(\"[WORKER] Connecting to NATS at {{}}...\", nats_url);")?;
    writeln!(output, "    let client = async_nats::connect(&nats_url).await")?;
    writeln!(output, "        .expect(\"Failed to connect to NATS\");")?;
    writeln!(output, "    eprintln!(\"[WORKER] Connected to NATS\");")?;
    writeln!(output, "    tracing::info!(\"Connected to NATS at {{}}\", nats_url);\n")?;

    writeln!(output, "    // Get JetStream context")?;
    writeln!(output, "    eprintln!(\"[WORKER] Creating JetStream context...\");")?;
    writeln!(output, "    let jetstream = jetstream::new(client);")?;
    writeln!(output, "    eprintln!(\"[WORKER] JetStream context created\");\n")?;

    writeln!(output, "    // Get or create stream")?;
    writeln!(output, "    eprintln!(\"[WORKER] Getting/creating stream '{{}}'...\", stream_name);")?;
    writeln!(output, "    let stream = jetstream")?;
    writeln!(output, "        .get_or_create_stream(jetstream::stream::Config {{")?;
    writeln!(output, "            name: stream_name.clone(),")?;
    writeln!(output, "            subjects: vec![\"messages.ingest.>\".to_string()],")?;
    writeln!(output, "            max_age: Duration::from_secs(24 * 60 * 60),")?;
    writeln!(output, "            max_bytes: 512 * 1024 * 1024,")?;
    writeln!(output, "            storage: jetstream::stream::StorageType::File,")?;
    writeln!(output, "            num_replicas: 1,")?;
    writeln!(output, "            ..Default::default()")?;
    writeln!(output, "        }})")?;
    writeln!(output, "        .await")?;
    writeln!(output, "        .expect(\"Failed to get/create stream\");")?;
    writeln!(output, "    eprintln!(\"[WORKER] Stream '{{}}' ready\", stream_name);\n")?;

    writeln!(output, "    // Create ENTITIES stream for entity publishing")?;
    writeln!(output, "    eprintln!(\"[WORKER] Getting/creating ENTITIES stream...\");")?;
    writeln!(output, "    let _entities_stream = jetstream")?;
    writeln!(output, "        .get_or_create_stream(jetstream::stream::Config {{")?;
    writeln!(output, "            name: \"ENTITIES\".to_string(),")?;
    writeln!(output, "            subjects: vec![\"entities.*\".to_string()],")?;
    writeln!(output, "            max_age: Duration::from_secs(24 * 60 * 60),")?;
    writeln!(output, "            max_bytes: 512 * 1024 * 1024,")?;
    writeln!(output, "            storage: jetstream::stream::StorageType::File,")?;
    writeln!(output, "            num_replicas: 1,")?;
    writeln!(output, "            ..Default::default()")?;
    writeln!(output, "        }})")?;
    writeln!(output, "        .await")?;
    writeln!(output, "        .expect(\"Failed to get/create ENTITIES stream\");")?;
    writeln!(output, "    eprintln!(\"[WORKER] ENTITIES stream ready\");")?;
    writeln!(output, "    tracing::info!(\"ENTITIES stream ready for entity publishing\");\n")?;

    writeln!(output, "    // Create or get consumer")?;
    writeln!(output, "    eprintln!(\"[WORKER] Getting/creating consumer '{{}}'...\", consumer_name);")?;
    writeln!(output, "    let consumer = stream")?;
    writeln!(output, "        .get_or_create_consumer(")?;
    writeln!(output, "            &consumer_name,")?;
    writeln!(output, "            jetstream::consumer::pull::Config {{")?;
    writeln!(output, "                durable_name: Some(consumer_name.clone()),")?;
    writeln!(output, "                ack_policy: jetstream::consumer::AckPolicy::Explicit,")?;
    writeln!(output, "                max_deliver,")?;
    writeln!(output, "                filter_subject: \"messages.ingest.>\".to_string(),")?;
    writeln!(output, "                ..Default::default()")?;
    writeln!(output, "            }}")?;
    writeln!(output, "        )")?;
    writeln!(output, "        .await")?;
    writeln!(output, "        .expect(\"Failed to create consumer\");")?;
    writeln!(output, "    eprintln!(\"[WORKER] Consumer '{{}}' ready\", consumer_name);\n")?;

    writeln!(output, "    eprintln!(\"[WORKER] ========== WORKER READY ==========\");")?;
    writeln!(output, "    tracing::info!(")?;
    writeln!(output, "        \"Worker ready - consuming from stream '{{}}' with consumer '{{}}'\",")?;
    writeln!(output, "        stream_name,")?;
    writeln!(output, "        consumer_name")?;
    writeln!(output, "    );\n")?;

    // Count persistent entities for logging
    let entity_count = entities.iter()
        .filter(|e| e.is_persistent(entities) && !e.is_abstract && e.source_type.to_lowercase() != "reference")
        .count();
    writeln!(output, "    tracing::info!(\"Processing messages for {} entities\");", entity_count)?;
    writeln!(output, "    tracing::info!(\"Worker configuration: batch_size={{}}, poll_interval_ms={{}}, max_deliver={{}}\", batch_size, poll_interval_ms, max_deliver);")?;
    writeln!(output)?;

    writeln!(output, "    // Main message processing loop")?;
    writeln!(output, "    let mut iteration = 0u64;")?;
    writeln!(output, "    loop {{")?;
    writeln!(output, "        iteration += 1;")?;
    writeln!(output, "        tracing::debug!(\"Loop iteration {{}}: Fetching batch of up to {{}} messages...\", iteration, batch_size);")?;
    writeln!(output)?;
    writeln!(output, "        // Fetch batch of messages")?;
    writeln!(output, "        let mut messages = consumer")?;
    writeln!(output, "            .fetch()")?;
    writeln!(output, "            .max_messages(batch_size)")?;
    writeln!(output, "            .messages()")?;
    writeln!(output, "            .await")?;
    writeln!(output, "            .expect(\"Failed to fetch messages\");\n")?;
    writeln!(output, "        let mut msg_count = 0;")?;

    writeln!(output, "        while let Some(msg) = messages.next().await {{")?;
    writeln!(output, "            msg_count += 1;")?;
    writeln!(output, "            tracing::info!(\"[Iteration {{}}] Processing message {{}} in batch\", iteration, msg_count);")?;
    writeln!(output, "            let msg = match msg {{")?;
    writeln!(output, "                Ok(m) => m,")?;
    writeln!(output, "                Err(e) => {{")?;
    writeln!(output, "                    tracing::error!(\"Error receiving message: {{}}\", e);")?;
    writeln!(output, "                    continue;")?;
    writeln!(output, "                }}")?;
    writeln!(output, "            }};\n")?;

    writeln!(output, "            // Process message")?;
    writeln!(output, "            match process_message(&msg.payload, &db_pool, &jetstream).await {{")?;
    writeln!(output, "                Ok(_) => {{")?;
    writeln!(output, "                    // Acknowledge successful processing")?;
    writeln!(output, "                    if let Err(e) = msg.ack().await {{")?;
    writeln!(output, "                        tracing::error!(\"Failed to ACK message: {{}}\", e);")?;
    writeln!(output, "                    }}")?;
    writeln!(output, "                }}")?;
    writeln!(output, "                Err(e) => {{")?;
    writeln!(output, "                    tracing::error!(\"Failed to process message: {{:?}}\", e);\n")?;

    writeln!(output, "                    // Get delivery count to check if we should route to DLQ")?;
    writeln!(output, "                    let delivery_count = msg.info()")?;
    writeln!(output, "                        .map(|info| info.delivered)")?;
    writeln!(output, "                        .unwrap_or(1);\n")?;

    writeln!(output, "                    // Extract message info for status updates and DLQ routing")?;
    writeln!(output, "                    if let Ok(envelope) = serde_json::from_slice::<serde_json::Value>(&msg.payload) {{")?;
    writeln!(output, "                        if let Some(msg_id) = envelope.get(\"message_id\").and_then(|v| v.as_str()) {{")?;
    writeln!(output, "                            if let Ok(uuid) = uuid::Uuid::parse_str(msg_id) {{")?;
    writeln!(output, "                                if delivery_count >= max_deliver {{")?;
    writeln!(output, "                                    // Max retries reached - route to DLQ")?;
    writeln!(output, "                                    tracing::warn!(")?;
    writeln!(output, "                                        \"Message {{}} failed after {{}} attempts, sending to DLQ\",")?;
    writeln!(output, "                                        msg_id,")?;
    writeln!(output, "                                        delivery_count")?;
    writeln!(output, "                                    );\n")?;

    writeln!(output, "                                    // Extract entity_type for DLQ subject")?;
    writeln!(output, "                                    let entity_type = envelope.get(\"entity_type\")")?;
    writeln!(output, "                                        .and_then(|v| v.as_str())")?;
    writeln!(output, "                                        .unwrap_or(\"unknown\");\n")?;

    writeln!(output, "                                    // Publish to DLQ stream")?;
    writeln!(output, "                                    let dlq_subject = format!(\"messages.dlq.{{}}\", entity_type);")?;
    writeln!(output, "                                    if let Err(dlq_err) = jetstream")?;
    writeln!(output, "                                        .publish(dlq_subject.clone(), msg.payload.clone())")?;
    writeln!(output, "                                        .await")?;
    writeln!(output, "                                    {{")?;
    writeln!(output, "                                        tracing::error!(\"Failed to publish to DLQ: {{}}\", dlq_err);")?;
    writeln!(output, "                                    }} else {{")?;
    writeln!(output, "                                        tracing::info!(\"Message {{}} routed to DLQ\", msg_id);")?;
    writeln!(output, "                                    }}\n")?;

    writeln!(output, "                                    // Update status to 'dlq'")?;
    writeln!(output, "                                    if let Ok(mut conn) = db_pool.get() {{")?;
    writeln!(output, "                                        diesel::sql_query(")?;
    writeln!(output, "                                            \"UPDATE message_status SET status = ?, error_message = ? WHERE message_id = ?\"")?;
    writeln!(output, "                                        )")?;
    writeln!(output, "                                        .bind::<Text, _>(\"dlq\")")?;
    writeln!(output, "                                        .bind::<Text, _>(&format!(\"Failed after {{}} attempts: {{:?}}\", delivery_count, e))")?;
    writeln!(output, "                                        .bind::<Text, _>(uuid.to_string())")?;
    writeln!(output, "                                        .execute(&mut conn)")?;
    writeln!(output, "                                        .map_err(|e| {{")?;
    writeln!(output, "                                            eprintln!(\"[WORKER] Failed to update message_status to dlq: {{:?}}\", e);")?;
    writeln!(output, "                                            e")?;
    writeln!(output, "                                        }})")?;
    writeln!(output, "                                        .ok();")?;
    writeln!(output, "                                    }}\n")?;

    writeln!(output, "                                    // ACK the original message (remove from main queue)")?;
    writeln!(output, "                                    if let Err(ack_err) = msg.ack().await {{")?;
    writeln!(output, "                                        tracing::error!(\"Failed to ACK DLQ message: {{}}\", ack_err);")?;
    writeln!(output, "                                    }}")?;
    writeln!(output, "                                }} else {{")?;
    writeln!(output, "                                    // Still have retries left - update status and NAK")?;
    writeln!(output, "                                    if let Ok(mut conn) = db_pool.get() {{")?;
    writeln!(output, "                                        diesel::sql_query(")?;
    writeln!(output, "                                            \"UPDATE message_status SET status = ?, error_message = ?, retry_count = retry_count + 1 WHERE message_id = ?\"")?;
    writeln!(output, "                                        )")?;
    writeln!(output, "                                        .bind::<Text, _>(\"failed\")")?;
    writeln!(output, "                                        .bind::<Text, _>(&format!(\"{{:?}}\", e))")?;
    writeln!(output, "                                        .bind::<Text, _>(uuid.to_string())")?;
    writeln!(output, "                                        .execute(&mut conn)")?;
    writeln!(output, "                                        .map_err(|e| {{")?;
    writeln!(output, "                                            eprintln!(\"[WORKER] Failed to update message_status retry: {{:?}}\", e);")?;
    writeln!(output, "                                            e")?;
    writeln!(output, "                                        }})")?;
    writeln!(output, "                                        .ok();")?;
    writeln!(output, "                                    }}\n")?;

    writeln!(output, "                                    // NAK for retry")?;
    writeln!(output, "                                    if let Err(nak_err) = msg.ack_with(jetstream::AckKind::Nak(None)).await {{")?;
    writeln!(output, "                                        tracing::error!(\"Failed to NAK message: {{}}\", nak_err);")?;
    writeln!(output, "                                    }}")?;
    writeln!(output, "                                }}")?;
    writeln!(output, "                            }}")?;
    writeln!(output, "                        }}")?;
    writeln!(output, "                    }}")?;
    writeln!(output, "                }}")?;
    writeln!(output, "            }}")?;
    writeln!(output, "        }}")?;
    writeln!(output)?;
    writeln!(output, "        if msg_count > 0 {{")?;
    writeln!(output, "            tracing::info!(\"[Iteration {{}}] Processed {{}} messages in batch\", iteration, msg_count);")?;
    writeln!(output, "        }} else {{")?;
    writeln!(output, "            tracing::debug!(\"[Iteration {{}}] No messages in batch, sleeping for {{}}ms\", iteration, poll_interval_ms);")?;
    writeln!(output, "        }}")?;
    writeln!(output)?;
    writeln!(output, "        // Small delay between batches")?;
    writeln!(output, "        tokio::time::sleep(Duration::from_millis(poll_interval_ms)).await;")?;
    writeln!(output, "    }}")?;
    writeln!(output, "}}\n")?;

    writeln!(output, "/// Process a single message")?;
    writeln!(output, "async fn process_message(")?;
    writeln!(output, "    payload: &[u8],")?;
    writeln!(output, "    pool: &database::DbPool,")?;
    writeln!(output, "    jetstream: &jetstream::Context,")?;
    writeln!(output, ") -> Result<(), AppError> {{")?;
    writeln!(output, "    tracing::info!(\"========== PROCESSING MESSAGE ==========\");")?;
    writeln!(output, "    eprintln!(\"[WORKER] Received message ({{}} bytes)\", payload.len());\n")?;

    writeln!(output, "    // Deserialize envelope")?;
    writeln!(output, "    let envelope: MessageEnvelope = serde_json::from_slice(payload)")?;
    writeln!(output, "        .map_err(|e| {{")?;
    writeln!(output, "            eprintln!(\"[WORKER] Envelope deserialization error: {{}}\", e);")?;
    writeln!(output, "            eprintln!(\"[WORKER] Raw payload: {{}}\", String::from_utf8_lossy(payload));")?;
    writeln!(output, "            AppError::ValidationError(format!(\"Invalid envelope: {{}}\", e))")?;
    writeln!(output, "        }})?;\n")?;

    writeln!(output, "    let message_id = envelope.message_id;")?;
    writeln!(output, "    eprintln!(\"[WORKER] Processing message {{}}\", message_id);")?;
    writeln!(output, "    if let Some(ref et) = envelope.entity_type {{")?;
    writeln!(output, "        eprintln!(\"[WORKER] Entity type: {{}}\", et);")?;
    writeln!(output, "    }}")?;
    writeln!(output, "    tracing::debug!(\"Processing message {{}}\", message_id);\n")?;

    writeln!(output, "    // Get database connection")?;
    writeln!(output, "    let mut conn = pool.get()?;\n")?;

    writeln!(output, "    // Update status to 'processing'")?;
    writeln!(output, "    diesel::sql_query(")?;
    writeln!(output, "        \"UPDATE message_status SET status = ? WHERE message_id = ?\"")?;
    writeln!(output, "    )")?;
    writeln!(output, "    .bind::<Text, _>(\"processing\")")?;
    writeln!(output, "    .bind::<Text, _>(message_id.to_string())")?;
    writeln!(output, "    .execute(&mut conn)")?;
    writeln!(output, "    .map_err(|e| {{")?;
    writeln!(output, "        eprintln!(\"[WORKER] Failed to update message_status to processing: {{:?}}\", e);")?;
    writeln!(output, "        e")?;
    writeln!(output, "    }})")?;
    writeln!(output, "    .ok(); // Ignore errors - status tracking is optional\n")?;

    writeln!(output, "    // Parse message body using entity-specific parsers")?;
    writeln!(output, "    // Use entity_type hint from envelope if available")?;
    writeln!(output, "    eprintln!(\"[WORKER] Parsing message body...\");")?;
    writeln!(output, "    let (entity_name, parsed, raw_json) = MessageParser::parse_json(&envelope.body, envelope.entity_type.as_deref())")?;
    writeln!(output, "        .map_err(|e| {{")?;
    writeln!(output, "            eprintln!(\"[WORKER] Parse error: {{:?}}\", e);")?;
    writeln!(output, "            // Try to pretty-print the JSON for debugging")?;
    writeln!(output, "            if let Ok(json_val) = serde_json::from_str::<serde_json::Value>(&envelope.body) {{")?;
    writeln!(output, "                if let Ok(pretty) = serde_json::to_string_pretty(&json_val) {{")?;
    writeln!(output, "                    eprintln!(\"[WORKER] Failed to parse entity from JSON:\\n{{}}\", pretty);")?;
    writeln!(output, "                }}")?;
    writeln!(output, "            }} else {{")?;
    writeln!(output, "                eprintln!(\"[WORKER] Raw body (not valid JSON): {{}}\", envelope.body);")?;
    writeln!(output, "            }}")?;
    writeln!(output, "            e")?;
    writeln!(output, "        }})?;")?;
    writeln!(output, "    eprintln!(\"[WORKER] Successfully parsed as entity: {{}}\", entity_name);\n")?;

    writeln!(output, "    // Publish entity to its entity-specific NATS stream for testing/observability")?;
    writeln!(output, "    let entity_stream_subject = format!(\"entities.{{}}\", entity_name);")?;
    writeln!(output, "    let entity_json = serde_json::to_string(&raw_json)")?;
    writeln!(output, "        .map_err(|e| {{")?;
    writeln!(output, "            eprintln!(\"[WORKER] Failed to serialize entity for publishing: {{:?}}\", e);")?;
    writeln!(output, "            AppError::ValidationError(format!(\"Entity serialization failed: {{}}\", e))")?;
    writeln!(output, "        }})?;")?;
    writeln!(output, "    jetstream.publish(entity_stream_subject.clone(), entity_json.clone().into()).await")?;
    writeln!(output, "        .map_err(|e| {{")?;
    writeln!(output, "            eprintln!(\"[WORKER] Failed to publish entity to stream {{}}: {{:?}}\", entity_stream_subject, e);")?;
    writeln!(output, "            AppError::ValidationError(format!(\"Entity publishing failed: {{}}\", e))")?;
    writeln!(output, "        }})?;")?;
    writeln!(output, "    eprintln!(\"[WORKER] Published {{}} entity to stream {{}}\", entity_name, entity_stream_subject);\n")?;

    writeln!(output, "    // Process message using dependency-based coordinator")?;
    writeln!(output, "    match parsed {{")?;
    writeln!(output, "        ParsedMessage::Hl7v2MessageFile(ref msg) => {{")?;
    writeln!(output, "            eprintln!(\"[WORKER] Processing message with coordinator...\");")?;
    writeln!(output, "            coordinator::process_message(msg, &raw_json, &mut conn, jetstream).await?;")?;
    writeln!(output, "            eprintln!(\"[WORKER] Successfully processed message\");")?;
    writeln!(output)?;
    writeln!(output, "            // Update status to 'processed'")?;
    writeln!(output, "            diesel::sql_query(")?;
    writeln!(output, "                \"UPDATE message_status SET status = ?, processed_at = NOW() WHERE message_id = ?\"")?;
    writeln!(output, "            )")?;
    writeln!(output, "            .bind::<Text, _>(\"processed\")")?;
    writeln!(output, "            .bind::<Text, _>(message_id.to_string())")?;
    writeln!(output, "            .execute(&mut conn)")?;
    writeln!(output, "            .ok(); // Ignore errors - status tracking is optional")?;
    writeln!(output)?;
    writeln!(output, "            tracing::info!(\"========== MESSAGE PROCESSED SUCCESSFULLY ==========\");")?;
    writeln!(output, "            Ok(())")?;
    writeln!(output, "        }}")?;
    writeln!(output, "    }}")?;
    writeln!(output, "}}")?;

    Ok(())
}

/// Generate derived entity processor functions (legacy - now in entity_processor_rs.rs)
#[allow(dead_code)]
pub(super) fn generate_derived_entity_processors_for_single_root(
    output: &mut std::fs::File,
    root_entity: &EntityDef,
    all_entities: &[EntityDef],
) -> Result<(), Box<dyn Error>> {
    // Find derived persistent entities for this root
    let derived_entities: Vec<&EntityDef> = all_entities.iter()
        .filter(|e| {
            e.is_persistent(all_entities) &&
            !e.is_root() &&
            !e.is_abstract &&
            e.source_type.to_lowercase() == "derived" &&
            e.derives_from(&root_entity.name, all_entities)
        })
        .collect();

    // Find derived transient entities for this root (for NATS publishing)
    let transient_entities: Vec<&EntityDef> = all_entities.iter()
        .filter(|e| {
            !e.is_persistent(all_entities) &&
            !e.is_root() &&
            !e.is_abstract &&
            e.source_type.to_lowercase() == "derived" &&
            e.derives_from(&root_entity.name, all_entities)
        })
        .collect();

    // Only generate processor function if there are derived entities (persistent OR transient)
    if derived_entities.is_empty() && transient_entities.is_empty() {
        return Ok(());
    }

    // Generate processor function for this root entity
    writeln!(output)?;
    writeln!(output, "/// Process derived entities for {} ({} entities)",
        root_entity.name,
        derived_entities.iter().map(|e| e.name.as_str()).collect::<Vec<_>>().join(", "))?;
    writeln!(output, "pub async fn process_derived_entities(",)?;
    writeln!(output, "    {}: &parsers::{}Message,",
        root_entity.name.to_lowercase(), root_entity.name)?;
    writeln!(output, "    raw_json: &serde_json::Value,")?;
    writeln!(output, "    conn: &mut DbConnection,")?;
    writeln!(output, "    jetstream: &jetstream::Context,")?;
    writeln!(output, ") -> Result<(), AppError> {{")?;
    writeln!(output, "    use crate::transforms::*;")?;
    writeln!(output)?;

    // Process each derived transient entity FIRST (publish to NATS only, no database)
    // These must be extracted before persistent entities that depend on them
    for transient_entity in &transient_entities {
        generate_transient_derived_entity_extraction(output, transient_entity, root_entity, all_entities)?;
    }

    // Process each derived persistent entity
    // (this also publishes transient intermediate entities to NATS inline)
    for derived_entity in &derived_entities {
        generate_derived_entity_extraction(output, derived_entity, root_entity, all_entities)?;
    }

    writeln!(output, "    tracing::info!(\"Processed derived entities for {}\");", root_entity.name)?;
    writeln!(output, "    Ok(())")?;
    writeln!(output, "}}")?;

    Ok(())
}

/// Recursively collect entity dependencies in topological order (dependencies first)
#[allow(dead_code)]
pub(super) fn collect_entity_dependencies(
    entity_name: &str,
    all_entities: &[EntityDef],
    root_entity: &EntityDef,
    ordered_list: &mut Vec<String>,
    seen: &mut std::collections::HashSet<String>,
) {
    // Normalize entity name to lowercase for comparison
    let entity_name_lower = entity_name.to_lowercase();

    // Skip if already processed or is the root entity
    if seen.contains(&entity_name_lower) || entity_name_lower == root_entity.name.to_lowercase() {
        return;
    }

    // Find the entity definition (case-insensitive match)
    if let Some(entity) = all_entities.iter().find(|e| e.name.eq_ignore_ascii_case(entity_name)) {
        // First, process parent dependencies (for derived entities)
        if let Some(ref parent_name) = entity.parent {
            // Only process parent if it's not the root entity
            let parent_is_root = parent_name.eq_ignore_ascii_case(&root_entity.name);
            eprintln!("DEBUG: Entity {} has parent {}, is_root={}", entity_name, parent_name, parent_is_root);
            if !parent_is_root {
                collect_entity_dependencies(
                    parent_name,
                    all_entities,
                    root_entity,
                    ordered_list,
                    seen
                );
            }
        }

        // Process derivation.source_entities dependencies
        if let Some(ref derivation) = entity.derivation {
            if let Some(ref source_entities) = derivation.source_entities {
                match source_entities {
                    serde_yaml::Value::Mapping(map) => {
                        for (_key, value) in map {
                            if let serde_yaml::Value::String(source_entity_name) = value {
                                if !source_entity_name.eq_ignore_ascii_case(&root_entity.name) {
                                    collect_entity_dependencies(
                                        source_entity_name,
                                        all_entities,
                                        root_entity,
                                        ordered_list,
                                        seen
                                    );
                                }
                            }
                        }
                    },
                    _ => {}
                }
            }
        }

        // Then, recursively process this entity's field dependencies
        for field in &entity.fields {
            if let Some(ref computed_from) = field.computed_from {
                for source in &computed_from.sources {
                    let source_entity = source.source_name();
                    collect_entity_dependencies(
                        source_entity,
                        all_entities,
                        root_entity,
                        ordered_list,
                        seen
                    );
                }
            }
        }

        // Then add this entity (after its dependencies) - use actual entity name, track lowercase
        seen.insert(entity_name_lower);
        ordered_list.push(entity.name.clone());
    }
}

/// Validate that at most one parent entity has repetition=repeated
#[allow(dead_code)]
pub(super) fn validate_parent_repetition(
    derived_entity: &EntityDef,
    all_entities: &[EntityDef],
) -> Result<(), String> {
    use std::collections::HashMap;

    // Build a map of entity names to entities for lookup
    let entities_by_name: HashMap<String, &EntityDef> = all_entities
        .iter()
        .map(|e| (e.name.clone(), e))
        .collect();

    // Get all parent names for this entity
    let parents: Vec<String> = if !derived_entity.parents.is_empty() {
        derived_entity.parents.iter().map(|p| p.parent_type.clone()).collect()
    } else if let Some(ref parent_name) = derived_entity.parent {
        vec![parent_name.clone()]
    } else {
        Vec::new()
    };

    // Find all parents that are marked as repeated
    let mut repeated_parents: Vec<String> = Vec::new();

    for parent_name in &parents {
        if let Some(parent_entity) = entities_by_name.get(parent_name) {
            if parent_entity.repetition.as_ref().map(|r| r == "repeated").unwrap_or(false) {
                repeated_parents.push(parent_name.clone());
            }
        }
    }

    // Validate: at most one parent can be repeated
    if repeated_parents.len() > 1 {
        return Err(format!(
            "Entity '{}' has multiple repeated parents: {:?}. Only one parent can be repeated.",
            derived_entity.name,
            repeated_parents
        ));
    }

    Ok(())
}

/// Generate extraction and persistence logic for a single derived entity
#[allow(dead_code)]
pub(super) fn generate_derived_entity_extraction(
    output: &mut std::fs::File,
    derived_entity: &EntityDef,
    root_entity: &EntityDef,
    all_entities: &[EntityDef],
) -> Result<(), Box<dyn Error>> {
    generate_derived_entity_extraction_internal(output, derived_entity, root_entity, all_entities, None)
}

/// Internal implementation with optional repeating context override
#[allow(dead_code)]
pub(super) fn generate_derived_entity_extraction_internal(
    output: &mut std::fs::File,
    derived_entity: &EntityDef,
    root_entity: &EntityDef,
    all_entities: &[EntityDef],
    repeating_context_override: Option<(&str, &str, &str)>, // (entity_name, segment_var, each_known_as)
) -> Result<(), Box<dyn Error>> {
    use std::collections::{HashMap, HashSet};

    // Validate parent repetition before processing
    validate_parent_repetition(derived_entity, all_entities)
        .map_err(|e| format!("Validation error for entity '{}': {}", derived_entity.name, e))?;

    // Get database config (may be inherited from parent via extends)
    let db_config = derived_entity.get_database_config(all_entities)
        .ok_or_else(|| format!("Entity '{}' is marked as persistent but has no database config", derived_entity.name))?;
    let table_name = &db_config.conformant_table;

    // Get field overrides (check own persistence first, then try parent)
    let fields = if let Some(ref persistence) = derived_entity.persistence {
        &persistence.field_overrides
    } else {
        // If no own persistence, find parent and get its field overrides
        if let Some(ref parent_name) = derived_entity.extends {
            if let Some(parent) = all_entities.iter().find(|e| &e.name == parent_name) {
                &parent.persistence.as_ref()
                    .ok_or_else(|| format!("Parent '{}' has no persistence config", parent_name))?
                    .field_overrides
            } else {
                return Err(format!("Parent entity '{}' not found", parent_name).into());
            }
        } else {
            return Err(format!("Entity '{}' has no persistence and no parent", derived_entity.name).into());
        }
    };

    // Generate root entity parameter name (lowercase)
    let root_param_name = root_entity.name.to_lowercase();

    writeln!(output, "    // Process {} entities", derived_entity.name)?;

    // Detect if this entity has a repeating parent
    let entities_by_name: HashMap<String, &EntityDef> = all_entities
        .iter()
        .map(|e| (e.name.clone(), e))
        .collect();

    let parents: Vec<String> = if !derived_entity.parents.is_empty() {
        derived_entity.parents.iter().map(|p| p.parent_type.clone()).collect()
    } else if let Some(ref parent_name) = derived_entity.parent {
        vec![parent_name.clone()]
    } else {
        Vec::new()
    };

    let mut repeating_parent_name: Option<String> = None;
    let mut repeating_field_name: Option<String> = None;
    let mut segments_source_entity: Option<String> = None;

    // Check if the derived entity itself has repeated_for configuration
    if let Some(ref repeated_for) = derived_entity.repeated_for {
        repeating_parent_name = Some(repeated_for.entity.clone());
        repeating_field_name = Some(repeated_for.field.clone());
        segments_source_entity = Some(repeated_for.entity.clone());
    } else {
        // Fallback: check if any parent has repetition: repeated
        for parent_name in &parents {
            if let Some(parent_entity) = entities_by_name.get(parent_name) {
                if parent_entity.repetition.as_ref().map(|r| r == "repeated").unwrap_or(false) {
                    repeating_parent_name = Some(parent_name.clone());

                    // Get the field name and source entity from the parent's repeated_for
                    if let Some(ref repeated_for) = parent_entity.repeated_for {
                        repeating_field_name = Some(repeated_for.field.clone());
                        segments_source_entity = Some(repeated_for.entity.clone());
                    }
                    break;
                }
            }
        }

        // Also check derivation.source_entities for repeated entities with repeated_for
        if repeating_parent_name.is_none() {
            if let Some(ref derivation) = derived_entity.derivation {
                if let Some(ref source_entities) = derivation.source_entities {
                    // Check each source entity for repeated_for
                    for (_key, value) in source_entities.as_mapping().unwrap() {
                        if let serde_yaml::Value::String(source_entity_name) = value {
                            if let Some(source_entity) = entities_by_name.get(source_entity_name.as_str()) {
                                if let Some(ref repeated_for) = source_entity.repeated_for {
                                    repeating_parent_name = Some(source_entity_name.clone());
                                    repeating_field_name = Some(repeated_for.field.clone());
                                    segments_source_entity = Some(repeated_for.entity.clone());
                                    break;
                                }
                            }
                        }
                    }
                }
            }
        }
    }

    let has_repeating_parent = repeating_parent_name.is_some();

    // Get each_known_as value for repeating loops
    let each_known_as = if has_repeating_parent {
        // First try the derived entity's own repeated_for
        if let Some(ref repeated_for) = derived_entity.repeated_for {
            repeated_for.each_known_as.clone()
        } else {
            // Fall back to checking the repeating parent entity's repeated_for
            let parent_name = repeating_parent_name.as_ref().unwrap();
            if let Some(parent_entity) = entities_by_name.get(parent_name.as_str()) {
                parent_entity.repeated_for.as_ref()
                    .expect(&format!("Parent entity {} with repetition: repeated must have repeated_for configuration", parent_name))
                    .each_known_as.clone()
            } else {
                panic!("Repeating parent {} not found in entities", parent_name);
            }
        }
    } else {
        String::new()
    };

    // Compute segments variable name if we have a repeating parent
    let segments_var = if has_repeating_parent {
        let source_entity = segments_source_entity.as_ref().unwrap();
        let source_snake = crate::codegen::utils::to_snake_case(source_entity);
        let field_name = repeating_field_name.as_ref().unwrap();
        Some(format!("{}_{}", source_snake, field_name))
    } else {
        None
    };

    // Build a map of field names to their definitions in the full entity
    // Include parent entity fields if entity extends another entity
    let mut field_defs: HashMap<String, &crate::codegen::types::FieldDef> = HashMap::new();

    // First, add parent entity fields (if extends)
    if let Some(ref parent_name) = derived_entity.extends {
        if let Some(parent_entity) = all_entities.iter().find(|e| &e.name == parent_name) {
            for field in &parent_entity.fields {
                field_defs.insert(field.name.clone(), field);
            }
        }
    }

    // Then, add/override with derived entity's own fields
    for field in &derived_entity.fields {
        field_defs.insert(field.name.clone(), field);
    }

    // Track which intermediate entities we need to instantiate (with dependencies)
    let mut needed_entities: Vec<String> = Vec::new();
    let mut seen: HashSet<String> = HashSet::new();

    // Check if entity has parent field
    if let Some(ref parent_name) = derived_entity.parent {
        if !parent_name.eq_ignore_ascii_case(&root_entity.name) {
            collect_entity_dependencies(
                parent_name,
                all_entities,
                root_entity,
                &mut needed_entities,
                &mut seen
            );
        }
    }

    // Check if entity has parents field (list format)
    for parent_def in &derived_entity.parents {
        let parent_type = &parent_def.parent_type;
        if !parent_type.eq_ignore_ascii_case(&root_entity.name) {
            collect_entity_dependencies(
                parent_type,
                all_entities,
                root_entity,
                &mut needed_entities,
                &mut seen
            );
        }
    }

    // Check if entity has derivation.source_entity (for multi-parent entities)
    if let Some(ref derivation) = derived_entity.derivation {
        if let Some(ref source_entities) = derivation.source_entities {
            // Handle source_entity as string, array, or mapping
            match source_entities {
                serde_yaml::Value::String(s) => {
                    if s.as_str() != root_entity.name.as_str() {
                        collect_entity_dependencies(
                            s.as_str(),
                            all_entities,
                            root_entity,
                            &mut needed_entities,
                            &mut seen
                        );
                    }
                },
                serde_yaml::Value::Mapping(map) => {
                    // For mappings like {mpi: MPI, event_type: EventType}, collect all values
                    for (_key, value) in map {
                        if let serde_yaml::Value::String(entity_name) = value {
                            if entity_name.as_str() != root_entity.name.as_str() {
                                collect_entity_dependencies(
                                    entity_name.as_str(),
                                    all_entities,
                                    root_entity,
                                    &mut needed_entities,
                                    &mut seen
                                );
                            }
                        }
                    }
                },
                _ => {} // Ignore other types
            }
        }
    }

    // First pass: recursively collect all intermediate entities needed
    for field in fields {
        let field_name = &field.name;

        if let Some(field_def) = field_defs.get(field_name) {
            if let Some(ref computed_from) = field_def.computed_from {
                for source in &computed_from.sources {
                    let source_entity = source.source_name();
                    if source_entity != root_entity.name.as_str() {
                        collect_entity_dependencies(
                            source_entity,
                            all_entities,
                            root_entity,
                            &mut needed_entities,
                            &mut seen
                        );
                    }
                }
            }
        }
    }

    // Generate intermediate entity instantiation (in dependency order)
    // Non-repeating intermediate entities are extracted outside the loop
    for entity_name in &needed_entities {
        // Skip the repeating parent - it will be generated inside the loop
        if has_repeating_parent && Some(entity_name.as_str()) == repeating_parent_name.as_deref() {
            continue;
        }

        if let Some(intermediate_entity) = all_entities.iter().find(|e| &e.name == entity_name) {
            writeln!(output, "    // Instantiate {} entity", entity_name)?;

            // For each field in the intermediate entity, generate extraction code
            for field in &intermediate_entity.fields {
                if let Some(ref computed_from) = field.computed_from {
                    let var_name = format!("{}_{}", crate::codegen::utils::to_snake_case(entity_name), field.name);
                    let field_type_str = field.field_type.as_str();
                    let is_nullable = field.nullable;
                    // Non-repeating intermediate entities are not from repeating parents, so pass None
                    generate_field_extraction(output, &var_name, field_type_str, computed_from, root_entity, &root_param_name, is_nullable, None, "    ", Some(intermediate_entity), true)?;
                }
            }

            // If this is a transient entity (no persistence), publish to NATS immediately
            if !intermediate_entity.is_persistent(all_entities) {
                publish_transient_entity_to_nats(output, intermediate_entity, "    ")?;
            }

            writeln!(output)?;
        }
    }

    // Open the loop if we have a repeating parent
    // BUT: skip loop creation if we have a repeating_context_override (already in a loop from parent)
    if has_repeating_parent && repeating_context_override.is_none() {
        writeln!(output, "")?;
        writeln!(output, "    // Loop over each {} segment and insert {} records",
            repeating_field_name.as_ref().unwrap(), derived_entity.name)?;
        writeln!(output, "    for {} in &{} {{", each_known_as, segments_var.as_ref().unwrap())?;

        // Inside the loop, generate the repeating parent entity fields
        if let Some(repeating_parent) = repeating_parent_name.as_ref() {
            if let Some(intermediate_entity) = all_entities.iter().find(|e| &e.name == repeating_parent) {
                writeln!(output, "        // Instantiate {} entity from current segment", repeating_parent)?;

                // For each field in the repeating entity, generate extraction code
                for field in &intermediate_entity.fields {
                    if let Some(ref computed_from) = field.computed_from {
                        let var_name = format!("{}_{}", crate::codegen::utils::to_snake_case(repeating_parent), field.name);
                        let field_type_str = field.field_type.as_str();
                        let is_nullable = field.nullable;
                        // Repeating parent fields are extracted from the loop variable
                        let repeating_info = Some((
                            repeating_parent.as_str(),
                            each_known_as.as_str(),
                            each_known_as.as_str(),
                        ));
                        generate_field_extraction(output, &var_name, field_type_str, computed_from, root_entity, &root_param_name, is_nullable, repeating_info, "        ", Some(intermediate_entity), true)?;
                    }
                }

                // If this is a transient entity (no persistence), publish to NATS immediately
                if !intermediate_entity.is_persistent(all_entities) {
                    publish_transient_entity_to_nats(output, intermediate_entity, "        ")?;
                }

                writeln!(output)?;
            }
        }
    } else {
        writeln!(output, "    // Extract fields from root entity data")?;
    }

    // Check if we should skip the autogenerated ID field
    let skip_autogenerated_id = db_config.autogenerate_conformant_id;
    let autogenerated_id_field = if skip_autogenerated_id {
        Some(&db_config.conformant_id_column)
    } else {
        None
    };

    // Prepare repeating parent info for field extraction
    let repeating_parent_info = if let Some(override_ctx) = repeating_context_override {
        // Use the provided override context (e.g., from repeated entity processor)
        Some(override_ctx)
    } else if has_repeating_parent {
        Some((
            repeating_parent_name.as_ref().unwrap().as_str(),
            each_known_as.as_str(),
            each_known_as.as_str(),
        ))
    } else {
        None
    };

    // Determine the base indent based on whether we're in a loop
    // When we have a repeating_context_override, we're not creating our own loop, so keep indent at "    "
    let base_indent = if has_repeating_parent && repeating_context_override.is_none() { "        " } else { "    " };

    // Generate field extraction for each field in derived entity (skip autogenerated ID unless it has computed_from)
    for field in fields {
        let field_name = &field.name;
        let is_nullable = field.nullable.unwrap_or(false);

        // Check if this field has computed_from defined
        let has_computed_from = field_defs.get(field_name)
            .and_then(|f| f.computed_from.as_ref())
            .is_some();

        // Skip autogenerated ID field ONLY if it doesn't have computed_from (database will handle it)
        if let Some(auto_id) = autogenerated_id_field {
            if field_name == auto_id && !has_computed_from {
                continue;
            }
        }

        // Try to find the field definition to get computed_from information
        if let Some(field_def) = field_defs.get(field_name) {
            if let Some(ref computed_from) = field_def.computed_from {
                // Generate extraction code based on computed_from configuration
                let field_type_str = field_def.field_type.as_str();
                generate_field_extraction(output, field_name, field_type_str, computed_from, root_entity, &root_param_name, is_nullable, repeating_parent_info, base_indent, Some(derived_entity), false)?;
                continue;
            }
        }

        // Fallback: if no computed_from, use placeholder
        if is_nullable {
            writeln!(output, "{}let {}: Option<String> = None; // No computed_from configuration",
                base_indent, field_name)?;
        } else {
            writeln!(output, "{}let {} = String::new(); // No computed_from configuration",
                base_indent, field_name)?;
        }
    }

    writeln!(output)?;

    // Build column names list (exclude autogenerated ID, use snake_case for SQL)
    let col_names: Vec<String> = fields.iter()
        .filter(|f| {
            if let Some(auto_id) = autogenerated_id_field {
                &f.name != auto_id
            } else {
                true
            }
        })
        .map(|f| to_snake_case(&f.name))
        .collect();

    // Database-specific SQL generation will be done inline below
    // (placeholders and conflict handling vary by database)

    // Find string-type unicity fields to check for emptiness
    // Use field names directly (not prefixed with entity name)
    let string_unicity_fields: Vec<String> = db_config.unicity_fields.iter()
        .filter_map(|field_name| {
            fields.iter().find(|f| &f.name == field_name)
                .and_then(|f| {
                    let field_type = f.field_type.as_deref().unwrap_or("String");
                    if field_type == "String" {
                        // Use field name directly (matches the variable name created above)
                        Some(field_name.to_string())
                    } else {
                        None
                    }
                })
        })
        .collect();

    // Generate conditional INSERT based on string unicity fields
    // Only insert if ALL string unicity fields have non-empty values
    let base_indent = if has_repeating_parent { "        " } else { "    " };
    let query_indent = if has_repeating_parent { "            " } else { "        " };

    if !string_unicity_fields.is_empty() {
        writeln!(output, "{}// Insert {} entity only if all string unicity fields are non-empty", base_indent, derived_entity.name)?;
        let conditions: Vec<String> = string_unicity_fields.iter()
            .map(|f| format!("({}.is_some() && {}.as_ref().map(|s| !s.is_empty()).unwrap_or(false))", f, f))
            .collect();
        writeln!(output, "{}if {} {{", base_indent, conditions.join(" && "))?;
    } else {
        writeln!(output, "{}// Insert {} entity", base_indent, derived_entity.name)?;
    }

    // Generate database-specific INSERT statement
    writeln!(output, "{}#[cfg(feature = \"postgres\")]", base_indent)?;
    writeln!(output, "{}{{", base_indent)?;

    // PostgreSQL: $1, $2, ... placeholders with ON CONFLICT
    let pg_placeholders: Vec<String> = (1..=col_names.len())
        .map(|i| format!("${}", i))
        .collect();
    let pg_on_conflict = if !db_config.unicity_fields.is_empty() {
        let snake_case_fields: Vec<String> = db_config.unicity_fields.iter()
            .map(|f| to_snake_case(f))
            .collect();
        format!(" ON CONFLICT ({}) DO NOTHING", snake_case_fields.join(", "))
    } else {
        String::new()
    };

    writeln!(output, "{}    diesel::sql_query(", base_indent)?;
    writeln!(output, "{}        r#\"INSERT INTO {} ({}) VALUES ({}){}\"#",
        base_indent,
        table_name,
        col_names.join(", "),
        pg_placeholders.join(", "),
        pg_on_conflict)?;
    writeln!(output, "{}    )", base_indent)?;

    writeln!(output, "{}}}", base_indent)?;
    writeln!(output, "{}#[cfg(feature = \"mysql\")]", base_indent)?;
    writeln!(output, "{}{{", base_indent)?;

    // MySQL: ? placeholders with INSERT IGNORE
    let mysql_placeholders = vec!["?"; col_names.len()].join(", ");
    let insert_keyword = if !db_config.unicity_fields.is_empty() {
        "INSERT IGNORE"
    } else {
        "INSERT"
    };

    writeln!(output, "{}    diesel::sql_query(", base_indent)?;
    writeln!(output, "{}        r#\"{} INTO {} ({}) VALUES ({})\"#",
        base_indent,
        insert_keyword,
        table_name,
        col_names.join(", "),
        mysql_placeholders)?;
    writeln!(output, "{}    )", base_indent)?;

    writeln!(output, "{}}}", base_indent)?;

    // Bind each field (skip autogenerated ID)
    for field in fields {
        // Skip autogenerated ID field
        if let Some(auto_id) = autogenerated_id_field {
            if &field.name == auto_id {
                continue;
            }
        }

        let field_type_str = field.field_type.as_deref().unwrap_or("String");
        let diesel_type = map_to_diesel_type(field_type_str);

        // Look up the entity field definition to check if runtime type is Option<T>
        // (Same logic as for root entities - check entity field's nullable, not field_override's nullable)
        let entity_field = field_defs.get(&field.name);
        let is_nullable = entity_field.map(|f| f.nullable).unwrap_or(false);

        if is_nullable {
            writeln!(output, "{}.bind::<Nullable<{}>, _>(&{})", base_indent, diesel_type, field.name)?;
        } else {
            writeln!(output, "{}.bind::<{}, _>(&{})", base_indent, diesel_type, field.name)?;
        }
    }

    writeln!(output, "{}.execute(conn)?;", base_indent)?;

    // Close the conditional if statement
    if !string_unicity_fields.is_empty() {
        if has_repeating_parent {
            writeln!(output, "        }}")?;
        } else {
            writeln!(output, "    }}")?;
        }
    }

    // Close the for loop if we have repeating parent
    if has_repeating_parent {
        writeln!(output, "    }}")?;
    }

    writeln!(output)?;

    Ok(())
}

/// Generate extraction and NATS publishing logic for a transient derived entity
#[allow(dead_code)]
pub(super) fn generate_transient_derived_entity_extraction(
    output: &mut std::fs::File,
    derived_entity: &EntityDef,
    root_entity: &EntityDef,
    all_entities: &[EntityDef],
) -> Result<(), Box<dyn Error>> {
    generate_transient_derived_entity_extraction_internal(output, derived_entity, root_entity, all_entities, None)
}

/// Internal implementation with optional repeating context
#[allow(dead_code)]
pub(super) fn generate_transient_derived_entity_extraction_internal(
    output: &mut std::fs::File,
    derived_entity: &EntityDef,
    root_entity: &EntityDef,
    all_entities: &[EntityDef],
    repeating_context: Option<(&str, &str, &str)>, // (entity_name, segment_var, each_known_as)
) -> Result<(), Box<dyn Error>> {
    use std::collections::{HashMap, HashSet};

    writeln!(output, "    // Extract and publish transient entity: {}", derived_entity.name)?;

    let entity_prefix = crate::codegen::utils::to_snake_case(&derived_entity.name);
    let root_param_name = root_entity.name.to_lowercase();

    // Build field definitions map
    let mut field_defs: HashMap<String, &crate::codegen::types::FieldDef> = HashMap::new();
    for field in &derived_entity.fields {
        field_defs.insert(field.name.clone(), field);
    }

    // Collect dependencies
    let mut needed_entities: Vec<String> = Vec::new();
    let mut seen: HashSet<String> = HashSet::new();

    // Check derivation.source_entities for dependencies
    if let Some(ref derivation) = derived_entity.derivation {
        if let Some(ref source_entities) = derivation.source_entities {
            match source_entities {
                serde_yaml::Value::Mapping(map) => {
                    for (_key, value) in map {
                        if let serde_yaml::Value::String(entity_name) = value {
                            if entity_name.as_str() != root_entity.name.as_str() {
                                collect_entity_dependencies(
                                    entity_name.as_str(),
                                    all_entities,
                                    root_entity,
                                    &mut needed_entities,
                                    &mut seen
                                );
                            }
                        }
                    }
                },
                _ => {}
            }
        }
    }

    // Check field sources for dependencies
    for field in &derived_entity.fields {
        if let Some(ref computed_from) = field.computed_from {
            for source in &computed_from.sources {
                let source_entity = source.source_name();
                if source_entity != root_entity.name.as_str() {
                    collect_entity_dependencies(
                        source_entity,
                        all_entities,
                        root_entity,
                        &mut needed_entities,
                        &mut seen
                    );
                }
            }
        }
    }

    // Extract intermediate entities first (dependencies)
    for entity_name in &needed_entities {
        if let Some(intermediate_entity) = all_entities.iter().find(|e| &e.name == entity_name) {
            writeln!(output, "    // Instantiate {} entity", entity_name)?;
            for field in &intermediate_entity.fields {
                if let Some(ref computed_from) = field.computed_from {
                    let var_name = format!("{}_{}", crate::codegen::utils::to_snake_case(entity_name), field.name);
                    let field_type_str = field.field_type.as_str();
                    let is_nullable = field.nullable;
                    generate_field_extraction(output, &var_name, field_type_str, computed_from, root_entity, &root_param_name, is_nullable, repeating_context, "    ", Some(intermediate_entity), true)?;
                }
            }
            if !intermediate_entity.is_persistent(all_entities) {
                publish_transient_entity_to_nats(output, intermediate_entity, "    ")?;
            }
            writeln!(output)?;
        }
    }

    // Extract this entity's fields (keep variables in main scope for downstream entities)
    for field in &derived_entity.fields {
        if let Some(ref computed_from) = field.computed_from {
            let var_name = format!("{}_{}", entity_prefix, field.name);
            let field_type_str = field.field_type.as_str();
            let is_nullable = field.nullable;
            generate_field_extraction(output, &var_name, field_type_str, computed_from, root_entity, &root_param_name, is_nullable, repeating_context, "    ", Some(derived_entity), false)?;
        }
    }

    // Publish to NATS
    publish_transient_entity_to_nats(output, derived_entity, "    ")?;

    writeln!(output)?;

    Ok(())
}

/// Publish a transient entity to NATS (fields already extracted)
#[allow(dead_code)]
pub(super) fn publish_transient_entity_to_nats(
    output: &mut std::fs::File,
    entity: &EntityDef,
    indent: &str,
) -> Result<(), Box<dyn Error>> {
    let entity_name = &entity.name;
    let entity_prefix = crate::codegen::utils::to_snake_case(entity_name);

    writeln!(output, "{}// Publish {} to NATS", indent, entity_name)?;
    writeln!(output, "{}{{", indent)?;
    writeln!(output, "{}    let mut entity_json = serde_json::Map::new();", indent)?;

    // Add all fields to the JSON map
    for field in &entity.fields {
        let field_name = &field.name;
        let var_name = format!("{}_{}", entity_prefix, field_name);
        let field_type = &field.field_type;
        let is_nullable = field.nullable;

        // Handle different field types
        if is_list_type(field_type) {
            writeln!(output, "{}    if !{}.is_empty() {{", indent, var_name)?;
            writeln!(output, "{}        entity_json.insert(\"{}\".to_string(), serde_json::json!(&{}));", indent, field_name, var_name)?;
            writeln!(output, "{}    }}", indent)?;
        } else if is_nullable {
            writeln!(output, "{}    if let Some(ref val) = {} {{", indent, var_name)?;
            writeln!(output, "{}        entity_json.insert(\"{}\".to_string(), serde_json::json!(val));", indent, field_name)?;
            writeln!(output, "{}    }}", indent)?;
        } else {
            writeln!(output, "{}    entity_json.insert(\"{}\".to_string(), serde_json::json!(&{}));", indent, field_name, var_name)?;
        }
    }

    writeln!(output, "{}    // Only publish if entity has actual data", indent)?;
    writeln!(output, "{}    if !entity_json.is_empty() {{", indent)?;
    writeln!(output, "{}        let entity_json_str = serde_json::to_string(&entity_json)", indent)?;
    writeln!(output, "{}            .map_err(|e| AppError::ValidationError(format!(\"Failed to serialize {}: {{}}\", e)))?;", indent, entity_name)?;
    writeln!(output, "{}        let stream_subject = format!(\"entities.{}\");", indent, entity_name)?;
    writeln!(output, "{}        jetstream.publish(stream_subject.clone(), entity_json_str.into()).await", indent)?;
    writeln!(output, "{}            .map_err(|e| {{", indent)?;
    writeln!(output, "{}                eprintln!(\"[WORKER] Failed to publish {} to {{}}: {{:?}}\", stream_subject, e);", indent, entity_name)?;
    writeln!(output, "{}                AppError::ValidationError(format!(\"NATS publish failed: {{}}\", e))", indent)?;
    writeln!(output, "{}            }})?;", indent)?;
    writeln!(output, "{}        eprintln!(\"[WORKER] ✓ Published {} to {{}}\", stream_subject);", indent, entity_name)?;
    writeln!(output, "{}    }} else {{", indent)?;
    writeln!(output, "{}        eprintln!(\"[WORKER] Skipped {} (no data extracted from segments)\");", indent, entity_name)?;
    writeln!(output, "{}    }}", indent)?;
    writeln!(output, "{}}}", indent)?;

    Ok(())
}

/// Check if a field type is a list/vector type
#[allow(dead_code)]
pub(super) fn is_list_type(field_type: &str) -> bool {
    field_type.starts_with("list[") || field_type.starts_with("List[") || field_type.starts_with("Vec<")
}

/// Determine the main data field for a root entity
/// For Hl7v2MessageFile, this is "hl7v2Message"
/// For Filename, this is "fileName"
#[allow(dead_code)]
pub(super) fn determine_root_data_field(entity: &EntityDef) -> &str {
    // Strategy: Prioritize fields with common data payload names, then fall back to first non-nullable string
    let priority_names = ["message", "hl7v2Message", "body", "data", "content", "text"];

    // First, try to find a field with a priority name (case-insensitive)
    for priority_name in &priority_names {
        if let Some(field) = entity.fields.iter().find(|f| {
            f.name.to_lowercase() == priority_name.to_lowercase()
        }) {
            return field.name.as_str();
        }
    }

    // Fall back to the first non-nullable string field
    entity.fields.iter()
        .find(|f| {
            let is_string = f.field_type.as_str() == "string" || f.field_type.as_str() == "String";
            let is_required = !f.nullable;
            is_string && is_required
        })
        .map(|f| f.name.as_str())
        .unwrap_or("body") // final fallback
}

/// Helper function to build variable name for a field source
fn build_source_variable_name(
    source: &crate::codegen::types::FieldSource,
    repeating_parent_info: Option<(&str, &str, &str)>,
    current_entity: Option<&EntityDef>,
    is_intermediate: bool, // true if generating for a dependency entity, false if main entity
) -> String {
    use crate::codegen::utils::to_snake_case;

    let src_entity = source.source_name();
    let src_field = source.field_name();

    if let Some(field) = src_field {
        // Handle "self" as source entity (references current entity's fields)
        if src_entity == "self" {
            if is_intermediate {
                // Dependency entity: use entity-prefixed name
                if let Some(current_ent) = current_entity {
                    format!("{}_{}", to_snake_case(&current_ent.name), field)
                } else {
                    field.to_string()
                }
            } else {
                // Main entity processor: use bare field name
                field.to_string()
            }
        } else {
            format!("{}_{}", to_snake_case(src_entity), field)
        }
    } else {
        // Direct source - check context
        if let Some((_parent_name, segment_var, each_known_as)) = repeating_parent_info {
            if src_entity == each_known_as {
                // Use the actual variable name (segment_var) not the logical name (each_known_as)
                segment_var.to_string()
            } else if let Some(current_ent) = current_entity {
                if current_ent.fields.iter().any(|f| f.name == src_entity) {
                    let current_entity_prefix = to_snake_case(&current_ent.name);
                    format!("{}_{}", current_entity_prefix, src_entity)
                } else {
                    to_snake_case(src_entity)
                }
            } else {
                to_snake_case(src_entity)
            }
        } else if let Some(current_ent) = current_entity {
            if current_ent.fields.iter().any(|f| f.name == src_entity) {
                let current_entity_prefix = to_snake_case(&current_ent.name);
                format!("{}_{}", current_entity_prefix, src_entity)
            } else {
                to_snake_case(src_entity)
            }
        } else {
            to_snake_case(src_entity)
        }
    }
}

/// Generate field extraction code based on computed_from configuration
#[allow(dead_code)]
pub(super) fn generate_field_extraction(
    output: &mut std::fs::File,
    field_name: &str,
    field_type: &str,
    computed_from: &crate::codegen::types::ComputedFrom,
    root_entity: &EntityDef,
    root_param_name: &str,
    is_nullable: bool,
    repeating_parent_info: Option<(&str, &str, &str)>, // (parent_name, segment_var, each_known_as)
    base_indent: &str,
    current_entity: Option<&EntityDef>, // Entity being generated (for resolving self-references)
    is_intermediate: bool, // true if generating for a dependency entity, false if main entity
) -> Result<(), Box<dyn Error>> {
    let transform = &computed_from.transform;
    let sources = &computed_from.sources;

    // Convert transform function name to snake_case for Rust function calls
    let transform_fn = to_snake_case(transform);

    // For simple cases, generate direct extraction from root message fields
    // This handles: copy_field, extract_filename_component, etc.

    // Check if sources is empty
    if sources.is_empty() {
        // Check for constant_value transform
        if transform == "constant_value" {
            if let Some(ref args) = computed_from.args {
                if let Some(value) = args.get("value").and_then(|v| v.as_str()) {
                    writeln!(output, "{}let {}: Option<String> = Some(\"{}\".to_string());",
                        base_indent, field_name, value)?;
                    return Ok(());
                }
            }
        }

        // No sources defined and not a constant_value - generate placeholder
        if is_list_type(field_type) {
            writeln!(output, "{}let {}: Vec<String> = vec![]; // TODO: No sources defined for transform '{}'",
                base_indent,
                field_name,
                transform)?;
        } else {
            writeln!(output, "{}let {}: Option<String> = {}; // TODO: No sources defined for transform '{}'",
                base_indent,
                field_name,
                if is_nullable { "None" } else { "Some(String::new())" },
                transform)?;
        }
        return Ok(());
    }

    let source = &sources[0];
    let source_field = source.field_name();

    if transform == "copy_field" && sources.len() == 1 && source_field.is_some() {
        // Direct copy from source field (only when field is specified)
        let source_entity = source.source_name();
        let src_field = source_field.unwrap();

        if source_entity == root_entity.name.as_str() {
            // Direct access from root message
            // Always wrap in Some() for intermediate entities since transforms expect Option types
            writeln!(output, "{}let {}: Option<String> = Some({}.{}.clone());",
                base_indent,
                field_name,
                root_param_name,
                src_field)?;
        } else {
            // Access from intermediate entity variable
            let intermediate_var = format!("{}_{}", crate::codegen::utils::to_snake_case(source_entity), src_field);
            writeln!(output, "{}let {}: Option<String> = {}.clone();",
                base_indent,
                field_name,
                intermediate_var)?;
        }
    } else if sources.len() == 1 {
        // Transform function with arguments
        let source = &sources[0];
        let source_entity = source.source_name();
        let source_field = source.field_name();

        // Generate transform function call arguments
        let args_list = if let Some(ref args) = computed_from.args {
            format_transform_args_list(args)
        } else {
            vec![]
        };

        if let Some(src_field) = source_field {
            if source_entity == root_entity.name.as_str() {
                // Call transform with root message field
                let all_args = if args_list.is_empty() {
                    format!("&{}.{}", root_param_name, src_field)
                } else {
                    format!("&{}.{}, {}", root_param_name, src_field, args_list.join(", "))
                };
                // Handle Result<Option<T>> -> Option<T> for Option types
                // Handle Result<Vec<T>, E> -> Vec<T> for List types
                if is_nullable {
                    writeln!(output, "{}let {} = {}({}).unwrap_or(None);",
                        base_indent,
                        field_name,
                        transform_fn,
                        all_args)?;
                } else {
                    // Non-nullable - check if it's a list or scalar type
                    if is_list_type(field_type) {
                        writeln!(output, "{}let {} = {}({}).unwrap_or_else(|_| vec![]);",
                            base_indent,
                            field_name,
                            transform_fn,
                            all_args)?;
                    } else {
                        writeln!(output, "{}let {} = {}({}).unwrap_or_else(|_| String::new());",
                            base_indent,
                            field_name,
                            transform_fn,
                            all_args)?;
                    }
                }
            } else {
                // Call transform with intermediate entity variable
                let intermediate_var = format!("{}_{}", crate::codegen::utils::to_snake_case(source_entity), src_field);

                // Pass &Option<String> to transform function
                let all_args = if args_list.is_empty() {
                    format!("&{}", intermediate_var)
                } else {
                    format!("&{}, {}", intermediate_var, args_list.join(", "))
                };

                writeln!(output, "{}let {} = if {}.is_some() {{ {}({}).unwrap_or(None) }} else {{ None }};",
                    base_indent,
                    field_name,
                    intermediate_var,
                    transform_fn,
                    all_args)?;
            }
        } else {
            // No source field - this is a Direct source (e.g., FieldSource::Direct("segment"))
            // Check if this references the loop variable for a repeating entity
            if let Some((_repeating_parent, segment_var, each_known_as)) = repeating_parent_info {
                if source_entity == each_known_as {
                    // Generate transform call with segment as first argument
                    let args_list = if let Some(ref args) = computed_from.args {
                        format_transform_args_list(args)
                    } else {
                        vec![]
                    };

                    // Check if segment_var is "segment_opt" (from repeated entity processor)
                    // or a regular loop variable
                    let all_args = if segment_var == "segment_opt" {
                        // segment_opt is already Option<String>, pass as &segment_opt
                        if args_list.is_empty() {
                            format!("&{}", segment_var)
                        } else {
                            format!("&{}, {}", segment_var, args_list.join(", "))
                        }
                    } else {
                        // Regular loop variable is a String, convert to &str
                        if args_list.is_empty() {
                            format!("{}.as_str()", segment_var)
                        } else {
                            format!("{}.as_str(), {}", segment_var, args_list.join(", "))
                        }
                    };

                    writeln!(output, "{}let {} = {}({}).unwrap_or(None);",
                        base_indent,
                        field_name,
                        transform_fn,
                        all_args)?;
                    return Ok(());
                }
            }

            // No source field - this is a Direct source (e.g., transform references entire entity)
            // For root entity direct sources, we need to determine which field to pass
            if source_entity == root_entity.name.as_str() {
                let root_data_field = determine_root_data_field(root_entity);

                let args_list = if let Some(ref args) = computed_from.args {
                    format_transform_args_list(args)
                } else {
                    vec![]
                };

                let all_args = if args_list.is_empty() {
                    format!("&{}.{}", root_param_name, root_data_field)
                } else {
                    format!("&{}.{}, {}", root_param_name, root_data_field, args_list.join(", "))
                };

                // Generate appropriate type based on field type
                if is_list_type(field_type) {
                    writeln!(output, "{}let {} = {}({}).unwrap_or_else(|_| vec![]);",
                        base_indent, field_name, transform_fn, all_args)?;
                } else if is_nullable {
                    writeln!(output, "{}let {} = {}({}).unwrap_or(None);",
                        base_indent, field_name, transform_fn, all_args)?;
                } else {
                    writeln!(output, "{}let {} = {}({}).unwrap_or_else(|_| String::new());",
                        base_indent, field_name, transform_fn, all_args)?;
                }
            } else {
                // Source is an intermediate entity (segment or derived entity)
                // Build variable name for the source
                let source_var = if let Some(src_field) = source_field {
                    // Source has a field: entity_field
                    let source_var_prefix = to_snake_case(source_entity);
                    format!("{}_{}", source_var_prefix, src_field)
                } else {
                    // Direct source (no field): check various contexts
                    // First check if it matches the loop variable from repeating context
                    if let Some((_parent_name, segment_var, each_known_as)) = repeating_parent_info {
                        if source_entity == each_known_as {
                            // This is the loop variable from repeated_for
                            // Use the actual variable name (segment_var) not the logical name (each_known_as)
                            segment_var.to_string()
                        } else if let Some(current_ent) = current_entity {
                            // Check if source_entity matches a field name in the current entity
                            if current_ent.fields.iter().any(|f| f.name == source_entity) {
                                // This is a reference to a field in the same entity
                                let current_entity_prefix = to_snake_case(&current_ent.name);
                                format!("{}_{}", current_entity_prefix, source_entity)
                            } else {
                                // Not a field reference, just use the entity name
                                to_snake_case(source_entity)
                            }
                        } else {
                            // No current entity context, just use the entity name
                            to_snake_case(source_entity)
                        }
                    } else if let Some(current_ent) = current_entity {
                        // Not in repeating context, check if it's a field self-reference
                        if current_ent.fields.iter().any(|f| f.name == source_entity) {
                            // This is a reference to a field in the same entity
                            let current_entity_prefix = to_snake_case(&current_ent.name);
                            format!("{}_{}", current_entity_prefix, source_entity)
                        } else {
                            // Not a field reference, just use the entity name
                            to_snake_case(source_entity)
                        }
                    } else {
                        // No current entity context, just use the entity name
                        to_snake_case(source_entity)
                    }
                };

                // Get transform arguments
                let args_list = if let Some(ref args) = computed_from.args {
                    format_transform_args_list(args)
                } else {
                    vec![]
                };

                // Generate extraction based on transform type
                match transform.as_str() {
                    "extract_from_hl7_segment" => {
                        // Segment extraction with field path
                        // Combine field_path and component_path into a single path string
                        let segment_path = if let Some(ref args) = computed_from.args {
                            let field = args.get("field_path")
                                .and_then(|v| v.as_str())
                                .unwrap_or("");
                            let component = args.get("component_path")
                                .and_then(|v| v.as_str());

                            if let Some(comp) = component {
                                if !field.is_empty() {
                                    format!("{}.{}", field, comp)
                                } else {
                                    comp.to_string()
                                }
                            } else {
                                field.to_string()
                            }
                        } else {
                            String::new()
                        };

                        writeln!(output, "{}let {}: Option<String> = extract_from_hl7_segment(",
                            base_indent, field_name)?;
                        writeln!(output, "{}    &{},", base_indent, source_var)?;
                        writeln!(output, "{}    \"{}\"", base_indent, segment_path)?;
                        writeln!(output, "{}).unwrap_or(None);", base_indent)?;
                    },
                    "copy_field" | "copy_field_direct" => {
                        // Direct field copy
                        writeln!(output, "{}let {}: Option<String> = {}.clone();",
                            base_indent, field_name, source_var)?;
                    },
                    _ => {
                        // Other transform function call
                        let all_args = if args_list.is_empty() {
                            format!("&{}", source_var)
                        } else {
                            format!("&{}, {}", source_var, args_list.join(", "))
                        };

                        if is_list_type(field_type) {
                            writeln!(output, "{}let {} = {}({}).unwrap_or_else(|_| vec![]);",
                                base_indent, field_name, transform_fn, all_args)?;
                        } else if is_nullable {
                            writeln!(output, "{}let {} = {}({}).unwrap_or(None);",
                                base_indent, field_name, transform_fn, all_args)?;
                        } else {
                            writeln!(output, "{}let {} = {}({}).unwrap_or_else(|_| String::new());",
                                base_indent, field_name, transform_fn, all_args)?;
                        }
                    }
                }
            }
        }
    } else {
        // Multiple sources - handle conditional selection or fallback
        if transform == "copy_field_conditional" {
            // Conditional selection based on a field value
            if let Some(ref condition) = computed_from.condition {
                let cond_source = condition.field.source_name();
                let cond_field = condition.field.field_name().unwrap_or("value");
                let cond_equals = &condition.equals;

                let cond_var = format!("{}_{}", to_snake_case(cond_source), cond_field);

                writeln!(output, "{}let {} = if {}.as_ref().map(|s| s == \"{}\").unwrap_or(false) {{",
                    base_indent, field_name, cond_var, cond_equals)?;

                // True branch: first source
                let src0_var = build_source_variable_name(&sources[0], repeating_parent_info, current_entity, is_intermediate);
                writeln!(output, "{}    {}.clone()", base_indent, src0_var)?;

                writeln!(output, "{}}} else {{", base_indent)?;

                // False branch: second source
                let src1_var = build_source_variable_name(&sources[1], repeating_parent_info, current_entity, is_intermediate);
                writeln!(output, "{}    {}.clone()", base_indent, src1_var)?;

                writeln!(output, "{}}};", base_indent)?;
            } else {
                writeln!(output, "{}let {}: Option<String> = None; // ERROR: conditional without condition",
                    base_indent, field_name)?;
            }
        } else {
            // Fallback: use first non-None source
            write!(output, "{}let {} = ", base_indent, field_name)?;
            for (i, source) in sources.iter().enumerate() {
                let src_var = build_source_variable_name(source, repeating_parent_info, current_entity, is_intermediate);

                if i == 0 {
                    write!(output, "{}.clone()", src_var)?;
                } else {
                    write!(output, ".or_else(|| {}.clone())", src_var)?;
                }
            }
            writeln!(output, ";")?;
        }
    }

    Ok(())
}

/// Format transform function arguments from YAML value as a list
#[allow(dead_code)]
pub(super) fn format_transform_args_list(args: &serde_yaml::Value) -> Vec<String> {
    match args {
        serde_yaml::Value::Sequence(seq) => {
            seq.iter()
                .map(|v| {
                    match v {
                        serde_yaml::Value::String(s) => format!("\"{}\"", s),
                        serde_yaml::Value::Number(n) => n.to_string(),
                        serde_yaml::Value::Bool(b) => b.to_string(),
                        _ => "/* unsupported */".to_string(),
                    }
                })
                .collect()
        }
        serde_yaml::Value::Mapping(map) => {
            map.iter()
                .map(|(_, v)| {
                    match v {
                        serde_yaml::Value::String(s) => format!("\"{}\"", s),
                        serde_yaml::Value::Number(n) => n.to_string(),
                        serde_yaml::Value::Bool(b) => b.to_string(),
                        _ => "/* unsupported */".to_string(),
                    }
                })
                .collect()
        }
        _ => vec![],
    }
}

/// Format transform function arguments from YAML value (deprecated - use format_transform_args_list)
#[allow(dead_code)]
pub(super) fn format_transform_args(args: &serde_yaml::Value) -> String {
    match args {
        serde_yaml::Value::Mapping(map) => {
            map.iter()
                .map(|(k, v)| {
                    let key = k.as_str().unwrap_or("");
                    let val = match v {
                        serde_yaml::Value::String(s) => format!("\"{}\"", s),
                        serde_yaml::Value::Number(n) => n.to_string(),
                        serde_yaml::Value::Bool(b) => b.to_string(),
                        _ => "/* unsupported */".to_string(),
                    };
                    format!("{}: {}", key, val)
                })
                .collect::<Vec<_>>()
                .join(", ")
        }
        _ => String::new(),
    }
}

/// Map field types to Diesel SQL types
fn map_to_diesel_type(field_type: &str) -> &'static str {
    match field_type {
        "String" => "Text",
        "i32" | "Integer" => "Integer",
        "i64" => "BigInt",
        "f64" | "Float" => "Double",
        "bool" => "Bool",
        "NaiveDate" => "Date",
        "Decimal" => "Numeric",
        _ => "Text", // Default to Text for unknown types
    }
}
