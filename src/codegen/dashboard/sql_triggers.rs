/// SQL migration generation for dashboard events table and triggers.

use super::utils::{DatabaseType, to_snake_case};
use crate::codegen::EntityDef;
use std::io::Write;
use std::path::Path;
use std::error::Error;

/// Generate SQL migrations for events table and entity triggers
pub fn generate_sql_migrations(
    entities: &[EntityDef],
    output_dir: &Path,
    _config_dir: &str,
    db_type: DatabaseType,
) -> Result<(), Box<dyn Error>> {
    let migration_file = output_dir.join("001_create_events_table.sql");
    let mut output = std::fs::File::create(&migration_file)?;

    writeln!(output, "-- Auto-generated SQL migration for real-time dashboard")?;
    writeln!(output, "-- Database type: {}\n", db_type.as_str())?;

    // Generate events table
    generate_events_table(&mut output, db_type)?;
    writeln!(output)?;

    // Generate triggers for each persistent entity
    for entity in entities {
        if !entity.is_persistent() || entity.is_abstract {
            continue;
        }

        // Skip reference entities (optional - could be configurable)
        if entity.source_type.to_lowercase() == "reference" {
            continue;
        }

        generate_trigger(&mut output, entity, db_type)?;
        writeln!(output)?;
    }

    // Generate cleanup script
    let cleanup_file = output_dir.join("cleanup_old_events.sql");
    let mut cleanup_output = std::fs::File::create(&cleanup_file)?;
    generate_cleanup_script(&mut cleanup_output, db_type)?;

    // Generate migration runner script
    let runner_file = output_dir.join("run.sh");
    let mut runner_output = std::fs::File::create(&runner_file)?;
    generate_migration_runner(&mut runner_output, db_type, &runner_file)?;

    println!("cargo:rerun-if-changed={}", migration_file.display());
    Ok(())
}

/// Generate the db_events table
fn generate_events_table(output: &mut std::fs::File, db_type: DatabaseType) -> Result<(), Box<dyn Error>> {
    writeln!(output, "-- Events/changelog table for real-time dashboard")?;
    writeln!(output, "CREATE TABLE IF NOT EXISTS db_events (")?;

    if db_type.is_mysql_like() {
        writeln!(output, "    id BIGINT AUTO_INCREMENT PRIMARY KEY,")?;
    } else {
        writeln!(output, "    id BIGSERIAL PRIMARY KEY,")?;
    }

    writeln!(output, "    entity VARCHAR(100) NOT NULL,")?;
    writeln!(output, "    event_type VARCHAR(20) NOT NULL,")?;

    if db_type.is_mysql_like() {
        writeln!(output, "    payload JSON NOT NULL,")?;
    } else {
        writeln!(output, "    payload JSONB NOT NULL,")?;
    }

    if db_type.is_mysql_like() {
        writeln!(output, "    created_at TIMESTAMP DEFAULT CURRENT_TIMESTAMP,")?;
        writeln!(output, "    INDEX idx_created_at (created_at),")?;
        writeln!(output, "    INDEX idx_entity (entity)")?;
    } else {
        writeln!(output, "    created_at TIMESTAMP DEFAULT CURRENT_TIMESTAMP")?;
    }

    writeln!(output, ");")?;

    // For PostgreSQL, add indexes separately
    if !db_type.is_mysql_like() {
        writeln!(output)?;
        writeln!(output, "CREATE INDEX IF NOT EXISTS idx_db_events_created_at ON db_events(created_at);")?;
        writeln!(output, "CREATE INDEX IF NOT EXISTS idx_db_events_entity ON db_events(entity);")?;
    }

    Ok(())
}

/// Generate trigger for a single entity
fn generate_trigger(
    output: &mut std::fs::File,
    entity: &EntityDef,
    db_type: DatabaseType,
) -> Result<(), Box<dyn Error>> {
    let db_config = entity.get_database_config()
        .ok_or("Entity has no database configuration")?;

    let table_name = &db_config.conformant_table;
    let function_name = format!("log_{}_insert", to_snake_case(&entity.name));
    let trigger_name = format!("{}_insert_event", to_snake_case(&entity.name));

    writeln!(output, "-- Trigger for {} entity", entity.name)?;

    if db_type.is_mysql_like() {
        generate_mysql_trigger(output, entity, table_name, &trigger_name)?;
    } else {
        generate_postgres_trigger(output, entity, table_name, &function_name, &trigger_name)?;
    }

    Ok(())
}

