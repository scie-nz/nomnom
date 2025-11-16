//! Integration test for dashboard code generation

use std::path::Path;
use nomnom::codegen;

#[test]
fn test_dashboard_generation() {
    let test_output = Path::new("/tmp/nomnom-dashboard-test");

    // Clean up any previous test output
    if test_output.exists() {
        std::fs::remove_dir_all(test_output).ok();
    }

    // Load TPCH entities
    let entities = codegen::load_entities("config/examples/tpch/entities")
        .expect("Failed to load entities");

    println!("\n📋 Loaded {} entities:", entities.len());
    for entity in &entities {
        if entity.is_persistent() {
            println!("  ✓ {} (persistent)", entity.name);
        }
    }

    // Generate dashboard with PostgreSQL and FastAPI backend
    codegen::dashboard::generate_all(
        &entities,
        test_output,
        "config/examples/tpch/entities",
        codegen::dashboard::DatabaseType::PostgreSQL,
        codegen::dashboard::BackendType::FastAPI,
    ).expect("Dashboard generation failed");

    println!("\n✨ Dashboard generated to: {}", test_output.display());

    // Verify generated files exist
    assert!(test_output.join("migrations/001_create_events_table.sql").exists());
    assert!(test_output.join("migrations/run.sh").exists());
    assert!(test_output.join("backend/main.py").exists());
    assert!(test_output.join("backend/config.py").exists());
    assert!(test_output.join("backend/requirements.txt").exists());
    assert!(test_output.join("frontend/package.json").exists());
    assert!(test_output.join("frontend/src/generated/entities.ts").exists());
    assert!(test_output.join("Dockerfile.backend").exists());
    assert!(test_output.join("Dockerfile.frontend").exists());
    assert!(test_output.join("docker-compose.dashboard.yml").exists());

    println!("\n✅ All generated files verified!");

    // Print the SQL migration for manual inspection
    let sql_path = test_output.join("migrations/001_create_events_table.sql");
    let sql_content = std::fs::read_to_string(&sql_path).unwrap();
    println!("\n📄 Generated SQL migration:\n{}", sql_content);
}
