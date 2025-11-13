// Auto-generated WebSocket handler

use crate::{AppState, config};
use axum::{
    extract::{State, WebSocketUpgrade, ws::{WebSocket, Message}},
    response::Response,
};
use std::sync::Arc;
use serde_json::{json, Value};
use sqlx::{Row, Column};
use chrono::Utc;

const INITIAL_RECORDS_LIMIT: i64 = 100;

/// Send initial data to a newly connected client
async fn send_initial_data(
    socket: &Arc<tokio::sync::Mutex<WebSocket>>,
    state: &AppState,
) {
    tracing::info!("Sending initial data to client...");

    for entity in config::ENTITIES {
        // Query most recent records from this table
        let query = format!(
            "SELECT * FROM {} ORDER BY id DESC LIMIT $1",
            entity.table
        );

        match sqlx::query(&query)
            .bind(INITIAL_RECORDS_LIMIT)
            .fetch_all(&state.pool)
            .await
        {
            Ok(rows) => {
                tracing::debug!("Sending {} initial records for {}", rows.len(), entity.name);

                // Send each record to the client (in reverse order so newest are last)
                for row in rows.iter().rev() {
                    let row_data = row_to_json(row);
                    let message = json!({
                        "entity": entity.name,
                        "event_type": "initial",
                        "data": row_data,
                        "timestamp": Utc::now().to_rfc3339(),
                    });

                    let mut sock = socket.lock().await;
                    if let Err(e) = sock.send(Message::Text(message.to_string())).await {
                        tracing::warn!("Failed to send initial data to client: {:?}", e);
                        return;
                    }
                }
            }
            Err(e) => {
                tracing::error!("Failed to load initial data for {}: {:?}", entity.name, e);
            }
        }
    }

    tracing::info!("Initial data sent to client");
}

/// Convert a database row to JSON
fn row_to_json(row: &sqlx::postgres::PgRow) -> Value {
    let mut map = serde_json::Map::new();

    for column in row.columns() {
        let name = column.name();

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
            json!(null)
        };

        map.insert(name.to_string(), value);
    }

    Value::Object(map)
}

/// WebSocket connection handler
pub async fn websocket_handler(
    ws: WebSocketUpgrade,
    State(state): State<AppState>,
) -> Response {
    ws.on_upgrade(|socket| handle_socket(socket, state))
}

/// Handle individual WebSocket connection
async fn handle_socket(
    socket: WebSocket,
    state: AppState,
) {
    tracing::info!("WebSocket client connected");

    // Wrap socket in Arc<Mutex> for sharing with broadcast
    let socket = Arc::new(tokio::sync::Mutex::new(socket));

    // Add to connected clients
    {
        let mut clients = state.clients.write().await;
        clients.push(socket.clone());
        tracing::info!("Client added. Total clients: {}", clients.len());
    }

    // Send initial data to newly connected client
    send_initial_data(&socket, &state).await;

    // Keep connection alive by reading messages
    // (Client may send ping/pong or filter commands)
    loop {
        let mut sock = socket.lock().await;
        match sock.recv().await {
            Some(Ok(msg)) => {
                match msg {
                    Message::Close(_) => {
                        tracing::info!("Client disconnected");
                        break;
                    }
                    Message::Ping(data) => {
                        if let Err(e) = sock.send(Message::Pong(data)).await {
                            tracing::warn!("Failed to send pong: {:?}", e);
                            break;
                        }
                    }
                    _ => {
                        // Handle other message types if needed
                    }
                }
            }
            Some(Err(e)) => {
                tracing::warn!("WebSocket error: {:?}", e);
                break;
            }
            None => {
                tracing::info!("Client closed connection");
                break;
            }
        }
    }

    // Remove from connected clients
    {
        let mut clients = state.clients.write().await;
        clients.retain(|c| !Arc::ptr_eq(c, &socket));
        tracing::info!("Client removed. Remaining clients: {}", clients.len());
    }
}
