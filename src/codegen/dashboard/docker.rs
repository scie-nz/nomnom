/// Docker configuration generation for dashboard.

use super::utils::DatabaseType;
use std::path::Path;
use std::error::Error;
use std::io::Write;

/// Generate Dockerfiles for backend and frontend
pub fn generate_dockerfiles(output_dir: &Path) -> Result<(), Box<dyn Error>> {
    generate_backend_dockerfile(output_dir)?;
    generate_frontend_dockerfile(output_dir)?;
    Ok(())
}

/// Generate Dockerfile.backend for Axum/Rust (Alpine + Security Hardened)
fn generate_backend_dockerfile(output_dir: &Path) -> Result<(), Box<dyn Error>> {
    let dockerfile = output_dir.join("Dockerfile.backend");
    let mut output = std::fs::File::create(&dockerfile)?;

    writeln!(output, "# Multi-stage Dockerfile for Rust dashboard backend (Alpine + Security Hardened)")?;
    writeln!(output, "FROM rust:alpine as builder\n")?;
    writeln!(output, "# Install build dependencies for musl")?;
    writeln!(output, "RUN apk add --no-cache \\")?;
    writeln!(output, "    musl-dev \\")?;
    writeln!(output, "    pkgconfig \\")?;
    writeln!(output, "    openssl-dev \\")?;
    writeln!(output, "    postgresql-dev\n")?;
    writeln!(output, "WORKDIR /build\n")?;
    writeln!(output, "# Copy manifests")?;
    writeln!(output, "COPY Cargo.toml Cargo.lock* ./\n")?;
    writeln!(output, "# Copy source")?;
    writeln!(output, "COPY src ./src\n")?;
    writeln!(output, "# Build release binary with musl target")?;
    writeln!(output, "RUN cargo build --release\n")?;
    writeln!(output, "# Runtime stage - minimal Alpine")?;
    writeln!(output, "FROM alpine:3.19\n")?;
    writeln!(output, "# Install runtime dependencies")?;
    writeln!(output, "RUN apk add --no-cache \\")?;
    writeln!(output, "    ca-certificates \\")?;
    writeln!(output, "    libgcc \\")?;
    writeln!(output, "    postgresql-libs\n")?;
    writeln!(output, "# Create non-root user")?;
    writeln!(output, "RUN addgroup -g 1000 appuser && \\")?;
    writeln!(output, "    adduser -D -u 1000 -G appuser appuser\n")?;
    writeln!(output, "WORKDIR /app\n")?;
    writeln!(output, "# Copy binary from builder and set ownership")?;
    writeln!(output, "COPY --from=builder --chown=appuser:appuser /build/target/release/dashboard /app/\n")?;
    writeln!(output, "# Switch to non-root user")?;
    writeln!(output, "USER appuser:appuser\n")?;
    writeln!(output, "# Expose default port")?;
    writeln!(output, "EXPOSE 3000\n")?;
    writeln!(output, "# Run the binary")?;
    writeln!(output, "CMD [\"/app/dashboard\"]")?;

    Ok(())
}

/// Generate Dockerfile.frontend
fn generate_frontend_dockerfile(output_dir: &Path) -> Result<(), Box<dyn Error>> {
    let dockerfile = output_dir.join("Dockerfile.frontend");
    let mut output = std::fs::File::create(&dockerfile)?;

    writeln!(output, "FROM node:20-alpine\n")?;
    writeln!(output, "WORKDIR /app\n")?;
    writeln!(output, "COPY frontend/package.json frontend/package-lock.json ./")?;
    writeln!(output, "RUN npm install\n")?;
    writeln!(output, "COPY frontend/ .\n")?;
    writeln!(output, "CMD [\"npm\", \"run\", \"dev\", \"--\", \"--host\", \"0.0.0.0\"]")?;

    Ok(())
}

/// Generate docker-compose configuration
pub fn generate_docker_compose(output_dir: &Path, _db_type: DatabaseType) -> Result<(), Box<dyn Error>> {
    let compose_file = output_dir.join("docker-compose.dashboard.yml");
    let mut output = std::fs::File::create(&compose_file)?;

    writeln!(output, "# Auto-generated docker-compose for dashboard")?;
    writeln!(output, "# Merge this with your main docker-compose.yml or use with:")?;
    writeln!(output, "# docker compose -f docker-compose.yml -f docker-compose.dashboard.yml up\n")?;

    writeln!(output, "version: '3.8'\n")?;
    writeln!(output, "services:")?;

    // Dashboard backend service
    writeln!(output, "  dashboard-backend:")?;
    writeln!(output, "    build:")?;
    writeln!(output, "      context: .")?;
    writeln!(output, "      dockerfile: Dockerfile.backend")?;
    writeln!(output, "    ports:")?;
    writeln!(output, "      - \"3000:3000\"")?;
    writeln!(output, "    environment:")?;
    writeln!(output, "      - DATABASE_URL=${{DATABASE_URL}}")?;
    writeln!(output, "      - PORT=3000")?;
    writeln!(output, "      - HOST=0.0.0.0")?;
    writeln!(output, "      - RUST_LOG=info")?;
    writeln!(output, "    depends_on:")?;
    writeln!(output, "      - postgres  # Or mysql/mariadb depending on your setup")?;
    writeln!(output, "    networks:")?;
    writeln!(output, "      - default\n")?;

    // Dashboard frontend service
    writeln!(output, "  dashboard-frontend:")?;
    writeln!(output, "    build:")?;
    writeln!(output, "      context: .")?;
    writeln!(output, "      dockerfile: Dockerfile.frontend")?;
    writeln!(output, "    ports:")?;
    writeln!(output, "      - \"5173:5173\"")?;
    writeln!(output, "    depends_on:")?;
    writeln!(output, "      - dashboard-backend")?;
    writeln!(output, "    volumes:")?;
    writeln!(output, "      - ./frontend:/app  # Hot reload for development")?;
    writeln!(output, "      - /app/node_modules  # Prevent overwriting node_modules")?;
    writeln!(output, "    networks:")?;
    writeln!(output, "      - default")?;

    Ok(())
}
