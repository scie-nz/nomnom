/// Generate Cargo.toml for the ingestion server

use super::{IngestionServerConfig, DatabaseType};
use std::path::Path;
use std::error::Error;
use std::io::Write;

pub fn generate_cargo_toml(
    output_dir: &Path,
    config: &IngestionServerConfig,
) -> Result<(), Box<dyn Error>> {
    let cargo_file = output_dir.join("Cargo.toml");
    let mut output = std::fs::File::create(&cargo_file)?;

    writeln!(output, "[package]")?;
    writeln!(output, "name = \"{}\"", config.server_name)?;
    writeln!(output, "version = \"0.1.0\"")?;
    writeln!(output, "edition = \"2021\"\n")?;

    // Generate default feature based on database type
    writeln!(output, "[features]")?;
    match config.database_type {
        DatabaseType::PostgreSQL => {
            writeln!(output, "default = [\"postgres\"]")?;
            writeln!(output, "postgres = [\"diesel/postgres\"]")?;
            writeln!(output, "mysql = [\"diesel/mysql\"]\n")?;
        }
        DatabaseType::MySQL | DatabaseType::MariaDB => {
            writeln!(output, "default = [\"mysql\"]")?;
            writeln!(output, "postgres = [\"diesel/postgres\"]")?;
            writeln!(output, "mysql = [\"diesel/mysql\"]\n")?;
        }
    }

    writeln!(output, "[dependencies]")?;
    writeln!(output, "# Web framework")?;
    writeln!(output, "axum = \"0.7\"")?;
    writeln!(output, "tokio = {{ version = \"1\", features = [\"full\"] }}")?;
    writeln!(output, "tower = \"0.4\"")?;
    writeln!(output, "tower-http = {{ version = \"0.5\", features = [\"cors\", \"trace\"] }}\n")?;

    writeln!(output, "# Serialization")?;
    writeln!(output, "serde = {{ version = \"1\", features = [\"derive\"] }}")?;
    writeln!(output, "serde_json = \"1\"")?;
    writeln!(output, "base64 = \"0.21\"\n")?;

    writeln!(output, "# Database")?;
    writeln!(output, "diesel = {{ version = \"2\", features = [\"r2d2\", \"chrono\", \"numeric\", \"uuid\"] }}")?;
    writeln!(output, "r2d2 = \"0.8\"\n")?;

    writeln!(output, "# Date/Time and numbers")?;
    writeln!(output, "chrono = {{ version = \"0.4\", features = [\"serde\"] }}")?;
    writeln!(output, "rust_decimal = \"1.33\"")?;
    writeln!(output, "uuid = {{ version = \"1\", features = [\"v4\", \"serde\"] }}\n")?;

    writeln!(output, "# NATS JetStream (message queue)")?;
    writeln!(output, "async-nats = \"0.35\"\n")?;

    writeln!(output, "# OpenAPI documentation")?;
    writeln!(output, "utoipa = {{ version = \"4\", features = [\"axum_extras\", \"chrono\"] }}")?;
    writeln!(output, "utoipa-swagger-ui = {{ version = \"6\", features = [\"axum\"] }}\n")?;

    writeln!(output, "# Observability")?;
    writeln!(output, "tracing = \"0.1\"")?;
    writeln!(output, "tracing-subscriber = {{ version = \"0.3\", features = [\"env-filter\"] }}\n")?;

    writeln!(output, "# Environment")?;
    writeln!(output, "dotenv = \"0.15\"\n")?;

    writeln!(output, "[dev-dependencies]")?;
    writeln!(output, "reqwest = \"0.11\"")?;

    Ok(())
}
