/// Diesel ORM code generation from entity definitions.
///
/// This module provides generic code generation for Diesel ORM from YAML entity configs.

mod schema;
mod models;
mod operations;
mod pyo3;

pub use schema::generate_schema;
pub use models::generate_models;
pub use operations::generate_operations;
pub use pyo3::generate_pyo3_persistence;

use std::path::Path;
use std::error::Error;

/// Generate all Diesel code from entity definitions.
///
/// This is a convenience function that generates all Diesel-related code:
/// - Schema definitions
/// - Model structs
/// - Database operations
/// - PyO3 persistence bindings
///
/// # Arguments
///
/// * `entities` - Slice of entity definitions
/// * `output_dir` - Base output directory (typically "src/")
/// * `config_dir` - Path to YAML config directory (e.g., "../../../config/entities")
///
/// # Example
///
/// ```ignore
/// use nomnom::codegen::diesel;
///
/// let entities = nomnom::load_entities("config/entities")?;
/// diesel::generate_all(&entities, Path::new("src"), "../../../config/entities")?;
/// ```
pub fn generate_all(
    entities: &[crate::codegen::EntityDef],
    output_dir: &Path,
    config_dir: &str,
) -> Result<(), Box<dyn Error>> {
    generate_schema(entities, &output_dir.join("schema.rs"), config_dir)?;
    generate_models(entities, &output_dir.join("models/mod.rs"), config_dir)?;
    generate_operations(entities, &output_dir.join("db/generated_operations.rs"), config_dir)?;
    generate_pyo3_persistence(entities, &output_dir.join("python/generated_persistence.rs"), config_dir)?;
    Ok(())
}
