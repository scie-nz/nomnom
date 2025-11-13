// Auto-generated Axum backend for real-time dashboard

mod config;
mod polling;
mod websocket;
mod api;

use axum::{
    Router,
    routing::get,
};
use sqlx::{Pool, Postgres};
use std::sync::Arc;
use tokio::sync::RwLock;
use std::collections::HashMap;
use tower_http::cors::CorsLayer;
use tracing_subscriber::{layer::SubscriberExt, util::SubscriberInitExt};

/// Global application state
#[derive(Clone)]
pub struct AppState {
    /// Database connection pool
    pub pool: Pool<Postgres>,
    /// Last seen ID per entity table
    pub last_ids: Arc<RwLock<HashMap<String, i64>>>,
    /// Connected WebSocket clients
    pub clients: Arc<RwLock<Vec<Arc<tokio::sync::Mutex<axum::extract::ws::WebSocket>>>>>,
}

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    // Initialize tracing
    tracing_subscriber::registry()
        .with(tracing_subscriber::EnvFilter::try_from_default_env()
            .unwrap_or_else(|_| "info".into()))
        .with(tracing_subscriber::fmt::layer())
        .init();

    // Load environment variables
    dotenv::dotenv().ok();

    // Get database URL
    let database_url = std::env::var("DATABASE_URL")
        .expect("DATABASE_URL must be set in .env file");

    // Connect to database
    tracing::info!("Connecting to database...");
    let pool = sqlx::postgres::PgPoolOptions::new()
        .max_connections(10)
        .connect(&database_url)
        .await?;

    tracing::info!("Database connected successfully");

    // Initialize application state
    let state = AppState {
        pool: pool.clone(),
        last_ids: Arc::new(RwLock::new(HashMap::new())),
        clients: Arc::new(RwLock::new(Vec::new())),
    };

    // Start polling tasks for each entity
    polling::start_all_polling_tasks(state.clone()).await;

    // Build router
    let app = Router::new()
        .route("/ws", get(websocket::websocket_handler))
        .route("/api/entities", get(api::get_entities))
        .route("/api/stats", get(api::get_stats))
        .route("/api/health", get(api::health_check))
        .layer(CorsLayer::permissive())
        .with_state(state);

    // Start server
    let addr = std::env::var("HOST")
        .unwrap_or_else(|_| "0.0.0.0".to_string());
    let port = std::env::var("PORT")
        .unwrap_or_else(|_| "8080".to_string());
    let listener_addr = format!("{}:{}", addr, port);

    tracing::info!("Starting server on {}", listener_addr);
    let listener = tokio::net::TcpListener::bind(&listener_addr).await?;
    tracing::info!("Dashboard server listening on http://{}", listener_addr);
    tracing::info!("WebSocket endpoint: ws://{}/ws", listener_addr);
    tracing::info!("API endpoints: http://{}/api/*", listener_addr);

    axum::serve(listener, app).await?;

    Ok(())
}
