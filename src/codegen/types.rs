//! Type definitions for entity configuration.
//!
//! These types represent the structure of entity YAML configurations
//! and are used during code generation.

use serde::{Deserialize, Serialize};
use std::collections::HashMap;

fn default_fk_nullable() -> bool {
    true
}

fn default_nullable() -> bool {
    false
}

/// Wrapper for entity YAML structure
#[derive(Debug, Clone, Deserialize, Serialize)]
pub struct EntitySpec {
    pub entity: EntityDef,
}

/// Foreign key configuration for database relationships
#[derive(Debug, Clone, Deserialize, Serialize)]
pub struct ForeignKeyConfig {
    pub name: String,
    pub references: String,  // Format: "table.column"
    pub parent_entity: String,
    #[serde(default = "default_fk_nullable")]
    pub nullable: bool,
}

/// Database configuration for persistent entities
#[derive(Debug, Clone, Deserialize, Serialize)]
#[serde(deny_unknown_fields)]
pub struct DatabaseConfig {
    #[serde(default)]
    pub legacy_table: Option<String>,
    pub conformant_table: String,
    #[serde(default)]
    pub legacy_id_column: Option<String>,
    pub conformant_id_column: String,
    #[serde(default)]
    pub autogenerate_conformant_id: bool,
    #[serde(default)]
    pub unicity_fields: Vec<String>,
    #[serde(default)]
    pub foreign_keys: Vec<ForeignKeyConfig>,
}

/// Field override configuration for persistence
#[derive(Debug, Clone, Deserialize, Serialize)]
pub struct FieldOverride {
    pub name: String,
    #[serde(rename = "type")]
    pub field_type: Option<String>,
    #[serde(default)]
    pub args: Vec<serde_yaml::Value>,
    #[serde(default)]
    pub nullable: Option<bool>,
    #[serde(default)]
    pub primary_key: Option<bool>,
    #[serde(default)]
    pub index: Option<bool>,
    #[serde(default)]
    pub doc: Option<String>,
}

/// Persistence configuration wrapper
#[derive(Debug, Clone, Deserialize, Serialize)]
#[serde(deny_unknown_fields)]
pub struct PersistenceConfig {
    #[serde(default)]
    pub database: Option<DatabaseConfig>,
    #[serde(default)]
    pub primary_key: Option<PrimaryKeyConfig>,
    #[serde(default)]
    pub field_overrides: Vec<FieldOverride>,
}

/// Derivation configuration for derived entities
#[derive(Debug, Clone, Deserialize, Serialize)]
pub struct DerivationConfig {
    /// Source entities this entity derives from (1 or more)
    #[serde(default, alias = "source_entity", alias = "parent_entities")]
    pub source_entities: Option<serde_yaml::Value>,
}

/// Abstract method implementation
#[derive(Debug, Clone, Deserialize, Serialize)]
pub struct AbstractImplementation {
    pub lambda: String,
}

/// Primary key configuration for auto-generated PKs
#[derive(Debug, Clone, Deserialize, Serialize)]
pub struct PrimaryKeyConfig {
    pub name: String,
    #[serde(rename = "type")]
    pub key_type: String,
    #[serde(default)]
    pub autogenerate: bool,
}

/// Extraction configuration for fields
#[derive(Debug, Clone, Deserialize, Serialize)]
pub struct ExtractionConfig {
    #[serde(default)]
    pub lambda: Option<String>,
    #[serde(default)]
    pub copy_from_context: bool,
    #[serde(default, alias = "copy_from_parent")]
    pub copy_from_source: Option<String>,
    #[serde(rename = "abstract", default)]
    pub abstract_method: Option<String>,
}

/// Computed field configuration
#[derive(Debug, Clone, Deserialize, Serialize)]
pub struct ComputedConfig {
    pub function: String,
    #[serde(default)]
    pub depends_on: Vec<String>,
}

/// Source type for an entity
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(rename_all = "snake_case")]
pub enum SourceType {
    /// Root entity (e.g., file-based source)
    Root,
    /// Derived from a parent entity
    Derived,
}

/// Repetition type for an entity
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(rename_all = "snake_case")]
pub enum Repetition {
    /// Single instance per parent
    Singleton,
    /// Multiple instances per parent
    Repeated,
}

/// Field type in entity definition
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub enum FieldType {
    String,
    Integer,
    Float,
    Boolean,
    DateTime,
    #[serde(rename = "List[String]")]
    ListString,
}

/// Source specification for computed fields
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(untagged)]
pub enum FieldSource {
    /// Reference to parent field
    Parent {
        #[serde(alias = "parent")]
        source: String,
        field: String,
        #[serde(default)]
        alias: Option<String>,
    },
    /// Direct field reference (for repeated_for sources)
    Direct(String),
}

impl FieldSource {
    /// Get the source entity name
    pub fn source_name(&self) -> &str {
        match self {
            FieldSource::Direct(name) => name,
            FieldSource::Parent { source, .. } => source,
        }
    }

