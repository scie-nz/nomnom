// Simple script to regenerate TPC-H code
use std::path::PathBuf;
use std::collections::HashMap;
use nomnom::codegen::orchestration::{generate_all_from_config, GenerationConfig};
use nomnom::codegen::project_config::RustTransformDef;
use serde::Deserialize;

#[derive(Deserialize)]
struct NomNomConfig {
    transforms: Option<TransformsConfig>,
}

#[derive(Deserialize)]
struct TransformsConfig {
    rust: Option<HashMap<String, RustTransformDef>>,
}

fn main() {
    let base_path = PathBuf::from("/Users/bogdanstate/nomnom/config/examples/tpch");

    // Load transforms from nomnom.yaml
    let config_path = base_path.join("nomnom.yaml");
    let yaml_content = std::fs::read_to_string(&config_path).unwrap();
    let nomnom_config: NomNomConfig = serde_yaml::from_str(&yaml_content).unwrap();
    let rust_transforms = nomnom_config.transforms.and_then(|t| t.rust);

    let config = GenerationConfig {
        config_dir: base_path.join("entities").to_str().unwrap().to_string(),
        rust_output: base_path.join("src/generated.rs").to_str().unwrap().to_string(),
        pyo3_bindings_output: base_path.join("src/generated_bindings.rs").to_str().unwrap().to_string(),
        diesel_schema_output: Some(base_path.join("src/schema.rs").to_str().unwrap().to_string()),
        diesel_models_output: Some(base_path.join("src/models/mod.rs").to_str().unwrap().to_string()),
        diesel_operations_output: Some(base_path.join("src/db/generated_operations.rs").to_str().unwrap().to_string()),
        diesel_pyo3_output: Some(base_path.join("src/python/generated_persistence.rs").to_str().unwrap().to_string()),
        python_mapping_output: None,
        python_module_name: "tpch_example._rust".to_string(),
        transform_registry_type: "crate::transform_registry::TransformRegistry".to_string(),
        additional_rust_header: None,
        transform_registry_output: None,
        python_transforms_module: None,
        python_bindings_output: None,
        transform_functions: None,
        lib_rs_output: None,
        dependency_exports: None,
        python_dependency_imports: None,
        python_rust_shim_output: None,
        python_package_init_output: None,
        rust_transforms,
    };

    generate_all_from_config(&config).unwrap();
    println!("✅ Code generation completed!");
}
