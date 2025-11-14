/// HTMX-based dashboard generation - server-side rendered alternative to React.
///
/// This module generates a lightweight dashboard using HTMX for dynamic updates,
/// eliminating the need for a separate frontend build process and WebSocket complexity.

use super::utils::{DatabaseType, generate_entity_display_config};
use crate::codegen::EntityDef;
use std::path::Path;
use std::error::Error;
use std::io::Write;

/// Generate complete HTMX dashboard
pub fn generate_htmx_dashboard(
    entities: &[EntityDef],
    output_dir: &Path,
    db_type: DatabaseType,
) -> Result<(), Box<dyn Error>> {
    println!("🎨 Generating HTMX dashboard...");

    // Create directory structure
    let src_dir = output_dir.join("src");
    let static_dir = output_dir.join("static");
    std::fs::create_dir_all(&src_dir)?;
    std::fs::create_dir_all(&static_dir)?;

    // Generate Rust code
    generate_cargo_toml(output_dir, db_type)?;
    generate_main_rs(entities, &src_dir, db_type)?;
    generate_config_rs(entities, &src_dir)?;
    generate_handlers_rs(&src_dir)?;
    generate_api_rs(entities, &src_dir, db_type)?;
    generate_templates_rs(entities, &src_dir)?;
    generate_env_example(output_dir, db_type)?;

    // Generate static assets
    generate_static_files(&static_dir)?;

    println!("✨ HTMX dashboard generated successfully!");
    println!("   📁 Output: {}", output_dir.display());
    println!("   🚀 To start:");
    println!("      cd {} && cargo build --release", output_dir.display());
    println!("      ./target/release/dashboard");

    Ok(())
}

/// Generate Cargo.toml with HTMX dashboard dependencies
fn generate_cargo_toml(
    output_dir: &Path,
    db_type: DatabaseType,
) -> Result<(), Box<dyn Error>> {
    let cargo_file = output_dir.join("Cargo.toml");
    let mut output = std::fs::File::create(&cargo_file)?;

    let db_feature = match db_type {
        DatabaseType::PostgreSQL => "postgres",
        DatabaseType::MySQL | DatabaseType::MariaDB => "mysql",
    };

    writeln!(output, "# Auto-generated Cargo.toml for HTMX dashboard")?;
    writeln!(output, "[package]")?;
    writeln!(output, "name = \"dashboard\"")?;
    writeln!(output, "version = \"0.1.0\"")?;
    writeln!(output, "edition = \"2021\"\n")?;

    writeln!(output, "[dependencies]")?;
    writeln!(output, "# Web framework")?;
    writeln!(output, "axum = {{ version = \"0.7\", features = [\"macros\"] }}")?;
    writeln!(output, "tokio = {{ version = \"1\", features = [\"full\"] }}")?;
    writeln!(output, "tower = \"0.4\"")?;
    writeln!(output, "tower-http = {{ version = \"0.5\", features = [\"cors\", \"fs\"] }}\n")?;

    writeln!(output, "# Database")?;
    writeln!(output, "sqlx = {{ version = \"0.7\", features = [\"runtime-tokio-rustls\", \"{}\", \"json\", \"chrono\"] }}\n", db_feature)?;

    writeln!(output, "# Templating")?;
    writeln!(output, "tera = \"1.19\"\n")?;

    writeln!(output, "# Serialization")?;
    writeln!(output, "serde = {{ version = \"1.0\", features = [\"derive\"] }}")?;
    writeln!(output, "serde_json = \"1.0\"\n")?;

    writeln!(output, "# Utilities")?;
    writeln!(output, "chrono = {{ version = \"0.4\", features = [\"serde\"] }}")?;
    writeln!(output, "tracing = \"0.1\"")?;
    writeln!(output, "tracing-subscriber = {{ version = \"0.3\", features = [\"env-filter\"] }}")?;
    writeln!(output, "dotenv = \"0.15\"")?;

    Ok(())
}

