//! Generic runtime for entity extraction and message parsing.
//!
//! This module provides format-agnostic infrastructure for parsing structured
//! messages and extracting entities in dependency order.

pub mod context;
pub mod message_parser;
pub mod config_loader;
pub mod transforms;
pub mod transform_registry;

// Re-export key types
pub use context::ExtractionContext;
pub use message_parser::{MessageParser, EntityExtractor};
pub use config_loader::{ParserConfig, compute_extraction_order};
pub use transforms::{
    TransformDef, TransformLanguage, Parameter, ReturnType,
    Implementation, TransformStep, TransformTest, TransformRegistry as TransformRegistryLoader,
    load_transform, load_transforms_from_dir
};
pub use transform_registry::TransformRegistry;
