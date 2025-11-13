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
    writeln!(output, "mod error;")?;
    writeln!(output, "mod transforms;\n")?;

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
    writeln!(output, "            subjects: vec![\"messages.ingest.>\".to_string()],")?;
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
    writeln!(output, "    // Use entity_type hint from envelope if available")?;
    writeln!(output, "    let (entity_name, parsed, raw_json) = MessageParser::parse_json(&envelope.body, envelope.entity_type.as_deref())?;\n")?;

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

        // Check if this is Order entity (hardcoded for now)
        let is_order = entity.name == "Order";

        if is_order {
            writeln!(output, "        ParsedMessage::{}(ref msg) => {{", entity.name)?;
        } else {
            writeln!(output, "        ParsedMessage::{}(msg) => {{", entity.name)?;
        }

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

        // If this is Order, process derived entities (OrderLineItems)
        if is_order {
            writeln!(output, "            // Process derived entities (OrderLineItems)")?;
            writeln!(output, "            process_order_derived_entities(msg, &raw_json, &mut conn)?;")?;
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

    // Generate derived entity processors
    generate_derived_entity_processors(&mut output, entities)?;

    Ok(())
}

/// Generate derived entity processor functions
fn generate_derived_entity_processors(
    output: &mut std::fs::File,
    entities: &[EntityDef],
) -> Result<(), Box<dyn Error>> {
    // Check if Order entity exists (hardcoded for now)
    let has_order = entities.iter().any(|e| e.name == "Order" && e.is_root() && e.is_persistent());

    if !has_order {
        return Ok(());
    }

    writeln!(output)?;
    writeln!(output, "/// Process derived entities for Order (OrderLineItems)")?;
    writeln!(output, "fn process_order_derived_entities(")?;
    writeln!(output, "    order: &parsers::OrderMessage,")?;
    writeln!(output, "    raw_json: &serde_json::Value,")?;
    writeln!(output, "    conn: &mut diesel::PgConnection,")?;
    writeln!(output, ") -> Result<(), AppError> {{")?;
    writeln!(output, "    use transforms::*;")?;
    writeln!(output)?;
    writeln!(output, "    // Get line_items array from raw JSON")?;
    writeln!(output, "    let line_items = match raw_json.get(\"line_items\").and_then(|v| v.as_array()) {{")?;
    writeln!(output, "        Some(items) => items,")?;
    writeln!(output, "        None => {{")?;
    writeln!(output, "            tracing::debug!(\"No line_items array found in Order message\");")?;
    writeln!(output, "            return Ok(()); // Not an error if no line items")?;
    writeln!(output, "        }}")?;
    writeln!(output, "    }};")?;
    writeln!(output)?;
    writeln!(output, "    tracing::debug!(\"Processing {{}} line items for Order {{}}\", line_items.len(), order.order_key);")?;
    writeln!(output)?;
    writeln!(output, "    // Process each line item")?;
    writeln!(output, "    for (index, item) in line_items.iter().enumerate() {{")?;
    writeln!(output, "        // Extract fields using transform functions")?;
    writeln!(output, "        let order_key = order.order_key.clone();")?;
    writeln!(output)?;
    writeln!(output, "        // Extract required fields with error handling")?;
    writeln!(output, "        let line_number = match json_get_int(item, \"line_number\") {{")?;
    writeln!(output, "            Ok(v) => v,")?;
    writeln!(output, "            Err(e) => {{")?;
    writeln!(output, "                tracing::warn!(\"Skipping line item at index {{}}: missing/invalid line_number: {{:?}}\", index, e);")?;
    writeln!(output, "                continue;")?;
    writeln!(output, "            }}")?;
    writeln!(output, "        }};")?;
    writeln!(output)?;
    writeln!(output, "        let part_key = match json_get_string(item, \"part_key\") {{")?;
    writeln!(output, "            Ok(v) => v,")?;
    writeln!(output, "            Err(e) => {{")?;
    writeln!(output, "                tracing::warn!(\"Skipping line item at index {{}}: missing/invalid part_key: {{:?}}\", index, e);")?;
    writeln!(output, "                continue;")?;
    writeln!(output, "            }}")?;
    writeln!(output, "        }};")?;
    writeln!(output)?;
    writeln!(output, "        let quantity = match json_get_int(item, \"quantity\") {{")?;
    writeln!(output, "            Ok(v) => v,")?;
    writeln!(output, "            Err(e) => {{")?;
    writeln!(output, "                tracing::warn!(\"Skipping line item at index {{}}: missing/invalid quantity: {{:?}}\", index, e);")?;
    writeln!(output, "                continue;")?;
    writeln!(output, "            }}")?;
    writeln!(output, "        }};")?;
    writeln!(output)?;
    writeln!(output, "        let extended_price = match json_get_float(item, \"extended_price\") {{")?;
    writeln!(output, "            Ok(v) => v,")?;
    writeln!(output, "            Err(e) => {{")?;
    writeln!(output, "                tracing::warn!(\"Skipping line item at index {{}}: missing/invalid extended_price: {{:?}}\", index, e);")?;
    writeln!(output, "                continue;")?;
    writeln!(output, "            }}")?;
    writeln!(output, "        }};")?;
    writeln!(output)?;
    writeln!(output, "        // Extract optional fields")?;
    writeln!(output, "        let supplier_key = json_get_optional_string(item, \"supplier_key\");")?;
    writeln!(output, "        let discount = json_get_optional_float(item, \"discount\");")?;
    writeln!(output, "        let tax = json_get_optional_float(item, \"tax\");")?;
    writeln!(output, "        let return_flag = json_get_optional_string(item, \"return_flag\");")?;
    writeln!(output, "        let line_status = json_get_optional_string(item, \"line_status\");")?;
    writeln!(output, "        let ship_date = json_get_optional_string(item, \"ship_date\");")?;
    writeln!(output, "        let commit_date = json_get_optional_string(item, \"commit_date\");")?;
    writeln!(output, "        let receipt_date = json_get_optional_string(item, \"receipt_date\");")?;
    writeln!(output)?;
    writeln!(output, "        // Insert OrderLineItem")?;
    writeln!(output, "        diesel::sql_query(")?;
    writeln!(output, "            r#\"INSERT INTO order_line_items")?;
    writeln!(output, "               (order_key, line_number, part_key, supplier_key, quantity, extended_price,")?;
    writeln!(output, "                discount, tax, return_flag, line_status, ship_date, commit_date, receipt_date)")?;
    writeln!(output, "               VALUES ($1, $2, $3, $4, $5, $6, $7, $8, $9, $10, $11, $12, $13)")?;
    writeln!(output, "               ON CONFLICT (order_key, line_number) DO NOTHING\"#")?;
    writeln!(output, "        )")?;
    writeln!(output, "        .bind::<Text, _>(&order_key)")?;
    writeln!(output, "        .bind::<Integer, _>(&line_number)")?;
    writeln!(output, "        .bind::<Text, _>(&part_key)")?;
    writeln!(output, "        .bind::<Nullable<Text>, _>(&supplier_key)")?;
    writeln!(output, "        .bind::<Integer, _>(&quantity)")?;
    writeln!(output, "        .bind::<Double, _>(&extended_price)")?;
    writeln!(output, "        .bind::<Nullable<Double>, _>(&discount)")?;
    writeln!(output, "        .bind::<Nullable<Double>, _>(&tax)")?;
    writeln!(output, "        .bind::<Nullable<Text>, _>(&return_flag)")?;
    writeln!(output, "        .bind::<Nullable<Text>, _>(&line_status)")?;
    writeln!(output, "        .bind::<Nullable<Text>, _>(&ship_date)")?;
    writeln!(output, "        .bind::<Nullable<Text>, _>(&commit_date)")?;
    writeln!(output, "        .bind::<Nullable<Text>, _>(&receipt_date)")?;
    writeln!(output, "        .execute(conn)?;")?;
    writeln!(output, "    }}")?;
    writeln!(output)?;
    writeln!(output, "    tracing::info!(\"Inserted {{}} OrderLineItems for Order {{}}\", line_items.len(), order.order_key);")?;
    writeln!(output, "    Ok(())")?;
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
