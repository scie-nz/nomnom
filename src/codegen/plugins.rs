//! Plugin system for domain-specific codegen extensions.
//!
//! This module provides a callback/plugin architecture that allows
//! domain-specific libraries (like data_processor) to inject custom
//! code generation logic without modifying nomnom's core.

use crate::codegen::types::EntityDef;
use crate::codegen::fs_utils;
use std::path::Path;

/// Callback trait for domain-specific code generation hooks
///
/// Implement this trait to add custom generation logic that runs
/// after the main codegen phases.
///
/// # Example
///
/// ```ignore
/// struct MyDomainCallbacks;
///
/// impl CodegenCallbacks for MyDomainCallbacks {
///     fn after_python_bindings(&self, entities: &[EntityDef], output_dir: &Path) {
///         // Generate custom mapping files, __init__.py, etc.
///         generate_my_custom_file(entities, output_dir);
///     }
/// }
/// ```
pub trait CodegenCallbacks: Send + Sync {
    /// Called after Python bindings are generated
    ///
    /// Use this to generate domain-specific Python files like:
    /// - ORM mapping files (e.g., SQLAlchemy, Django models)
    /// - Custom __init__.py with domain-specific imports
    /// - Integration glue code
    ///
    /// # Arguments
    ///
    /// * `all_entities` - All entities (transient and permanent)
    /// * `permanent_entities` - Subset of entities with database config
    /// * `output_dir` - Directory where generated files should be written
    fn after_python_bindings(
        &self,
        all_entities: &[EntityDef],
        permanent_entities: &[EntityDef],
        output_dir: &Path,
    );

    /// Called after Rust code is generated
    ///
    /// Use this to generate domain-specific Rust files like:
    /// - Custom trait implementations
    /// - Domain-specific helper functions
    /// - Integration modules
    fn after_rust_code(&self, entities: &[EntityDef], output_dir: &Path) {
        // Default: no-op
        let _ = (entities, output_dir);
    }

    /// Called at the very end of code generation
    ///
    /// Use this for final post-processing steps like:
    /// - Running code formatters
    /// - Generating documentation
    /// - Creating package metadata
    fn finalize(&self, output_dir: &Path) {
        // Default: no-op
        let _ = output_dir;
    }
}

/// No-op implementation of CodegenCallbacks
///
/// Use this when you don't need any custom generation logic.
pub struct NoOpCallbacks;

impl CodegenCallbacks for NoOpCallbacks {
    fn after_python_bindings(
        &self,
        _all_entities: &[EntityDef],
        _permanent_entities: &[EntityDef],
        _output_dir: &Path,
    ) {
        // No-op
    }
}

/// Configuration for the complete codegen pipeline
pub struct CodegenPipeline<'a> {
    /// All entity definitions
    pub entities: &'a [EntityDef],

    /// Output directory for generated files
    pub output_dir: &'a Path,

    /// Rust code generation configuration
    pub rust_config: crate::codegen::rust_codegen::RustCodegenConfig,

    /// PyO3 configuration (optional)
    pub pyo3_config: Option<crate::codegen::pyo3_codegen::PyO3Config>,

    /// Custom callbacks for domain-specific generation
    pub callbacks: Option<&'a dyn CodegenCallbacks>,
}

impl<'a> CodegenPipeline<'a> {
    /// Create a new codegen pipeline
    pub fn new(entities: &'a [EntityDef], output_dir: &'a Path) -> Self {
        Self {
            entities,
            output_dir,
            rust_config: Default::default(),
            pyo3_config: None,
            callbacks: None,
        }
    }

    /// Set Rust code generation configuration
    pub fn with_rust_config(mut self, config: crate::codegen::rust_codegen::RustCodegenConfig) -> Self {
        self.rust_config = config;
        self
    }

    /// Set PyO3 configuration
    pub fn with_pyo3_config(mut self, config: crate::codegen::pyo3_codegen::PyO3Config) -> Self {
        self.pyo3_config = Some(config);
        self
    }

    /// Set custom callbacks
    pub fn with_callbacks(mut self, callbacks: &'a dyn CodegenCallbacks) -> Self {
        self.callbacks = Some(callbacks);
        self
    }

    /// Run the complete codegen pipeline
    pub fn run(&self) -> Result<(), Box<dyn std::error::Error>> {
        use crate::codegen::rust_codegen::generate_rust_code;
        use crate::codegen::pyo3_codegen::generate_python_bindings;
        

        // Phase 1: Generate Rust entity code
        let rust_output_path = self.output_dir.join("generated.rs");
        let mut rust_output = fs_utils::create_file(&rust_output_path)?;
        generate_rust_code(&mut rust_output, self.entities, &self.rust_config)?;
        println!("✓ Generated Rust entity code: {}", rust_output_path.display());

        // Invoke after_rust_code callback
        if let Some(callbacks) = self.callbacks {
            callbacks.after_rust_code(self.entities, self.output_dir);
        }

        // Phase 2: Generate Python bindings (if PyO3 config provided)
        if let Some(ref pyo3_config) = self.pyo3_config {
            let bindings_output_path = self.output_dir.join("generated_bindings.rs");
            let mut bindings_output = fs_utils::create_file(&bindings_output_path)?;
            generate_python_bindings(&mut bindings_output, self.entities, pyo3_config)?;
            println!("✓ Generated Python bindings: {}", bindings_output_path.display());

            // Invoke after_python_bindings callback
            if let Some(callbacks) = self.callbacks {
                let permanent_entities: Vec<&EntityDef> = self.entities.iter()
                    .filter(|e| e.database.is_some())
                    .collect();
                callbacks.after_python_bindings(
                    self.entities,
                    &permanent_entities.iter().copied().cloned().collect::<Vec<_>>(),
                    self.output_dir,
                );
            }
        }

        // Phase 3: Finalize
        if let Some(callbacks) = self.callbacks {
            callbacks.finalize(self.output_dir);
        }

        println!("✓ Codegen pipeline completed successfully");
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_no_op_callbacks() {
        let callbacks = NoOpCallbacks;
        callbacks.after_python_bindings(&[], &[], Path::new("/tmp"));
        callbacks.after_rust_code(&[], Path::new("/tmp"));
        callbacks.finalize(Path::new("/tmp"));
        // Test passes if no panics occur
    }
}
