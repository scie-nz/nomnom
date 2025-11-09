//! Code generation framework for entities and transforms.
//!
//! This module provides YAML-to-Rust code generation for entities and transforms,
//! supporting various derivation patterns (parent, repeated, multi-parent).

pub mod types;
pub mod rust_codegen;
pub mod python_codegen;
pub mod pyo3_codegen;
pub mod yaml_loader;
pub mod utils;
pub mod plugins;
pub mod diesel;
pub mod dashboard;
pub mod ingestion_server;
pub mod orchestration;
pub mod transform_codegen;
pub mod build_config;
pub mod project_config;
pub mod fs_utils;
pub mod parser_binary;
pub mod lineage;

// Re-export key types
pub use types::{EntityDef, FieldDef, ComputedFrom, SourceType, Repetition};
pub use yaml_loader::{load_entities, load_entity};
pub use rust_codegen::{generate_rust_code, RustCodegenConfig};
pub use python_codegen::{generate_python_bindings, generate_python_core_mapping};
pub use pyo3_codegen::{generate_python_bindings as generate_pyo3_bindings, PyO3Config};
pub use plugins::{CodegenCallbacks, CodegenPipeline, NoOpCallbacks};
pub use orchestration::{GenerationConfig, generate_all_from_config};
pub use transform_codegen::{
    generate_rust_transform, generate_pyo3_binding, generate_python_transform,
    generate_transform_tests, generate_transforms_module, generate_python_transforms
};
// Re-export unified BuildConfig from project_config
pub use project_config::{
    ProjectConfig, ProjectMetadata, PythonConfig, RustConfig, Dependency,
    BuildConfig as ProjectBuildConfig, BuildProjectInfo, RustPackageConfig, PythonPackageConfig,
    DependencyConfig, BuildPathsConfig, BuildOutputsConfig, BuildTransformsConfig,
    BuildHelperConfig,
};
// Keep old build_config exports for backward compatibility (deprecated)
pub use build_config::{
    BuildConfig, generate_cargo_toml, generate_pyproject_toml, generate_lib_rs,
    generate_readme, write_build_configs
};

/// Generate all code from a nomnom.yaml configuration file
///
/// This is the main entry point for YAML-based code generation from build.rs.
///
/// # Example
///
/// ```rust,no_run
/// fn main() {
///     nomnom::codegen::generate_from_yaml("nomnom.yaml")
///         .expect("Code generation failed");
/// }
/// ```
pub fn generate_from_yaml(yaml_path: impl AsRef<std::path::Path>) -> Result<(), String> {
    println!("📋 Loading configuration from {}...", yaml_path.as_ref().display());

    // Load and validate build config
    let build_config = project_config::BuildConfig::from_file(&yaml_path)?;
    build_config.validate()?;

    println!("  ✓ Configuration loaded: {}", build_config.project.name);

    // Convert to GenerationConfig
    let generation_config = build_config.to_generation_config()?;

    // Generate all code
    orchestration::generate_all_from_config(&generation_config)
        .map_err(|e| format!("Code generation failed: {}", e))?;

    println!("✨ Code generation complete!");

    Ok(())
}
