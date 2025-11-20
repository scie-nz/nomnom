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

/// Source entity specification
#[derive(Debug, Clone, Deserialize, Serialize)]
#[serde(untagged)]
pub enum SourceEntitySpec {
    /// Simple string format: just entity name (defaults to core)
    Simple(String),

    /// Detailed format: with configuration
    Detailed {
        entity: String,
        #[serde(default)]
        ancillary: bool,
    },
}

impl SourceEntitySpec {
    /// Get the entity name
    pub fn entity_name(&self) -> &str {
        match self {
            SourceEntitySpec::Simple(name) => name,
            SourceEntitySpec::Detailed { entity, .. } => entity,
        }
    }

    /// Check if this source is ancillary
    pub fn is_ancillary(&self) -> bool {
        match self {
            SourceEntitySpec::Simple(_) => false,  // Default to core
            SourceEntitySpec::Detailed { ancillary, .. } => *ancillary,
        }
    }
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

/// Condition specification for conditional field copying
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct FieldCondition {
    /// Field to check (from parent entity or self)
    pub field: FieldSource,
    /// Expected value to match
    pub equals: String,
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
    /// Condition for conditional field copying (copy_field_conditional)
    #[serde(default)]
    pub condition: Option<FieldCondition>,
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

/// Minimal existence constraint for entity creation
/// Specifies which fields must be non-empty for the entity to be created
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MinimalExistence {
    /// Require at least ONE of these fields to be non-empty (OR logic)
    #[serde(default)]
    pub require_any: Option<Vec<String>>,

    /// Require ALL of these fields to be non-empty (AND logic)
    #[serde(default)]
    pub require_all: Option<Vec<String>>,
}

impl MinimalExistence {
    /// Get the list of fields to check
    pub fn fields(&self) -> Option<&Vec<String>> {
        self.require_any.as_ref().or(self.require_all.as_ref())
    }

    /// Check if this is require_any (OR) logic
    pub fn is_require_any(&self) -> bool {
        self.require_any.is_some()
    }

    /// Validate the minimal existence configuration
    pub fn validate(&self, entity: &EntityDef) -> Result<(), String> {
        // Rule: Cannot have both require_any and require_all
        if self.require_any.is_some() && self.require_all.is_some() {
            return Err(format!(
                "Entity '{}': Cannot specify both require_any and require_all in minimal_existence",
                entity.name
            ));
        }

        // Get fields to check
        let fields_to_check = self.fields()
            .ok_or_else(|| format!(
                "Entity '{}': minimal_existence must specify either require_any or require_all",
                entity.name
            ))?;

        // Rule: At least one field required
        if fields_to_check.is_empty() {
            return Err(format!(
                "Entity '{}': minimal_existence must specify at least one field",
                entity.name
            ));
        }

        // Rule: All fields must exist in entity
        for field_name in fields_to_check {
            let field = entity.fields.iter()
                .find(|f| &f.name == field_name)
                .ok_or_else(|| format!(
                    "Entity '{}': Field '{}' in minimal_existence not found in entity definition",
                    entity.name, field_name
                ))?;

            // Warning: computed fields
            if field.computed.is_some() {
                eprintln!(
                    "WARNING: Entity '{}': Field '{}' in minimal_existence is computed. \
                    Consider using source fields instead.",
                    entity.name, field_name
                );
            }
        }

        Ok(())
    }
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
    /// Minimal existence constraint - which fields must be present for entity to exist
    #[serde(default)]
    pub minimal_existence: Option<MinimalExistence>,
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

    /// Get source entity specs with ancillary information
    /// Returns a map from alias to (entity_name, is_ancillary)
    pub fn get_source_entity_specs(&self) -> std::collections::HashMap<String, (String, bool)> {
        use std::collections::HashMap;

        let mut specs = HashMap::new();

        if let Some(ref derivation) = self.derivation {
            if let Some(ref source_entities) = derivation.source_entities {
                match source_entities {
                    // Single source: "ParentEntity"
                    serde_yaml::Value::String(entity_name) => {
                        specs.insert(entity_name.clone(), (entity_name.clone(), false));
                    }
                    // Multiple sources: {alias: EntityName} or {alias: {entity: EntityName, ancillary: bool}}
                    serde_yaml::Value::Mapping(map) => {
                        for (key, value) in map {
                            if let Some(alias) = key.as_str() {
                                match value {
                                    // Simple string: alias: EntityName
                                    serde_yaml::Value::String(entity_name) => {
                                        specs.insert(alias.to_string(), (entity_name.clone(), false));
                                    }
                                    // Detailed object: alias: {entity: EntityName, ancillary: true}
                                    serde_yaml::Value::Mapping(obj) => {
                                        let entity_name = obj.get(&serde_yaml::Value::String("entity".to_string()))
                                            .and_then(|v| v.as_str())
                                            .map(String::from);
                                        let ancillary = obj.get(&serde_yaml::Value::String("ancillary".to_string()))
                                            .and_then(|v| v.as_bool())
                                            .unwrap_or(false);

                                        if let Some(entity_name) = entity_name {
                                            specs.insert(alias.to_string(), (entity_name, ancillary));
                                        }
                                    }
                                    _ => {}
                                }
                            }
                        }
                    }
                    _ => {}
                }
            }
        }

        specs
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
    /// Also checks parent entities via the extends field for inheritance
    pub fn get_database_config<'a>(&'a self, all_entities: &'a [EntityDef]) -> Option<&'a DatabaseConfig> {
        // Prefer persistence.database (new format)
        if let Some(ref persistence) = self.persistence {
            if let Some(ref db) = persistence.database {
                return Some(db);
            }
        }
        // Fall back to legacy database field
        if let Some(ref db) = self.database {
            return Some(db);
        }

        // Check parent entity via extends field (inheritance)
        if let Some(ref parent_name) = self.extends {
            if let Some(parent_entity) = all_entities.iter().find(|e| &e.name == parent_name) {
                return parent_entity.get_database_config(all_entities);
            }
        }

        None
    }

    /// Check if entity is persistent (has database configuration)
    /// Also checks parent entities via the extends field for inheritance
    pub fn is_persistent(&self, all_entities: &[EntityDef]) -> bool {
        self.get_database_config(all_entities).is_some()
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

    /// Validate ancillary source entity rules
    /// Returns Err if validation fails
    pub fn validate_ancillary_sources(&self, all_entities: &[EntityDef]) -> Result<(), String> {
        let source_specs = self.get_source_entity_specs();

        if source_specs.is_empty() {
            return Ok(());  // No source entities to validate
        }

        let mut has_core_entity = false;

        for (alias, (entity_name, is_ancillary)) in &source_specs {
            let source_entity = all_entities.iter()
                .find(|e| &e.name == entity_name)
                .ok_or(format!("Source entity '{}' not found for entity '{}'", entity_name, self.name))?;

            if *is_ancillary {
                // Rule: Ancillary entities must be singleton
                let is_repeated = source_entity.repetition.as_ref()
                    .map(|r| r.to_lowercase() == "repeated")
                    .unwrap_or(false);

                if is_repeated {
                    return Err(format!(
                        "Entity '{}': Ancillary source entity '{}' (alias '{}') must be singleton, but is repeated",
                        self.name, entity_name, alias
                    ));
                }
            } else {
                has_core_entity = true;
            }
        }

        // Rule: Must have at least one core entity
        if !has_core_entity {
            return Err(format!(
                "Entity '{}' must have at least one core (non-ancillary) source entity",
                self.name
            ));
        }

        Ok(())
    }
}

// ============================================================================
// Entity Schema v1 Types (K8s-inspired, camelCase)
// ============================================================================

/// Entity Schema v1 - K8s-style top-level structure
#[derive(Debug, Clone, Deserialize, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct EntityV1 {
    pub api_version: String,  // "nomnom.io/v1"
    pub kind: String,          // "Entity"
    pub metadata: MetadataV1,
    pub spec: SpecV1,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub status: Option<StatusV1>,
}

/// Metadata section (K8s-style)
#[derive(Debug, Clone, Deserialize, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct MetadataV1 {
    pub name: String,  // Entity name in PascalCase
    #[serde(default, skip_serializing_if = "HashMap::is_empty")]
    pub labels: HashMap<String, String>,
    #[serde(default, skip_serializing_if = "HashMap::is_empty")]
    pub annotations: HashMap<String, String>,
}

/// Spec section - entity specification
#[derive(Debug, Clone, Deserialize, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct SpecV1 {
    #[serde(rename = "type")]
    pub entity_type: String,  // "root" | "derived"
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub repetition: Option<String>,  // "singleton" | "repeated"
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub derivation: Option<DerivationV1>,
    #[serde(default)]
    pub fields: Vec<FieldDefV1>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub persistence: Option<PersistenceV1>,
}

/// Derivation configuration v1
#[derive(Debug, Clone, Deserialize, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct DerivationV1 {
    #[serde(skip_serializing_if = "Option::is_none")]
    pub parent: Option<String>,  // Single parent (simple)
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub parents: Vec<ParentDefV1>,  // Multiple parents
    #[serde(skip_serializing_if = "Option::is_none")]
    pub repeated_for: Option<RepeatedForV1>,
}

