// Auto-generated polling logic

use crate::{AppState, config};
use serde_json::{json, Value};
use sqlx::{Row, Column};
use std::time::Duration;
use chrono::Utc;

const POLL_INTERVAL_MS: u64 = 500;
const MAX_RECORDS_PER_POLL: i64 = 100;

/// Start polling tasks for all entities
pub async fn start_all_polling_tasks(state: AppState) {
    tracing::info!("Starting polling tasks for all entities...");

    // Spawn polling task for OrderLineItem
    {
        let state = state.clone();
        tokio::spawn(async move {
            poll_entity_table(
                "order_line_items".to_string(),
                "OrderLineItem".to_string(),
                state,
            ).await;
        });
    }

    // Spawn polling task for Order
    {
        let state = state.clone();
        tokio::spawn(async move {
            poll_entity_table(
                "orders".to_string(),
                "Order".to_string(),
                state,
            ).await;
        });
    }

    tracing::info!("All polling tasks started");
}

/// Poll a single entity table for new records
async fn poll_entity_table(
    table: String,
    entity_name: String,
    state: AppState,
) {
    tracing::info!("Starting polling for table: {}", table);

    loop {
        // Get last seen ID for this table
        let last_id = {
            let ids = state.last_ids.read().await;
            ids.get(&table).copied().unwrap_or(0)
        };

        // Query for new records
        let query = format!(
            "SELECT * FROM {} WHERE id > $1 ORDER BY id ASC LIMIT $2",
            table
        );

        match sqlx::query(&query)
            .bind(last_id)
            .bind(MAX_RECORDS_PER_POLL)
            .fetch_all(&state.pool)
            .await
        {
            Ok(rows) => {
                if !rows.is_empty() {
                    tracing::debug!("Found {} new records in {}", rows.len(), table);

                    // Get max ID from results
                    if let Some(max_id) = rows.iter()
                        .filter_map(|row| row.try_get::<i64, _>("id").ok())
                        .max()
                    {
                        // Update last seen ID
                        state.last_ids.write().await.insert(table.clone(), max_id);
                    }

                    // Broadcast each new record to WebSocket clients
                    for row in rows {
                        let row_data = row_to_json(&row);
                        let message = json!({
                            "entity": entity_name,
                            "event_type": "insert",
                            "data": row_data,
                            "timestamp": Utc::now().to_rfc3339(),
                        });

                        broadcast_to_clients(&state, message).await;
                    }
                }
            }
            Err(e) => {
                tracing::error!("Error polling table {}: {:?}", table, e);
            }
        }

        tokio::time::sleep(Duration::from_millis(POLL_INTERVAL_MS)).await;
    }
}

/// Convert a database row to JSON
fn row_to_json(row: &sqlx::postgres::PgRow) -> Value {
    let mut map = serde_json::Map::new();

    // Iterate through all columns and convert to JSON
    for column in row.columns() {
        let name = column.name();
        

        // Try to extract value as different types
        let value = if let Ok(v) = row.try_get::<i64, _>(name) {
            json!(v)
        } else if let Ok(v) = row.try_get::<i32, _>(name) {
            json!(v)
        } else if let Ok(v) = row.try_get::<f64, _>(name) {
            json!(v)
        } else if let Ok(v) = row.try_get::<String, _>(name) {
            json!(v)
        } else if let Ok(v) = row.try_get::<bool, _>(name) {
            json!(v)
        } else if let Ok(v) = row.try_get::<chrono::NaiveDate, _>(name) {
            json!(v.to_string())
        } else if let Ok(v) = row.try_get::<chrono::NaiveDateTime, _>(name) {
            json!(v.to_string())
        } else {
            // Handle NULL or unsupported types
            json!(null)
        };

        map.insert(name.to_string(), value);
    }

    Value::Object(map)
}

/// Broadcast a message to all connected WebSocket clients
async fn broadcast_to_clients(state: &AppState, message: Value) {
    let clients = state.clients.read().await;
    let message_text = message.to_string();

    for client in clients.iter() {
        let mut socket = client.lock().await;
        if let Err(e) = socket.send(axum::extract::ws::Message::Text(message_text.clone())).await {
            tracing::warn!("Failed to send message to client: {:?}", e);
        }
    }
}