/// Generate main.rs with Axum server and template setup
fn generate_main_rs(
    _entities: &[EntityDef],
    src_dir: &Path,
    db_type: DatabaseType,
) -> Result<(), Box<dyn Error>> {
    let main_file = src_dir.join("main.rs");
    let mut output = std::fs::File::create(&main_file)?;

    let db_pool = match db_type {
        DatabaseType::PostgreSQL => "sqlx::postgres::PgPoolOptions",
        DatabaseType::MySQL | DatabaseType::MariaDB => "sqlx::mysql::MySqlPoolOptions",
    };

    let pool_type = match db_type {
        DatabaseType::PostgreSQL => "sqlx::Pool<sqlx::Postgres>",
        DatabaseType::MySQL | DatabaseType::MariaDB => "sqlx::Pool<sqlx::MySql>",
    };

    writeln!(output, "// Auto-generated HTMX dashboard backend\n")?;
    writeln!(output, "mod config;")?;
    writeln!(output, "mod handlers;")?;
    writeln!(output, "mod api;")?;
    writeln!(output, "mod templates;\n")?;

    writeln!(output, "use axum::{{")?;
    writeln!(output, "    Router,")?;
    writeln!(output, "    routing::get,")?;
    writeln!(output, "}};")?;
    writeln!(output, "use tower_http::{{")?;
    writeln!(output, "    services::ServeDir,")?;
    writeln!(output, "    cors::CorsLayer,")?;
    writeln!(output, "}};")?;
    writeln!(output, "use tera::Tera;")?;
    writeln!(output, "use tracing_subscriber::{{layer::SubscriberExt, util::SubscriberInitExt}};\n")?;

    writeln!(output, "/// Application state")?;
    writeln!(output, "#[derive(Clone)]")?;
    writeln!(output, "pub struct AppState {{")?;
    writeln!(output, "    pub pool: {},", pool_type)?;
    writeln!(output, "    pub templates: Tera,")?;
    writeln!(output, "}}\n")?;

    writeln!(output, "#[tokio::main]")?;
    writeln!(output, "async fn main() -> Result<(), Box<dyn std::error::Error>> {{")?;
    writeln!(output, "    // Initialize tracing")?;
    writeln!(output, "    tracing_subscriber::registry()")?;
    writeln!(output, "        .with(tracing_subscriber::EnvFilter::try_from_default_env()")?;
    writeln!(output, "            .unwrap_or_else(|_| \"info\".into()))")?;
    writeln!(output, "        .with(tracing_subscriber::fmt::layer())")?;
    writeln!(output, "        .init();\n")?;

    writeln!(output, "    // Load environment")?;
    writeln!(output, "    dotenv::dotenv().ok();\n")?;

    writeln!(output, "    // Connect to database")?;
    writeln!(output, "    let database_url = std::env::var(\"DATABASE_URL\")")?;
    writeln!(output, "        .expect(\"DATABASE_URL must be set\");")?;
    writeln!(output, "    tracing::info!(\"Connecting to database...\");")?;
    writeln!(output, "    let pool = {}::new()", db_pool)?;
    writeln!(output, "        .max_connections(10)")?;
    writeln!(output, "        .connect(&database_url)")?;
    writeln!(output, "        .await?;")?;
    writeln!(output, "    tracing::info!(\"Database connected\");\n")?;

    writeln!(output, "    // Load templates")?;
    writeln!(output, "    let templates = load_templates()?;\n")?;

    writeln!(output, "    let state = AppState {{")?;
    writeln!(output, "        pool,")?;
    writeln!(output, "        templates,")?;
    writeln!(output, "    }};\n")?;

    writeln!(output, "    // Build router")?;
    writeln!(output, "    let app = Router::new()")?;
    writeln!(output, "        .route(\"/\", get(handlers::index))")?;
    writeln!(output, "        .route(\"/api/entity/:name/recent\", get(api::entity_recent))")?;
    writeln!(output, "        .route(\"/api/stats\", get(api::stats))")?;
    writeln!(output, "        .route(\"/health\", get(api::health_check))")?;
    writeln!(output, "        .nest_service(\"/static\", ServeDir::new(\"static\"))")?;
    writeln!(output, "        .layer(CorsLayer::permissive())")?;
    writeln!(output, "        .with_state(state);\n")?;

    writeln!(output, "    // Start server")?;
    writeln!(output, "    let addr = std::env::var(\"HOST\").unwrap_or_else(|_| \"0.0.0.0\".to_string());")?;
    writeln!(output, "    let port = std::env::var(\"PORT\").unwrap_or_else(|_| \"8080\".to_string());")?;
    writeln!(output, "    let listener_addr = format!(\"{{}}:{{}}\", addr, port);\n")?;

    writeln!(output, "    tracing::info!(\"Starting HTMX dashboard on http://{{}}\", listener_addr);")?;
    writeln!(output, "    let listener = tokio::net::TcpListener::bind(&listener_addr).await?;")?;
    writeln!(output, "    axum::serve(listener, app).await?;\n")?;

    writeln!(output, "    Ok(())")?;
    writeln!(output, "}}\n")?;

    writeln!(output, "fn load_templates() -> Result<Tera, Box<dyn std::error::Error>> {{")?;
    writeln!(output, "    let mut tera = Tera::default();")?;
    writeln!(output, "    tera.add_raw_template(\"base.html\", templates::BASE)?;")?;
    writeln!(output, "    tera.add_raw_template(\"index.html\", templates::INDEX)?;")?;
    writeln!(output, "    tera.add_raw_template(\"partials/entity_recent.html\", templates::ENTITY_RECENT)?;")?;
    writeln!(output, "    tera.add_raw_template(\"partials/stats.html\", templates::STATS)?;")?;
    writeln!(output, "    Ok(tera)")?;
    writeln!(output, "}}")?;

    Ok(())
}

