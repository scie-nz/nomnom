//! PyO3 wrapper for Database connection pool
//!
//! Exposes Diesel database operations to Python.
//!
//! This module is only available when the `python-bridge` feature is enabled.

use pyo3::prelude::*;
use crate::diesel_runtime::{Database, DatabaseConfig, PooledConnection};

/// Python-exposed database connection pool
///
/// # Example (Python)
///
/// ```python
/// from my_module import Database
///
/// # Create database with default config
/// db = Database("mysql://user:pass@localhost/mydb")
///
/// # Or with custom config
/// db = Database.with_config(
///     "mysql://user:pass@localhost/mydb",
///     max_connections=20,
///     min_idle=10
/// )
///
/// # Test connectivity
/// db.test_connection()
/// ```
#[pyclass(name = "Database")]
pub struct PyDatabase {
    db: Database,
}

#[pymethods]
impl PyDatabase {
    /// Create a new database connection pool
    ///
    /// Args:
        ///     database_url: MySQL connection string (e.g., "mysql://user:pass@localhost/db")
    ///
    /// Returns:
    ///     Database instance with connection pool
    #[new]
    pub fn new(database_url: &str) -> PyResult<Self> {
        let db = Database::new(database_url)
            .map_err(|e| PyErr::new::<pyo3::exceptions::PyRuntimeError, _>(
                format!("Failed to create database: {}", e)
            ))?;

        Ok(PyDatabase { db })
    }

    /// Create database with custom configuration
    ///
    /// Args:
    ///     database_url: MySQL connection string
    ///     max_connections: Maximum connections in pool (default: 15)
    ///     min_idle: Minimum idle connections (default: 5)
    ///     connection_timeout_secs: Connection timeout in seconds (default: 30)
    ///
    /// Returns:
    ///     Database instance with connection pool
    #[staticmethod]
    #[pyo3(signature = (database_url, max_connections=15, min_idle=5, connection_timeout_secs=30))]
    pub fn with_config(
        database_url: &str,
        max_connections: u32,
        min_idle: u32,
        connection_timeout_secs: u64,
    ) -> PyResult<Self> {
        let config = DatabaseConfig {
            max_connections,
            min_idle,
            connection_timeout_secs,
            idle_timeout_secs: 600,
            max_lifetime_secs: 1800,
        };

        let db = Database::new_with_config(database_url, config)
            .map_err(|e| PyErr::new::<pyo3::exceptions::PyRuntimeError, _>(
                format!("Failed to create database: {}", e)
            ))?;

        Ok(PyDatabase { db })
    }

    /// Test database connectivity
    ///
    /// Raises:
    ///     RuntimeError: If connection test fails
    pub fn test_connection(&self) -> PyResult<()> {
        self.db.test_connection()
            .map_err(|e| PyErr::new::<pyo3::exceptions::PyRuntimeError, _>(
                format!("Connection test failed: {}", e)
            ))
    }

    fn __repr__(&self) -> String {
        "Database(connected=True)".to_string()
    }
}

impl PyDatabase {
    /// Internal method to get a connection from the pool
    ///
    /// Not exposed to Python - used by entity get_or_create methods in generated code
    pub fn get_connection(&self) -> PyResult<PooledConnection> {
        self.db.get_connection()
            .map_err(|e| PyErr::new::<pyo3::exceptions::PyRuntimeError, _>(
                format!("Failed to get connection: {}", e)
            ))
    }
}
