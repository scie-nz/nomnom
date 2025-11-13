/// Generate database.rs for connection pooling

use crate::codegen::EntityDef;
use super::{WorkerConfig, DatabaseType};
use std::path::Path;
use std::error::Error;
use std::io::Write;

pub fn generate_database_rs(
    entities: &[EntityDef],
    output_dir: &Path,
    config: &WorkerConfig,
) -> Result<(), Box<dyn Error>> {
    let database_file = output_dir.join("src/database.rs");
    let mut output = std::fs::File::create(&database_file)?;

    writeln!(output, "// Auto-generated database connection pooling")?;
    writeln!(output)?;
    writeln!(output, "use diesel::prelude::*;")?;

    match config.database_type {
        DatabaseType::PostgreSQL => {
            writeln!(output, "use diesel::pg::PgConnection;")?;
        }
        DatabaseType::MySQL | DatabaseType::MariaDB => {
            writeln!(output, "use diesel::mysql::MysqlConnection;")?;
        }
    }

    writeln!(output, "use r2d2::{{Pool, PooledConnection}};")?;
    writeln!(output, "use std::env;\n")?;

    // Connection type alias
    match config.database_type {
        DatabaseType::PostgreSQL => {
            writeln!(output, "pub type DbConnection = PgConnection;")?;
        }
        DatabaseType::MySQL | DatabaseType::MariaDB => {
            writeln!(output, "pub type DbConnection = MysqlConnection;")?;
        }
    }
    writeln!(output, "pub type DbPool = Pool<diesel::r2d2::ConnectionManager<DbConnection>>;")?;
    writeln!(output, "pub type DbPooledConnection = PooledConnection<diesel::r2d2::ConnectionManager<DbConnection>>;\n")?;

    // Create pool function
    writeln!(output, "/// Create database connection pool")?;
    writeln!(output, "pub fn create_pool() -> Result<DbPool, Box<dyn std::error::Error>> {{")?;
    writeln!(output, "    let database_url = env::var(\"DATABASE_URL\")")?;
    writeln!(output, "        .expect(\"DATABASE_URL must be set\");\n")?;

    writeln!(output, "    let manager = diesel::r2d2::ConnectionManager::<DbConnection>::new(database_url);\n")?;

    writeln!(output, "    let pool = Pool::builder()")?;
    writeln!(output, "        .max_size(10)")?;
    writeln!(output, "        .build(manager)?;\n")?;

    writeln!(output, "    Ok(pool)")?;
    writeln!(output, "}}\n")?;

    // Add ensure_tables function
    writeln!(output, "/// Ensure database tables exist")?;
    writeln!(output, "pub fn ensure_tables(conn: &mut DbConnection) -> Result<(), Box<dyn std::error::Error>> {{")?;
    writeln!(output, "    tracing::info!(\"Ensuring database tables exist...\");\n")?;
    writeln!(output, "    // Note: In production, use proper migrations instead of CREATE TABLE IF NOT EXISTS")?;
    writeln!(output, "    // This is a convenience for development/testing\n")?;

    // Generate CREATE TABLE statements for each persistent entity
    for entity in entities {
        // Skip entities without persistence or abstract entities
        if !entity.is_persistent() || entity.is_abstract {
            continue;
        }

        // Skip reference entities (read from external sources, not stored locally)
        if entity.source_type.to_lowercase() == "reference" {
            continue;
        }

        let db_config = entity.get_database_config().unwrap();
        let table_name = &db_config.conformant_table;

        writeln!(output, "    // Create {} table", entity.name)?;
        writeln!(output, "    diesel::sql_query(r#\"")?;
        writeln!(output, "        CREATE TABLE IF NOT EXISTS {} (", table_name)?;

        // Generate fields from persistence.field_overrides
        if let Some(ref persistence) = entity.persistence {
            let mut field_lines = Vec::new();

            // FIX 1: Add primary key column FIRST if autogenerate=true
            if let Some(ref pk_config) = persistence.primary_key {
                if pk_config.autogenerate {
                    let pk_type = match pk_config.key_type.as_str() {
                        "i64" | "BigInt" => "BIGSERIAL",
                        _ => "SERIAL"
                    };
                    field_lines.push(format!(
                        "            {} {} PRIMARY KEY",
                        pk_config.name.to_lowercase(),
                        pk_type
                    ));
                }
            }

            // FIX 2: Add all field_overrides with proper SQL type mapping
            for field in &persistence.field_overrides {
                let col_name = field.name.to_lowercase();
                let field_type_str = field.field_type.as_deref().unwrap_or("String");
                let sql_type = match field_type_str {
                    "String" => "TEXT",
                    "i32" | "Integer" => "INTEGER",
                    "i64" | "BigInt" => "BIGINT",
                    "f64" | "Float" | "Decimal" => "NUMERIC",
                    "bool" | "Boolean" => "BOOLEAN",
                    "NaiveDate" => "DATE",
                    "NaiveDateTime" | "DateTime" => "TIMESTAMP",
                    "Json" | "Object" | "List[Object]" => "JSONB",
                    _ => "TEXT",
                };

                let nullable = if field.nullable.unwrap_or(false) { "" } else { " NOT NULL" };
                field_lines.push(format!("            {} {}{}", col_name, sql_type, nullable));
            }

            for (i, line) in field_lines.iter().enumerate() {
                if i < field_lines.len() - 1 {
                    writeln!(output, "{},", line)?;
                } else {
                    writeln!(output, "{}", line)?;
                }
            }

            // FIX 3: Add UNIQUE constraints for unicity_fields
            if let Some(ref db_config) = persistence.database {
                for unicity_field in &db_config.unicity_fields {
                    writeln!(output, "            ,CONSTRAINT {}_{}_unique UNIQUE ({})",
                        table_name,
                        unicity_field.to_lowercase(),
                        unicity_field.to_lowercase()
                    )?;
                }
            }
        }

        writeln!(output, "        )")?;
        writeln!(output, "    \"#)")?;
        writeln!(output, "    .execute(conn)?;\n")?;

        // FIX 4: Create indices for unicity fields
        if let Some(ref persistence) = entity.persistence {
            if let Some(ref db_config) = persistence.database {
                for unicity_field in &db_config.unicity_fields {
                    writeln!(output, "    // Index for {}", unicity_field)?;
                    writeln!(output, "    diesel::sql_query(r#\"")?;
                    writeln!(output, "        CREATE INDEX IF NOT EXISTS idx_{}_{}",
                        table_name,
                        unicity_field.to_lowercase()
                    )?;
                    writeln!(output, "        ON {}({})",
                        table_name,
                        unicity_field.to_lowercase()
                    )?;
                    writeln!(output, "    \"#)")?;
                    writeln!(output, "    .execute(conn)?;\n")?;
                }
            }
        }
    }

    // Add message_status table for tracking message processing
    writeln!(output, "    // Create message_status table for tracking message processing")?;
    writeln!(output, "    diesel::sql_query(r#\"")?;
    writeln!(output, "        CREATE TABLE IF NOT EXISTS message_status (")?;
    writeln!(output, "            message_id UUID PRIMARY KEY,")?;
    writeln!(output, "            entity_type VARCHAR(50) NOT NULL,")?;
    writeln!(output, "            status VARCHAR(20) NOT NULL,")?;
    writeln!(output, "            received_at TIMESTAMP NOT NULL,")?;
    writeln!(output, "            processed_at TIMESTAMP,")?;
    writeln!(output, "            retry_count INTEGER DEFAULT 0,")?;
    writeln!(output, "            error_message TEXT,")?;
    writeln!(output, "            source VARCHAR(255)")?;
    writeln!(output, "        )")?;
    writeln!(output, "    \"#)")?;
    writeln!(output, "    .execute(conn)?;\n")?;

    writeln!(output, "    // Create indices for message_status")?;
    writeln!(output, "    diesel::sql_query(r#\"")?;
    writeln!(output, "        CREATE INDEX IF NOT EXISTS idx_message_status_received_at ON message_status(received_at)")?;
    writeln!(output, "    \"#)")?;
    writeln!(output, "    .execute(conn)?;\n")?;

    writeln!(output, "    diesel::sql_query(r#\"")?;
    writeln!(output, "        CREATE INDEX IF NOT EXISTS idx_message_status_status ON message_status(status)")?;
    writeln!(output, "    \"#)")?;
    writeln!(output, "    .execute(conn)?;\n")?;

    writeln!(output, "    tracing::info!(\"All tables ensured\");")?;
    writeln!(output, "    Ok(())")?;
    writeln!(output, "}}")?;

    Ok(())
}
