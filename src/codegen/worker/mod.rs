/// Worker binary generation for NATS JetStream message processing
///
/// Generates an entity-specific worker binary that:
/// - Consumes messages from NATS JetStream
/// - Parses message bodies using entity-specific parsers
/// - Writes to database using entity-specific models
/// - ACKs successful processing, NAKs failures

use crate::codegen::EntityDef;
use std::path::Path;
use std::error::Error;

mod cargo_toml;
mod main_rs;
mod parsers_rs;
mod models_rs;
mod database_rs;
mod error_rs;

pub use cargo_toml::generate_cargo_toml;
pub use main_rs::generate_main_rs;
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
pub struct WorkerConfig {
    pub database_type: DatabaseType,
    pub worker_name: String,
}

impl Default for WorkerConfig {
    fn default() -> Self {
        Self {
            database_type: DatabaseType::PostgreSQL,
            worker_name: "worker".to_string(),
        }
    }
}

/// Generate complete NATS worker binary
pub fn generate_all(
    entities: &[EntityDef],
    output_dir: &Path,
    config: &WorkerConfig,
) -> Result<(), Box<dyn Error>> {
    // Create directory structure
    std::fs::create_dir_all(output_dir)?;
    std::fs::create_dir_all(output_dir.join("src"))?;

    println!("🚀 Generating NATS worker binary...");
    println!("  📁 Output: {}", output_dir.display());
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

    println!("  ✓ Generating models.rs...");
    generate_models_rs(output_dir)?;

    println!("  ✓ Generating database.rs...");
    generate_database_rs(entities, output_dir, config)?;

    println!("  ✓ Generating error.rs...");
    generate_error_rs(output_dir)?;

    // Generate .env.example
    println!("  ✓ Generating .env.example...");
    generate_env_example(output_dir, config)?;

    // Generate Dockerfile
    println!("  ✓ Generating Dockerfile...");
    generate_dockerfile(output_dir)?;

    println!();
    println!("✨ Worker binary generated successfully!");
    println!();
    println!("📖 Next steps:");
    println!("  1. cd {}", output_dir.display());
    println!("  2. cp .env.example .env");
    println!("  3. Edit .env with your database and NATS credentials");
    println!("  4. cargo build --release");
    println!("  5. cargo run --release");
    println!();
    println!("🐳 Or run with Docker:");
    println!("  docker build -t worker .");
    println!("  docker run --env-file .env worker");
    println!();

    Ok(())
}

fn generate_env_example(output_dir: &Path, config: &WorkerConfig) -> Result<(), Box<dyn Error>> {
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
    writeln!(output, "# NATS Configuration")?;
    writeln!(output, "NATS_URL=nats://localhost:4222")?;
    writeln!(output, "NATS_STREAM=MESSAGES")?;
    writeln!(output, "NATS_CONSUMER=workers")?;
    writeln!(output)?;
    writeln!(output, "# Logging")?;
    writeln!(output, "RUST_LOG=info")?;

    Ok(())
}

fn generate_dockerfile(output_dir: &Path) -> Result<(), Box<dyn Error>> {
    use std::io::Write;

    let dockerfile_path = output_dir.join("Dockerfile");
    let mut file = std::fs::File::create(&dockerfile_path)?;

    writeln!(file, "# Multi-stage Dockerfile with cargo-chef for dependency caching")?;
    writeln!(file)?;
    writeln!(file, "# Stage 1: Chef - prepare recipe for dependencies")?;
    writeln!(file, "FROM rust:alpine as chef")?;
    writeln!(file, "RUN apk add --no-cache musl-dev pkgconfig openssl-dev postgresql-dev")?;
    writeln!(file, "RUN cargo install cargo-chef")?;
    writeln!(file, "WORKDIR /build")?;
    writeln!(file)?;
    writeln!(file, "# Stage 2: Planner - analyze dependencies")?;
    writeln!(file, "FROM chef as planner")?;
    writeln!(file, "COPY Cargo.toml Cargo.lock* ./")?;
    writeln!(file, "COPY src ./src")?;
    writeln!(file, "RUN cargo chef prepare --recipe-path recipe.json")?;
    writeln!(file)?;
    writeln!(file, "# Stage 3: Builder - build dependencies (CACHED!) then app")?;
    writeln!(file, "FROM chef as builder")?;
    writeln!(file)?;
    writeln!(file, "# Use dynamic linking for PostgreSQL")?;
    writeln!(file, "ENV RUSTFLAGS=\"-C target-feature=-crt-static\"")?;
    writeln!(file)?;
    writeln!(file, "# Build dependencies - this layer is cached until Cargo.toml changes")?;
    writeln!(file, "COPY --from=planner /build/recipe.json recipe.json")?;
    writeln!(file, "RUN cargo chef cook --release --recipe-path recipe.json")?;
    writeln!(file)?;
    writeln!(file, "# Build application - only this runs when source code changes")?;
    writeln!(file, "COPY Cargo.toml Cargo.lock* ./")?;
    writeln!(file, "COPY src ./src")?;
    writeln!(file, "RUN cargo build --release")?;
    writeln!(file)?;
    writeln!(file, "# Stage 4: Runtime - minimal Alpine")?;
    writeln!(file, "FROM alpine:3.19")?;
    writeln!(file)?;
    writeln!(file, "# Install runtime dependencies")?;
    writeln!(file, "RUN apk add --no-cache \\")?;
    writeln!(file, "    ca-certificates \\")?;
    writeln!(file, "    libgcc \\")?;
    writeln!(file, "    postgresql-libs")?;
    writeln!(file)?;
    writeln!(file, "# Create non-root user")?;
    writeln!(file, "RUN addgroup -g 1000 appuser && \\")?;
    writeln!(file, "    adduser -D -u 1000 -G appuser appuser")?;
    writeln!(file)?;
    writeln!(file, "WORKDIR /app")?;
    writeln!(file)?;
    writeln!(file, "# Copy binary from builder")?;
    writeln!(file, "COPY --from=builder --chown=appuser:appuser /build/target/release/worker /app/")?;
    writeln!(file)?;
    writeln!(file, "# Switch to non-root user")?;
    writeln!(file, "USER appuser:appuser")?;
    writeln!(file)?;
    writeln!(file, "CMD [\"/app/worker\"]")?;

    Ok(())
}