    /// Get the field name (None for Direct sources)
    pub fn field_name(&self) -> Option<&str> {
        match self {
            FieldSource::Direct(_) => None,
            FieldSource::Parent { field, .. } => Some(field),
        }
    }
}

/// Computed field specification
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ComputedFrom {
    /// Transform function to call
    pub transform: String,
    /// Source fields/entities
    pub sources: Vec<FieldSource>,
    /// Additional arguments to pass to transform
    #[serde(default)]
    pub args: Option<serde_yaml::Value>,
}

/// Field definition in entity YAML
#[derive(Debug, Clone, Serialize, Deserialize, Default)]
#[serde(deny_unknown_fields)]
pub struct FieldDef {
    /// Field name
    pub name: String,
    /// Field type string (e.g., "String", "Integer")
    #[serde(rename = "type")]
    pub field_type: String,
    /// Whether field is nullable
    #[serde(default = "default_nullable")]
    pub nullable: bool,
    /// Computed field specification (for generic entities)
    #[serde(default)]
    pub computed_from: Option<ComputedFrom>,
    /// Documentation string
    #[serde(default)]
    pub doc: Option<String>,
    /// Database-specific: whether this is a primary key
    #[serde(default)]
    pub primary_key: bool,
    /// Database-specific: whether this field is indexed
    #[serde(default)]
    pub index: bool,
    /// Extraction configuration (for permanent entities)
    #[serde(default)]
    pub extraction: Option<ExtractionConfig>,
    /// Computed field configuration (for permanent entities)
    #[serde(default)]
    pub computed: Option<ComputedConfig>,
    /// Field arguments (e.g., String length)
    #[serde(default)]
    pub args: Option<Vec<serde_yaml::Value>>,
    /// Constant value
    #[serde(default)]
    pub constant: Option<serde_yaml::Value>,
    /// Domain-specific: segment field path (e.g., for HL7 segments)
    #[serde(default)]
    pub segment_field: Option<Vec<usize>>,
    /// Domain-specific: filename component extraction config
    #[serde(default)]
    pub filename_component: Option<serde_yaml::Value>,
    /// Domain-specific: derived-from configuration
    #[serde(default)]
    pub derived_from: Option<DerivedFrom>,
    /// Domain-specific: root source (e.g., "raw" for raw_message field)
    #[serde(default)]
    pub root_source: Option<String>,
}

/// Repeated-for specification (for repeated derived entities)
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RepeatedFor {
    /// Parent entity name
    pub entity: String,
    /// Field in parent that contains repeated data
    pub field: String,
    /// Name to use for each item
    pub each_known_as: String,
}

/// Parent definition with source metadata
#[derive(Debug, Clone, Deserialize, Serialize, Default)]
#[serde(deny_unknown_fields)]
pub struct ParentDef {
    #[serde(default)]
    pub name: String,
    #[serde(rename = "type", default)]
    pub parent_type: String,
    /// Source type (e.g., "transient", "permanent")
    #[serde(default)]
    pub source: String,
    #[serde(default)]
    pub doc: Option<String>,
    /// For union semantics: indicates this source comes from the same data segment as another
    /// Example: multiple optional fields from the same HL7 segment, CSV row, or JSON object
    #[serde(default)]
    pub same_segment_as: Option<String>,
}

/// Derived-from configuration for simple field derivation
#[derive(Debug, Clone, Deserialize, Serialize)]
pub struct DerivedFrom {
    #[serde(alias = "parent")]
    pub source: String,
    pub field: String,
    #[serde(default)]
    pub transform: Option<String>,
}

/// Entity definition from YAML
#[derive(Debug, Clone, Serialize, Deserialize, Default)]
#[serde(deny_unknown_fields)]
pub struct EntityDef {
    /// Entity name
    #[serde(default)]
    pub name: String,
    /// Source type string (e.g., "root", "derived", "segment", "filename")
    #[serde(default, alias = "type")]
    pub source_type: String,
    /// Repetition (singleton or repeated)
    #[serde(default)]
    pub repetition: Option<String>,
    /// Parent entity (for simple single-parent derivation)
    #[serde(default)]
    pub parent: Option<String>,
    /// Parents (for multi-parent derived entities with metadata)
    #[serde(default)]
    pub parents: Vec<ParentDef>,
    /// Repeated-for specification
    #[serde(default)]
    pub repeated_for: Option<RepeatedFor>,
    /// Entity fields
    #[serde(default)]
    pub fields: Vec<FieldDef>,
    /// Documentation string
    #[serde(default)]
    pub doc: Option<String>,
    /// Persistence configuration - if present with database, this is a persistent entity
    #[serde(default)]
    pub persistence: Option<PersistenceConfig>,
    /// Database configuration - DEPRECATED: use persistence.database instead (kept for backward compatibility)
    #[serde(default)]
    pub database: Option<DatabaseConfig>,
    /// Derivation configuration - for derived entities
    #[serde(default)]
    pub derivation: Option<DerivationConfig>,
    /// Whether this entity is abstract (base class)
    #[serde(rename = "abstract", default)]
    pub is_abstract: bool,
    /// Parent class to extend (inheritance)
    #[serde(default)]
    pub extends: Option<String>,
    /// Concrete implementations of abstract extraction methods
    #[serde(default)]
    pub abstract_implementations: Option<HashMap<String, AbstractImplementation>>,
    /// Serialization methods to generate
    #[serde(default)]
    pub serialization: Vec<String>,
    /// Message prefix for ingestion server parsing (e.g., "O" for Order, "L" for LineItem)
    #[serde(default)]
    pub prefix: Option<String>,
}

