/// Axum backend generation for real-time dashboard.
///
/// This module generates a complete Axum-based dashboard backend that:
/// - Polls entity tables directly (no db_events needed)
/// - Broadcasts changes via WebSocket
/// - Provides REST API for stats and metadata
///
/// Unlike the FastAPI backend which polls db_events table, this
/// implementation directly queries each entity table for new records.

use super::utils::{DatabaseType, generate_entity_display_config, DashboardConfig};
use crate::codegen::EntityDef;
use std::path::Path;
use std::error::Error;
use std::io::Write;

/// Generate Axum backend code
pub fn generate_backend(
    entities: &[EntityDef],
    output_dir: &Path,
    _config_dir: &str,
    db_type: DatabaseType,
) -> Result<(), Box<dyn Error>> {
    // Create src directory
    let src_dir = output_dir.join("src");
    std::fs::create_dir_all(&src_dir)?;

    // Generate Cargo.toml
    generate_cargo_toml(output_dir, db_type)?;

    // Generate main.rs
    generate_main_rs(entities, &src_dir, db_type)?;

    // Generate config.rs (entity metadata)
    generate_config_rs(entities, &src_dir)?;

    // Generate polling.rs (table polling logic)
    generate_polling_rs(entities, &src_dir, db_type)?;

    // Generate websocket.rs (WebSocket handler)
    generate_websocket_rs(&src_dir)?;

    // Generate api.rs (REST endpoints)
    generate_api_rs(&src_dir)?;

    // Generate .env.example
    generate_env_example(output_dir, db_type)?;

    Ok(())
}

/// Generate Cargo.toml with all required dependencies
fn generate_cargo_toml(
    output_dir: &Path,
    db_type: DatabaseType,
) -> Result<(), Box<dyn Error>> {
    let cargo_file = output_dir.join("Cargo.toml");
    let mut output = std::fs::File::create(&cargo_file)?;

    writeln!(output, "# Auto-generated Cargo.toml for Axum dashboard backend")?;
    writeln!(output, "[package]")?;
    writeln!(output, "name = \"dashboard\"")?;
    writeln!(output, "version = \"0.1.0\"")?;
    writeln!(output, "edition = \"2021\"\n")?;

    writeln!(output, "[dependencies]")?;
    writeln!(output, "# Web framework")?;
    writeln!(output, "axum = {{ version = \"0.7\", features = [\"ws\"] }}")?;
    writeln!(output, "tokio = {{ version = \"1\", features = [\"full\"] }}")?;
    writeln!(output, "tower = \"0.4\"")?;
    writeln!(output, "tower-http = {{ version = \"0.5\", features = [\"cors\"] }}\n")?;

    writeln!(output, "# Database")?;
    match db_type {
        DatabaseType::PostgreSQL => {
            writeln!(output, "sqlx = {{ version = \"0.7\", features = [\"runtime-tokio-rustls\", \"postgres\", \"json\", \"chrono\"] }}")?;
        }
        DatabaseType::MySQL | DatabaseType::MariaDB => {
            writeln!(output, "sqlx = {{ version = \"0.7\", features = [\"runtime-tokio-rustls\", \"mysql\", \"json\", \"chrono\"] }}")?;
        }
    }

    writeln!(output, "\n# Serialization")?;
    writeln!(output, "serde = {{ version = \"1.0\", features = [\"derive\"] }}")?;
    writeln!(output, "serde_json = \"1.0\"\n")?;

    writeln!(output, "# Utilities")?;
    writeln!(output, "chrono = {{ version = \"0.4\", features = [\"serde\"] }}")?;
    writeln!(output, "tracing = \"0.1\"")?;
    writeln!(output, "tracing-subscriber = {{ version = \"0.3\", features = [\"env-filter\"] }}")?;
    writeln!(output, "dotenv = \"0.15\"")?;

    Ok(())
}

