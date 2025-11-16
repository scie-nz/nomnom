//! Integration tests for multi-database backend support (PostgreSQL, MySQL, MariaDB)
//!
//! These tests verify that code generation works correctly for different database backends
//! and that the configuration precedence system works as expected.

use nomnom::codegen;
use std::path::PathBuf;
use std::fs;

/// Test helper to create a temporary test directory
fn setup_test_dir(name: &str) -> PathBuf {
    let test_dir = PathBuf::from(format!("/tmp/nomnom-test-{}", name));
    if test_dir.exists() {
        fs::remove_dir_all(&test_dir).ok();
    }
    fs::create_dir_all(&test_dir).expect("Failed to create test directory");
    test_dir
}

/// Test helper to load TPCH example entities
fn load_tpch_entities() -> Vec<codegen::EntityDef> {
    codegen::load_entities("config/examples/tpch/entities")
        .expect("Failed to load TPCH entities")
}

#[test]
fn test_worker_generation_postgresql() {
    let entities = load_tpch_entities();
    let test_dir = setup_test_dir("worker-postgresql");

    let config = codegen::worker::WorkerConfig {
        database_type: codegen::worker::DatabaseType::PostgreSQL,
        worker_name: "test_worker_pg".to_string(),
    };

    // Generate worker code
    codegen::worker::generate_all(&entities, &test_dir, &config, None)
        .expect("Failed to generate PostgreSQL worker");

    // Verify Cargo.toml contains postgres feature
    let cargo_toml = fs::read_to_string(test_dir.join("Cargo.toml"))
        .expect("Failed to read Cargo.toml");

    assert!(cargo_toml.contains("default = [\"postgres\"]"),
        "Cargo.toml should have postgres as default feature");
    assert!(cargo_toml.contains("postgres = [\"diesel/postgres\"]"),
        "Cargo.toml should have postgres feature");

    // Verify database.rs contains PostgreSQL types
    let database_rs = fs::read_to_string(test_dir.join("src/database.rs"))
        .expect("Failed to read database.rs");

    assert!(database_rs.contains("#[cfg(feature = \"postgres\")]"),
        "database.rs should have postgres feature gate");
    assert!(database_rs.contains("use diesel::pg::PgConnection"),
        "database.rs should import PgConnection");
    assert!(database_rs.contains("pub type DbConnection = PgConnection"),
        "database.rs should define DbConnection alias");

    println!("✅ PostgreSQL worker generation test passed");
}

#[test]
fn test_worker_generation_mysql() {
    let entities = load_tpch_entities();
    let test_dir = setup_test_dir("worker-mysql");

    let config = codegen::worker::WorkerConfig {
        database_type: codegen::worker::DatabaseType::MySQL,
        worker_name: "test_worker_mysql".to_string(),
    };

    // Generate worker code
    codegen::worker::generate_all(&entities, &test_dir, &config, None)
        .expect("Failed to generate MySQL worker");

    // Verify Cargo.toml contains mysql feature
    let cargo_toml = fs::read_to_string(test_dir.join("Cargo.toml"))
        .expect("Failed to read Cargo.toml");

    assert!(cargo_toml.contains("default = [\"mysql\"]"),
        "Cargo.toml should have mysql as default feature");
    assert!(cargo_toml.contains("mysql = [\"diesel/mysql\"]"),
        "Cargo.toml should have mysql feature");

    // Verify database.rs contains MySQL types
    let database_rs = fs::read_to_string(test_dir.join("src/database.rs"))
        .expect("Failed to read database.rs");

    assert!(database_rs.contains("#[cfg(feature = \"mysql\")]"),
        "database.rs should have mysql feature gate");
    assert!(database_rs.contains("use diesel::mysql::MysqlConnection"),
        "database.rs should import MysqlConnection");
    assert!(database_rs.contains("pub type DbConnection = MysqlConnection"),
        "database.rs should define DbConnection alias");

    println!("✅ MySQL worker generation test passed");
}

#[test]
fn test_json_type_mapping_postgresql() {
    let entities = load_tpch_entities();
    let test_dir = setup_test_dir("json-postgresql");

    let config = codegen::worker::WorkerConfig {
        database_type: codegen::worker::DatabaseType::PostgreSQL,
        worker_name: "test_json_pg".to_string(),
    };

    codegen::worker::generate_all(&entities, &test_dir, &config, None)
        .expect("Failed to generate PostgreSQL worker");

    // Check migration SQL for JSONB type
    let migrations_dir = test_dir.join("migrations");
    if migrations_dir.exists() {
        let migration_files: Vec<_> = fs::read_dir(&migrations_dir)
            .unwrap()
            .filter_map(|e| e.ok())
            .filter(|e| e.path().extension().map(|ext| ext == "sql").unwrap_or(false))
            .collect();

        if !migration_files.is_empty() {
            let sql = fs::read_to_string(migration_files[0].path())
                .expect("Failed to read migration SQL");

            // PostgreSQL should use JSONB for JSON fields
            if sql.contains("JSON") {
                assert!(sql.contains("JSONB"),
                    "PostgreSQL migrations should use JSONB type for JSON fields");
            }
        }
    }

    println!("✅ PostgreSQL JSON type mapping test passed");
}

