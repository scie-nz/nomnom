//! nomnom CLI - YAML-based code generation for data transformation frameworks
//!
//! This CLI tool generates Rust code and Python bindings from YAML entity and transform definitions.

use clap::{Parser, Subcommand};
use std::path::PathBuf;
use std::process;

#[derive(Parser)]
#[command(name = "nomnom")]
#[command(version, about = "YAML-based code generation for data transformation frameworks", long_about = None)]
struct Cli {
    #[command(subcommand)]
    command: Commands,
}

#[derive(Subcommand)]
enum Commands {
    /// Generate Rust code from YAML configurations
    Generate {
        /// Path to config directory containing entities/ and transforms/
        #[arg(short, long, default_value = "config")]
        config: PathBuf,

        /// Output directory for generated code
        #[arg(short, long, default_value = ".build")]
        output: PathBuf,
    },

    /// Build Rust extension and Python wheel from YAML configurations
    Build {
        /// Path to config directory containing entities/ and transforms/
        #[arg(short, long, default_value = "config")]
        config: PathBuf,

        /// Output directory for generated code and build artifacts
        #[arg(short, long, default_value = ".build")]
        output: PathBuf,

        /// Build in release mode (optimized)
        #[arg(short, long)]
        release: bool,
    },

    /// Validate YAML configurations without generating code
    Validate {
        /// Path to config directory containing entities/ and transforms/
        #[arg(short, long, default_value = "config")]
        config: PathBuf,
    },

    /// Build complete project from nomnom.yaml (Phase 4: Zero build.rs)
    BuildFromConfig {
        /// Path to nomnom.yaml configuration file
        #[arg(short, long, default_value = "nomnom.yaml")]
        config: PathBuf,

        /// Override output directory (default: uses paths.source_root from config)
        #[arg(short, long)]
        output: Option<PathBuf>,

        /// Build in release mode (optimized)
        #[arg(short, long)]
        release: bool,

        /// Run tests after building
        #[arg(short, long)]
        test: bool,

        /// Database type (postgresql, mysql, mariadb) - overrides config file
        #[arg(short, long)]
        database: Option<String>,
    },

    /// Generate real-time dashboard for database monitoring
    GenerateDashboard {
        /// Path to entities directory
        #[arg(short, long, default_value = "entities")]
        entities: PathBuf,

        /// Output directory for dashboard
        #[arg(short, long, default_value = "dashboard")]
        output: PathBuf,

        /// Database type (postgresql, mysql, mariadb)
        #[arg(short, long, default_value = "postgresql")]
        database: String,

        /// Backend type (axum, fastapi)
        #[arg(short, long, default_value = "axum")]
        backend: String,
    },

    /// Generate Axum-based HTTP ingestion server
    GenerateIngestionServer {
        /// Path to entities directory
        #[arg(short, long, default_value = "entities")]
        entities: PathBuf,

        /// Output directory for ingestion server
        #[arg(short, long, default_value = "ingestion-server")]
        output: PathBuf,

        /// Database type (postgresql, mysql, mariadb)
        #[arg(short, long, default_value = "postgresql")]
        database: String,

        /// Server port
        #[arg(short, long, default_value = "8080")]
        port: u16,

        /// Server name for Cargo.toml
        #[arg(short, long, default_value = "ingestion-server")]
        name: String,
    },

    /// Generate NATS worker binary (consumes from NATS JetStream)
    GenerateWorker {
        /// Path to entities directory
        #[arg(short, long, default_value = "entities")]
        entities: PathBuf,

        /// Output directory for worker
        #[arg(short, long, default_value = "worker")]
        output: PathBuf,

        /// Database type (postgresql, mysql, mariadb)
        #[arg(short, long, default_value = "postgresql")]
        database: String,

        /// Worker name for Cargo.toml
        #[arg(short, long, default_value = "worker")]
        name: String,
    },
}

