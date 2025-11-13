// Auto-generated REST API endpoints

use crate::{AppState, config};
use axum::{
    extract::State,
    Json,
};
use serde_json::{json, Value};

/// Get all entity configurations
pub async fn get_entities() -> Json<Value> {
    let entities_json: Vec<Value> = config::ENTITIES.iter()
        .map(|e| json!({
            "name": e.name,
            "table": e.table,
            "color": e.color,
            "icon": e.icon,
            "fields": e.fields,
            "max_records": e.max_records,
        }))
        .collect();
    Json(json!(entities_json))
}

/// Get statistics for all entities
pub async fn get_stats(State(state): State<AppState>) -> Json<Value> {
    let mut stats = serde_json::Map::new();

    for entity in config::ENTITIES {
        let count_query = format!("SELECT COUNT(*) as count FROM {}", entity.table);
        

        match sqlx::query_scalar::<_, i64>(&count_query)
            .fetch_one(&state.pool)
            .await
        {
            Ok(count) => {
                let mut entity_stats = serde_json::Map::new();
                entity_stats.insert("total".to_string(), json!(count));
                stats.insert(entity.name.to_string(), json!(entity_stats));
            }
            Err(e) => {
                tracing::error!("Failed to get count for {}: {:?}", entity.name, e);
                let mut entity_stats = serde_json::Map::new();
                entity_stats.insert("total".to_string(), json!(0));
                stats.insert(entity.name.to_string(), json!(entity_stats));
            }
        }
    }

    Json(json!(stats))
}

/// Health check
pub async fn health_check(State(state): State<AppState>) -> Json<Value> {
    let client_count = state.clients.read().await.len();
    Json(json!({"status": "ok", "connected_clients": client_count}))
}
