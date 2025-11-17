/// Generate database.rs for connection pooling

use crate::codegen::EntityDef;
use super::{WorkerConfig, DatabaseType};
use std::path::Path;
use std::error::Error;
use std::io::Write;

/// Convert field names to snake_case for SQL
/// Handles both camelCase and already-snake_case inputs
fn to_snake_case(s: &str) -> String {
    // If already contains underscores (except f_ prefix), likely already snake_case
    let stripped = if s.starts_with("f_") {
        &s[2..]
    } else {
        s
    };

    // Check if already snake_case (has underscores or all lowercase)
    if stripped.contains('_') || stripped.chars().all(|c| !c.is_uppercase()) {
        return s.to_string();
    }

    // Convert camelCase to snake_case
    let mut result = String::new();
    let mut prev_lowercase = false;

    for (i, ch) in s.chars().enumerate() {
        if ch.is_uppercase() {
            // Add underscore before uppercase if:
            // 1. Not at start
            // 2. Previous char was lowercase
            // 3. OR next char is lowercase (handles "XMLParser" -> "xml_parser")
            if i > 0 && (prev_lowercase || s.chars().nth(i + 1).map_or(false, |c| c.is_lowercase())) {
                result.push('_');
            }
            result.push(ch.to_lowercase().next().unwrap());
            prev_lowercase = false;
        } else {
            result.push(ch);
            prev_lowercase = ch.is_lowercase();
        }
    }
    result
}

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
    writeln!(output)?;

    // PostgreSQL imports with feature gate
    writeln!(output, "#[cfg(feature = \"postgres\")]")?;
    writeln!(output, "use diesel::pg::PgConnection;")?;
    writeln!(output)?;

    // MySQL imports with feature gate
    writeln!(output, "#[cfg(feature = \"mysql\")]")?;
    writeln!(output, "use diesel::mysql::MysqlConnection;")?;
    writeln!(output)?;

    writeln!(output, "use r2d2::{{Pool, PooledConnection}};")?;
    writeln!(output, "use std::env;\n")?;

    // Connection type alias with feature gates
    writeln!(output, "#[cfg(feature = \"postgres\")]")?;
    writeln!(output, "pub type DbConnection = PgConnection;")?;
    writeln!(output)?;
    writeln!(output, "#[cfg(feature = \"mysql\")]")?;
    writeln!(output, "pub type DbConnection = MysqlConnection;")?;
    writeln!(output)?;

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
        if !entity.is_persistent(entities) || entity.is_abstract {
            continue;
        }

        // Skip reference entities (read from external sources, not stored locally)
        if entity.source_type.to_lowercase() == "reference" {
            continue;
        }

        let db_config = entity.get_database_config(entities).unwrap();
        let table_name = &db_config.conformant_table;

        writeln!(output, "    // Create {} table", entity.name)?;
        writeln!(output, "    diesel::sql_query(r#\"")?;
        writeln!(output, "        CREATE TABLE IF NOT EXISTS {} (", table_name)?;

        // Generate fields from persistence.field_overrides
        // Check this entity first, then parent entity via extends field (inheritance)
        let persistence_ref = if let Some(ref persistence) = entity.persistence {
            Some(persistence)
        } else if let Some(ref parent_name) = entity.extends {
            entities.iter()
                .find(|e| &e.name == parent_name)
                .and_then(|parent| parent.persistence.as_ref())
        } else {
            None
        };

        if let Some(persistence) = persistence_ref {
            let mut field_lines = Vec::new();

            // FIX 1: Add primary key column FIRST if autogenerate=true
            if let Some(ref pk_config) = persistence.primary_key {
                if pk_config.autogenerate {
                    let pk_type = match config.database_type {
                        DatabaseType::PostgreSQL => {
                            match pk_config.key_type.as_str() {
                                "i64" | "BigInt" => "BIGSERIAL",
                                _ => "SERIAL"
                            }
                        },
                        DatabaseType::MySQL | DatabaseType::MariaDB => {
                            match pk_config.key_type.as_str() {
                                "i64" | "BigInt" => "BIGINT AUTO_INCREMENT",
                                _ => "INT AUTO_INCREMENT"
                            }
                        }
                    };
                    field_lines.push(format!(
                        "            {} {} PRIMARY KEY",
                        to_snake_case(&pk_config.name),
                        pk_type
                    ));
                }
            }

            // FIX 2: Add all field_overrides with proper SQL type mapping
            for field in &persistence.field_overrides {
                let col_name = to_snake_case(&field.name);
                let field_type_str = field.field_type.as_deref().unwrap_or("String");
                let sql_type = match field_type_str {
                    // Note: TEXT works for both PostgreSQL and MySQL (MySQL supports TEXT since 5.0.3)
                    // MySQL TEXT has 64KB limit vs PostgreSQL's 1GB, but this is sufficient for most use cases
                    "String" => "TEXT",
                    "i32" | "Integer" => "INTEGER",
                    "i64" | "BigInt" => "BIGINT",
                    "f64" | "Float" | "Decimal" => "NUMERIC",
                    "bool" | "Boolean" => "BOOLEAN",
                    "NaiveDate" => "DATE",
                    "NaiveDateTime" | "DateTime" => "TIMESTAMP",
                    "Json" | "Object" | "List[Object]" => {
                        match config.database_type {
                            DatabaseType::PostgreSQL => "JSONB",
                            DatabaseType::MySQL | DatabaseType::MariaDB => "JSON",
                        }
                    },
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

            // FIX 3: Add composite UNIQUE constraint for unicity_fields
            if let Some(ref db_config) = persistence.database {
                if !db_config.unicity_fields.is_empty() {
                    let fields_list: Vec<String> = db_config.unicity_fields
                        .iter()
                        .map(|f| to_snake_case(f))
                        .collect();
                    writeln!(output, "            ,CONSTRAINT {}_unique UNIQUE ({})",
                        table_name,
                        fields_list.join(", ")
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
                        to_snake_case(unicity_field)
                    )?;
                    writeln!(output, "        ON {}({})",
                        table_name,
                        to_snake_case(unicity_field)
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