/// Parent definition v1
#[derive(Debug, Clone, Deserialize, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct ParentDefV1 {
    pub name: String,     // Variable name (camelCase)
    pub entity: String,   // Entity type (PascalCase)
    #[serde(skip_serializing_if = "Option::is_none")]
    pub doc: Option<String>,
}

/// Repeated-for specification v1
#[derive(Debug, Clone, Deserialize, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct RepeatedForV1 {
    pub entity: String,       // Parent entity name
    pub field: String,        // Field containing array (camelCase)
    pub item_name: String,    // Variable name for loop item
}

/// Field definition v1 - unified (no more overrides!)
#[derive(Debug, Clone, Deserialize, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct FieldDefV1 {
    pub name: String,         // Field name (camelCase)
    #[serde(rename = "type")]
    pub field_type: String,   // Logical type (lowercase)
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub constraints: Option<ConstraintsV1>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub source: Option<SourceV1>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub doc: Option<String>,
}

/// Field constraints v1 - unified database + validation
#[derive(Debug, Clone, Deserialize, Serialize, Default)]
#[serde(rename_all = "camelCase")]
pub struct ConstraintsV1 {
    #[serde(skip_serializing_if = "Option::is_none")]
    pub nullable: Option<bool>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub max_length: Option<usize>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub min_length: Option<usize>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub minimum: Option<serde_yaml::Value>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub maximum: Option<serde_yaml::Value>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub pattern: Option<String>,
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub r#enum: Vec<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub default: Option<serde_yaml::Value>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub primary_key: Option<bool>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub indexed: Option<bool>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub unique: Option<bool>,
    // Type modifiers
    #[serde(skip_serializing_if = "Option::is_none")]
    pub precision: Option<usize>,  // For decimal types
    #[serde(skip_serializing_if = "Option::is_none")]
    pub scale: Option<usize>,      // For decimal types
    #[serde(skip_serializing_if = "Option::is_none")]
    pub items: Option<Box<ConstraintsV1>>,  // For array types
}