/// Generate main.rs with Axum server setup
fn generate_main_rs(
    entities: &[EntityDef],
    src_dir: &Path,
    db_type: DatabaseType,
) -> Result<(), Box<dyn Error>> {
    let main_file = src_dir.join("main.rs");
    let mut output = std::fs::File::create(&main_file)?;

    writeln!(output, "// Auto-generated Axum backend for real-time dashboard\n")?;

    // Imports
    writeln!(output, "mod config;")?;
    writeln!(output, "mod polling;")?;
    writeln!(output, "mod websocket;")?;
    writeln!(output, "mod api;\n")?;

    writeln!(output, "use axum::{{")?;
    writeln!(output, "    Router,")?;
    writeln!(output, "    routing::get,")?;
    writeln!(output, "}};")?;
    writeln!(output, "use sqlx::{{Pool, Postgres}};")?;
    writeln!(output, "use std::sync::Arc;")?;
    writeln!(output, "use tokio::sync::RwLock;")?;
    writeln!(output, "use std::collections::HashMap;")?;
    writeln!(output, "use tower_http::cors::CorsLayer;")?;
    writeln!(output, "use tracing_subscriber::{{layer::SubscriberExt, util::SubscriberInitExt}};\n")?;

    // AppState struct
    writeln!(output, "/// Global application state")?;
    writeln!(output, "#[derive(Clone)]")?;
    writeln!(output, "pub struct AppState {{")?;
    writeln!(output, "    /// Database connection pool")?;
    writeln!(output, "    pub pool: Pool<Postgres>,")?;
    writeln!(output, "    /// Last seen ID per entity table")?;
    writeln!(output, "    pub last_ids: Arc<RwLock<HashMap<String, i64>>>,")?;
    writeln!(output, "    /// Connected WebSocket clients")?;
    writeln!(output, "    pub clients: Arc<RwLock<Vec<Arc<tokio::sync::Mutex<axum::extract::ws::WebSocket>>>>>,")?;
    writeln!(output, "}}\n")?;

    // Main function
    writeln!(output, "#[tokio::main]")?;
    writeln!(output, "async fn main() -> Result<(), Box<dyn std::error::Error>> {{")?;
    writeln!(output, "    // Initialize tracing")?;
    writeln!(output, "    tracing_subscriber::registry()")?;
    writeln!(output, "        .with(tracing_subscriber::EnvFilter::try_from_default_env()")?;
    writeln!(output, "            .unwrap_or_else(|_| \"info\".into()))")?;
    writeln!(output, "        .with(tracing_subscriber::fmt::layer())")?;
    writeln!(output, "        .init();\n")?;

    writeln!(output, "    // Load environment variables")?;
    writeln!(output, "    dotenv::dotenv().ok();\n")?;

    writeln!(output, "    // Get database URL")?;
    writeln!(output, "    let database_url = std::env::var(\"DATABASE_URL\")")?;
    writeln!(output, "        .expect(\"DATABASE_URL must be set in .env file\");\n")?;

    writeln!(output, "    // Connect to database")?;
    writeln!(output, "    tracing::info!(\"Connecting to database...\");")?;
    writeln!(output, "    let pool = sqlx::postgres::PgPoolOptions::new()")?;
    writeln!(output, "        .max_connections(10)")?;
    writeln!(output, "        .connect(&database_url)")?;
    writeln!(output, "        .await?;\n")?;

    writeln!(output, "    tracing::info!(\"Database connected successfully\");\n")?;

    // Initialize state
    writeln!(output, "    // Initialize application state")?;
    writeln!(output, "    let state = AppState {{")?;
    writeln!(output, "        pool: pool.clone(),")?;
    writeln!(output, "        last_ids: Arc::new(RwLock::new(HashMap::new())),")?;
    writeln!(output, "        clients: Arc::new(RwLock::new(Vec::new())),")?;
    writeln!(output, "    }};\n")?;

    // Start polling tasks
    writeln!(output, "    // Start polling tasks for each entity")?;
    writeln!(output, "    polling::start_all_polling_tasks(state.clone()).await;\n")?;

    // Build router
    writeln!(output, "    // Build router")?;
    writeln!(output, "    let app = Router::new()")?;
    writeln!(output, "        .route(\"/ws\", get(websocket::websocket_handler))")?;
    writeln!(output, "        .route(\"/api/entities\", get(api::get_entities))")?;
    writeln!(output, "        .route(\"/api/stats\", get(api::get_stats))")?;
    writeln!(output, "        .route(\"/api/health\", get(api::health_check))")?;
    writeln!(output, "        .layer(CorsLayer::permissive())")?;
    writeln!(output, "        .with_state(state);\n")?;

    // Start server
    writeln!(output, "    // Start server")?;
    writeln!(output, "    let addr = std::env::var(\"HOST\")")?;
    writeln!(output, "        .unwrap_or_else(|_| \"0.0.0.0\".to_string());")?;
    writeln!(output, "    let port = std::env::var(\"PORT\")")?;
    writeln!(output, "        .unwrap_or_else(|_| \"8080\".to_string());")?;
    writeln!(output, "    let listener_addr = format!(\"{{}}:{{}}\", addr, port);\n")?;

    writeln!(output, "    tracing::info!(\"Starting server on {{}}\", listener_addr);")?;
    writeln!(output, "    let listener = tokio::net::TcpListener::bind(&listener_addr).await?;")?;
    writeln!(output, "    tracing::info!(\"Dashboard server listening on http://{{}}\", listener_addr);")?;
    writeln!(output, "    tracing::info!(\"WebSocket endpoint: ws://{{}}/ws\", listener_addr);")?;
    writeln!(output, "    tracing::info!(\"API endpoints: http://{{}}/api/*\", listener_addr);\n")?;

    writeln!(output, "    axum::serve(listener, app).await?;\n")?;

    writeln!(output, "    Ok(())")?;
    writeln!(output, "}}")?;

    Ok(())
}

