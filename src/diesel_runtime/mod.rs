//! Diesel ORM runtime infrastructure
//!
//! This module provides generic database connection pooling and operation traits
//! for use with Diesel ORM. It complements the codegen module which generates
//! entity-specific implementations.
//!
//! # Features
//!
//! - `diesel-runtime`: Core Diesel database and connection pool (required)
//! - `python-bridge`: PyO3 bindings for Python interop (optional)

pub mod database;
pub mod operations;

#[cfg(feature = "python-bridge")]
pub mod python;

// Re-export key types
pub use database::{Database, DatabaseConfig, Pool, PooledConnection};
pub use operations::{GetOrCreate, BulkInsert};

#[cfg(feature = "python-bridge")]
pub use python::PyDatabase;