/// Generate PostgreSQL trigger (uses row_to_json)
fn generate_postgres_trigger(
    output: &mut std::fs::File,
    entity: &EntityDef,
    table_name: &str,
    function_name: &str,
    trigger_name: &str,
) -> Result<(), Box<dyn Error>> {
    writeln!(output, "CREATE OR REPLACE FUNCTION {}()", function_name)?;
    writeln!(output, "RETURNS trigger AS $$")?;
    writeln!(output, "BEGIN")?;
    writeln!(output, "  INSERT INTO db_events (entity, event_type, payload)")?;
    writeln!(output, "  VALUES ('{}', 'insert', row_to_json(NEW)::jsonb);", entity.name)?;
    writeln!(output, "  RETURN NEW;")?;
    writeln!(output, "END;")?;
    writeln!(output, "$$ LANGUAGE plpgsql;")?;
    writeln!(output)?;
    writeln!(output, "CREATE TRIGGER {}", trigger_name)?;
    writeln!(output, "AFTER INSERT ON {}", table_name)?;
    writeln!(output, "FOR EACH ROW EXECUTE FUNCTION {}();", function_name)?;

    Ok(())
}

/// Generate MySQL/MariaDB trigger (uses JSON_OBJECT)
fn generate_mysql_trigger(
    output: &mut std::fs::File,
    entity: &EntityDef,
    table_name: &str,
    trigger_name: &str,
) -> Result<(), Box<dyn Error>> {
    writeln!(output, "DELIMITER $$")?;
    writeln!(output, "CREATE TRIGGER {}", trigger_name)?;
    writeln!(output, "AFTER INSERT ON {}", table_name)?;
    writeln!(output, "FOR EACH ROW")?;
    writeln!(output, "BEGIN")?;
    writeln!(output, "  INSERT INTO db_events (entity, event_type, payload)")?;
    writeln!(output, "  VALUES (")?;
    writeln!(output, "    '{}',", entity.name)?;
    writeln!(output, "    'insert',")?;
    write!(output, "    JSON_OBJECT(")?;

    // Build JSON_OBJECT with all fields from field_overrides
    if let Some(ref persistence) = entity.persistence {
        let field_count = persistence.field_overrides.len();
        for (i, field) in persistence.field_overrides.iter().enumerate() {
            let comma = if i < field_count - 1 { "," } else { "" };
            writeln!(output)?;
            write!(output, "      '{}', NEW.{}{}", field.name, field.name, comma)?;
        }
    }

    writeln!(output)?;
    writeln!(output, "    )")?;
    writeln!(output, "  );")?;
    writeln!(output, "END$$")?;
    writeln!(output, "DELIMITER ;")?;

    Ok(())
}

/// Generate cleanup script to delete old events
fn generate_cleanup_script(output: &mut std::fs::File, db_type: DatabaseType) -> Result<(), Box<dyn Error>> {
    writeln!(output, "-- Cleanup old events (run periodically to prevent table growth)")?;
    writeln!(output, "-- Delete events older than 7 days")?;
    writeln!(output)?;

    if db_type.is_mysql_like() {
        writeln!(output, "DELETE FROM db_events")?;
        writeln!(output, "WHERE created_at < DATE_SUB(NOW(), INTERVAL 7 DAY);")?;
    } else {
        writeln!(output, "DELETE FROM db_events")?;
        writeln!(output, "WHERE created_at < NOW() - INTERVAL '7 days';")?;
    }

    Ok(())
}

