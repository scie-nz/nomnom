use crate::codegen::EntityDef;
use std::collections::HashMap;
use std::error::Error;
use std::fs;
use std::path::{Path, PathBuf};

mod values;
pub use values::generate_values_yaml;

/// Configuration for Helm chart generation
pub struct HelmChartConfig {
    pub chart_version: String,
    pub app_version: String,
    pub database_backend: String,  // "mysql" or "postgresql"
    pub nats_url: String,
}

impl Default for HelmChartConfig {
    fn default() -> Self {
        Self {
            chart_version: "0.2.0".to_string(),
            app_version: "0.2.0".to_string(),
            database_backend: "mysql".to_string(),
            nats_url: "nats://{{ .Release.Name }}-nats:4222".to_string(),
        }
    }
}

/// Generate complete Helm chart from entity definitions
pub fn generate_helm_chart(
    entities: &[EntityDef],
    output_dir: &Path,
    config: &HelmChartConfig,
) -> Result<(), Box<dyn Error>> {
    println!("⎈ Generating Helm chart...\n");

    // Create chart directory structure
    let chart_dir = output_dir.join("hl7-nomnom-parser");
    let templates_dir = chart_dir.join("templates");
    fs::create_dir_all(&templates_dir)?;

    // Get template base directory (bundled with binary)
    let template_base = get_template_dir()?;

    // 1. Generate Chart.yaml
    println!("  📄 Generating Chart.yaml...");
    generate_chart_yaml(&chart_dir, config)?;

    // 2. Generate values.yaml
    println!("  📄 Generating values.yaml...");
    let values_yaml = generate_values_yaml(entities, config)?;
    fs::write(chart_dir.join("values.yaml"), values_yaml)?;

    // 3. Copy static templates
    println!("  📄 Copying static templates...");
    copy_static_templates(&template_base, &templates_dir)?;

    // 4. Generate Benthos templates (reuse existing logic)
    println!("  📄 Generating Benthos templates...");
    generate_benthos_templates(entities, &templates_dir, config)?;

    // 5. Generate README
    println!("  📄 Generating README.md...");
    generate_readme(&chart_dir, entities, config)?;

    println!("\n✨ Helm chart generation complete!");
    println!("📁 Chart location: {}", chart_dir.display());
    println!("\nNext steps:");
    println!("  1. cd {}", chart_dir.display());
    println!("  2. helm dependency update");
    println!("  3. helm lint .");
    println!("  4. helm install hl7-parser . -f values.yaml");

    Ok(())
}

/// Get the template directory (relative to binary or source)
fn get_template_dir() -> Result<PathBuf, Box<dyn Error>> {
    // First try relative to cargo manifest (development)
    let manifest_dir = env!("CARGO_MANIFEST_DIR");
    let dev_template_dir = PathBuf::from(manifest_dir).join("templates/helm");

    if dev_template_dir.exists() {
        return Ok(dev_template_dir);
    }

    // TODO: In production, templates would be bundled with binary
    // For now, return error if not in development mode
    Err("Template directory not found. Run from development environment.".into())
}

/// Generate Chart.yaml from template
fn generate_chart_yaml(chart_dir: &Path, config: &HelmChartConfig) -> Result<(), Box<dyn Error>> {
    let template_dir = get_template_dir()?;
    let template_path = template_dir.join("Chart.yaml");
    let template = fs::read_to_string(template_path)?;

    let mut replacements = HashMap::new();
    replacements.insert("{{VERSION}}", config.chart_version.as_str());
    replacements.insert("{{APP_VERSION}}", config.app_version.as_str());

    let output = replace_all(&template, &replacements);
    fs::write(chart_dir.join("Chart.yaml"), output)?;

    Ok(())
}

