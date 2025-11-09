/// Generate main.rs with Axum server setup

use crate::codegen::EntityDef;
use super::IngestionServerConfig;
use std::path::Path;
use std::error::Error;
use std::io::Write;

pub fn generate_main_rs(
    entities: &[EntityDef],
    output_dir: &Path,
    config: &IngestionServerConfig,
) -> Result<(), Box<dyn Error>> {
    let main_file = output_dir.join("src/main.rs");
    let mut output = std::fs::File::create(&main_file)?;

    writeln!(output, "// Auto-generated Axum ingestion server")?;
    writeln!(output, "// Generated from entity definitions\n")?;

    writeln!(output, "use axum::{{")?;
    writeln!(output, "    routing::{{get, post}},")?;
    writeln!(output, "    Router,")?;
    writeln!(output, "}};")?;
    writeln!(output, "use std::net::SocketAddr;")?;
    writeln!(output, "use tower_http::cors::CorsLayer;")?;
    writeln!(output, "use utoipa::OpenApi;")?;
    writeln!(output, "use utoipa_swagger_ui::SwaggerUi;\n")?;

    writeln!(output, "mod handlers;")?;
    writeln!(output, "mod models;")?;
    writeln!(output, "mod parsers;")?;
    writeln!(output, "mod database;")?;
    writeln!(output, "mod error;\n")?;

    writeln!(output, "use database::create_pool;\n")?;

    // Generate OpenAPI spec
    writeln!(output, "#[derive(OpenApi)]")?;
    writeln!(output, "#[openapi(")?;
    writeln!(output, "    paths(")?;
    writeln!(output, "        handlers::ingest_message,")?;
    writeln!(output, "        handlers::ingest_batch,")?;
    writeln!(output, "        handlers::health_check,")?;
    writeln!(output, "    ),")?;
    writeln!(output, "    components(schemas(")?;
    writeln!(output, "        models::IngestResponse,")?;
    writeln!(output, "        models::BatchResponse,")?;
    writeln!(output, "        models::HealthResponse,")?;
    writeln!(output, "    ))")?;
    writeln!(output, ")]")?;
    writeln!(output, "struct ApiDoc;\n")?;

    // Main function
    writeln!(output, "#[tokio::main]")?;
    writeln!(output, "async fn main() {{")?;
    writeln!(output, "    // Initialize tracing")?;
    writeln!(output, "    tracing_subscriber::fmt::init();\n")?;

    writeln!(output, "    // Load environment variables")?;
    writeln!(output, "    dotenv::dotenv().ok();\n")?;

    writeln!(output, "    // Create database pool")?;
    writeln!(output, "    let pool = create_pool()")?;
    writeln!(output, "        .expect(\"Failed to create database pool\");\n")?;

    writeln!(output, "    // Build router")?;
    writeln!(output, "    let app = Router::new()")?;
    writeln!(output, "        // Ingestion endpoints")?;
    writeln!(output, "        .route(\"/ingest/message\", post(handlers::ingest_message))")?;
    writeln!(output, "        .route(\"/ingest/batch\", post(handlers::ingest_batch))")?;
    writeln!(output, "        // Utility endpoints")?;
    writeln!(output, "        .route(\"/health\", get(handlers::health_check))")?;
    writeln!(output, "        .route(\"/stats\", get(handlers::stats))")?;
    writeln!(output, "        // Swagger UI")?;
    writeln!(output, "        .merge(SwaggerUi::new(\"/swagger-ui\")")?;
    writeln!(output, "            .url(\"/api-docs/openapi.json\", ApiDoc::openapi()))")?;
    writeln!(output, "        // Middleware")?;
    writeln!(output, "        .layer(CorsLayer::permissive())")?;
    writeln!(output, "        .with_state(pool);\n")?;

    writeln!(output, "    // Run server")?;
    writeln!(output, "    let addr = SocketAddr::from(([0, 0, 0, 0], {}));", config.port)?;
    writeln!(output, "    tracing::info!(\"Ingestion server listening on {{}}\", addr);")?;
    writeln!(output, "    tracing::info!(\"Swagger UI available at http://localhost:{{}}/swagger-ui\", {});", config.port)?;
    writeln!(output)?;

    // Count entities for logging
    let entity_count = entities.iter()
        .filter(|e| e.is_persistent() && !e.is_abstract && e.source_type.to_lowercase() != "reference")
        .count();

    writeln!(output, "    tracing::info!(\"Ready to ingest messages for {} entities\");", entity_count)?;
    writeln!(output)?;

    writeln!(output, "    let listener = tokio::net::TcpListener::bind(&addr).await.unwrap();")?;
    writeln!(output, "    axum::serve(listener, app).await.unwrap();")?;
    writeln!(output, "}}")?;

    Ok(())
}