/// Determine database type with precedence: CLI > ENV > config file > DATABASE_URL > default
fn detect_database_type(
    cli_override: Option<String>,
    config_db_type: Option<String>,
) -> Result<String, String> {
    // 1. CLI flag (highest priority)
    if let Some(db_type) = cli_override {
        let normalized = db_type.to_lowercase();
        if matches!(normalized.as_str(), "postgresql" | "postgres" | "pg" | "mysql" | "mariadb") {
            println!("  ℹ Using database type from CLI flag: {}", normalized);
            return Ok(match normalized.as_str() {
                "postgres" | "pg" => "postgresql".to_string(),
                other => other.to_string(),
            });
        } else {
            return Err(format!(
                "Unsupported database type: '{}'. Supported types: postgresql, mysql, mariadb",
                db_type
            ));
        }
    }

    // 2. Environment variable NOMNOM_DATABASE_TYPE
    if let Ok(db_type) = std::env::var("NOMNOM_DATABASE_TYPE") {
        let normalized = db_type.to_lowercase();
        if matches!(normalized.as_str(), "postgresql" | "postgres" | "pg" | "mysql" | "mariadb") {
            println!("  ℹ Using database type from NOMNOM_DATABASE_TYPE: {}", normalized);
            return Ok(match normalized.as_str() {
                "postgres" | "pg" => "postgresql".to_string(),
                other => other.to_string(),
            });
        }
    }

    // 3. Config file database.type
    if let Some(db_type) = config_db_type {
        let normalized = db_type.to_lowercase();
        if matches!(normalized.as_str(), "postgresql" | "postgres" | "pg" | "mysql" | "mariadb") {
            println!("  ℹ Using database type from config file: {}", normalized);
            return Ok(match normalized.as_str() {
                "postgres" | "pg" => "postgresql".to_string(),
                other => other.to_string(),
            });
        } else {
            return Err(format!(
                "Unsupported database type in config: '{}'. Supported types: postgresql, mysql, mariadb",
                db_type
            ));
        }
    }

    // 4. Detect from DATABASE_URL scheme
    if let Ok(database_url) = std::env::var("DATABASE_URL") {
        if database_url.starts_with("postgres://") || database_url.starts_with("postgresql://") {
            println!("  ℹ Detected PostgreSQL from DATABASE_URL");
            return Ok("postgresql".to_string());
        } else if database_url.starts_with("mysql://") {
            println!("  ℹ Detected MySQL from DATABASE_URL");
            return Ok("mysql".to_string());
        }
    }

    // 5. Default to PostgreSQL
    println!("  ℹ Using default database type: postgresql");
    Ok("postgresql".to_string())
}

fn main() {
    let cli = Cli::parse();

    let result = match cli.command {
        Commands::Generate { config, output } => {
            generate_code(config, output)
        }
        Commands::Build { config, output, release } => {
            build_project(config, output, release)
        }
        Commands::Validate { config } => {
            validate_config(config)
        }
        Commands::BuildFromConfig { config, output, release, test, database } => {
            build_from_config(config, output, release, test, database)
        }
        Commands::GenerateDashboard { entities, output, database, backend } => {
            generate_dashboard(entities, output, database, backend)
        }
        Commands::GenerateIngestionServer { entities, output, database, port, name } => {
            generate_ingestion_server(entities, output, database, port, name)
        }
        Commands::GenerateWorker { entities, output, database, name } => {
            generate_worker(entities, output, database, name)
        }
    };

    if let Err(e) = result {
        eprintln!("Error: {}", e);
        process::exit(1);
    }
}

/// Generate Rust code from YAML configurations
fn generate_code(config: PathBuf, output: PathBuf) -> Result<(), String> {
    println!("🔧 Generating code from {}...", config.display());

    // Load entity configurations
    let entities_dir = config.join("entities");
    if !entities_dir.exists() {
        return Err(format!("Entities directory not found: {}", entities_dir.display()));
    }

    let entities = nomnom::codegen::load_entities(&entities_dir)
        .map_err(|e| format!("Failed to load entities: {}", e))?;

    println!("  ✓ Loaded {} entities", entities.len());

    // Load transform configurations (optional)
    let transforms_dir = config.join("transforms");
    let transforms = if transforms_dir.exists() {
        let loaded = nomnom::runtime::load_transforms_from_dir(&transforms_dir)
            .map_err(|e| format!("Failed to load transforms: {}", e))?;
        println!("  ✓ Loaded {} transforms", loaded.len());
        loaded
    } else {
        println!("  ℹ No transforms directory found (optional)");
        Vec::new()
    };

    // Create output directory
    std::fs::create_dir_all(&output)
        .map_err(|e| format!("Failed to create output directory: {}", e))?;

    // Generate Rust entity code
    let entities_file = output.join("entities.rs");
    let mut file = std::fs::File::create(&entities_file)
        .map_err(|e| format!("Failed to create entities.rs: {}", e))?;

    // Write header with imports
    use std::io::Write;
    writeln!(file, "//! Auto-generated entity definitions.\n")
        .map_err(|e| format!("Failed to write header: {}", e))?;
    writeln!(file, "use serde::{{Serialize, Deserialize}};")
        .map_err(|e| format!("Failed to write imports: {}", e))?;
    writeln!(file, "use std::collections::HashMap;")
        .map_err(|e| format!("Failed to write imports: {}", e))?;
    writeln!(file)
        .map_err(|e| format!("Failed to write imports: {}", e))?;

    // Configure code generation with transform registry
    let codegen_config = nomnom::codegen::RustCodegenConfig {
        transform_registry_type: Some("crate::transform_registry::TransformRegistry".to_string()),
    };

    nomnom::codegen::generate_rust_code(&mut file, &entities, &codegen_config)
        .map_err(|e| format!("Failed to generate entity code: {}", e))?;

    println!("  ✓ Generated {}", entities_file.display());

    // Generate transform code (if any)
    if !transforms.is_empty() {
        let rust_transforms: Vec<_> = transforms.iter()
            .filter(|t| t.language == nomnom::runtime::transforms::TransformLanguage::Rust)
            .collect();

        if !rust_transforms.is_empty() {
            let transforms_code = nomnom::codegen::generate_transforms_module(
                &rust_transforms,
                "transforms"
            ).map_err(|e| format!("Failed to generate transform code: {}", e))?;

            let transforms_file = output.join("transforms.rs");
            std::fs::write(&transforms_file, transforms_code)
                .map_err(|e| format!("Failed to write transforms.rs: {}", e))?;

            println!("  ✓ Generated {}", transforms_file.display());
        }

        // Generate Python transforms (if any)
        let python_transforms: Vec<_> = transforms.iter()
            .filter(|t| t.language == nomnom::runtime::transforms::TransformLanguage::Python)
            .collect();

        if !python_transforms.is_empty() {
            let python_code = nomnom::codegen::generate_python_transforms(&python_transforms);

            let python_file = output.join("python_transforms.py");
            std::fs::write(&python_file, python_code)
                .map_err(|e| format!("Failed to write python_transforms.py: {}", e))?;

            println!("  ✓ Generated {}", python_file.display());
        }
    }

    println!("✨ Code generation complete!");

    Ok(())
}