impl EntityDef {
    /// Get parent entity names (single or multiple)
    pub fn get_parents(&self) -> Vec<String> {
        if !self.parents.is_empty() {
            // Use parent_type (PascalCase entity type) not name (snake_case param name)
            self.parents.iter().map(|p| p.parent_type.clone()).collect()
        } else if let Some(ref parent) = self.parent {
            vec![parent.clone()]
        } else if let Some(ref repeated_for) = self.repeated_for {
            vec![repeated_for.entity.clone()]
        } else if let Some(ref derivation) = self.derivation {
            // Parse source_entities from derivation config
            if let Some(ref source_entities) = derivation.source_entities {
                match source_entities {
                    // Single parent: "ParentEntity"
                    serde_yaml::Value::String(parent) => vec![parent.clone()],
                    // Multiple parents: {mpi: MPI, filename: Filename, ...}
                    serde_yaml::Value::Mapping(map) => {
                        map.values()
                            .filter_map(|v| v.as_str().map(String::from))
                            .collect()
                    }
                    _ => vec![]
                }
            } else {
                vec![]
            }
        } else {
            vec![]
        }
    }

    /// Check if entity is a root entity
    pub fn is_root(&self) -> bool {
        self.source_type.to_lowercase() == "root"
    }

    /// Check if entity is derived
    pub fn is_derived(&self) -> bool {
        self.source_type.to_lowercase() == "derived"
    }

    /// Check if entity has multiple parents
    pub fn has_multiple_parents(&self) -> bool {
        self.parents.len() > 1 || (self.parent.is_none() && self.derivation.is_some())
    }

    /// Get database configuration from either persistence.database or legacy database field
    pub fn get_database_config(&self) -> Option<&DatabaseConfig> {
        // Prefer persistence.database (new format)
        if let Some(ref persistence) = self.persistence {
            if let Some(ref db) = persistence.database {
                return Some(db);
            }
        }
        // Fall back to legacy database field
        self.database.as_ref()
    }

    /// Check if entity is persistent (has database configuration)
    pub fn is_persistent(&self) -> bool {
        self.get_database_config().is_some()
    }

    /// Check if this entity derives from the specified ancestor (directly or indirectly)
    /// Used to build entity hierarchies for derived entity processing
    pub fn derives_from(&self, ancestor_name: &str, all_entities: &[EntityDef]) -> bool {
        // Get immediate parents
        let parents = self.get_parents();

        // Check if ancestor is an immediate parent
        if parents.iter().any(|p| p == ancestor_name) {
            return true;
        }

        // Recursively check if any parent derives from ancestor
        for parent_name in parents {
            if let Some(parent_entity) = all_entities.iter().find(|e| e.name == parent_name) {
                if parent_entity.derives_from(ancestor_name, all_entities) {
                    return true;
                }
            }
        }

        false
    }

    /// Find the field in a root entity that contains/generates this derived entity
    /// Returns the field name if this entity is derived from a repeating field, None otherwise
    pub fn find_source_field_in_root(&self, root_entity: &EntityDef) -> Option<String> {
        // Check if this entity has a repeated_for config pointing to a field
        if let Some(ref repeated_for) = self.repeated_for {
            return Some(repeated_for.field.clone());
        }

        // Check if this derives from a parent that has a repeated_for
        let parents = self.get_parents();
        for parent_name in parents {
            // Look for the parent's definition to check its source field
            // This would need access to all entities, which we'll handle in the codegen
            // For now, return None to indicate single-instance derivation
        }

        None
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_entity_def_get_parents() {
        let entity = EntityDef {
            name: "TestEntity".to_string(),
            parent: Some("ParentEntity".to_string()),
            ..Default::default()
        };

        assert_eq!(entity.get_parents(), vec!["ParentEntity".to_string()]);
    }

    #[test]
    fn test_entity_def_multiple_parents() {
        let entity = EntityDef {
            name: "TestEntity".to_string(),
            parents: vec![
                ParentDef {
                    name: "parent1".to_string(),
                    parent_type: "Parent1".to_string(),
                    ..Default::default()
                },
                ParentDef {
                    name: "parent2".to_string(),
                    parent_type: "Parent2".to_string(),
                    ..Default::default()
                }
            ],
            ..Default::default()
        };

        assert!(entity.has_multiple_parents());
        assert_eq!(
            entity.get_parents(),
            vec!["Parent1".to_string(), "Parent2".to_string()]
        );
    }
}