/// Field source v1 - where data comes from
#[derive(Debug, Clone, Deserialize, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct SourceV1 {
    // Option 1: Copy from parent field
    #[serde(skip_serializing_if = "Option::is_none")]
    pub copy_from: Option<String>,  // Parent entity name
    #[serde(skip_serializing_if = "Option::is_none")]
    pub field: Option<String>,      // Field name in parent

    // Option 2: Transform/compute
    #[serde(skip_serializing_if = "Option::is_none")]
    pub transform: Option<String>,  // Transform function name
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub inputs: Vec<String>,        // Input sources
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub args: Vec<serde_yaml::Value>,  // Transform arguments

    // Option 3: Constant value
    #[serde(skip_serializing_if = "Option::is_none")]
    pub constant: Option<serde_yaml::Value>,
}

/// Persistence configuration v1
#[derive(Debug, Clone, Deserialize, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct PersistenceV1 {
    #[serde(default = "default_persistence_enabled")]
    pub enabled: bool,
    pub table: String,
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub indexes: Vec<IndexV1>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub unicity: Option<UnicityV1>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub legacy_mapping: Option<LegacyMappingV1>,
}

fn default_persistence_enabled() -> bool {
    true
}

/// Index definition v1
#[derive(Debug, Clone, Deserialize, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct IndexV1 {
    pub name: String,
    pub fields: Vec<String>,  // Field names (camelCase)
    #[serde(default)]
    pub unique: bool,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub method: Option<String>,  // "btree", "hash", "gin", "gist"
}

/// Unicity constraint v1 - for upsert logic
#[derive(Debug, Clone, Deserialize, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct UnicityV1 {
    pub fields: Vec<String>,  // Field names (camelCase)
}

/// Legacy table mapping v1 - for migration
#[derive(Debug, Clone, Deserialize, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct LegacyMappingV1 {
    #[serde(default)]
    pub enabled: bool,
    pub table: String,
    pub id_column: String,
}

/// Status section (runtime, managed by system)
#[derive(Debug, Clone, Deserialize, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct StatusV1 {
    #[serde(skip_serializing_if = "Option::is_none")]
    pub phase: Option<String>,
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub conditions: Vec<ConditionV1>,
}

/// Condition for status
#[derive(Debug, Clone, Deserialize, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct ConditionV1 {
    pub r#type: String,
    pub status: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub reason: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub message: Option<String>,
}