/// Build Rust extension and Python wheel from YAML configurations
fn build_project(config: PathBuf, output: PathBuf, release: bool) -> Result<(), String> {
    println!("🔨 Building project from {}...", config.display());

    // First, generate all code
    generate_code(config.clone(), output.clone())?;

    println!("\n📦 Generating build configuration...");

    // Check what was generated
    let entities_exists = output.join("entities.rs").exists();
    let transforms_exists = output.join("transforms.rs").exists();

    // Calculate path to nomnom
    // Use absolute path to avoid issues with relative paths
    let nomnom_path = if let Ok(cwd) = std::env::current_dir() {
        let nomnom_dir = cwd.join("nomnom");
        if nomnom_dir.exists() {
            nomnom_dir.display().to_string()
        } else {
            // Fallback to sibling directory
            "../nomnom".to_string()
        }
    } else {
        "../nomnom".to_string()
    };

    // Generate build configuration files
    let build_config = nomnom::codegen::BuildConfig {
        package_name: "generated_project".to_string(),
        version: "0.1.0".to_string(),
        description: "Auto-generated data transformation library".to_string(),
        python_module_name: "_rust".to_string(),
        min_python_version: "3.8".to_string(),
        nomnom_path,
        dependencies: Vec::new(),
    };

    nomnom::codegen::write_build_configs(&output, &build_config, entities_exists, transforms_exists)
        .map_err(|e| format!("Failed to write build configs: {}", e))?;

    println!("  ✓ Generated Cargo.toml");
    println!("  ✓ Generated pyproject.toml");
    println!("  ✓ Generated src/lib.rs");
    println!("  ✓ Generated README.md");

    // Move generated entity and transform files to src/ directory
    if entities_exists {
        std::fs::rename(output.join("entities.rs"), output.join("src/entities.rs"))
            .map_err(|e| format!("Failed to move entities.rs: {}", e))?;
        println!("  ✓ Moved entities.rs to src/");
    }

    if transforms_exists {
        std::fs::rename(output.join("transforms.rs"), output.join("src/transforms.rs"))
            .map_err(|e| format!("Failed to move transforms.rs: {}", e))?;
        println!("  ✓ Moved transforms.rs to src/");
    }

    println!("\n🔧 Building Rust extension...");

    // Run cargo build
    let mut cargo_cmd = std::process::Command::new("cargo");
    cargo_cmd.arg("build");
    if release {
        cargo_cmd.arg("--release");
    }
    let cargo_result = cargo_cmd.current_dir(&output).output();

    match cargo_result {
        Ok(output_result) => {
            if output_result.status.success() {
                println!("  ✓ Cargo build completed successfully");
            } else {
                let stderr = String::from_utf8_lossy(&output_result.stderr);
                return Err(format!("Cargo build failed:\n{}", stderr));
            }
        }
        Err(e) => {
            return Err(format!("Failed to run cargo build: {}. Is cargo installed?", e));
        }
    }

    println!("\n🐍 Building Python wheel...");

    // Run maturin build
    let mut maturin_cmd = std::process::Command::new("maturin");
    maturin_cmd.arg("build");
    if release {
        maturin_cmd.arg("--release");
    }
    let maturin_result = maturin_cmd.current_dir(&output).output();

    match maturin_result {
        Ok(output_result) => {
            if output_result.status.success() {
                println!("  ✓ Maturin build completed successfully");
                let stdout = String::from_utf8_lossy(&output_result.stdout);
                // Extract wheel path from output
                for line in stdout.lines() {
                    if line.contains(".whl") {
                        println!("  📦 {}", line.trim());
                    }
                }
            } else {
                let stderr = String::from_utf8_lossy(&output_result.stderr);
                return Err(format!("Maturin build failed:\n{}", stderr));
            }
        }
        Err(e) => {
            return Err(format!("Failed to run maturin build: {}. Is maturin installed?", e));
        }
    }

    println!("\n✨ Build complete!");
    println!("  Output directory: {}", output.display());
    println!("\nTo install the package:");
    println!("  pip install {}/target/wheels/*.whl", output.display());

    Ok(())
}