/// Copy static template files (Helm templates that don't need generation)
fn copy_static_templates(template_base: &Path, templates_dir: &Path) -> Result<(), Box<dyn Error>> {
    let template_files = vec![
        "_helpers.tpl",
        "ingestion-server-deployment.yaml",
        "ingestion-server-service.yaml",
        "worker-deployment.yaml",
        "nats-stream-init-job.yaml",
    ];

    let source_templates = template_base.join("templates");
    for file in template_files {
        let source = source_templates.join(file);
        let dest = templates_dir.join(file);
        fs::copy(&source, &dest)?;
    }

    Ok(())
}

/// Generate Benthos templates (reuse existing benthos/helm module)
fn generate_benthos_templates(
    entities: &[EntityDef],
    templates_dir: &Path,
    _config: &HelmChartConfig,
) -> Result<(), Box<dyn Error>> {
    // Filter transient entities
    let transient_entities: Vec<_> = entities
        .iter()
        .filter(|e| e.source_type.to_lowercase() == "derived" && !e.is_abstract)
        .collect();

    if transient_entities.is_empty() {
        println!("    ⚠ No transient entities found, skipping Benthos templates");
        return Ok(());
    }

    // Create Benthos config (reuse from existing code)
    let benthos_config = crate::codegen::benthos::BenthosConfig {
        database_type: crate::codegen::benthos::DatabaseType::MySQL,
        nats_url: "nats://{{ .Release.Name }}-nats:4222".to_string(),
        mysql_host: "{{ include \"hl7-nomnom-parser.fullname\" . }}-mysql".to_string(),
        mysql_port: 3306,
        mysql_database: "{{ .Values.mysql.auth.database }}".to_string(),
    };

    // Generate templates using existing logic
    let benthos_templates = crate::codegen::benthos::helm::generate_helm_templates(
        &transient_entities,
        &benthos_config,
    )?;

    // Write Benthos templates
    fs::write(
        templates_dir.join("benthos-deployment.yaml"),
        benthos_templates.deployment,
    )?;
    fs::write(
        templates_dir.join("benthos-configmap.yaml"),
        benthos_templates.configmap,
    )?;
    fs::write(
        templates_dir.join("benthos-service.yaml"),
        benthos_templates.service,
    )?;
    fs::write(
        templates_dir.join("schema-init-job.yaml"),
        benthos_templates.schema_init_job,
    )?;

    println!("    ✓ Generated {} Benthos pipelines", transient_entities.len());

    Ok(())
}

/// Generate README.md from template
fn generate_readme(
    chart_dir: &Path,
    entities: &[EntityDef],
    config: &HelmChartConfig,
) -> Result<(), Box<dyn Error>> {
    let template_dir = get_template_dir()?;
    let template_path = template_dir.join("README.md");
    let template = fs::read_to_string(template_path)?;

    let permanent_count = entities
        .iter()
        .filter(|e| e.source_type.to_lowercase() == "permanent")
        .count();
    let transient_count = entities
        .iter()
        .filter(|e| e.source_type.to_lowercase() == "derived")
        .count();

    let timestamp = chrono::Local::now().format("%Y-%m-%d %H:%M:%S").to_string();
    let entity_count_str = entities.len().to_string();
    let permanent_count_str = permanent_count.to_string();
    let transient_count_str = transient_count.to_string();

    let mut replacements = HashMap::new();
    replacements.insert("{{VERSION}}", config.chart_version.as_str());
    replacements.insert("{{ENTITY_COUNT}}", &entity_count_str);
    replacements.insert("{{PERMANENT_COUNT}}", &permanent_count_str);
    replacements.insert("{{TRANSIENT_COUNT}}", &transient_count_str);
    replacements.insert("{{TIMESTAMP}}", &timestamp);

    let output = replace_all(&template, &replacements);
    fs::write(chart_dir.join("README.md"), output)?;

    Ok(())
}

/// Simple string replacement helper
fn replace_all(template: &str, replacements: &HashMap<&str, &str>) -> String {
    let mut result = template.to_string();
    for (key, value) in replacements {
        result = result.replace(key, value);
    }
    result
}
