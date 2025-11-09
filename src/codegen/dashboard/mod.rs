/// Real-time dashboard code generation from entity definitions.
///
/// This module generates a complete real-time dashboard that visualizes
/// database inserts as they happen. The dashboard is fully code-generated
/// from entity YAML configurations.
///
/// ## Backends
///
/// Two backend options are available:
/// - **FastAPI** (Python): Event-based polling using db_events table
/// - **Axum** (Rust): Direct table polling, single binary deployment
///
/// ## Architecture
///
/// ### FastAPI Backend
/// - **SQL Triggers**: Populate `db_events` table on INSERT
/// - **FastAPI Backend**: Polls db_events and broadcasts via WebSocket
/// - **React Frontend**: Displays real-time updates
///
/// ### Axum Backend (Recommended)
/// - **No SQL Triggers**: Directly polls entity tables
/// - **Axum Backend**: Rust server with table polling + WebSocket
/// - **React Frontend**: Same frontend, connects to Axum backend
///
/// ## Database Support
///
/// The generated dashboard works with:
/// - PostgreSQL
/// - MySQL (FastAPI only)
/// - MariaDB (FastAPI only)
///
/// ## Usage
///
/// ```ignore
/// use nomnom::codegen::dashboard::{self, DatabaseType, BackendType};
///
/// let entities = nomnom::load_entities("config/entities")?;
/// dashboard::generate_all(
///     &entities,
///     Path::new("dashboard"),
///     "../../../config/entities",
///     DatabaseType::PostgreSQL,
///     BackendType::Axum,
/// )?;
/// ```

mod sql_triggers;
mod fastapi_backend;
mod axum_backend;
mod react_frontend;
mod docker;
mod utils;

pub use sql_triggers::generate_sql_migrations;
pub use fastapi_backend::generate_backend as generate_fastapi_backend;
pub use axum_backend::generate_backend as generate_axum_backend;
pub use react_frontend::generate_frontend;
pub use docker::{generate_dockerfiles, generate_docker_compose};
pub use utils::{DatabaseType, DashboardConfig, EntityDisplayConfig};

/// Backend type for dashboard
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum BackendType {
    /// FastAPI backend (Python) - uses db_events table
    FastAPI,
    /// Axum backend (Rust) - direct table polling
    Axum,
}

use std::path::Path;
use std::error::Error;

/// Generate all dashboard code from entity definitions.
///
/// This is a convenience function that generates all dashboard-related code:
/// - SQL migrations (events table + triggers, FastAPI only)
/// - Backend (FastAPI or Axum)
/// - React frontend (TypeScript + components)
/// - Docker configuration
///
/// # Arguments
///
/// * `entities` - Slice of entity definitions
/// * `output_dir` - Output directory for dashboard (e.g., "dashboard/")
/// * `config_dir` - Path to YAML config directory (e.g., "../../../config/entities")
/// * `db_type` - Database type (PostgreSQL, MySQL, or MariaDB)
/// * `backend_type` - Backend type (FastAPI or Axum)
///
/// # Example
///
/// ```ignore
/// use nomnom::codegen::dashboard::{self, DatabaseType, BackendType};
///
/// let entities = nomnom::load_entities("config/entities")?;
/// dashboard::generate_all(
///     &entities,
///     Path::new("dashboard"),
///     "../../../config/entities",
///     DatabaseType::PostgreSQL,
///     BackendType::Axum,
/// )?;
/// ```
pub fn generate_all(
    entities: &[crate::codegen::EntityDef],
    output_dir: &Path,
    config_dir: &str,
    db_type: DatabaseType,
    backend_type: BackendType,
) -> Result<(), Box<dyn Error>> {
    println!("🎨 Generating real-time dashboard...");
    println!("   Backend: {:?}", backend_type);
    println!("   Database: {:?}", db_type);

    // Create dashboard directory structure
    std::fs::create_dir_all(output_dir)?;
    std::fs::create_dir_all(output_dir.join("frontend/src"))?;

    // Generate backend-specific code
    match backend_type {
        BackendType::FastAPI => {
            // FastAPI requires SQL migrations and backend directory
            std::fs::create_dir_all(output_dir.join("migrations"))?;
            std::fs::create_dir_all(output_dir.join("backend"))?;

            // Generate SQL migrations (events table + triggers)
            println!("  ✓ Generating SQL migrations...");
            generate_sql_migrations(
                entities,
                &output_dir.join("migrations"),
                config_dir,
                db_type,
            )?;

            // Generate FastAPI backend
            println!("  ✓ Generating FastAPI backend...");
            generate_fastapi_backend(
                entities,
                &output_dir.join("backend"),
                config_dir,
                db_type,
            )?;
        }
        BackendType::Axum => {
            // Axum backend is a Rust project
            println!("  ✓ Generating Axum backend...");
            generate_axum_backend(
                entities,
                output_dir,
                config_dir,
                db_type,
            )?;
        }
    }

    // Generate React frontend (same for both backends)
    println!("  ✓ Generating React frontend...");
    generate_frontend(
        entities,
        &output_dir.join("frontend"),
        config_dir,
    )?;

    // Generate Docker configuration
    println!("  ✓ Generating Docker configuration...");
    generate_dockerfiles(output_dir)?;
    generate_docker_compose(output_dir, db_type)?;

    println!("✨ Dashboard generation complete!");
    println!("   📁 Output: {}", output_dir.display());

    match backend_type {
        BackendType::FastAPI => {
            println!("   🚀 To start: cd {} && docker compose up", output_dir.display());
        }
        BackendType::Axum => {
            println!("   🚀 To start:");
            println!("      cd {} && cargo build --release", output_dir.display());
            println!("      ./target/release/dashboard");
            println!("   📝 Configure DATABASE_URL in .env");
        }
    }

    Ok(())
}
