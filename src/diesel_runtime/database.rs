//! Database connection management and operations
//!
//! This module provides Diesel-based database connectivity with connection pooling.

use diesel::prelude::*;
use diesel::r2d2::{self, ConnectionManager};
use std::sync::Arc;
use std::time::Duration;

// Conditional imports based on database backend
#[cfg(feature = "postgres")]
use diesel::pg::PgConnection;
#[cfg(feature = "postgres")]
pub type DbConnection = PgConnection;

#[cfg(feature = "mysql")]
use diesel::mysql::MysqlConnection;
#[cfg(feature = "mysql")]
pub type DbConnection = MysqlConnection;

pub type Pool = r2d2::Pool<ConnectionManager<DbConnection>>;
pub type PooledConnection = r2d2::PooledConnection<ConnectionManager<DbConnection>>;

/// Database connection pool manager
pub struct Database {
    pool: Arc<Pool>,
}

impl Database {
    /// Create a new database connection pool
    ///
    /// # Arguments
    /// * `database_url` - Database connection string (e.g., "postgres://user:pass@localhost/db" or "mysql://user:pass@localhost/db")
    ///
    /// # Returns
    /// Database instance with connection pool
    ///
    /// # Example
    /// ```ignore
    /// let db = Database::new("postgres://postgres:password@localhost:5432/test_db")?;
    /// // or for MySQL:
    /// let db = Database::new("mysql://root:password@localhost:3306/test_db")?;
    /// ```
    pub fn new(database_url: &str) -> Result<Self, Box<dyn std::error::Error>> {
        Self::new_with_config(database_url, DatabaseConfig::default())
    }

    /// Create a new database with custom configuration
    pub fn new_with_config(
        database_url: &str,
        config: DatabaseConfig,
    ) -> Result<Self, Box<dyn std::error::Error>> {
        let manager = ConnectionManager::<DbConnection>::new(database_url);

        let pool = r2d2::Pool::builder()
            .max_size(config.max_connections)
            .min_idle(Some(config.min_idle))
            .connection_timeout(Duration::from_secs(config.connection_timeout_secs))
            .idle_timeout(Some(Duration::from_secs(config.idle_timeout_secs)))
            .max_lifetime(Some(Duration::from_secs(config.max_lifetime_secs)))
            .build(manager)?;

        Ok(Database {
            pool: Arc::new(pool),
        })
    }

    /// Get a connection from the pool
    pub fn get_connection(&self) -> Result<PooledConnection, r2d2::PoolError> {
        self.pool.get()
    }

    /// Test database connectivity
    pub fn test_connection(&self) -> Result<(), Box<dyn std::error::Error>> {
        let mut conn = self.get_connection()?;
        diesel::sql_query("SELECT 1").execute(&mut conn)?;
        Ok(())
    }

    /// Get a reference to the connection pool
    pub fn pool(&self) -> &Pool {
        &self.pool
    }
}

/// Database configuration options
#[derive(Debug, Clone)]
pub struct DatabaseConfig {
    pub max_connections: u32,
    pub min_idle: u32,
    pub connection_timeout_secs: u64,
    pub idle_timeout_secs: u64,
    pub max_lifetime_secs: u64,
}

impl Default for DatabaseConfig {
    fn default() -> Self {
        DatabaseConfig {
            max_connections: 15,      // Max connections in pool
            min_idle: 5,              // Keep minimum idle connections
            connection_timeout_secs: 30,  // Wait up to 30s for connection
            idle_timeout_secs: 600,   // Close idle connections after 10 min
            max_lifetime_secs: 1800,  // Recycle connections after 30 min
        }
    }
}
