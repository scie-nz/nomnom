/// Generate database.rs for connection pooling

use super::{IngestionServerConfig, DatabaseType};
use std::path::Path;
use std::error::Error;
use std::io::Write;

pub fn generate_database_rs(
    output_dir: &Path,
    config: &IngestionServerConfig,
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
    writeln!(output, "}}")?;

    Ok(())
}