/// Generate config.rs with entity metadata
fn generate_config_rs(
    entities: &[EntityDef],
    src_dir: &Path,
) -> Result<(), Box<dyn Error>> {
    let config_file = src_dir.join("config.rs");
    let mut output = std::fs::File::create(&config_file)?;

    writeln!(output, "// Auto-generated entity configuration\n")?;

    // EntityConfig struct
    writeln!(output, "#[derive(Debug, Clone)]")?;
    writeln!(output, "pub struct EntityConfig {{")?;
    writeln!(output, "    pub name: &'static str,")?;
    writeln!(output, "    pub table: &'static str,")?;
    writeln!(output, "    pub primary_key: &'static str,")?;
    writeln!(output, "    pub color: &'static str,")?;
    writeln!(output, "    pub icon: &'static str,")?;
    writeln!(output, "    pub fields: &'static [&'static str],")?;
    writeln!(output, "    pub max_records: usize,")?;
    writeln!(output, "}}\n")?;

    // ENTITIES constant
    writeln!(output, "/// All entity configurations")?;
    writeln!(output, "pub const ENTITIES: &[EntityConfig] = &[")?;

    for entity in entities {
        // Include ALL persistent entities (both root and derived)
        if !entity.is_persistent() || entity.is_abstract {
            continue;
        }
        if entity.source_type.to_lowercase() == "reference" {
            continue;
        }

        let display_config = generate_entity_display_config(entity);

        // Get primary key field (prefer primary_key config, fall back to conformant_id_column)
        let primary_key = if let Some(ref persistence) = entity.persistence {
            if let Some(ref pk_config) = persistence.primary_key {
                // Use the explicit primary_key name
                pk_config.name.to_lowercase()
            } else if let Some(ref db_config) = persistence.database {
                // Fall back to conformant_id_column
                db_config.conformant_id_column.to_lowercase()
            } else {
                "id".to_string()
            }
        } else {
            "id".to_string()
        };

        writeln!(output, "    EntityConfig {{")?;
        writeln!(output, "        name: \"{}\",", display_config.name)?;
        writeln!(output, "        table: \"{}\",", display_config.table)?;
        writeln!(output, "        primary_key: \"{}\",", primary_key)?;
        writeln!(output, "        color: \"{}\",", display_config.color)?;
        writeln!(output, "        icon: \"{}\",", display_config.icon)?;
        write!(output, "        fields: &[")?;
        for (i, field) in display_config.display_fields.iter().enumerate() {
            if i > 0 {
                write!(output, ", ")?;
            }
            write!(output, "\"{}\"", field)?;
        }
        writeln!(output, "],")?;
        writeln!(output, "        max_records: {},", display_config.max_records)?;
        writeln!(output, "    }},")?;
    }

    writeln!(output, "];")?;

    Ok(())
}

