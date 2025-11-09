//! # Nomnom: General-Purpose Data Transformation Library
//!
//! Nomnom provides a format-agnostic data parsing, transformation, and entity derivation
//! framework with YAML-based code generation.
//!
//! ## Features
//!
//! - **Format-agnostic entity framework**: Define entities for CSV, JSON, XML, EDI, or any structured format
//! - **Transform registry system**: Plugin architecture for registering transformation functions
//! - **Code generation**: Auto-generate Rust structs and Python bindings from YAML configs
//! - **Derivation patterns**: Support for parent, repeated, and multi-parent entity derivation
//! - **Python bridge**: Optional PyO3 integration for Python interop (feature: `python-bridge`)
//!
//! ## Example: CSV Parser
//!
//! ```yaml
//! entity:
//!   name: CsvRow
//!   source_type: derived
//!   parent: CsvFile
//!   fields:
//!     - name: first_name
//!       type: String
//!       computed_from:
//!         transform: extract_csv_field
//!         sources:
//!           - source: parent
//!             field: raw_line
//!         args:
//!           column_index: 0
//! ```
//!
//! ## Example: JSON Parser
//!
//! ```yaml
//! entity:
//!   name: User
//!   source_type: derived
//!   parent: JsonDocument
//!   fields:
//!     - name: username
//!       type: String
//!       computed_from:
//!         transform: extract_json_field
//!         sources:
//!           - source: parent
//!             field: raw_json
//!         args:
//!           json_path: "$.user.name"
//! ```

// Core modules
pub mod entity;
pub mod transform_registry;
pub mod extraction;
pub mod serialization;

// Generic runtime for message parsing and entity extraction
pub mod runtime;

// Optional Python bridge (feature-gated)
#[cfg(feature = "python-bridge")]
pub mod python_bridge;

// Code generation framework
pub mod codegen;

// Diesel ORM runtime infrastructure
pub mod diesel_runtime;

// NATS JetStream integration
pub mod nats;

// Re-export key types
pub use entity::{Entity, FieldValue, EntityError, Context, IntoOptionString};
pub use transform_registry::{TransformRegistry, TransformError};
pub use extraction::{FieldPath, Extractor};

// Re-export runtime types
pub use runtime::{
    ExtractionContext, MessageParser, EntityExtractor, ParserConfig, compute_extraction_order
};

// Re-export codegen types when building
pub use codegen::{EntityDef, FieldDef, ComputedFrom};

// Re-export diesel_runtime types
pub use diesel_runtime::{Database, DatabaseConfig, GetOrCreate, BulkInsert};

// Re-export nats types
pub use nats::{MessageEnvelope, IngestionResponse, IngestionStatus, NatsClient, NatsConfig};

#[cfg(feature = "python-bridge")]
pub use diesel_runtime::PyDatabase;

#[cfg(feature = "python-bridge")]
pub use python_bridge::PyTransformRegistry;