/// Generate config.rs (reuse from existing axum_backend)
fn generate_config_rs(
    entities: &[EntityDef],
    src_dir: &Path,
) -> Result<(), Box<dyn Error>> {
    let config_file = src_dir.join("config.rs");
    let mut output = std::fs::File::create(&config_file)?;

    writeln!(output, "// Auto-generated entity configuration\n")?;

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

    writeln!(output, "pub const ENTITIES: &[EntityConfig] = &[")?;

    for entity in entities {
        if !entity.is_persistent() || entity.is_abstract {
            continue;
        }
        // Include all persistent entities, even reference data
        // (reference entities are still useful to monitor in dashboards)

        let display_config = generate_entity_display_config(entity);

        let primary_key = if let Some(ref persistence) = entity.persistence {
            if let Some(ref pk_config) = persistence.primary_key {
                pk_config.name.to_lowercase()
            } else if let Some(ref db_config) = persistence.database {
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

/// Generate handlers.rs for HTML page rendering
fn generate_handlers_rs(src_dir: &Path) -> Result<(), Box<dyn Error>> {
    let handlers_file = src_dir.join("handlers.rs");
    let mut output = std::fs::File::create(&handlers_file)?;

    writeln!(output, "// Auto-generated HTML page handlers\n")?;
    writeln!(output, "use crate::{{AppState, config}};")?;
    writeln!(output, "use axum::{{")?;
    writeln!(output, "    extract::State,")?;
    writeln!(output, "    response::{{Html, IntoResponse, Response}},")?;
    writeln!(output, "    http::StatusCode,")?;
    writeln!(output, "}};")?;
    writeln!(output, "use tera::Context;\n")?;

    writeln!(output, "#[derive(Debug)]")?;
    writeln!(output, "pub enum AppError {{")?;
    writeln!(output, "    Template(tera::Error),")?;
    writeln!(output, "}}\n")?;

    writeln!(output, "impl IntoResponse for AppError {{")?;
    writeln!(output, "    fn into_response(self) -> Response {{")?;
    writeln!(output, "        let body = match self {{")?;
    writeln!(output, "            AppError::Template(e) => format!(\"Template error: {{}}\", e),")?;
    writeln!(output, "        }};")?;
    writeln!(output, "        (StatusCode::INTERNAL_SERVER_ERROR, body).into_response()")?;
    writeln!(output, "    }}")?;
    writeln!(output, "}}\n")?;

    writeln!(output, "impl From<tera::Error> for AppError {{")?;
    writeln!(output, "    fn from(e: tera::Error) -> Self {{")?;
    writeln!(output, "        AppError::Template(e)")?;
    writeln!(output, "    }}")?;
    writeln!(output, "}}\n")?;

    writeln!(output, "/// Render main dashboard page")?;
    writeln!(output, "pub async fn index(State(state): State<AppState>) -> Result<Html<String>, AppError> {{")?;
    writeln!(output, "    let mut context = Context::new();")?;
    writeln!(output, "    context.insert(\"title\", \"Real-Time Dashboard\");")?;
    writeln!(output, "    \n")?;
    writeln!(output, "    // Convert entities to serializable format")?;
    writeln!(output, "    let entities: Vec<serde_json::Value> = config::ENTITIES.iter()")?;
    writeln!(output, "        .map(|e| serde_json::json!({{")?;
    writeln!(output, "            \"name\": e.name,")?;
    writeln!(output, "            \"color\": e.color,")?;
    writeln!(output, "            \"icon\": e.icon,")?;
    writeln!(output, "        }}))")?;
    writeln!(output, "        .collect();")?;
    writeln!(output, "    context.insert(\"entities\", &entities);\n")?;
    writeln!(output, "    let html = state.templates.render(\"index.html\", &context)?;")?;
    writeln!(output, "    Ok(Html(html))")?;
    writeln!(output, "}}")?;

    Ok(())
}

/// Generate api.rs for HTMX partial endpoints
fn generate_api_rs(
    entities: &[EntityDef],
    src_dir: &Path,
    db_type: DatabaseType,
) -> Result<(), Box<dyn Error>> {
    let api_file = src_dir.join("api.rs");
    let mut output = std::fs::File::create(&api_file)?;

    writeln!(output, "// Auto-generated HTMX partial endpoints\n")?;
    writeln!(output, "use crate::{{AppState, config, handlers::AppError}};")?;
    writeln!(output, "use axum::{{")?;
    writeln!(output, "    extract::{{State, Path}},")?;
    writeln!(output, "    response::{{Html, Json}},")?;
    writeln!(output, "}};")?;
    writeln!(output, "use tera::Context;")?;
    writeln!(output, "use serde_json::{{json, Value}};")?;
    writeln!(output, "use sqlx::{{Row, Column}};\n")?;

    // Generate database-specific row conversion helper
    match db_type {
        DatabaseType::PostgreSQL => {
            writeln!(output, "/// Convert database row to JSON")?;
            writeln!(output, "fn row_to_json(row: &sqlx::postgres::PgRow) -> Value {{")?;
            writeln!(output, "    let mut map = serde_json::Map::new();\n")?;
            writeln!(output, "    for column in row.columns() {{")?;
            writeln!(output, "        let name = column.name();")?;
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
            writeln!(output, "        }} else {{")?;
            writeln!(output, "            json!(null)")?;
            writeln!(output, "        }};")?;
            writeln!(output, "        map.insert(name.to_string(), value);")?;
            writeln!(output, "    }}")?;
            writeln!(output, "    Value::Object(map)")?;
            writeln!(output, "}}\n")?;
        }
        DatabaseType::MySQL | DatabaseType::MariaDB => {
            writeln!(output, "/// Convert database row to JSON")?;
            writeln!(output, "fn row_to_json(row: &sqlx::mysql::MySqlRow) -> Value {{")?;
            writeln!(output, "    let mut map = serde_json::Map::new();\n")?;
            writeln!(output, "    for column in row.columns() {{")?;
            writeln!(output, "        let name = column.name();")?;
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
            writeln!(output, "        }} else {{")?;
            writeln!(output, "            json!(null)")?;
            writeln!(output, "        }};")?;
            writeln!(output, "        map.insert(name.to_string(), value);")?;
            writeln!(output, "    }}")?;
            writeln!(output, "    Value::Object(map)")?;
            writeln!(output, "}}\n")?;
        }
    }

    // Entity recent records endpoint
    writeln!(output, "/// Get recent records for an entity (HTMX partial)")?;
    writeln!(output, "pub async fn entity_recent(")?;
    writeln!(output, "    State(state): State<AppState>,")?;
    writeln!(output, "    Path(entity_name): Path<String>,")?;
    writeln!(output, ") -> Result<Html<String>, AppError> {{")?;
    writeln!(output, "    // Find entity")?;
    writeln!(output, "    let entity = config::ENTITIES.iter()")?;
    writeln!(output, "        .find(|e| e.name == entity_name)")?;
    writeln!(output, "        .ok_or_else(|| AppError::Template(tera::Error::msg(\"Entity not found\")))?;\n")?;

    writeln!(output, "    // Query recent records")?;
    writeln!(output, "    let query = format!(")?;
    writeln!(output, "        \"SELECT * FROM {{}} ORDER BY {{}} DESC LIMIT {{}}\",")?;
    writeln!(output, "        entity.table, entity.primary_key, entity.max_records")?;
    writeln!(output, "    );\n")?;

    writeln!(output, "    let rows = sqlx::query(&query)")?;
    writeln!(output, "        .fetch_all(&state.pool)")?;
    writeln!(output, "        .await")?;
    writeln!(output, "        .map_err(|e| AppError::Template(tera::Error::msg(format!(\"Database error: {{}}\", e))))?;\n")?;

    writeln!(output, "    let records: Vec<Value> = rows.iter().map(row_to_json).collect();\n")?;

    writeln!(output, "    // Get total count")?;
    writeln!(output, "    let count_query = format!(\"SELECT COUNT(*) as count FROM {{}}\", entity.table);")?;
    writeln!(output, "    let total: i64 = sqlx::query_scalar(&count_query)")?;
    writeln!(output, "        .fetch_one(&state.pool)")?;
    writeln!(output, "        .await")?;
    writeln!(output, "        .unwrap_or(0);\n")?;

    writeln!(output, "    // Render template")?;
    writeln!(output, "    let mut context = Context::new();")?;
    writeln!(output, "    context.insert(\"entity\", &serde_json::json!({{")?;
    writeln!(output, "        \"name\": entity.name,")?;
    writeln!(output, "        \"fields\": entity.fields,")?;
    writeln!(output, "    }}));")?;
    writeln!(output, "    context.insert(\"records\", &records);")?;
    writeln!(output, "    context.insert(\"total\", &total);\n")?;

    writeln!(output, "    let html = state.templates.render(\"partials/entity_recent.html\", &context)?;")?;
    writeln!(output, "    Ok(Html(html))")?;
    writeln!(output, "}}\n")?;

    // Stats endpoint
    writeln!(output, "/// Get stats for all entities (HTMX partial)")?;
    writeln!(output, "pub async fn stats(State(state): State<AppState>) -> Result<Html<String>, AppError> {{")?;
    writeln!(output, "    let mut stats = std::collections::HashMap::new();\n")?;
    writeln!(output, "    for entity in config::ENTITIES {{")?;
    writeln!(output, "        let query = format!(\"SELECT COUNT(*) as count FROM {{}}\", entity.table);")?;
    writeln!(output, "        let count: i64 = sqlx::query_scalar(&query)")?;
    writeln!(output, "            .fetch_one(&state.pool)")?;
    writeln!(output, "            .await")?;
    writeln!(output, "            .unwrap_or(0);")?;
    writeln!(output, "        stats.insert(entity.name, json!({{\"total\": count}}));")?;
    writeln!(output, "    }}\n")?;
    writeln!(output, "    let mut context = Context::new();")?;
    writeln!(output, "    let entities: Vec<Value> = config::ENTITIES.iter()")?;
    writeln!(output, "        .map(|e| json!({{")?;
    writeln!(output, "            \"name\": e.name,")?;
    writeln!(output, "            \"color\": e.color,")?;
    writeln!(output, "        }}))")?;
    writeln!(output, "        .collect();")?;
    writeln!(output, "    context.insert(\"entities\", &entities);")?;
    writeln!(output, "    context.insert(\"stats\", &stats);\n")?;
    writeln!(output, "    let html = state.templates.render(\"partials/stats.html\", &context)?;")?;
    writeln!(output, "    Ok(Html(html))")?;
    writeln!(output, "}}\n")?;

    // Health check
    writeln!(output, "/// Health check endpoint")?;
    writeln!(output, "pub async fn health_check() -> Json<Value> {{")?;
    writeln!(output, "    Json(json!({{\"status\": \"ok\"}}))")?;
    writeln!(output, "}}")?;

    Ok(())
}

/// Generate templates.rs with embedded HTML templates
fn generate_templates_rs(
    entities: &[EntityDef],
    src_dir: &Path,
) -> Result<(), Box<dyn Error>> {
    let templates_file = src_dir.join("templates.rs");
    let mut output = std::fs::File::create(&templates_file)?;

    writeln!(output, "// Auto-generated HTML templates\n")?;

    // Base template
    writeln!(output, "pub const BASE: &str = r#\"")?;
    writeln!(output, "<!DOCTYPE html>")?;
    writeln!(output, "<html lang=\"en\">")?;
    writeln!(output, "<head>")?;
    writeln!(output, "    <meta charset=\"UTF-8\">")?;
    writeln!(output, "    <meta name=\"viewport\" content=\"width=device-width, initial-scale=1.0\">")?;
    writeln!(output, "    <title>{{{{ title }}}}</title>")?;
    writeln!(output, "    <script src=\"/static/htmx.min.js\"></script>")?;
    writeln!(output, "    <script src=\"https://cdn.tailwindcss.com\"></script>")?;
    writeln!(output, "    <link rel=\"stylesheet\" href=\"/static/styles.css\">")?;
    writeln!(output, "</head>")?;
    writeln!(output, "<body class=\"bg-gray-900 text-gray-100 min-h-screen\">")?;
    writeln!(output, "    <header class=\"bg-gray-800 border-b border-gray-700 px-6 py-4\">")?;
    writeln!(output, "        <h1 class=\"text-2xl font-bold mb-2\">{{{{ title }}}}</h1>")?;
    writeln!(output, "        <div id=\"stats-bar\" hx-get=\"/api/stats\" hx-trigger=\"every 5s\" hx-swap=\"innerHTML\">")?;
    writeln!(output, "            <div class=\"text-sm text-gray-400\">Loading stats...</div>")?;
    writeln!(output, "        </div>")?;
    writeln!(output, "    </header>")?;
    writeln!(output, "    <main class=\"container mx-auto px-6 py-8\">")?;
    writeln!(output, "        {{% block content %}}{{% endblock %}}")?;
    writeln!(output, "    </main>")?;
    writeln!(output, "</body>")?;
    writeln!(output, "</html>")?;
    writeln!(output, "\"#;\n")?;

    // Index template
    writeln!(output, "pub const INDEX: &str = r#\"")?;
    writeln!(output, "{{% extends \"base.html\" %}}")?;
    writeln!(output, "{{% block content %}}")?;
    writeln!(output, "<div class=\"grid grid-cols-1 md:grid-cols-2 lg:grid-cols-3 gap-6\">")?;
    writeln!(output, "    {{% for entity in entities %}}")?;
    writeln!(output, "    <div class=\"bg-gray-800 rounded-lg shadow-lg p-6 border border-gray-700\">")?;
    writeln!(output, "        <div class=\"flex items-center justify-between mb-4\">")?;
    writeln!(output, "            <h2 class=\"text-xl font-semibold\">{{{{ entity.name }}}}</h2>")?;
    writeln!(output, "            <span class=\"text-2xl\">{{{{ entity.icon }}}}</span>")?;
    writeln!(output, "        </div>")?;
    writeln!(output, "        <div")?;
    writeln!(output, "            hx-get=\"/api/entity/{{{{ entity.name }}}}/recent\"")?;
    writeln!(output, "            hx-trigger=\"load, every 2s\"")?;
    writeln!(output, "            hx-swap=\"innerHTML\"")?;
    writeln!(output, "            class=\"space-y-2\">")?;
    writeln!(output, "            <div class=\"text-sm text-gray-400\">Loading...</div>")?;
    writeln!(output, "        </div>")?;
    writeln!(output, "    </div>")?;
    writeln!(output, "    {{% endfor %}}")?;
    writeln!(output, "</div>")?;
    writeln!(output, "{{% endblock %}}")?;
    writeln!(output, "\"#;\n")?;

    // Entity recent partial
    writeln!(output, "pub const ENTITY_RECENT: &str = r#\"")?;
    writeln!(output, "<div class=\"text-sm text-gray-400 mb-2\">")?;
    writeln!(output, "    Total: {{{{ total }}}} | Showing {{{{ records | length }}}} most recent")?;
    writeln!(output, "</div>")?;
    writeln!(output, "{{% if records | length == 0 %}}")?;
    writeln!(output, "<div class=\"text-gray-500 italic\">No records yet</div>")?;
    writeln!(output, "{{% else %}}")?;
    writeln!(output, "<div class=\"space-y-1 max-h-96 overflow-y-auto\">")?;
    writeln!(output, "    {{% for record in records %}}")?;
    writeln!(output, "    <div class=\"bg-gray-700 rounded px-3 py-2 text-xs animate-fade-in\">")?;
    writeln!(output, "        {{% for field in entity.fields %}}")?;
    writeln!(output, "        <div class=\"flex justify-between gap-2\">")?;
    writeln!(output, "            <span class=\"text-gray-400\">{{{{ field }}}}:</span>")?;
    writeln!(output, "            <span class=\"font-mono text-right truncate\">{{{{ record[field] }}}}</span>")?;
    writeln!(output, "        </div>")?;
    writeln!(output, "        {{% endfor %}}")?;
    writeln!(output, "    </div>")?;
    writeln!(output, "    {{% endfor %}}")?;
    writeln!(output, "</div>")?;
    writeln!(output, "{{% endif %}}")?;
    writeln!(output, "\"#;\n")?;

    // Stats partial
    writeln!(output, "pub const STATS: &str = r#\"")?;
    writeln!(output, "<div class=\"flex flex-wrap gap-4 text-sm\">")?;
    writeln!(output, "    {{% for entity in entities %}}")?;
    writeln!(output, "    <div class=\"flex items-center gap-2\">")?;
    writeln!(output, "        <span class=\"text-gray-400\">{{{{ entity.name }}}}:</span>")?;
    writeln!(output, "        <span class=\"font-bold text-{{{{ entity.color }}}}-400\">{{{{ stats[entity.name].total }}}}</span>")?;
    writeln!(output, "    </div>")?;
    writeln!(output, "    {{% endfor %}}")?;
    writeln!(output, "</div>")?;
    writeln!(output, "\"#;")?;

    Ok(())
}

/// Generate static files (htmx.js and CSS)
fn generate_static_files(static_dir: &Path) -> Result<(), Box<dyn Error>> {
    // HTMX library (minified v1.9.10)
    let htmx_js = include_str!("../../../assets/htmx.min.js");
    std::fs::write(static_dir.join("htmx.min.js"), htmx_js)?;

    // Minimal CSS for animations
    let css = r#"
@keyframes fade-in {
    from {
        opacity: 0;
        transform: translateY(-10px);
    }
    to {
        opacity: 1;
        transform: translateY(0);
    }
}

.animate-fade-in {
    animation: fade-in 0.3s ease-out;
}

/* Scrollbar styling */
::-webkit-scrollbar {
    width: 8px;
}

::-webkit-scrollbar-track {
    background: #1f2937;
}

::-webkit-scrollbar-thumb {
    background: #4b5563;
    border-radius: 4px;
}

::-webkit-scrollbar-thumb:hover {
    background: #6b7280;
}
"#;
    std::fs::write(static_dir.join("styles.css"), css)?;

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