/// Generate polling.rs with table polling logic
fn generate_polling_rs(
    entities: &[EntityDef],
    src_dir: &Path,
    _db_type: DatabaseType,
) -> Result<(), Box<dyn Error>> {
    let polling_file = src_dir.join("polling.rs");
    let mut output = std::fs::File::create(&polling_file)?;

    writeln!(output, "// Auto-generated polling logic\n")?;
    writeln!(output, "use crate::{{AppState, config}};")?;
    writeln!(output, "use serde_json::{{json, Value}};")?;
    writeln!(output, "use sqlx::{{Row, Column}};")?;
    writeln!(output, "use std::time::Duration;")?;
    writeln!(output, "use chrono::Utc;\n")?;

    // Constants
    writeln!(output, "const POLL_INTERVAL_MS: u64 = 500;")?;
    writeln!(output, "const MAX_RECORDS_PER_POLL: i64 = 100;\n")?;

    // Start all polling tasks
    writeln!(output, "/// Start polling tasks for all entities")?;
    writeln!(output, "pub async fn start_all_polling_tasks(state: AppState) {{")?;
    writeln!(output, "    tracing::info!(\"Starting polling tasks for all entities...\");\n")?;

    // Spawn a task for each entity table
    for entity in entities {
        if !entity.is_persistent() || entity.is_abstract {
            continue;
        }
        if entity.source_type.to_lowercase() == "reference" {
            continue;
        }

        let display_config = generate_entity_display_config(entity);
        let table_name = &display_config.table;
        let entity_name = &display_config.name;

        // Extract primary key for this entity (prefer primary_key config, fall back to conformant_id_column)
        let primary_key = if let Some(ref persistence) = entity.persistence {
            if let Some(ref pk_config) = persistence.primary_key {
                // Use the explicit primary_key name
                pk_config.name.to_lowercase()
            } else if let Some(ref db_config) = persistence.database {
                // Fall back to conformant_id_column
                db_config.conformant_id_column.to_lowercase()
            } else {
                "id".to_string()
            }
        } else {
            "id".to_string()
        };

        writeln!(output, "    // Spawn polling task for {}", entity_name)?;
        writeln!(output, "    {{")?;
        writeln!(output, "        let state = state.clone();")?;
        writeln!(output, "        tokio::spawn(async move {{")?;
        writeln!(output, "            poll_entity_table(")?;
        writeln!(output, "                \"{}\".to_string(),", table_name)?;
        writeln!(output, "                \"{}\".to_string(),", entity_name)?;
        writeln!(output, "                \"{}\".to_string(),", primary_key)?;
        writeln!(output, "                state,")?;
        writeln!(output, "            ).await;")?;
        writeln!(output, "        }});")?;
        writeln!(output, "    }}\n")?;
    }

    writeln!(output, "    tracing::info!(\"All polling tasks started\");")?;
    writeln!(output, "}}\n")?;

    // Poll entity table function
    writeln!(output, "/// Poll a single entity table for new records")?;
    writeln!(output, "async fn poll_entity_table(")?;
    writeln!(output, "    table: String,")?;
    writeln!(output, "    entity_name: String,")?;
    writeln!(output, "    primary_key: String,")?;
    writeln!(output, "    state: AppState,")?;
    writeln!(output, ") {{")?;
    writeln!(output, "    tracing::info!(\"Starting polling for table: {{}}\", table);\n")?;

    writeln!(output, "    loop {{")?;
    writeln!(output, "        // Get last seen ID for this table")?;
    writeln!(output, "        let last_id = {{")?;
    writeln!(output, "            let ids = state.last_ids.read().await;")?;
    writeln!(output, "            ids.get(&table).copied().unwrap_or(0)")?;
    writeln!(output, "        }};\n")?;

    writeln!(output, "        // Query for new records")?;
    writeln!(output, "        let query = format!(")?;
    writeln!(output, "            \"SELECT * FROM {{}} WHERE {{}} > $1 ORDER BY {{}} ASC LIMIT $2\",")?;
    writeln!(output, "            table, primary_key, primary_key")?;
    writeln!(output, "        );\n")?;

    writeln!(output, "        match sqlx::query(&query)")?;
    writeln!(output, "            .bind(last_id)")?;
    writeln!(output, "            .bind(MAX_RECORDS_PER_POLL)")?;
    writeln!(output, "            .fetch_all(&state.pool)")?;
    writeln!(output, "            .await")?;
    writeln!(output, "        {{")?;
    writeln!(output, "            Ok(rows) => {{")?;
    writeln!(output, "                if !rows.is_empty() {{")?;
    writeln!(output, "                    tracing::debug!(\"Found {{}} new records in {{}}\", rows.len(), table);\n")?;

    writeln!(output, "                    // Get last primary key value from results")?;
    writeln!(output, "                    // Handle both integer and string primary keys")?;
    writeln!(output, "                    if let Some(last_row) = rows.last() {{")?;
    writeln!(output, "                        // Try integer first, fall back to string")?;
    writeln!(output, "                        if let Ok(pk_val) = last_row.try_get::<i64, _>(primary_key.as_str()) {{")?;
    writeln!(output, "                            state.last_ids.write().await.insert(table.clone(), pk_val);")?;
    writeln!(output, "                        }} else if let Ok(pk_str) = last_row.try_get::<String, _>(primary_key.as_str()) {{")?;
    writeln!(output, "                            // For string keys, use hash as tracking ID")?;
    writeln!(output, "                            let hash_id = pk_str.bytes().fold(0i64, |acc, b| acc.wrapping_mul(31).wrapping_add(b as i64));")?;
    writeln!(output, "                            state.last_ids.write().await.insert(table.clone(), hash_id);")?;
    writeln!(output, "                        }}")?;
    writeln!(output, "                    }}\n")?;

    writeln!(output, "                    // Broadcast each new record to WebSocket clients")?;
    writeln!(output, "                    for row in rows {{")?;
    writeln!(output, "                        let row_data = row_to_json(&row);")?;
    writeln!(output, "                        let message = json!({{")?;
    writeln!(output, "                            \"entity\": entity_name,")?;
    writeln!(output, "                            \"event_type\": \"insert\",")?;
    writeln!(output, "                            \"data\": row_data,")?;
    writeln!(output, "                            \"timestamp\": Utc::now().to_rfc3339(),")?;
    writeln!(output, "                        }});\n")?;

    writeln!(output, "                        broadcast_to_clients(&state, message).await;")?;
    writeln!(output, "                    }}")?;
    writeln!(output, "                }}")?;
    writeln!(output, "            }}")?;
    writeln!(output, "            Err(e) => {{")?;
    writeln!(output, "                tracing::error!(\"Error polling table {{}}: {{:?}}\", table, e);")?;
    writeln!(output, "            }}")?;
    writeln!(output, "        }}\n")?;

    writeln!(output, "        tokio::time::sleep(Duration::from_millis(POLL_INTERVAL_MS)).await;")?;
    writeln!(output, "    }}")?;
    writeln!(output, "}}\n")?;

    // Helper: Convert Row to JSON
    writeln!(output, "/// Convert a database row to JSON")?;
    writeln!(output, "fn row_to_json(row: &sqlx::postgres::PgRow) -> Value {{")?;
    writeln!(output, "    let mut map = serde_json::Map::new();\n")?;

    writeln!(output, "    // Iterate through all columns and convert to JSON")?;
    writeln!(output, "    for column in row.columns() {{")?;
    writeln!(output, "        let name = column.name();")?;
    writeln!(output, "        \n")?;
    writeln!(output, "        // Try to extract value as different types")?;
    writeln!(output, "        let value = if let Ok(v) = row.try_get::<i64, _>(name) {{")?;
    writeln!(output, "            json!(v)")?;
    writeln!(output, "        }} else if let Ok(v) = row.try_get::<i32, _>(name) {{")?;
    writeln!(output, "            json!(v)")?;
    writeln!(output, "        }} else if let Ok(v) = row.try_get::<f64, _>(name) {{")?;
    writeln!(output, "            json!(v)")?;
    writeln!(output, "        }} else if let Ok(v) = row.try_get::<String, _>(name) {{")?;
    writeln!(output, "            json!(v)")?;
    writeln!(output, "        }} else if let Ok(v) = row.try_get::<bool, _>(name) {{")?;
    writeln!(output, "            json!(v)")?;
    writeln!(output, "        }} else if let Ok(v) = row.try_get::<chrono::NaiveDate, _>(name) {{")?;
    writeln!(output, "            json!(v.to_string())")?;
    writeln!(output, "        }} else if let Ok(v) = row.try_get::<chrono::NaiveDateTime, _>(name) {{")?;
    writeln!(output, "            json!(v.to_string())")?;
    writeln!(output, "        }} else {{")?;
    writeln!(output, "            // Handle NULL or unsupported types")?;
    writeln!(output, "            json!(null)")?;
    writeln!(output, "        }};\n")?;

    writeln!(output, "        map.insert(name.to_string(), value);")?;
    writeln!(output, "    }}\n")?;

    writeln!(output, "    Value::Object(map)")?;
    writeln!(output, "}}\n")?;

    // Broadcast helper
    writeln!(output, "/// Broadcast a message to all connected WebSocket clients")?;
    writeln!(output, "async fn broadcast_to_clients(state: &AppState, message: Value) {{")?;
    writeln!(output, "    let clients = state.clients.read().await;")?;
    writeln!(output, "    let message_text = message.to_string();\n")?;

    writeln!(output, "    for client in clients.iter() {{")?;
    writeln!(output, "        let mut socket = client.lock().await;")?;
    writeln!(output, "        if let Err(e) = socket.send(axum::extract::ws::Message::Text(message_text.clone())).await {{")?;
    writeln!(output, "            tracing::warn!(\"Failed to send message to client: {{:?}}\", e);")?;
    writeln!(output, "        }}")?;
    writeln!(output, "    }}")?;
    writeln!(output, "}}")?;

    Ok(())
}