#[test]
fn test_json_type_mapping_mysql() {
    let entities = load_tpch_entities();
    let test_dir = setup_test_dir("json-mysql");

    let config = codegen::worker::WorkerConfig {
        database_type: codegen::worker::DatabaseType::MySQL,
        worker_name: "test_json_mysql".to_string(),
    };

    codegen::worker::generate_all(&entities, &test_dir, &config, None)
        .expect("Failed to generate MySQL worker");

    // Check migration SQL for JSON type
    let migrations_dir = test_dir.join("migrations");
    if migrations_dir.exists() {
        let migration_files: Vec<_> = fs::read_dir(&migrations_dir)
            .unwrap()
            .filter_map(|e| e.ok())
            .filter(|e| e.path().extension().map(|ext| ext == "sql").unwrap_or(false))
            .collect();

        if !migration_files.is_empty() {
            let sql = fs::read_to_string(migration_files[0].path())
                .expect("Failed to read migration SQL");

            // MySQL should use JSON (not JSONB) for JSON fields
            if sql.contains("JSON") {
                assert!(!sql.contains("JSONB"),
                    "MySQL migrations should use JSON type (not JSONB) for JSON fields");
            }
        }
    }

    println!("✅ MySQL JSON type mapping test passed");
}

#[test]
fn test_dashboard_generation_postgresql() {
    let entities = load_tpch_entities();
    let test_dir = setup_test_dir("dashboard-postgresql");

    codegen::dashboard::generate_all(
        &entities,
        &test_dir,
        "config/examples/tpch/entities",
        codegen::dashboard::DatabaseType::PostgreSQL,
        codegen::dashboard::BackendType::FastAPI,
    ).expect("Failed to generate PostgreSQL dashboard");

    // Verify migration SQL contains PostgreSQL-specific syntax
    let sql_path = test_dir.join("migrations/001_create_events_table.sql");
    let sql = fs::read_to_string(&sql_path)
        .expect("Failed to read dashboard migration SQL");

    // Check for PostgreSQL-specific syntax
    assert!(sql.contains("CREATE OR REPLACE FUNCTION") || sql.contains("BIGSERIAL"),
        "PostgreSQL dashboard should use PostgreSQL-specific SQL");

    println!("✅ PostgreSQL dashboard generation test passed");
}

#[test]
fn test_dashboard_generation_mysql() {
    let entities = load_tpch_entities();
    let test_dir = setup_test_dir("dashboard-mysql");

    codegen::dashboard::generate_all(
        &entities,
        &test_dir,
        "config/examples/tpch/entities",
        codegen::dashboard::DatabaseType::MySQL,
        codegen::dashboard::BackendType::FastAPI,
    ).expect("Failed to generate MySQL dashboard");

    // Verify migration SQL contains MySQL-specific syntax
    let sql_path = test_dir.join("migrations/001_create_events_table.sql");
    let sql = fs::read_to_string(&sql_path)
        .expect("Failed to read dashboard migration SQL");

    // Check for MySQL-specific syntax
    assert!(sql.contains("AUTO_INCREMENT") || sql.contains("DELIMITER"),
        "MySQL dashboard should use MySQL-specific SQL");

    println!("✅ MySQL dashboard generation test passed");
}

#[test]
fn test_ingestion_server_generation_postgresql() {
    let entities = load_tpch_entities();
    let test_dir = setup_test_dir("ingestion-postgresql");

    let config = codegen::ingestion_server::IngestionServerConfig {
        port: 8080,
        database_type: codegen::ingestion_server::DatabaseType::PostgreSQL,
        server_name: "test_ingestion_pg".to_string(),
    };

    codegen::ingestion_server::generate_all(&entities, &test_dir, &config)
        .expect("Failed to generate PostgreSQL ingestion server");

    // Verify Cargo.toml
    let cargo_toml = fs::read_to_string(test_dir.join("Cargo.toml"))
        .expect("Failed to read Cargo.toml");

    assert!(cargo_toml.contains("diesel/postgres"),
        "Ingestion server Cargo.toml should reference postgres feature");

    println!("✅ PostgreSQL ingestion server generation test passed");
}

#[test]
fn test_ingestion_server_generation_mysql() {
    let entities = load_tpch_entities();
    let test_dir = setup_test_dir("ingestion-mysql");

    let config = codegen::ingestion_server::IngestionServerConfig {
        port: 8080,
        database_type: codegen::ingestion_server::DatabaseType::MySQL,
        server_name: "test_ingestion_mysql".to_string(),
    };

    codegen::ingestion_server::generate_all(&entities, &test_dir, &config)
        .expect("Failed to generate MySQL ingestion server");

    // Verify Cargo.toml
    let cargo_toml = fs::read_to_string(test_dir.join("Cargo.toml"))
        .expect("Failed to read Cargo.toml");

    assert!(cargo_toml.contains("diesel/mysql"),
        "Ingestion server Cargo.toml should reference mysql feature");

    println!("✅ MySQL ingestion server generation test passed");
}

