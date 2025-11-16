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
mod message_envelope_rs;
mod nats_client_rs;

pub use cargo_toml::generate_cargo_toml;
pub use main_rs::generate_main_rs;
pub use handlers_rs::generate_handlers_rs;
pub use parsers_rs::generate_parsers_rs;
pub use models_rs::generate_models_rs;
pub use database_rs::generate_database_rs;
pub use error_rs::generate_error_rs;
pub use message_envelope_rs::generate_message_envelope_rs;
pub use nats_client_rs::generate_nats_client_rs;

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

    /// Check if this is MySQL or MariaDB (similar syntax)
    pub fn is_mysql_like(&self) -> bool {
        matches!(self, DatabaseType::MySQL | DatabaseType::MariaDB)
    }

    /// Detect database type from DATABASE_URL
    ///
    /// # Examples
    /// ```no_run
    /// use nomnom::codegen::ingestion_server::DatabaseType;
    ///
    /// let db_type = DatabaseType::from_url("postgres://localhost/mydb");
    /// assert_eq!(db_type, DatabaseType::PostgreSQL);
    ///
    /// let db_type = DatabaseType::from_url("mysql://localhost/mydb");
    /// assert_eq!(db_type, DatabaseType::MySQL);
    /// ```
    pub fn from_url(url: &str) -> DatabaseType {
        if url.starts_with("postgres://") || url.starts_with("postgresql://") {
            DatabaseType::PostgreSQL
        } else if url.starts_with("mysql://") {
            DatabaseType::MySQL
        } else {
            // Default to PostgreSQL for backward compatibility
            DatabaseType::PostgreSQL
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

    println!("  ✓ Generating message_envelope.rs...");
    generate_message_envelope_rs(output_dir)?;

    println!("  ✓ Generating nats_client.rs...");
    generate_nats_client_rs(output_dir)?;

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

    // Generate Dockerfile
    println!("  ✓ Generating Dockerfile...");
    generate_dockerfile(output_dir)?;

    // Generate docker-compose.nats.yml
    println!("  ✓ Generating docker-compose.nats.yml...");
    generate_docker_compose_nats(output_dir, config)?;

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
    println!("🐳 Or run with Docker:");
    println!("  docker build -t ingestion-api .");
    println!("  docker run -p {}:{} --env-file .env ingestion-api", config.port, config.port);
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
    writeln!(output, "# NATS Configuration")?;
    writeln!(output, "NATS_URL=nats://localhost:4222")?;
    writeln!(output, "NATS_STREAM=MESSAGES")?;
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
    writeln!(file, "COPY --from=builder --chown=appuser:appuser /build/target/release/ingestion-server /app/")?;
    writeln!(file)?;
    writeln!(file, "# Switch to non-root user")?;
    writeln!(file, "USER appuser:appuser")?;
    writeln!(file)?;
    writeln!(file, "# Expose default port")?;
    writeln!(file, "EXPOSE 8080")?;
    writeln!(file)?;
    writeln!(file, "CMD [\"/app/ingestion-server\"]")?;

    Ok(())
}

fn generate_docker_compose_nats(output_dir: &Path, config: &IngestionServerConfig) -> Result<(), Box<dyn Error>> {
    use std::io::Write;

    let compose_file = output_dir.join("docker-compose.nats.yml");
    let mut output = std::fs::File::create(&compose_file)?;

    writeln!(output, "# Docker Compose for NATS JetStream + Ingestion API")?;
    writeln!(output, "# This includes:")?;
    writeln!(output, "#  - NATS JetStream (message queue)")?;
    writeln!(output, "#  - PostgreSQL (database)")?;
    writeln!(output, "#  - Ingestion API (HTTP -> NATS publisher)")?;
    writeln!(output, "#  - TODO: Worker (NATS -> Database consumer)")?;
    writeln!(output)?;
    writeln!(output, "services:")?;
    writeln!(output, "  # NATS JetStream server")?;
    writeln!(output, "  nats:")?;
    writeln!(output, "    image: nats:latest")?;
    writeln!(output, "    ports:")?;
    writeln!(output, "      - \"4222:4222\"  # NATS client port")?;
    writeln!(output, "      - \"8222:8222\"  # NATS monitoring port")?;
    writeln!(output, "    command:")?;
    writeln!(output, "      - \"-js\"             # Enable JetStream")?;
    writeln!(output, "      - \"-m\"              # Enable monitoring")?;
    writeln!(output, "      - \"8222\"")?;
    writeln!(output, "    healthcheck:")?;
    writeln!(output, "      test: [\"CMD-SHELL\", \"timeout 1 sh -c 'cat < /dev/null > /dev/tcp/127.0.0.1/4222' || exit 1\"]")?;
    writeln!(output, "      interval: 5s")?;
    writeln!(output, "      timeout: 3s")?;
    writeln!(output, "      retries: 3")?;
    writeln!(output, "      start_period: 5s")?;
    writeln!(output)?;

    writeln!(output, "  # PostgreSQL database")?;
    writeln!(output, "  postgres:")?;
    writeln!(output, "    image: postgres:16-alpine")?;
    writeln!(output, "    environment:")?;
    writeln!(output, "      POSTGRES_USER: nomnom")?;
    writeln!(output, "      POSTGRES_PASSWORD: nomnom")?;
    writeln!(output, "      POSTGRES_DB: nomnom")?;
    writeln!(output, "    ports:")?;
    writeln!(output, "      - \"5432:5432\"")?;
    writeln!(output, "    volumes:")?;
    writeln!(output, "      - postgres_data:/var/lib/postgresql/data")?;
    writeln!(output, "    healthcheck:")?;
    writeln!(output, "      test: [\"CMD-SHELL\", \"pg_isready -U nomnom\"]")?;
    writeln!(output, "      interval: 5s")?;
    writeln!(output, "      timeout: 3s")?;
    writeln!(output, "      retries: 3")?;
    writeln!(output)?;

    writeln!(output, "  # Ingestion API (HTTP -> NATS publisher)")?;
    writeln!(output, "  ingestion-api:")?;
    writeln!(output, "    build: .")?;
    writeln!(output, "    ports:")?;
    writeln!(output, "      - \"{}:8080\"", config.port)?;
    writeln!(output, "    environment:")?;

    match config.database_type {
        DatabaseType::PostgreSQL => {
            writeln!(output, "      DATABASE_URL: postgresql://nomnom:nomnom@postgres:5432/nomnom")?;
        }
        DatabaseType::MySQL | DatabaseType::MariaDB => {
            writeln!(output, "      DATABASE_URL: mysql://nomnom:nomnom@mysql:3306/nomnom")?;
        }
    }

    writeln!(output, "      NATS_URL: nats://nats:4222")?;
    writeln!(output, "      NATS_STREAM: MESSAGES")?;
    writeln!(output, "      RUST_LOG: info")?;
    writeln!(output, "    depends_on:")?;
    writeln!(output, "      nats:")?;
    writeln!(output, "        condition: service_healthy")?;
    writeln!(output, "      postgres:")?;
    writeln!(output, "        condition: service_healthy")?;
    writeln!(output)?;

    writeln!(output, "  # TODO: Worker service (NATS consumer -> Database writer)")?;
    writeln!(output, "  # worker:")?;
    writeln!(output, "  #   build:")?;
    writeln!(output, "  #     context: ./worker")?;
    writeln!(output, "  #   environment:")?;
    writeln!(output, "  #     DATABASE_URL: postgresql://nomnom:nomnom@postgres:5432/nomnom")?;
    writeln!(output, "  #     NATS_URL: nats://nats:4222")?;
    writeln!(output, "  #     NATS_STREAM: MESSAGES")?;
    writeln!(output, "  #     RUST_LOG: info")?;
    writeln!(output, "  #   depends_on:")?;
    writeln!(output, "  #     nats:")?;
    writeln!(output, "  #       condition: service_healthy")?;
    writeln!(output, "  #     postgres:")?;
    writeln!(output, "  #       condition: service_healthy")?;
    writeln!(output)?;

    writeln!(output, "volumes:")?;
    writeln!(output, "  postgres_data:")?;

    Ok(())
}
