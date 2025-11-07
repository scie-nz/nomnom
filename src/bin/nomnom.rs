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
    },
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
        Commands::BuildFromConfig { config, output, release, test } => {
            build_from_config(config, output, release, test)
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
) -> Result<(), String> {
    println!("🔨 Building project from {}...", config_file.display());

    // Load ProjectBuildConfig (extended YAML)
    let build_config = nomnom::codegen::ProjectBuildConfig::from_file(&config_file)?;
    build_config.validate()?;

    println!("  ✓ Loaded project: {}", build_config.project.name);
    println!("  ✓ Version: {}", build_config.project.version);

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
    let generation_config = build_config.to_generation_config()?;
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
