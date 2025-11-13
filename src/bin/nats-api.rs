/// Generic NATS API - HTTP ingestion endpoint that publishes to NATS JetStream
///
/// This is a reusable binary (NOT codegen'd) that accepts any JSON and publishes
/// to NATS for async processing by entity-specific worker binaries.

use axum::{
    extract::{Path, State},
    http::StatusCode,
    response::IntoResponse,
    routing::{get, post},
    Json, Router,
};
use std::net::SocketAddr;
use std::sync::Arc;
use tower_http::cors::CorsLayer;
use uuid::Uuid;

use nomnom::{MessageEnvelope, IngestionResponse, IngestionStatus, NatsClient, NatsConfig};

#[derive(Clone)]
struct AppState {
    nats: NatsClient,
}

#[tokio::main]
async fn main() {
    // Initialize tracing
    tracing_subscriber::fmt::init();

    // Load environment variables
    dotenv::dotenv().ok();

    // Connect to NATS JetStream
    let nats_config = NatsConfig::default();
    let nats = NatsClient::connect(nats_config).await
        .expect("Failed to connect to NATS");

    // Create application state
    let state = Arc::new(AppState { nats });

    // Build router
    let app = Router::new()
        .route("/ingest/message", post(ingest_message))
        .route("/ingest/batch", post(ingest_batch))
        .route("/ingest/status/:message_id", get(check_status))
        .route("/health", get(health_check))
        .route("/ready", get(readiness_check))
        .layer(CorsLayer::permissive())
        .with_state(state);

    // Run server
    let port = std::env::var("PORT")
        .unwrap_or_else(|_| "8080".to_string())
        .parse()
        .expect("Invalid PORT");

    let addr = SocketAddr::from(([0, 0, 0, 0], port));
    tracing::info!("Generic NATS API listening on {}", addr);
    tracing::info!("Publishing to NATS stream: {}",
        std::env::var("NATS_STREAM").unwrap_or_else(|_| "MESSAGES".to_string()));

    let listener = tokio::net::TcpListener::bind(&addr).await.unwrap();
    axum::serve(listener, app).await.unwrap();
}

/// Ingest a single message (async via NATS)
async fn ingest_message(
    State(state): State<Arc<AppState>>,
    body: String,
) -> Result<(StatusCode, Json<IngestionResponse>), AppError> {
    // Validate JSON format (but don't parse entities - that's worker's job)
    let _: serde_json::Value = serde_json::from_str(&body)
        .map_err(|e| AppError::ValidationError(format!("Invalid JSON: {}", e)))?;

    // Extract entity_type hint if available
    let entity_type = serde_json::from_str::<serde_json::Value>(&body)
        .ok()
        .and_then(|v| v.get("entity_type").and_then(|t| t.as_str().map(String::from)));

    // Create message envelope
    let envelope = MessageEnvelope::new(body, entity_type);

    // Publish to NATS JetStream
    state.nats.publish_message(&envelope).await
        .map_err(|e| AppError::InternalError(format!("NATS publish failed: {}", e)))?;

    tracing::info!("Message {} queued for processing", envelope.message_id);

    Ok((
        StatusCode::ACCEPTED,
        Json(IngestionResponse {
            message_id: envelope.message_id.to_string(),
            status: IngestionStatus::Accepted,
            timestamp: envelope.received_at,
        })
    ))
}

/// Ingest a batch of messages (async via NATS)
async fn ingest_batch(
    State(state): State<Arc<AppState>>,
    body: String,
) -> Result<(StatusCode, Json<BatchResponse>), AppError> {
    let start = std::time::Instant::now();
    let lines: Vec<&str> = body.lines().collect();

    let mut processed = 0;
    let mut inserted = 0;
    let mut failed = 0;
    let mut errors = Vec::new();

    for (line_num, line) in lines.iter().enumerate() {
        processed += 1;

        // Validate JSON format
        match serde_json::from_str::<serde_json::Value>(line) {
            Ok(_) => {
                // Create envelope and publish to NATS
                let envelope = MessageEnvelope::new(line.to_string(), None);
                match state.nats.publish_message(&envelope).await {
                    Ok(_) => inserted += 1,
                    Err(e) => {
                        failed += 1;
                        errors.push(format!("Line {}: NATS error: {}", line_num + 1, e));
                    }
                }
            }
            Err(e) => {
                failed += 1;
                errors.push(format!("Line {}: Invalid JSON: {}", line_num + 1, e));
            }
        }
    }

    Ok((
        StatusCode::ACCEPTED,
        Json(BatchResponse {
            status: if failed == 0 { "success" } else { "partial" }.to_string(),
            processed,
            inserted,
            failed,
            errors,
            duration_ms: start.elapsed().as_millis() as u64,
        })
    ))
}

/// Check message processing status by ID
async fn check_status(
    Path(message_id): Path<String>,
) -> Result<Json<serde_json::Value>, AppError> {
    // Parse message_id as UUID
    let _uuid = Uuid::parse_str(&message_id)
        .map_err(|_| AppError::ValidationError("Invalid UUID format".to_string()))?;

    // TODO: Query message_status table for processing status
    // For now, return a placeholder response
    Ok(Json(serde_json::json!({
        "message_id": message_id,
        "status": "accepted",
        "message": "Status tracking requires message_status table (to be implemented by worker)"
    })))
}

/// Health check endpoint (liveness)
async fn health_check() -> Json<serde_json::Value> {
    Json(serde_json::json!({
        "status": "healthy",
        "service": "nats-api",
        "version": env!("CARGO_PKG_VERSION"),
    }))
}

/// Readiness check endpoint - verifies NATS connection
async fn readiness_check(
    State(state): State<Arc<AppState>>,
) -> Result<Json<serde_json::Value>, StatusCode> {
    // Check if NATS client is connected
    if state.nats.is_connected() {
        Ok(Json(serde_json::json!({
            "status": "ready",
            "service": "nats-api",
            "nats": "connected"
        })))
    } else {
        Err(StatusCode::SERVICE_UNAVAILABLE)
    }
}

// Error handling

#[derive(Debug)]
enum AppError {
    ValidationError(String),
    InternalError(String),
}

impl IntoResponse for AppError {
    fn into_response(self) -> axum::response::Response {
        let (status, message) = match self {
            AppError::ValidationError(msg) => (StatusCode::BAD_REQUEST, msg),
            AppError::InternalError(msg) => (StatusCode::INTERNAL_SERVER_ERROR, msg),
        };

        (status, Json(serde_json::json!({
            "error": message
        }))).into_response()
    }
}

// Response types

#[derive(Debug, serde::Serialize)]
struct BatchResponse {
    status: String,
    processed: usize,
    inserted: usize,
    failed: usize,
    errors: Vec<String>,
    duration_ms: u64,
}