/// Generate websocket.rs with WebSocket connection handling
fn generate_websocket_rs(src_dir: &Path) -> Result<(), Box<dyn Error>> {
    let ws_file = src_dir.join("websocket.rs");
    let mut output = std::fs::File::create(&ws_file)?;

    writeln!(output, "// Auto-generated WebSocket handler\n")?;
    writeln!(output, "use crate::{{AppState, config}};")?;
    writeln!(output, "use axum::{{")?;
    writeln!(output, "    extract::{{State, WebSocketUpgrade, ws::{{WebSocket, Message}}}},")?;
    writeln!(output, "    response::Response,")?;
    writeln!(output, "}};")?;
    writeln!(output, "use std::sync::Arc;")?;
    writeln!(output, "use serde_json::{{json, Value}};")?;
    writeln!(output, "use sqlx::{{Row, Column}};")?;
    writeln!(output, "use chrono::Utc;\n")?;

    writeln!(output, "const INITIAL_RECORDS_LIMIT: i64 = 100;\n")?;

    writeln!(output, "/// Send initial data to a newly connected client")?;
    writeln!(output, "async fn send_initial_data(")?;
    writeln!(output, "    socket: &Arc<tokio::sync::Mutex<WebSocket>>,")?;
    writeln!(output, "    state: &AppState,")?;
    writeln!(output, ") {{")?;
    writeln!(output, "    tracing::info!(\"Sending initial data to client...\");\n")?;

    writeln!(output, "    for entity in config::ENTITIES {{")?;
    writeln!(output, "        // Query most recent records from this table")?;
    writeln!(output, "        let query = format!(")?;
    writeln!(output, "            \"SELECT * FROM {{}} ORDER BY {{}} DESC LIMIT $1\",")?;
    writeln!(output, "            entity.table, entity.primary_key")?;
    writeln!(output, "        );\n")?;

    writeln!(output, "        match sqlx::query(&query)")?;
    writeln!(output, "            .bind(INITIAL_RECORDS_LIMIT)")?;
    writeln!(output, "            .fetch_all(&state.pool)")?;
    writeln!(output, "            .await")?;
    writeln!(output, "        {{")?;
    writeln!(output, "            Ok(rows) => {{")?;
    writeln!(output, "                tracing::debug!(\"Sending {{}} initial records for {{}}\", rows.len(), entity.name);\n")?;

    writeln!(output, "                // Send each record to the client (in reverse order so newest are last)")?;
    writeln!(output, "                for row in rows.iter().rev() {{")?;
    writeln!(output, "                    let row_data = row_to_json(row);")?;
    writeln!(output, "                    let message = json!({{")?;
    writeln!(output, "                        \"entity\": entity.name,")?;
    writeln!(output, "                        \"event_type\": \"initial\",")?;
    writeln!(output, "                        \"data\": row_data,")?;
    writeln!(output, "                        \"timestamp\": Utc::now().to_rfc3339(),")?;
    writeln!(output, "                    }});\n")?;

    writeln!(output, "                    let mut sock = socket.lock().await;")?;
    writeln!(output, "                    if let Err(e) = sock.send(Message::Text(message.to_string())).await {{")?;
    writeln!(output, "                        tracing::warn!(\"Failed to send initial data to client: {{:?}}\", e);")?;
    writeln!(output, "                        return;")?;
    writeln!(output, "                    }}")?;
    writeln!(output, "                }}")?;
    writeln!(output, "            }}")?;
    writeln!(output, "            Err(e) => {{")?;
    writeln!(output, "                tracing::error!(\"Failed to load initial data for {{}}: {{:?}}\", entity.name, e);")?;
    writeln!(output, "            }}")?;
    writeln!(output, "        }}")?;
    writeln!(output, "    }}\n")?;

    writeln!(output, "    tracing::info!(\"Initial data sent to client\");")?;
    writeln!(output, "}}\n")?;

    writeln!(output, "/// Convert a database row to JSON")?;
    writeln!(output, "fn row_to_json(row: &sqlx::postgres::PgRow) -> Value {{")?;
    writeln!(output, "    let mut map = serde_json::Map::new();\n")?;

    writeln!(output, "    for column in row.columns() {{")?;
    writeln!(output, "        let name = column.name();\n")?;

    writeln!(output, "        let value = if let Ok(v) = row.try_get::<i64, _>(name) {{")?;
    writeln!(output, "            json!(v)")?;
    writeln!(output, "        }} else if let Ok(v) = row.try_get::<i32, _>(name) {{")?;
    writeln!(output, "            json!(v)")?;
    writeln!(output, "        }} else if let Ok(v) = row.try_get::<f64, _>(name) {{")?;
    writeln!(output, "            json!(v)")?;
    writeln!(output, "        }} else if let Ok(v) = row.try_get::<String, _>(name) {{")?;
    writeln!(output, "            json!(v)")?;
    writeln!(output, "        }} else if let Ok(v) = row.try_get::<bool, _>(name) {{")?;
    writeln!(output, "            json!(v)")?;
    writeln!(output, "        }} else if let Ok(v) = row.try_get::<chrono::NaiveDate, _>(name) {{")?;
    writeln!(output, "            json!(v.to_string())")?;
    writeln!(output, "        }} else if let Ok(v) = row.try_get::<chrono::NaiveDateTime, _>(name) {{")?;
    writeln!(output, "            json!(v.to_string())")?;
    writeln!(output, "        }} else {{")?;
    writeln!(output, "            json!(null)")?;
    writeln!(output, "        }};\n")?;

    writeln!(output, "        map.insert(name.to_string(), value);")?;
    writeln!(output, "    }}\n")?;

    writeln!(output, "    Value::Object(map)")?;
    writeln!(output, "}}\n")?;

    writeln!(output, "/// WebSocket connection handler")?;
    writeln!(output, "pub async fn websocket_handler(")?;
    writeln!(output, "    ws: WebSocketUpgrade,")?;
    writeln!(output, "    State(state): State<AppState>,")?;
    writeln!(output, ") -> Response {{")?;
    writeln!(output, "    ws.on_upgrade(|socket| handle_socket(socket, state))")?;
    writeln!(output, "}}\n")?;

    writeln!(output, "/// Handle individual WebSocket connection")?;
    writeln!(output, "async fn handle_socket(")?;
    writeln!(output, "    socket: WebSocket,")?;
    writeln!(output, "    state: AppState,")?;
    writeln!(output, ") {{")?;
    writeln!(output, "    tracing::info!(\"WebSocket client connected\");\n")?;

    writeln!(output, "    // Wrap socket in Arc<Mutex> for sharing with broadcast")?;
    writeln!(output, "    let socket = Arc::new(tokio::sync::Mutex::new(socket));\n")?;

    writeln!(output, "    // Add to connected clients")?;
    writeln!(output, "    {{")?;
    writeln!(output, "        let mut clients = state.clients.write().await;")?;
    writeln!(output, "        clients.push(socket.clone());")?;
    writeln!(output, "        tracing::info!(\"Client added. Total clients: {{}}\", clients.len());")?;
    writeln!(output, "    }}\n")?;

    writeln!(output, "    // Send initial data to newly connected client")?;
    writeln!(output, "    send_initial_data(&socket, &state).await;\n")?;

    writeln!(output, "    // Keep connection alive by reading messages")?;
    writeln!(output, "    // (Client may send ping/pong or filter commands)")?;
    writeln!(output, "    loop {{")?;
    writeln!(output, "        let mut sock = socket.lock().await;")?;
    writeln!(output, "        match sock.recv().await {{")?;
    writeln!(output, "            Some(Ok(msg)) => {{")?;
    writeln!(output, "                match msg {{")?;
    writeln!(output, "                    Message::Close(_) => {{")?;
    writeln!(output, "                        tracing::info!(\"Client disconnected\");")?;
    writeln!(output, "                        break;")?;
    writeln!(output, "                    }}")?;
    writeln!(output, "                    Message::Ping(data) => {{")?;
    writeln!(output, "                        if let Err(e) = sock.send(Message::Pong(data)).await {{")?;
    writeln!(output, "                            tracing::warn!(\"Failed to send pong: {{:?}}\", e);")?;
    writeln!(output, "                            break;")?;
    writeln!(output, "                        }}")?;
    writeln!(output, "                    }}")?;
    writeln!(output, "                    _ => {{")?;
    writeln!(output, "                        // Handle other message types if needed")?;
    writeln!(output, "                    }}")?;
    writeln!(output, "                }}")?;
    writeln!(output, "            }}")?;
    writeln!(output, "            Some(Err(e)) => {{")?;
    writeln!(output, "                tracing::warn!(\"WebSocket error: {{:?}}\", e);")?;
    writeln!(output, "                break;")?;
    writeln!(output, "            }}")?;
    writeln!(output, "            None => {{")?;
    writeln!(output, "                tracing::info!(\"Client closed connection\");")?;
    writeln!(output, "                break;")?;
    writeln!(output, "            }}")?;
    writeln!(output, "        }}")?;
    writeln!(output, "    }}\n")?;

    writeln!(output, "    // Remove from connected clients")?;
    writeln!(output, "    {{")?;
    writeln!(output, "        let mut clients = state.clients.write().await;")?;
    writeln!(output, "        clients.retain(|c| !Arc::ptr_eq(c, &socket));")?;
    writeln!(output, "        tracing::info!(\"Client removed. Remaining clients: {{}}\", clients.len());")?;
    writeln!(output, "    }}")?;
    writeln!(output, "}}")?;

    Ok(())
}

