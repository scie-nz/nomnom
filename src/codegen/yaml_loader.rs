//! YAML entity configuration loader.
//!
//! This module handles loading entity definitions from YAML files and
//! parsing them into EntityDef structures.
//!
//! Supports both:
//! - Entity Schema v1 (K8s-style with apiVersion, kind, metadata, spec)
//! - Legacy format (entity wrapper with snake_case fields)

use crate::codegen::types::{EntityDef, EntityV1, FieldDef};
use serde::Deserialize;
use std::fs;
use std::path::Path;

/// Wrapper for legacy entity YAML structure
#[derive(Debug, Deserialize)]
struct EntitySpec {
    entity: EntityDef,
}

/// Load all entity definitions from a directory
///
/// # Arguments
///
/// * `dir` - Path to directory containing YAML entity files
///
/// # Returns
///
/// Vector of EntityDef structures
///
/// # Example
///
/// ```ignore
/// use nomnom::codegen::load_entities;
///
/// let entities = load_entities("config/entities").unwrap();
/// ```
pub fn load_entities<P: AsRef<Path>>(dir: P) -> Result<Vec<EntityDef>, String> {
    let dir_path = dir.as_ref();

    if !dir_path.exists() {
        return Err(format!("Directory does not exist: {}", dir_path.display()));
    }

    if !dir_path.is_dir() {
        return Err(format!("Path is not a directory: {}", dir_path.display()));
    }

    let mut entities = Vec::new();

    // Read all YAML files in directory
    let read_dir = fs::read_dir(dir_path)
        .map_err(|e| format!("Failed to read directory {}: {}", dir_path.display(), e))?;

    for entry in read_dir {
        let entry = entry.map_err(|e| format!("Failed to read directory entry: {}", e))?;
        let path = entry.path();

        // Only process .yaml and .yml files
        if let Some(ext) = path.extension() {
            if ext == "yaml" || ext == "yml" {
                match load_entity(&path) {
                    Ok(entity) => entities.push(entity),
                    Err(e) => return Err(format!("Failed to load {}: {}", path.display(), e)),
                }
            }
        }
    }

    // Validate ancillary source entity rules
    for entity in &entities {
        if let Err(e) = entity.validate_ancillary_sources(&entities) {
            return Err(e);
        }
    }

    // Validate minimal existence constraints
    for entity in &entities {
        if let Some(ref minimal_existence) = entity.minimal_existence {
            if let Err(e) = minimal_existence.validate(entity) {
                return Err(e);
            }
        }
    }

    Ok(entities)
}

/// Load a single entity definition from a YAML file
///
/// Supports both Entity Schema v1 (K8s-style) and legacy format.
/// Tries v1 first, then falls back to legacy format.
///
/// # Arguments
///
/// * `path` - Path to YAML file
///
/// # Returns
///
/// EntityDef structure
pub fn load_entity<P: AsRef<Path>>(path: P) -> Result<EntityDef, String> {
    let path = path.as_ref();

    let yaml_content = fs::read_to_string(path)
        .map_err(|e| format!("Failed to read file: {}", e))?;

    // Try Entity Schema v1 first (K8s-style with apiVersion, kind, metadata, spec)
    if let Ok(entity_v1) = serde_yaml::from_str::<EntityV1>(&yaml_content) {
        let entity = entity_v1.to_legacy();
        validate_entity(&entity)?;
        return Ok(entity);
    }

    // Fall back to legacy format (entity wrapper)
    let spec: EntitySpec = serde_yaml::from_str(&yaml_content)
        .map_err(|e| format!("Failed to parse YAML (tried both v1 and legacy formats): {}", e))?;

    // Validate entity
    validate_entity(&spec.entity)?;

    Ok(spec.entity)
}

/// Validate entity definition
///
/// Checks for:
/// - Valid entity name
/// - Proper source_type and parent configuration
/// - Valid field definitions
pub fn validate_entity(entity: &EntityDef) -> Result<(), String> {
    // Check entity name is not empty
    if entity.name.is_empty() {
        return Err("Entity name cannot be empty".to_string());
    }

    // Check that derived entities have parent(s)
    if entity.is_derived() {
        let parents = entity.get_parents();
        if parents.is_empty() && entity.repeated_for.is_none() {
            return Err(format!(
                "Derived entity '{}' must specify parent, parents, or repeated_for",
                entity.name
            ));
        }
    }

    // Validate fields
    for field in &entity.fields {
        validate_field(field, &entity.name)?;
    }

    // Validate root entity has no parent
    if entity.is_root() && !entity.get_parents().is_empty() {
        return Err(format!(
            "Root entity '{}' cannot have parent, parents, or repeated_for",
            entity.name
        ));
    }

    Ok(())
}