/// Generate migration runner script
fn generate_migration_runner(output: &mut std::fs::File, db_type: DatabaseType, runner_path: &Path) -> Result<(), Box<dyn Error>> {
    writeln!(output, "#!/bin/bash")?;
    writeln!(output, "# Auto-generated migration runner for dashboard")?;
    writeln!(output)?;
    writeln!(output, "set -e")?;
    writeln!(output)?;

    match db_type {
        DatabaseType::PostgreSQL => {
            writeln!(output, "# Check if psql is installed")?;
            writeln!(output, "if ! command -v psql &> /dev/null; then")?;
            writeln!(output, "  echo \"Error: psql command not found\"")?;
            writeln!(output, "  echo \"Please install PostgreSQL client:\"")?;
            writeln!(output, "  echo \"  macOS:   brew install libpq && export PATH=\\\"/opt/homebrew/opt/libpq/bin:\\$PATH\\\"\"")?;
            writeln!(output, "  echo \"  Ubuntu:  sudo apt-get install postgresql-client\"")?;
            writeln!(output, "  echo \"  Fedora:  sudo dnf install postgresql\"")?;
            writeln!(output, "  exit 1")?;
            writeln!(output, "fi")?;
            writeln!(output)?;
            writeln!(output, "# Load database URL from .env if it exists")?;
            writeln!(output, "if [ -f ../backend/.env ]; then")?;
            writeln!(output, "  export $(cat ../backend/.env | xargs)")?;
            writeln!(output, "fi")?;
            writeln!(output)?;
            writeln!(output, "if [ -z \"$DATABASE_URL\" ]; then")?;
            writeln!(output, "  echo \"Error: DATABASE_URL not set\"")?;
            writeln!(output, "  echo \"Please set DATABASE_URL environment variable or create ../backend/.env\"")?;
            writeln!(output, "  exit 1")?;
            writeln!(output, "fi")?;
            writeln!(output)?;
            writeln!(output, "echo \"Running PostgreSQL migrations...\"")?;
            writeln!(output, "psql \"$DATABASE_URL\" < 001_create_events_table.sql")?;
        }
        DatabaseType::MySQL | DatabaseType::MariaDB => {
            writeln!(output, "# Check if mysql is installed")?;
            writeln!(output, "if ! command -v mysql &> /dev/null; then")?;
            writeln!(output, "  echo \"Error: mysql command not found\"")?;
            writeln!(output, "  echo \"Please install MySQL client:\"")?;
            writeln!(output, "  echo \"  macOS:   brew install mysql-client && export PATH=\\\"/opt/homebrew/opt/mysql-client/bin:\\$PATH\\\"\"")?;
            writeln!(output, "  echo \"  Ubuntu:  sudo apt-get install mysql-client\"")?;
            writeln!(output, "  echo \"  Fedora:  sudo dnf install mysql\"")?;
            writeln!(output, "  exit 1")?;
            writeln!(output, "fi")?;
            writeln!(output)?;
            writeln!(output, "# Load database URL from .env if it exists")?;
            writeln!(output, "if [ -f ../backend/.env ]; then")?;
            writeln!(output, "  export $(cat ../backend/.env | xargs)")?;
            writeln!(output, "fi")?;
            writeln!(output)?;
            writeln!(output, "if [ -z \"$DATABASE_URL\" ]; then")?;
            writeln!(output, "  echo \"Error: DATABASE_URL not set\"")?;
            writeln!(output, "  echo \"Please set DATABASE_URL environment variable or create ../backend/.env\"")?;
            writeln!(output, "  exit 1")?;
            writeln!(output, "fi")?;
            writeln!(output)?;
            writeln!(output, "# Parse MySQL connection string")?;
            writeln!(output, "# Format: mysql://user:password@host:port/database")?;
            writeln!(output, "DB_USER=$(echo $DATABASE_URL | sed -n 's/mysql:\\/\\/\\([^:]*\\):.*/\\1/p')")?;
            writeln!(output, "DB_PASS=$(echo $DATABASE_URL | sed -n 's/mysql:\\/\\/[^:]*:\\([^@]*\\)@.*/\\1/p')")?;
            writeln!(output, "DB_HOST=$(echo $DATABASE_URL | sed -n 's/mysql:\\/\\/[^@]*@\\([^:]*\\):.*/\\1/p')")?;
            writeln!(output, "DB_PORT=$(echo $DATABASE_URL | sed -n 's/mysql:\\/\\/[^@]*@[^:]*:\\([^\\/]*\\)\\/.*/\\1/p')")?;
            writeln!(output, "DB_NAME=$(echo $DATABASE_URL | sed -n 's/.*\\/\\(.*\\)/\\1/p')")?;
            writeln!(output)?;
            writeln!(output, "echo \"Running MySQL migrations...\"")?;
            writeln!(output, "mysql -h \"$DB_HOST\" -P \"$DB_PORT\" -u \"$DB_USER\" -p\"$DB_PASS\" \"$DB_NAME\" < 001_create_events_table.sql")?;
        }
    }

    writeln!(output)?;
    writeln!(output, "echo \"✓ Migrations complete!\"")?;

    // Make script executable
    #[cfg(unix)]
    {
        use std::os::unix::fs::PermissionsExt;
        let metadata = output.metadata()?;
        let mut permissions = metadata.permissions();
        permissions.set_mode(0o755);
        std::fs::set_permissions(runner_path, permissions)?;
    }

    Ok(())
}
