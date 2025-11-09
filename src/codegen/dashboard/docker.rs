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

/// Generate Dockerfile.backend
fn generate_backend_dockerfile(output_dir: &Path) -> Result<(), Box<dyn Error>> {
    let dockerfile = output_dir.join("Dockerfile.backend");
    let mut output = std::fs::File::create(&dockerfile)?;

    writeln!(output, "FROM python:3.11-slim\n")?;
    writeln!(output, "WORKDIR /app\n")?;
    writeln!(output, "COPY backend/requirements.txt .")?;
    writeln!(output, "RUN pip install --no-cache-dir -r requirements.txt\n")?;
    writeln!(output, "COPY backend/ .\n")?;
    writeln!(output, "CMD [\"uvicorn\", \"main:app\", \"--host\", \"0.0.0.0\", \"--port\", \"8000\", \"--reload\"]")?;

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
    writeln!(output, "      - \"8000:8000\"")?;
    writeln!(output, "    environment:")?;
    writeln!(output, "      - DATABASE_URL=${{DATABASE_URL}}")?;
    writeln!(output, "    depends_on:")?;
    writeln!(output, "      - postgres  # Or mysql/mariadb depending on your setup")?;
    writeln!(output, "    volumes:")?;
    writeln!(output, "      - ./backend:/app  # Hot reload for development")?;
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
