/// Generate Cargo.toml for the worker binary

use super::{WorkerConfig, DatabaseType};
use std::path::Path;
use std::error::Error;
use std::io::Write;

pub fn generate_cargo_toml(
    output_dir: &Path,
    config: &WorkerConfig,
) -> Result<(), Box<dyn Error>> {
    let cargo_file = output_dir.join("Cargo.toml");
    let mut output = std::fs::File::create(&cargo_file)?;

    writeln!(output, "[package]")?;
    writeln!(output, "name = \"{}\"", config.worker_name)?;
    writeln!(output, "version = \"0.1.0\"")?;
    writeln!(output, "edition = \"2021\"\n")?;

    writeln!(output, "[dependencies]")?;
    writeln!(output, "# Async runtime")?;
    writeln!(output, "tokio = {{ version = \"1\", features = [\"full\"] }}")?;
    writeln!(output, "futures = \"0.3\"")?;
    writeln!(output)?;

    writeln!(output, "# Serialization")?;
    writeln!(output, "serde = {{ version = \"1\", features = [\"derive\"] }}")?;
    writeln!(output, "serde_json = \"1\"\n")?;

    writeln!(output, "# Database")?;
    match config.database_type {
        DatabaseType::PostgreSQL => {
            writeln!(output, "diesel = {{ version = \"2\", features = [\"postgres\", \"r2d2\", \"chrono\", \"numeric\", \"uuid\"] }}")?;
        }
        DatabaseType::MySQL | DatabaseType::MariaDB => {
            writeln!(output, "diesel = {{ version = \"2\", features = [\"mysql\", \"r2d2\", \"chrono\", \"numeric\", \"uuid\"] }}")?;
        }
    }
    writeln!(output, "r2d2 = \"0.8\"\n")?;

    writeln!(output, "# Date/Time and numbers")?;
    writeln!(output, "chrono = {{ version = \"0.4\", features = [\"serde\"] }}")?;
    writeln!(output, "rust_decimal = \"1.33\"")?;
    writeln!(output, "uuid = {{ version = \"1\", features = [\"v4\", \"serde\"] }}\n")?;

    writeln!(output, "# NATS JetStream (message queue)")?;
    writeln!(output, "async-nats = \"0.35\"\n")?;

    writeln!(output, "# Observability")?;
    writeln!(output, "tracing = \"0.1\"")?;
    writeln!(output, "tracing-subscriber = {{ version = \"0.3\", features = [\"env-filter\"] }}\n")?;

    writeln!(output, "# Environment")?;
    writeln!(output, "dotenv = \"0.15\"\n")?;

    writeln!(output, "# Transform utilities")?;
    writeln!(output, "regex = \"1\"")?;
    writeln!(output, "once_cell = \"1\"\n")?;

    Ok(())
}
