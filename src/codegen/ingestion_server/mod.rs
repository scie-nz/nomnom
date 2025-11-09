/// Axum-based ingestion server generation for HTTP message ingestion.

use crate::codegen::EntityDef;
use std::path::Path;
use std::error::Error;

mod cargo_toml;
mod main_rs;
mod handlers_rs;
mod parsers_rs;
mod models_rs;
mod database_rs;
mod error_rs;

pub use cargo_toml::generate_cargo_toml;
pub use main_rs::generate_main_rs;
pub use handlers_rs::generate_handlers_rs;
pub use parsers_rs::generate_parsers_rs;
pub use models_rs::generate_models_rs;
pub use database_rs::generate_database_rs;
pub use error_rs::generate_error_rs;

/// Database type enumeration
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum DatabaseType {
    PostgreSQL,
    MySQL,
    MariaDB,
}

impl DatabaseType {
    pub fn as_str(&self) -> &str {
        match self {
            DatabaseType::PostgreSQL => "postgresql",
            DatabaseType::MySQL => "mysql",
            DatabaseType::MariaDB => "mariadb",
        }
    }
}

#[derive(Debug, Clone)]
pub struct IngestionServerConfig {
    pub port: u16,
    pub database_type: DatabaseType,
    pub server_name: String,
}

impl Default for IngestionServerConfig {
    fn default() -> Self {
        Self {
            port: 8080,
            database_type: DatabaseType::PostgreSQL,
            server_name: "ingestion-server".to_string(),
        }
    }
}

/// Generate complete Axum ingestion server
pub fn generate_all(
    entities: &[EntityDef],
    output_dir: &Path,
    config: &IngestionServerConfig,
) -> Result<(), Box<dyn Error>> {
    // Create directory structure
    std::fs::create_dir_all(output_dir)?;
    std::fs::create_dir_all(output_dir.join("src"))?;

    println!("🚀 Generating Axum ingestion server...");
    println!("  📁 Output: {}", output_dir.display());
    println!("  🔌 Port: {}", config.port);
    println!("  🗄️  Database: {}", config.database_type.as_str());
    println!();

    // Generate Cargo.toml
    println!("  ✓ Generating Cargo.toml...");
    generate_cargo_toml(output_dir, config)?;

    // Generate source files
    println!("  ✓ Generating main.rs...");
    generate_main_rs(entities, output_dir, config)?;

    println!("  ✓ Generating parsers.rs...");
    generate_parsers_rs(entities, output_dir)?;

    println!("  ✓ Generating handlers.rs...");
    generate_handlers_rs(entities, output_dir)?;

    println!("  ✓ Generating models.rs...");
    generate_models_rs(output_dir)?;

    println!("  ✓ Generating database.rs...");
    generate_database_rs(output_dir, config)?;

    println!("  ✓ Generating error.rs...");
    generate_error_rs(output_dir)?;

    // Generate .env.example
    println!("  ✓ Generating .env.example...");
    generate_env_example(output_dir, config)?;

    println!();
    println!("✨ Ingestion server generated successfully!");
    println!();
    println!("📖 Next steps:");
    println!("  1. cd {}", output_dir.display());
    println!("  2. cp .env.example .env");
    println!("  3. Edit .env with your database credentials");
    println!("  4. cargo build --release");
    println!("  5. cargo run --release");
    println!();
    println!("🌐 Server will be available at:");
    println!("  API:     http://localhost:{}", config.port);
    println!("  Swagger: http://localhost:{}/swagger-ui", config.port);
    println!();

    Ok(())
}

fn generate_env_example(output_dir: &Path, config: &IngestionServerConfig) -> Result<(), Box<dyn Error>> {
    use std::io::Write;

    let env_file = output_dir.join(".env.example");
    let mut output = std::fs::File::create(&env_file)?;

    writeln!(output, "# Database connection")?;
    match config.database_type {
        DatabaseType::PostgreSQL => {
            writeln!(output, "DATABASE_URL=postgresql://user:password@localhost:5432/dbname")?;
        }
        DatabaseType::MySQL | DatabaseType::MariaDB => {
            writeln!(output, "DATABASE_URL=mysql://user:password@localhost:3306/dbname")?;
        }
    }
    writeln!(output)?;
    writeln!(output, "# Server configuration")?;
    writeln!(output, "PORT={}", config.port)?;
    writeln!(output, "HOST=0.0.0.0")?;
    writeln!(output)?;
    writeln!(output, "# Logging")?;
    writeln!(output, "RUST_LOG=info")?;

    Ok(())
}