// Conversion helpers
impl EntityV1 {
    /// Convert v1 schema to legacy EntityDef for code generation
    pub fn to_legacy(&self) -> EntityDef {
        let mut entity = EntityDef {
            name: self.metadata.name.clone(),
            source_type: self.spec.entity_type.clone(),
            repetition: self.spec.repetition.clone(),
            fields: self.spec.fields.iter().map(|f| f.to_legacy()).collect(),
            doc: self.metadata.annotations.get("description").cloned(),
            ..Default::default()
        };

        // Convert derivation
        if let Some(ref derivation) = self.spec.derivation {
            entity.parent = derivation.parent.clone();
            if !derivation.parents.is_empty() {
                entity.parents = derivation.parents.iter().map(|p| ParentDef {
                    name: p.name.clone(),
                    parent_type: p.entity.clone(),
                    source: String::new(),
                    doc: p.doc.clone(),
                    same_segment_as: None,
                }).collect();
            }
            if let Some(ref repeated_for) = derivation.repeated_for {
                entity.repeated_for = Some(RepeatedFor {
                    entity: repeated_for.entity.clone(),
                    field: repeated_for.field.clone(),
                    each_known_as: repeated_for.item_name.clone(),
                });
            }
        }

        // Convert persistence
        if let Some(ref persistence) = self.spec.persistence {
            entity.database = Some(DatabaseConfig {
                legacy_table: None,
                conformant_table: persistence.table.clone(),
                legacy_id_column: None,
                conformant_id_column: "id".to_string(),  // Default
                autogenerate_conformant_id: false,
                unicity_fields: persistence.unicity
                    .as_ref()
                    .map(|u| u.fields.clone())
                    .unwrap_or_default(),
                foreign_keys: vec![],
            });

            // Generate field_overrides from v1 field constraints
            let field_overrides: Vec<FieldOverride> = self.spec.fields.iter()
                .filter_map(|field| {
                    if let Some(ref constraints) = field.constraints {
                        Some(FieldOverride {
                            name: field.name.clone(),
                            field_type: Some(field.field_type.clone()),
                            args: if let Some(max_len) = constraints.max_length {
                                vec![serde_yaml::Value::Number(max_len.into())]
                            } else {
                                vec![]
                            },
                            nullable: constraints.nullable,
                            primary_key: constraints.primary_key,
                            index: constraints.indexed,
                            doc: field.doc.clone(),
                        })
                    } else {
                        // Include field even without constraints
                        Some(FieldOverride {
                            name: field.name.clone(),
                            field_type: Some(field.field_type.clone()),
                            args: vec![],
                            nullable: Some(false),  // Default
                            primary_key: None,
                            index: None,
                            doc: field.doc.clone(),
                        })
                    }
                })
                .collect();

            entity.persistence = Some(PersistenceConfig {
                database: entity.database.clone(),
                primary_key: None,
                field_overrides,
            });
        }

        entity
    }
}

impl FieldDefV1 {
    /// Convert v1 field to legacy FieldDef
    pub fn to_legacy(&self) -> FieldDef {
        let mut field = FieldDef {
            name: self.name.clone(),
            field_type: self.field_type.clone(),
            doc: self.doc.clone(),
            ..Default::default()
        };

        // Convert constraints
        if let Some(ref constraints) = self.constraints {
            field.nullable = constraints.nullable.unwrap_or(false);
            field.primary_key = constraints.primary_key.unwrap_or(false);
            field.index = constraints.indexed.unwrap_or(false);

            // Convert max_length to args
            if let Some(max_length) = constraints.max_length {
                field.args = Some(vec![serde_yaml::Value::Number(max_length.into())]);
            }
        }

        // Convert source
        if let Some(ref source) = self.source {
            // Copy from parent
            if let (Some(ref copy_from), Some(ref field_name)) = (&source.copy_from, &source.field) {
                field.extraction = Some(ExtractionConfig {
                    lambda: None,
                    copy_from_context: false,
                    copy_from_source: Some(copy_from.clone()),
                    abstract_method: None,
                });
            }

            // Transform
            if let Some(ref transform) = source.transform {
                let sources: Vec<FieldSource> = source.inputs.iter()
                    .map(|input| FieldSource::Direct(input.clone()))
                    .collect();

                field.computed_from = Some(ComputedFrom {
                    transform: transform.clone(),
                    sources,
                    args: if source.args.is_empty() {
                        None
                    } else {
                        Some(serde_yaml::Value::Sequence(source.args.clone()))
                    },
                    condition: None,
                });
            }

            // Constant
            if let Some(ref constant) = source.constant {
                field.constant = Some(constant.clone());
            }
        }

        field
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

    #[test]
    fn test_entity_v1_to_legacy_simple() {
        let entity_v1 = EntityV1 {
            api_version: "nomnom.io/v1".to_string(),
            kind: "Entity".to_string(),
            metadata: MetadataV1 {
                name: "TestEntity".to_string(),
                labels: HashMap::new(),
                annotations: HashMap::new(),
            },
            spec: SpecV1 {
                entity_type: "root".to_string(),
                repetition: None,
                derivation: None,
                fields: vec![],
                persistence: None,
            },
            status: None,
        };

        let legacy = entity_v1.to_legacy();
        assert_eq!(legacy.name, "TestEntity");
        assert_eq!(legacy.source_type, "root");
    }
}