/// Generate api.rs with REST API endpoints
fn generate_api_rs(src_dir: &Path) -> Result<(), Box<dyn Error>> {
    let api_file = src_dir.join("api.rs");
    let mut output = std::fs::File::create(&api_file)?;

    writeln!(output, "// Auto-generated REST API endpoints\n")?;
    writeln!(output, "use crate::{{AppState, config}};")?;
    writeln!(output, "use axum::{{")?;
    writeln!(output, "    extract::State,")?;
    writeln!(output, "    Json,")?;
    writeln!(output, "}};")?;
    writeln!(output, "use serde_json::{{json, Value}};\n")?;

    writeln!(output, "/// Get all entity configurations")?;
    writeln!(output, "pub async fn get_entities() -> Json<Value> {{")?;
    writeln!(output, "    let entities_json: Vec<Value> = config::ENTITIES.iter()")?;
    writeln!(output, "        .map(|e| json!({{")?;
    writeln!(output, "            \"name\": e.name,")?;
    writeln!(output, "            \"table\": e.table,")?;
    writeln!(output, "            \"color\": e.color,")?;
    writeln!(output, "            \"icon\": e.icon,")?;
    writeln!(output, "            \"fields\": e.fields,")?;
    writeln!(output, "            \"max_records\": e.max_records,")?;
    writeln!(output, "        }}))")?;
    writeln!(output, "        .collect();")?;
    writeln!(output, "    Json(json!(entities_json))")?;
    writeln!(output, "}}\n")?;

    writeln!(output, "/// Get statistics for all entities")?;
    writeln!(output, "pub async fn get_stats(State(state): State<AppState>) -> Json<Value> {{")?;
    writeln!(output, "    let mut stats = serde_json::Map::new();\n")?;

    writeln!(output, "    for entity in config::ENTITIES {{")?;
    writeln!(output, "        let count_query = format!(\"SELECT COUNT(*) as count FROM {{}}\", entity.table);")?;
    writeln!(output, "        \n")?;
    writeln!(output, "        match sqlx::query_scalar::<_, i64>(&count_query)")?;
    writeln!(output, "            .fetch_one(&state.pool)")?;
    writeln!(output, "            .await")?;
    writeln!(output, "        {{")?;
    writeln!(output, "            Ok(count) => {{")?;
    writeln!(output, "                let mut entity_stats = serde_json::Map::new();")?;
    writeln!(output, "                entity_stats.insert(\"total\".to_string(), json!(count));")?;
    writeln!(output, "                stats.insert(entity.name.to_string(), json!(entity_stats));")?;
    writeln!(output, "            }}")?;
    writeln!(output, "            Err(e) => {{")?;
    writeln!(output, "                tracing::error!(\"Failed to get count for {{}}: {{:?}}\", entity.name, e);")?;
    writeln!(output, "                let mut entity_stats = serde_json::Map::new();")?;
    writeln!(output, "                entity_stats.insert(\"total\".to_string(), json!(0));")?;
    writeln!(output, "                stats.insert(entity.name.to_string(), json!(entity_stats));")?;
    writeln!(output, "            }}")?;
    writeln!(output, "        }}")?;
    writeln!(output, "    }}\n")?;

    writeln!(output, "    Json(json!(stats))")?;
    writeln!(output, "}}\n")?;

    writeln!(output, "/// Health check")?;
    writeln!(output, "pub async fn health_check(State(state): State<AppState>) -> Json<Value> {{")?;
    writeln!(output, "    let client_count = state.clients.read().await.len();")?;
    writeln!(output, "    Json(json!({{\"status\": \"ok\", \"connected_clients\": client_count}}))")?;
    writeln!(output, "}}")?;

    Ok(())
}

/// Generate .env.example
fn generate_env_example(
    output_dir: &Path,
    db_type: DatabaseType,
) -> Result<(), Box<dyn Error>> {
    let env_file = output_dir.join(".env.example");
    let mut output = std::fs::File::create(&env_file)?;

    writeln!(output, "# Database connection")?;
    match db_type {
        DatabaseType::PostgreSQL => {
            writeln!(output, "DATABASE_URL=postgresql://user:password@localhost:5432/dbname")?;
        }
        DatabaseType::MySQL | DatabaseType::MariaDB => {
            writeln!(output, "DATABASE_URL=mysql://user:password@localhost:3306/dbname")?;
        }
    }

    writeln!(output, "\n# Server configuration")?;
    writeln!(output, "HOST=0.0.0.0")?;
    writeln!(output, "PORT=8080")?;

    writeln!(output, "\n# Logging")?;
    writeln!(output, "RUST_LOG=info")?;

    Ok(())
}