/// Validate YAML configurations without generating code
fn validate_config(config: PathBuf) -> Result<(), String> {
    println!("🔍 Validating configurations in {}...", config.display());

    // Validate entity configurations
    let entities_dir = config.join("entities");
    if !entities_dir.exists() {
        return Err(format!("Entities directory not found: {}", entities_dir.display()));
    }

    let entities = nomnom::codegen::load_entities(&entities_dir)
        .map_err(|e| format!("Failed to load entities: {}", e))?;

    println!("  ✓ {} entities validated", entities.len());

    // Validate transform configurations (optional)
    let transforms_dir = config.join("transforms");
    if transforms_dir.exists() {
        let transforms = nomnom::runtime::load_transforms_from_dir(&transforms_dir)
            .map_err(|e| format!("Failed to load transforms: {}", e))?;
        println!("  ✓ {} transforms validated", transforms.len());
    }

    println!("✅ All configurations are valid!");

    Ok(())
}

/// Build complete project from nomnom.yaml (Phase 4: Zero build.rs)
fn build_from_config(
    config_file: PathBuf,
    output: Option<PathBuf>,
    release: bool,
    run_tests: bool,
    database_override: Option<String>,
) -> Result<(), String> {
    println!("🔨 Building project from {}...", config_file.display());

    // Load ProjectBuildConfig (extended YAML)
    let build_config = nomnom::codegen::ProjectBuildConfig::from_file(&config_file)?;
    build_config.validate()?;

    println!("  ✓ Loaded project: {}", build_config.project.name);
    println!("  ✓ Version: {}", build_config.project.version);

    // Extract database type from config file
    let config_db_type = build_config.database
        .as_ref()
        .and_then(|db| db.r#type.clone());

    // Detect database type with precedence: CLI > ENV > config file > DATABASE_URL > default
    let database_type = detect_database_type(database_override, config_db_type)?;
    println!("  ✓ Database type: {}", database_type);

    // Determine source root (where Cargo.toml will be written)
    let source_root = output.unwrap_or_else(|| {
        PathBuf::from(&build_config.paths.source_root.clone().unwrap_or_else(|| ".".to_string()))
    });

    println!("  ✓ Source root: {}", source_root.display());

    // Create source root if needed
    std::fs::create_dir_all(&source_root)
        .map_err(|e| format!("Failed to create source root: {}", e))?;

    // Generate Cargo.toml
    println!("\n📦 Generating build configuration...");
    let cargo_toml_path = source_root.join("Cargo.toml");
    let cargo_toml = build_config.generate_cargo_toml();
    std::fs::write(&cargo_toml_path, cargo_toml)
        .map_err(|e| format!("Failed to write Cargo.toml: {}", e))?;
    println!("  ✓ Generated Cargo.toml");

    // Generate pyproject.toml
    let pyproject_path = source_root.join("pyproject.toml");
    let pyproject = build_config.generate_pyproject_toml();
    std::fs::write(&pyproject_path, pyproject)
        .map_err(|e| format!("Failed to write pyproject.toml: {}", e))?;
    println!("  ✓ Generated pyproject.toml");

    // Generate README.md
    let readme_path = source_root.join("README.md");
    let readme = build_config.generate_readme();
    std::fs::write(&readme_path, readme)
        .map_err(|e| format!("Failed to write README.md: {}", e))?;
    println!("  ✓ Generated README.md");

    // Generate all code
    println!("\n🔧 Generating Rust code...");
    let source_root_str = source_root.to_str().ok_or("Invalid source_root path")?;
    let generation_config = build_config.to_generation_config_with_root(Some(source_root_str))?;
    nomnom::codegen::generate_all_from_config(&generation_config)
        .map_err(|e| format!("Code generation failed: {}", e))?;
    println!("  ✓ Code generation complete");

    // Generate parser binary
    println!("\n🔍 Generating parser binary...");
    let entities_dir = PathBuf::from(&build_config.paths.config_dir);
    let entities = nomnom::codegen::load_entities(&entities_dir)
        .map_err(|e| format!("Failed to load entities: {}", e))?;
    let parser_source = nomnom::codegen::parser_binary::generate_parser_binary(
        &build_config,
        &entities,
    )?;

    // Write to rust_build/src/bin/record_parser.rs
    // Get rust package directory from outputs path
    let rust_entities_path = PathBuf::from(&build_config.paths.outputs.rust_entities);
    let rust_package_dir = rust_entities_path.parent()
        .and_then(|p| p.parent())
        .ok_or("Could not determine rust package directory from rust_entities path")?;
    let bin_dir = source_root.join(rust_package_dir).join("src").join("bin");
    std::fs::create_dir_all(&bin_dir)
        .map_err(|e| format!("Failed to create bin directory: {}", e))?;
    let parser_path = bin_dir.join("record_parser.rs");
    std::fs::write(&parser_path, parser_source)
        .map_err(|e| format!("Failed to write parser binary: {}", e))?;
    println!("  ✓ Generated parser binary at {}", parser_path.display());

    // Run cargo build
    println!("\n🦀 Building Rust extension...");
    let mut cargo_cmd = std::process::Command::new("cargo");
    cargo_cmd.arg("build").current_dir(&source_root);
    if release {
        cargo_cmd.arg("--release");
    }

    let cargo_output = cargo_cmd.output()
        .map_err(|e| format!("Failed to run cargo: {}", e))?;

    if !cargo_output.status.success() {
        let stderr = String::from_utf8_lossy(&cargo_output.stderr);
        return Err(format!("Cargo build failed:\n{}", stderr));
    }
    println!("  ✓ Cargo build complete");

    // Run maturin develop (installs into current venv)
    println!("\n🐍 Installing Python extension...");

    // Find maturin executable - prefer venv location
    let maturin_exe = if let Ok(venv_path) = std::env::var("VIRTUAL_ENV") {
        let venv_maturin = std::path::PathBuf::from(&venv_path).join("bin").join("maturin");
        if venv_maturin.exists() {
            println!("  Using maturin from venv: {}", venv_maturin.display());
            venv_maturin
        } else {
            println!("  Warning: maturin not found in venv, falling back to PATH");
            std::path::PathBuf::from("maturin")
        }
    } else {
        println!("  Warning: VIRTUAL_ENV not set - using maturin from PATH");
        std::path::PathBuf::from("maturin")
    };

    let mut maturin_cmd = std::process::Command::new(&maturin_exe);
    maturin_cmd
        .arg("develop")
        .current_dir(&source_root)
        .envs(std::env::vars()); // Inherit environment variables to use activated venv

    if release {
        maturin_cmd.arg("--release");
    }

    let maturin_output = maturin_cmd.output()
        .map_err(|e| format!("Failed to run maturin: {}", e))?;

    // Always print stdout/stderr for debugging
    let stdout = String::from_utf8_lossy(&maturin_output.stdout);
    let stderr = String::from_utf8_lossy(&maturin_output.stderr);
    if !stdout.is_empty() {
        println!("{}", stdout);
    }
    if !stderr.is_empty() {
        eprintln!("{}", stderr);
    }

    if !maturin_output.status.success() {
        return Err(format!("Maturin develop failed (see output above)"));
    }
    println!("  ✓ Python extension installed");

    // Create .pth file to add project root to Python path if needed
    // This allows the Python package (e.g., data_processor/) to be found
    if let Ok(venv_path) = std::env::var("VIRTUAL_ENV") {
        // Check if we have a Python package directory (e.g., data_processor/)
        let python_package_name = build_config.project.name.replace("_rust", "");
        let package_dir = source_root.join(&python_package_name);

        if package_dir.exists() {
            let site_packages = std::path::PathBuf::from(&venv_path)
                .join("lib")
                .join(format!("python{}.{}",
                    std::env::var("PYTHON_VERSION_MAJOR").unwrap_or_else(|_| "3".to_string()),
                    std::env::var("PYTHON_VERSION_MINOR").unwrap_or_else(|_| "12".to_string())))
                .join("site-packages");

            let pth_file = site_packages.join(format!("{}.pth", build_config.project.name));
            let project_root = source_root.canonicalize()
                .map_err(|e| format!("Failed to resolve project root path: {}", e))?;

            std::fs::write(&pth_file, format!("{}\n", project_root.display()))
                .map_err(|e| format!("Failed to write .pth file: {}", e))?;

            println!("  ✓ Added project root to Python path via {}.pth", build_config.project.name);
        }
    }

    // Run tests if requested
    if run_tests {
        println!("\n🧪 Running tests...");

        // Use the config file's grandparent directory as the repository root
        // (config file is at config/nomnom.yaml, so parent is config/, grandparent is repo root)
        let repo_root = config_file
            .canonicalize()
            .map_err(|e| format!("Failed to canonicalize config path: {}", e))?
            .parent()
            .and_then(|p| p.parent())
            .ok_or_else(|| "Could not determine repository root from config file".to_string())?
            .to_path_buf();

        // Use full path to venv's python3 to run pytest
        let python_exe = if let Ok(venv_path) = std::env::var("VIRTUAL_ENV") {
            let py_path = std::path::PathBuf::from(&venv_path).join("bin").join("python3");

            // Verify the python3 executable exists before trying to run it
            if !py_path.exists() {
                return Err(format!("Python3 executable not found at: {}", py_path.display()));
            }

            println!("  Using python from venv: {}", py_path.display());
            // Don't canonicalize - the venv's python3 symlink has special handling
            // that makes Python aware of the venv packages
            py_path
        } else {
            println!("  Warning: VIRTUAL_ENV not set, using 'python3' from PATH");
            std::path::PathBuf::from("python3") // Fallback to PATH
        };

        println!("  Working directory: {}", repo_root.display());
        println!("  Command: {} -m pytest tests/ -v", python_exe.display());

        let mut pytest_cmd = std::process::Command::new(&python_exe);
        pytest_cmd
            .args(["-m", "pytest", "tests/", "-v"])
            .current_dir(&repo_root);

        // Ensure VIRTUAL_ENV is set so Python can find packages
        if let Ok(venv_path) = std::env::var("VIRTUAL_ENV") {
            pytest_cmd.env("VIRTUAL_ENV", venv_path);
        }

        let pytest_output = pytest_cmd.output()
            .map_err(|e| format!("Failed to run pytest: {}", e))?;

        if pytest_output.status.success() {
            println!("  ✓ All tests passed");
        } else {
            let stderr = String::from_utf8_lossy(&pytest_output.stderr);
            let stdout = String::from_utf8_lossy(&pytest_output.stdout);
            return Err(format!("Tests failed:\n{}\n{}", stdout, stderr));
        }
    }

    println!("\n✨ Build complete!");
    println!("  Project: {}", build_config.project.name);
    println!("  Version: {}", build_config.project.version);
    println!("  Extension installed to: {}", source_root.display());

    Ok(())
}

/// Generate real-time dashboard for database monitoring
fn generate_dashboard(
    entities_dir: PathBuf,
    output: PathBuf,
    database_str: String,
    backend_str: String,
) -> Result<(), String> {
    println!("🎨 Generating real-time dashboard...\n");

    // Validate entities directory
    if !entities_dir.exists() {
        return Err(format!("Entities directory not found: {}", entities_dir.display()));
    }

    // Load entities
    println!("📋 Loading entities from {}...", entities_dir.display());
    let entities = nomnom::codegen::load_entities(&entities_dir)
        .map_err(|e| format!("Failed to load entities: {}", e))?;

    println!("  ✓ Loaded {} entities", entities.len());

    // Count persistent entities
    let persistent_count = entities.iter().filter(|e| e.is_persistent()).count();
    if persistent_count == 0 {
        return Err("No persistent entities found. Dashboard requires entities with database configuration.".to_string());
    }

    println!("  ✓ Found {} persistent entities for monitoring", persistent_count);

    // List persistent entities
    for entity in &entities {
        if entity.is_persistent() && !entity.is_abstract {
            println!("    - {} (table: {})",
                entity.name,
                entity.get_database_config()
                    .map(|db| db.conformant_table.as_str())
                    .unwrap_or("unknown"));
        }
    }
    println!();

    // Parse database type
    let db_type = match database_str.to_lowercase().as_str() {
        "postgresql" | "postgres" | "pg" => nomnom::codegen::dashboard::DatabaseType::PostgreSQL,
        "mysql" => nomnom::codegen::dashboard::DatabaseType::MySQL,
        "mariadb" => nomnom::codegen::dashboard::DatabaseType::MariaDB,
        _ => {
            return Err(format!(
                "Unsupported database type: '{}'. Supported types: postgresql, mysql, mariadb",
                database_str
            ));
        }
    };

    // Parse backend type
    let backend_type = match backend_str.to_lowercase().as_str() {
        "axum" => nomnom::codegen::dashboard::BackendType::Axum,
        "fastapi" => nomnom::codegen::dashboard::BackendType::FastAPI,
        _ => {
            return Err(format!(
                "Unsupported backend type: '{}'. Supported types: axum, fastapi",
                backend_str
            ));
        }
    };

    println!("🗄️  Database type: {}", db_type.as_str());
    println!("🔧 Backend type: {:?}", backend_type);
    println!();

    // Generate dashboard
    nomnom::codegen::dashboard::generate_all(
        &entities,
        &output,
        entities_dir.to_str().ok_or("Invalid entities directory path")?,
        db_type,
        backend_type,
    ).map_err(|e| format!("Dashboard generation failed: {}", e))?;

    println!("\n✨ Dashboard generated successfully!");
    println!("📁 Output directory: {}", output.display());

    println!("\n📖 Next steps:");

    match backend_type {
        nomnom::codegen::dashboard::BackendType::FastAPI => {
            println!("  1. Review generated files:");
            println!("     - SQL migrations:  {}/migrations/", output.display());
            println!("     - Backend code:    {}/backend/", output.display());
            println!("     - Frontend code:   {}/frontend/", output.display());
            println!();
            println!("  2. Run database migrations:");
            println!("     cd {}/migrations && ./run.sh", output.display());
            println!();
            println!("  3. Install frontend dependencies:");
            println!("     cd {}/frontend && npm install", output.display());
            println!();
            println!("  4. Start dashboard services:");
            println!("     docker compose -f docker-compose.yml -f {}/docker-compose.dashboard.yml up", output.display());
            println!();
            println!("  5. Access dashboard:");
            println!("     Frontend: http://localhost:5173");
            println!("     Backend:  http://localhost:8000/docs");
        }
        nomnom::codegen::dashboard::BackendType::Axum => {
            println!("  1. Configure database connection:");
            println!("     cd {}", output.display());
            println!("     cp .env.example .env");
            println!("     # Edit .env with your DATABASE_URL");
            println!();
            println!("  2. Build the Axum backend:");
            println!("     cargo build --release");
            println!();
            println!("  3. Run the backend:");
            println!("     cargo run --release");
            println!();
            println!("  4. Install frontend dependencies:");
            println!("     cd frontend && npm install");
            println!();
            println!("  5. Start frontend (in another terminal):");
            println!("     cd frontend && npm run dev");
            println!();
            println!("  6. Access dashboard:");
            println!("     Frontend: http://localhost:5173");
            println!("     Backend API: http://localhost:3000/api/health");
        }
    }

    Ok(())
}

/// Generate Axum-based HTTP ingestion server
fn generate_ingestion_server(
    entities_dir: PathBuf,
    output: PathBuf,
    database_str: String,
    port: u16,
    server_name: String,
) -> Result<(), String> {
    println!("🚀 Generating Axum ingestion server...\n");

    // Validate entities directory
    if !entities_dir.exists() {
        return Err(format!("Entities directory not found: {}", entities_dir.display()));
    }

    // Load entities
    println!("📋 Loading entities from {}...", entities_dir.display());
    let entities = nomnom::codegen::load_entities(&entities_dir)
        .map_err(|e| format!("Failed to load entities: {}", e))?;

    println!("  ✓ Loaded {} entities", entities.len());

    // Count persistent entities
    let persistent_count = entities.iter()
        .filter(|e| e.is_persistent() && !e.is_abstract && e.source_type.to_lowercase() != "reference")
        .count();

    if persistent_count == 0 {
        return Err("No persistent entities found. Ingestion server requires entities with database configuration.".to_string());
    }

    println!("  ✓ Found {} persistent entities for ingestion", persistent_count);

    // List persistent entities
    for entity in &entities {
        if entity.is_persistent() && !entity.is_abstract && entity.source_type.to_lowercase() != "reference" {
            println!("    - {} (table: {})",
                entity.name,
                entity.get_database_config()
                    .map(|db| db.conformant_table.as_str())
                    .unwrap_or("unknown"));
        }
    }
    println!();

    // Parse database type
    let db_type = match database_str.to_lowercase().as_str() {
        "postgresql" | "postgres" | "pg" => nomnom::codegen::ingestion_server::DatabaseType::PostgreSQL,
        "mysql" => nomnom::codegen::ingestion_server::DatabaseType::MySQL,
        "mariadb" => nomnom::codegen::ingestion_server::DatabaseType::MariaDB,
        _ => {
            return Err(format!(
                "Unsupported database type: '{}'. Supported types: postgresql, mysql, mariadb",
                database_str
            ));
        }
    };

    println!("🗄️  Database type: {}", db_type.as_str());
    println!();

    // Create ingestion server config
    let config = nomnom::codegen::ingestion_server::IngestionServerConfig {
        database_type: db_type,
        port,
        server_name: server_name.clone(),
    };

    // Generate ingestion server
    nomnom::codegen::ingestion_server::generate_all(
        &entities,
        &output,
        &config,
    ).map_err(|e| format!("Ingestion server generation failed: {}", e))?;

    println!("\n✨ Ingestion server generated successfully!");
    println!("📁 Output directory: {}", output.display());

    println!("\n📖 Next steps:");
    println!("  1. Configure database connection:");
    println!("     cd {}", output.display());
    println!("     cp .env.example .env");
    println!("     # Edit .env with your database credentials");
    println!();
    println!("  2. Build the server:");
    println!("     cargo build --release");
    println!();
    println!("  3. Run the server:");
    println!("     cargo run --release");
    println!();
    println!("  4. Send test messages:");
    println!("     curl -X POST http://localhost:{}/ingest/message \\", port);
    println!("       -H \"Content-Type: text/plain\" \\");
    println!("       -d \"YOUR_MESSAGE_HERE\"");
    println!();
    println!("  5. View API documentation:");
    println!("     http://localhost:{}/swagger-ui", port);

    Ok(())
}

/// Generate NATS worker binary
fn generate_worker(
    entities_dir: PathBuf,
    output: PathBuf,
    database_str: String,
    worker_name: String,
) -> Result<(), String> {
    println!("🚀 Generating NATS worker binary...\n");

    // Validate entities directory
    if !entities_dir.exists() {
        return Err(format!("Entities directory not found: {}", entities_dir.display()));
    }

    // Load entities
    println!("📋 Loading entities from {}...", entities_dir.display());
    let entities = nomnom::codegen::load_entities(&entities_dir)
        .map_err(|e| format!("Failed to load entities: {}", e))?;

    println!("  ✓ Loaded {} entities", entities.len());

    // Try to load nomnom.yaml for transforms (optional)
    // Look for nomnom.yaml in: entities parent dir, current dir, and entities_dir itself
    let mut nomnom_yaml_candidates = vec![];

    // Try parent of entities dir
    if let Some(parent) = entities_dir.parent() {
        nomnom_yaml_candidates.push(parent.join("nomnom.yaml"));
    }

    // Try current directory
    nomnom_yaml_candidates.push(PathBuf::from("nomnom.yaml"));

    // Try entities dir itself
    nomnom_yaml_candidates.push(entities_dir.join("nomnom.yaml"));

    let nomnom_yaml = nomnom_yaml_candidates.iter()
        .find(|path| path.exists())
        .cloned();

    let transforms = if let Some(nomnom_yaml_path) = nomnom_yaml {
        println!("📋 Loading transforms from {}...", nomnom_yaml_path.display());
        match nomnom::codegen::project_config::BuildConfig::from_file(&nomnom_yaml_path) {
            Ok(config) => {
                let transform_count = config.transforms.as_ref()
                    .map(|t| t.rust.len())
                    .unwrap_or(0);
                println!("  ✓ Loaded {} custom transforms", transform_count);
                config.transforms.map(|t| t.rust)
            }
            Err(e) => {
                println!("  ⚠ Warning: Failed to load nomnom.yaml: {}", e);
                println!("  ℹ Continuing without custom transforms...");
                None
            }
        }
    } else {
        println!("  ℹ No nomnom.yaml found, generating without custom transforms");
        None
    };

    // Count persistent entities
    let persistent_count = entities.iter()
        .filter(|e| e.is_persistent() && !e.is_abstract && e.source_type.to_lowercase() != "reference")
        .count();

    if persistent_count == 0 {
        return Err("No persistent entities found. Worker requires entities with database configuration.".to_string());
    }

    println!("  ✓ Found {} persistent entities for processing", persistent_count);

    // List persistent entities
    for entity in &entities {
        if entity.is_persistent() && !entity.is_abstract && entity.source_type.to_lowercase() != "reference" {
            println!("    - {} (table: {})",
                entity.name,
                entity.get_database_config()
                    .map(|db| db.conformant_table.as_str())
                    .unwrap_or("unknown"));
        }
    }
    println!();

    // Parse database type
    let db_type = match database_str.to_lowercase().as_str() {
        "postgresql" | "postgres" | "pg" => nomnom::codegen::worker::DatabaseType::PostgreSQL,
        "mysql" => nomnom::codegen::worker::DatabaseType::MySQL,
        "mariadb" => nomnom::codegen::worker::DatabaseType::MariaDB,
        _ => {
            return Err(format!(
                "Unsupported database type: '{}'. Supported types: postgresql, mysql, mariadb",
                database_str
            ));
        }
    };

    println!("🗄️  Database type: {}", db_type.as_str());
    println!();

    // Create worker config
    let config = nomnom::codegen::worker::WorkerConfig {
        database_type: db_type,
        worker_name: worker_name.clone(),
    };

    // Generate worker
    nomnom::codegen::worker::generate_all(
        &entities,
        &output,
        &config,
        transforms.as_ref(),
    ).map_err(|e| format!("Worker generation failed: {}", e))?;

    println!("\n✨ Worker binary generated successfully!");
    println!("📁 Output directory: {}", output.display());

    println!("\n📖 Next steps:");
    println!("  1. Configure database and NATS:");
    println!("     cd {}", output.display());
    println!("     cp .env.example .env");
    println!("     # Edit .env with your credentials");
    println!();
    println!("  2. Build the worker:");
    println!("     cargo build --release");
    println!();
    println!("  3. Run the worker:");
    println!("     cargo run --release");
    println!();
    println!("  4. The worker will:");
    println!("     - Connect to NATS JetStream");
    println!("     - Consume messages from the queue");
    println!("     - Parse and validate message bodies");
    println!("     - Write to database");
    println!("     - ACK/NAK messages");

    Ok(())
}