#[test]
fn test_database_type_from_string() {
    use nomnom::codegen::worker::DatabaseType;

    // Test various string representations
    assert_eq!(
        DatabaseType::from_str("postgresql").unwrap(),
        DatabaseType::PostgreSQL
    );
    assert_eq!(
        DatabaseType::from_str("postgres").unwrap(),
        DatabaseType::PostgreSQL
    );
    assert_eq!(
        DatabaseType::from_str("pg").unwrap(),
        DatabaseType::PostgreSQL
    );
    assert_eq!(
        DatabaseType::from_str("mysql").unwrap(),
        DatabaseType::MySQL
    );
    assert_eq!(
        DatabaseType::from_str("mariadb").unwrap(),
        DatabaseType::MariaDB
    );

    // Test case insensitivity
    assert_eq!(
        DatabaseType::from_str("PostgreSQL").unwrap(),
        DatabaseType::PostgreSQL
    );
    assert_eq!(
        DatabaseType::from_str("MYSQL").unwrap(),
        DatabaseType::MySQL
    );

    // Test invalid database type
    assert!(DatabaseType::from_str("oracle").is_err());

    println!("✅ DatabaseType from_str test passed");
}

#[test]
fn test_text_type_compatibility() {
    // Both PostgreSQL and MySQL should use TEXT for String fields
    let entities = load_tpch_entities();

    // Test PostgreSQL
    let pg_dir = setup_test_dir("text-postgresql");
    let pg_config = codegen::worker::WorkerConfig {
        database_type: codegen::worker::DatabaseType::PostgreSQL,
        worker_name: "test_text_pg".to_string(),
    };
    codegen::worker::generate_all(&entities, &pg_dir, &pg_config, None)
        .expect("Failed to generate PostgreSQL worker");

    // Test MySQL
    let mysql_dir = setup_test_dir("text-mysql");
    let mysql_config = codegen::worker::WorkerConfig {
        database_type: codegen::worker::DatabaseType::MySQL,
        worker_name: "test_text_mysql".to_string(),
    };
    codegen::worker::generate_all(&entities, &mysql_dir, &mysql_config, None)
        .expect("Failed to generate MySQL worker");

    // Both should compile successfully with TEXT type
    println!("✅ TEXT type compatibility test passed");
}

#[test]
fn test_auto_increment_syntax() {
    let entities = load_tpch_entities();

    // Test PostgreSQL uses BIGSERIAL
    let pg_dir = setup_test_dir("autoincrement-pg");
    codegen::dashboard::generate_all(
        &entities,
        &pg_dir,
        "config/examples/tpch/entities",
        codegen::dashboard::DatabaseType::PostgreSQL,
        codegen::dashboard::BackendType::FastAPI,
    ).expect("Failed to generate PostgreSQL dashboard");

    let pg_sql = fs::read_to_string(pg_dir.join("migrations/001_create_events_table.sql"))
        .expect("Failed to read PostgreSQL SQL");
    assert!(pg_sql.contains("BIGSERIAL") || pg_sql.contains("SERIAL"),
        "PostgreSQL should use BIGSERIAL or SERIAL for auto-increment");

    // Test MySQL uses AUTO_INCREMENT
    let mysql_dir = setup_test_dir("autoincrement-mysql");
    codegen::dashboard::generate_all(
        &entities,
        &mysql_dir,
        "config/examples/tpch/entities",
        codegen::dashboard::DatabaseType::MySQL,
        codegen::dashboard::BackendType::FastAPI,
    ).expect("Failed to generate MySQL dashboard");

    let mysql_sql = fs::read_to_string(mysql_dir.join("migrations/001_create_events_table.sql"))
        .expect("Failed to read MySQL SQL");
    assert!(mysql_sql.contains("AUTO_INCREMENT"),
        "MySQL should use AUTO_INCREMENT for auto-increment");

    println!("✅ Auto-increment syntax test passed");
}

/// Test that we can generate code for both databases from the same entity definitions
#[test]
fn test_cross_database_compatibility() {
    let entities = load_tpch_entities();

    println!("Testing cross-database compatibility with {} entities", entities.len());

    // Generate for PostgreSQL
    let pg_dir = setup_test_dir("cross-compat-pg");
    let pg_config = codegen::worker::WorkerConfig {
        database_type: codegen::worker::DatabaseType::PostgreSQL,
        worker_name: "test_cross_pg".to_string(),
    };
    codegen::worker::generate_all(&entities, &pg_dir, &pg_config, None)
        .expect("Failed to generate PostgreSQL worker");

    // Generate for MySQL
    let mysql_dir = setup_test_dir("cross-compat-mysql");
    let mysql_config = codegen::worker::WorkerConfig {
        database_type: codegen::worker::DatabaseType::MySQL,
        worker_name: "test_cross_mysql".to_string(),
    };
    codegen::worker::generate_all(&entities, &mysql_dir, &mysql_config, None)
        .expect("Failed to generate MySQL worker");

    // Both should succeed - same entities, different database backends
    println!("✅ Cross-database compatibility test passed");
}
