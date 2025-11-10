/// Generate main.rs with NATS consumer loop

use crate::codegen::EntityDef;
use super::WorkerConfig;
use std::path::Path;
use std::error::Error;
use std::io::Write;

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
    writeln!(output, "mod error;\n")?;

    writeln!(output, "use database::{{create_pool, ensure_tables}};")?;
    writeln!(output, "use parsers::{{MessageParser, ParsedMessage}};")?;
    writeln!(output, "use error::AppError;\n")?;

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
    writeln!(output, "    tracing_subscriber::fmt::init();\n")?;

    writeln!(output, "    // Load environment variables")?;
    writeln!(output, "    dotenv::dotenv().ok();\n")?;

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
    writeln!(output, "    let db_pool = create_pool()")?;
    writeln!(output, "        .expect(\"Failed to create database pool\");\n")?;

    writeln!(output, "    // Ensure tables exist")?;
    writeln!(output, "    {{")?;
    writeln!(output, "        let mut conn = db_pool.get()")?;
    writeln!(output, "            .expect(\"Failed to get database connection\");")?;
    writeln!(output, "        ensure_tables(&mut conn)")?;
    writeln!(output, "            .expect(\"Failed to ensure tables exist\");")?;
    writeln!(output, "    }}\n")?;

    writeln!(output, "    // Connect to NATS")?;
    writeln!(output, "    let client = async_nats::connect(&nats_url).await")?;
    writeln!(output, "        .expect(\"Failed to connect to NATS\");")?;
    writeln!(output, "    tracing::info!(\"Connected to NATS at {{}}\", nats_url);\n")?;

    writeln!(output, "    // Get JetStream context")?;
    writeln!(output, "    let jetstream = jetstream::new(client);\n")?;

    writeln!(output, "    // Get or create stream")?;
    writeln!(output, "    let stream = jetstream")?;
    writeln!(output, "        .get_or_create_stream(jetstream::stream::Config {{")?;
    writeln!(output, "            name: stream_name.clone(),")?;
    writeln!(output, "            subjects: vec![\"messages.>\".to_string()],")?;
    writeln!(output, "            max_age: Duration::from_secs(24 * 60 * 60),")?;
    writeln!(output, "            max_bytes: 1024 * 1024 * 1024,")?;
    writeln!(output, "            storage: jetstream::stream::StorageType::File,")?;
    writeln!(output, "            num_replicas: 1,")?;
    writeln!(output, "            ..Default::default()")?;
    writeln!(output, "        }})")?;
    writeln!(output, "        .await")?;
    writeln!(output, "        .expect(\"Failed to get/create stream\");\n")?;

    writeln!(output, "    // Create or get consumer")?;
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
    writeln!(output, "        .expect(\"Failed to create consumer\");\n")?;

    writeln!(output, "    tracing::info!(")?;
    writeln!(output, "        \"Worker ready - consuming from stream '{{}}' with consumer '{{}}'\",")?;
    writeln!(output, "        stream_name,")?;
    writeln!(output, "        consumer_name")?;
    writeln!(output, "    );\n")?;

    // Count persistent entities for logging
    let entity_count = entities.iter()
        .filter(|e| e.is_persistent() && !e.is_abstract && e.source_type.to_lowercase() != "reference")
        .count();
    writeln!(output, "    tracing::info!(\"Processing messages for {} entities\");", entity_count)?;
    writeln!(output)?;

    writeln!(output, "    // Main message processing loop")?;
    writeln!(output, "    loop {{")?;
    writeln!(output, "        // Fetch batch of messages")?;
    writeln!(output, "        let mut messages = consumer")?;
    writeln!(output, "            .fetch()")?;
    writeln!(output, "            .max_messages(batch_size)")?;
    writeln!(output, "            .messages()")?;
    writeln!(output, "            .await")?;
    writeln!(output, "            .expect(\"Failed to fetch messages\");\n")?;

    writeln!(output, "        while let Some(msg) = messages.next().await {{")?;
    writeln!(output, "            let msg = match msg {{")?;
    writeln!(output, "                Ok(m) => m,")?;
    writeln!(output, "                Err(e) => {{")?;
    writeln!(output, "                    tracing::error!(\"Error receiving message: {{}}\", e);")?;
    writeln!(output, "                    continue;")?;
    writeln!(output, "                }}")?;
    writeln!(output, "            }};\n")?;

    writeln!(output, "            // Process message")?;
    writeln!(output, "            match process_message(&msg.payload, &db_pool) {{")?;
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
    writeln!(output, "                                            r#\"UPDATE message_status")?;
    writeln!(output, "                                               SET status = $1, error_message = $2")?;
    writeln!(output, "                                               WHERE message_id = $3\"#")?;
    writeln!(output, "                                        )")?;
    writeln!(output, "                                        .bind::<Text, _>(\"dlq\")")?;
    writeln!(output, "                                        .bind::<Text, _>(&format!(\"Failed after {{}} attempts: {{:?}}\", delivery_count, e))")?;
    writeln!(output, "                                        .bind::<diesel::sql_types::Uuid, _>(&uuid)")?;
    writeln!(output, "                                        .execute(&mut conn)")?;
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
    writeln!(output, "                                            r#\"UPDATE message_status")?;
    writeln!(output, "                                               SET status = $1, error_message = $2, retry_count = retry_count + 1")?;
    writeln!(output, "                                               WHERE message_id = $3\"#")?;
    writeln!(output, "                                        )")?;
    writeln!(output, "                                        .bind::<Text, _>(\"failed\")")?;
    writeln!(output, "                                        .bind::<Text, _>(&format!(\"{{:?}}\", e))")?;
    writeln!(output, "                                        .bind::<diesel::sql_types::Uuid, _>(&uuid)")?;
    writeln!(output, "                                        .execute(&mut conn)")?;
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
    writeln!(output, "        // Small delay between batches")?;
    writeln!(output, "        tokio::time::sleep(Duration::from_millis(poll_interval_ms)).await;")?;
    writeln!(output, "    }}")?;
    writeln!(output, "}}\n")?;

    writeln!(output, "/// Process a single message")?;
    writeln!(output, "fn process_message(")?;
    writeln!(output, "    payload: &[u8],")?;
    writeln!(output, "    pool: &database::DbPool,")?;
    writeln!(output, ") -> Result<(), AppError> {{")?;
    writeln!(output, "    // Deserialize envelope")?;
    writeln!(output, "    let envelope: MessageEnvelope = serde_json::from_slice(payload)")?;
    writeln!(output, "        .map_err(|e| AppError::ValidationError(format!(\"Invalid envelope: {{}}\", e)))?;\n")?;

    writeln!(output, "    let message_id = envelope.message_id;")?;
    writeln!(output, "    tracing::debug!(\"Processing message {{}}\", message_id);\n")?;

    writeln!(output, "    // Get database connection")?;
    writeln!(output, "    let mut conn = pool.get()?;\n")?;

    writeln!(output, "    // Update status to 'processing'")?;
    writeln!(output, "    diesel::sql_query(")?;
    writeln!(output, "        r#\"UPDATE message_status SET status = $1 WHERE message_id = $2\"#")?;
    writeln!(output, "    )")?;
    writeln!(output, "    .bind::<Text, _>(\"processing\")")?;
    writeln!(output, "    .bind::<diesel::sql_types::Uuid, _>(&message_id)")?;
    writeln!(output, "    .execute(&mut conn)")?;
    writeln!(output, "    .ok(); // Ignore errors - status tracking is optional\n")?;

    writeln!(output, "    // Parse message body using entity-specific parsers")?;
    writeln!(output, "    let (entity_name, parsed) = MessageParser::parse_json(&envelope.body)?;\n")?;

    writeln!(output, "    // Insert into database based on entity type")?;
    writeln!(output, "    match parsed {{")?;

    for entity in entities {
        // Only include root entities that are persistent
        if !entity.is_root() || !entity.is_persistent() || entity.is_abstract {
            continue;
        }
        if entity.source_type.to_lowercase() == "reference" {
            continue;
        }

        let db_config = entity.get_database_config().unwrap();
        let table_name = &db_config.conformant_table;

        writeln!(output, "        ParsedMessage::{}(msg) => {{", entity.name)?;

        // Get fields from persistence config
        if let Some(ref persistence) = entity.persistence {
            let fields = &persistence.field_overrides;

            // Build column names list
            let col_names: Vec<String> = fields.iter()
                .map(|f| f.name.to_lowercase())
                .collect();

            // Build placeholder list ($1, $2, ...)
            let placeholders: Vec<String> = (1..=col_names.len())
                .map(|i| format!("${}", i))
                .collect();

            writeln!(output, "            diesel::sql_query(")?;
            writeln!(output, "                r#\"INSERT INTO {} ({}) VALUES ({}) ON CONFLICT DO NOTHING\"#",
                table_name,
                col_names.join(", "),
                placeholders.join(", "))?;
            writeln!(output, "            )")?;

            // Bind each field
            for field in fields {
                let field_type_str = field.field_type.as_deref().unwrap_or("String");
                let diesel_type = map_to_diesel_type(field_type_str);
                let is_nullable = field.nullable.unwrap_or(false);

                if is_nullable {
                    writeln!(output, "            .bind::<Nullable<{}>, _>(&msg.{})", diesel_type, field.name)?;
                } else {
                    writeln!(output, "            .bind::<{}, _>(&msg.{})", diesel_type, field.name)?;
                }
            }

            writeln!(output, "            .execute(&mut conn)?;")?;
            writeln!(output)?;
        }

        writeln!(output, "            tracing::info!(\"Inserted {{}} message\", entity_name);\n")?;

        writeln!(output, "            // Update status to 'completed'")?;
        writeln!(output, "            diesel::sql_query(")?;
        writeln!(output, "                r#\"UPDATE message_status")?;
        writeln!(output, "                   SET status = $1, processed_at = NOW()")?;
        writeln!(output, "                   WHERE message_id = $2\"#")?;
        writeln!(output, "            )")?;
        writeln!(output, "            .bind::<Text, _>(\"completed\")")?;
        writeln!(output, "            .bind::<diesel::sql_types::Uuid, _>(&message_id)")?;
        writeln!(output, "            .execute(&mut conn)")?;
        writeln!(output, "            .ok(); // Ignore errors - status tracking is optional\n")?;

        writeln!(output, "            Ok(())")?;
        writeln!(output, "        }}")?;
    }

    writeln!(output, "    }}")?;
    writeln!(output, "}}")?;

    Ok(())
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