/// Validate field definition
fn validate_field(field: &FieldDef, entity_name: &str) -> Result<(), String> {
    // Check field name is not empty
    if field.name.is_empty() {
        return Err(format!(
            "Field name cannot be empty in entity '{}'",
            entity_name
        ));
    }

    // If field has computed_from, validate transform is specified
    if let Some(ref computed) = field.computed_from {
        if computed.transform.is_empty() {
            return Err(format!(
                "Field '{}' in entity '{}' has computed_from but no transform specified",
                field.name, entity_name
            ));
        }

        // Note: sources can be empty for transforms that get data from context
        // (e.g., extract_filename_component gets filename from parent context)
    }

    Ok(())
}

/// Load a parent entity definition by name
///
/// This helper function loads a parent entity from the same directory
/// as the current entity. Useful for resolving parent references.
///
/// # Arguments
///
/// * `parent_name` - Name of parent entity (e.g., "ParentEntity")
/// * `search_dir` - Directory to search for parent YAML file
///
/// # Returns
///
/// Option<EntityDef> - Parent entity definition if found
pub fn load_parent_entity<P: AsRef<Path>>(
    parent_name: &str,
    search_dir: P,
) -> Option<EntityDef> {
    use crate::codegen::utils::to_snake_case;

    // Convert to snake_case (ParentEntity -> parent_entity)
    let snake_name = to_snake_case(parent_name);

    // Try loading YAML file
    let path = search_dir.as_ref().join(format!("{}.yaml", snake_name));

    if let Ok(entity) = load_entity(&path) {
        return Some(entity);
    }

    // Try .yml extension
    let path = search_dir.as_ref().join(format!("{}.yml", snake_name));

    if let Ok(entity) = load_entity(&path) {
        return Some(entity);
    }

    None
}

/// Resolve all fields for an entity, including inherited fields from parent
///
/// If the entity extends a parent, this function recursively loads parent
/// fields and merges them with the entity's own fields.
pub fn resolve_all_fields<P: AsRef<Path>>(
    entity: &EntityDef,
    search_dir: P,
) -> Vec<FieldDef> {
    let mut all_fields = Vec::new();

    // If entity has single parent, load parent fields first
    if let Some(parent_name) = &entity.parent {
        if let Some(parent_entity) = load_parent_entity(parent_name, search_dir.as_ref()) {
            // Recursively resolve parent's fields
            let parent_fields = resolve_all_fields(&parent_entity, search_dir.as_ref());

            // Add parent fields, but skip ones that are overridden in child
            for parent_field in parent_fields {
                let is_overridden = entity.fields.iter().any(|f| f.name == parent_field.name);
                if !is_overridden {
                    all_fields.push(parent_field);
                }
            }
        }
    }

    // Add entity's own fields (these can override parent fields)
    all_fields.extend(entity.fields.clone());

    all_fields
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_validate_entity_root() {
        let entity = EntityDef {
            name: "TestEntity".to_string(),
            source_type: "root".to_string(),
            ..Default::default()
        };

        assert!(validate_entity(&entity).is_ok());
    }

    #[test]
    fn test_validate_entity_derived_needs_parent() {
        let entity = EntityDef {
            name: "TestEntity".to_string(),
            source_type: "derived".to_string(),
            ..Default::default()
        };

        assert!(validate_entity(&entity).is_err());
    }

    #[test]
    fn test_validate_entity_root_cannot_have_parent() {
        let entity = EntityDef {
            name: "TestEntity".to_string(),
            source_type: "root".to_string(),
            parent: Some("Parent".to_string()),
            ..Default::default()
        };

        assert!(validate_entity(&entity).is_err());
    }

    #[test]
    fn test_validate_field_with_computed_from() {
        use crate::codegen::types::{ComputedFrom, FieldSource};

        let field = FieldDef {
            name: "test_field".to_string(),
            field_type: "String".to_string(),
            computed_from: Some(ComputedFrom {
                transform: "extract".to_string(),
                sources: vec![FieldSource::Direct("source".to_string())],
                args: Default::default(),
            }),
            ..Default::default()
        };

        assert!(validate_field(&field, "TestEntity").is_ok());
    }

    #[test]
    fn test_validate_field_computed_from_needs_transform() {
        use crate::codegen::types::{ComputedFrom, FieldSource};

        let field = FieldDef {
            name: "test_field".to_string(),
            field_type: "String".to_string(),
            computed_from: Some(ComputedFrom {
                transform: "".to_string(),
                sources: vec![FieldSource::Direct("source".to_string())],
                args: Default::default(),
            }),
            ..Default::default()
        };

        assert!(validate_field(&field, "TestEntity").is_err());
    }
}
