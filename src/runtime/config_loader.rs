//! Parser configuration loader and topological sort.
//!
//! Loads parser configuration from YAML and computes entity extraction order
//! using topological sort (Kahn's algorithm).

use std::collections::{HashMap, HashSet, VecDeque};
use std::fs;
use std::path::Path;
use serde::{Deserialize, Serialize};
use serde_json::Value as JsonValue;

/// Parser configuration defining entities and extraction order.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ParserConfig {
    /// Entity configurations: entity_name -> config
    pub entities: HashMap<String, HashMap<String, JsonValue>>,

    /// Entity extraction order (topologically sorted by dependencies)
    pub extraction_order: Vec<String>,
}

impl ParserConfig {
    /// Load parser configuration from YAML file.
    ///
    /// # Arguments
    /// * `path` - Path to parser_config.yaml
    ///
    /// # Returns
    /// Parsed configuration with computed extraction order
    ///
    /// # Errors
    /// Returns error if file doesn't exist or has invalid format
    ///
    /// # Example
    /// ```ignore
    /// use nomnom::runtime::ParserConfig;
    ///
    /// let config = ParserConfig::load_from_file("config/parser_config.yaml")?;
    /// println!("Extraction order: {:?}", config.extraction_order);
    /// ```
    pub fn load_from_file<P: AsRef<Path>>(path: P) -> Result<Self, String> {
        let path = path.as_ref();

        // Read file
        let contents = fs::read_to_string(path)
            .map_err(|e| format!("Failed to read config file {}: {}", path.display(), e))?;

        // Parse YAML
        let yaml: serde_yaml::Value = serde_yaml::from_str(&contents)
            .map_err(|e| format!("Failed to parse YAML: {}", e))?;

        // Validate structure
        let entities_yaml = yaml
            .get("entities")
            .ok_or_else(|| "Config missing 'entities' field".to_string())?;

        // Convert entities to HashMap
        let entities: HashMap<String, HashMap<String, JsonValue>> = serde_yaml::from_value(
            entities_yaml.clone()
        )
        .map_err(|e| format!("Failed to parse entities: {}", e))?;

        // Compute extraction order via topological sort
        let extraction_order = compute_extraction_order(&entities)?;

        Ok(Self {
            entities,
            extraction_order,
        })
    }

    /// Create parser config from pre-parsed entities.
    ///
    /// Useful for testing or when entities are built programmatically.
    ///
    /// # Arguments
    /// * `entities` - Entity configurations
    ///
    /// # Returns
    /// Parser config with computed extraction order
    pub fn from_entities(
        entities: HashMap<String, HashMap<String, JsonValue>>
    ) -> Result<Self, String> {
        let extraction_order = compute_extraction_order(&entities)?;

        Ok(Self {
            entities,
            extraction_order,
        })
    }

    /// Get entity configuration by name.
    pub fn get_entity(&self, name: &str) -> Option<&HashMap<String, JsonValue>> {
        self.entities.get(name)
    }

    /// Check if an entity is defined.
    pub fn has_entity(&self, name: &str) -> bool {
        self.entities.contains_key(name)
    }

    /// Get all entity names.
    pub fn entity_names(&self) -> Vec<&String> {
        self.entities.keys().collect()
    }
}

/// Compute entity extraction order using topological sort (Kahn's algorithm).
///
/// Entities are sorted such that dependencies are always extracted before
/// dependent entities. Detects and reports circular dependencies.
///
/// # Algorithm
/// 1. Build dependency graph from entity configurations
/// 2. Find entities with no dependencies (in-degree = 0)
/// 3. Process entities level-by-level, removing edges as we go
/// 4. If any entities remain unprocessed, there's a cycle
///
/// # Arguments
/// * `entities` - Entity configurations
///
/// # Returns
/// * `Ok(order)` - List of entity names in extraction order
/// * `Err(msg)` - If circular dependency detected or invalid config
///
/// # Example
/// ```ignore
/// use nomnom::runtime::compute_extraction_order;
///
/// let mut entities = HashMap::new();
/// // ... populate entities ...
///
/// let order = compute_extraction_order(&entities)?;
/// ```
pub fn compute_extraction_order(
    entities: &HashMap<String, HashMap<String, JsonValue>>
) -> Result<Vec<String>, String> {
    let mut in_degree: HashMap<String, usize> = HashMap::new();
    let mut graph: HashMap<String, Vec<String>> = HashMap::new();

    // Initialize in-degree and graph for all entities
    for entity_name in entities.keys() {
        in_degree.insert(entity_name.clone(), 0);
        graph.insert(entity_name.clone(), Vec::new());
    }

    // Build dependency graph
    for (entity_name, config) in entities {
        let dependencies = extract_dependencies(config);

        // Validate dependencies exist
        for dep in &dependencies {
            if !entities.contains_key(dep) {
                return Err(format!(
                    "Entity '{}' depends on undefined entity '{}'",
                    entity_name, dep
                ));
            }
        }

        // Update in-degree and graph
        *in_degree.get_mut(entity_name).unwrap() += dependencies.len();

        for dep in dependencies {
            graph.get_mut(&dep).unwrap().push(entity_name.clone());
        }
    }

    // Kahn's algorithm: process entities with no dependencies
    let mut queue: VecDeque<String> = in_degree
        .iter()
        .filter(|(_, &degree)| degree == 0)
        .map(|(name, _)| name.clone())
        .collect();

    let mut sorted = Vec::new();

    while let Some(entity_name) = queue.pop_front() {
        sorted.push(entity_name.clone());

        // Process all entities that depend on this one
        if let Some(dependents) = graph.get(&entity_name) {
            for dependent in dependents {
                let degree = in_degree.get_mut(dependent).unwrap();
                *degree -= 1;

                if *degree == 0 {
                    queue.push_back(dependent.clone());
                }
            }
        }
    }

    // Check if all entities were processed
    if sorted.len() != entities.len() {
        // Circular dependency detected - find the cycle
        let unprocessed: Vec<_> = entities
            .keys()
            .filter(|name| !sorted.contains(name))
            .collect();

        return Err(format!(
            "Circular dependency detected involving entities: {:?}",
            unprocessed
        ));
    }

    Ok(sorted)
}

/// Extract dependency list from entity configuration.
///
/// # Arguments
/// * `config` - Entity configuration
///
/// # Returns
/// List of entity names this entity depends on
fn extract_dependencies(config: &HashMap<String, JsonValue>) -> Vec<String> {
    config
        .get("dependencies")
        .and_then(|v| v.as_array())
        .map(|arr| {
            arr.iter()
                .filter_map(|v| v.as_str())
                .map(|s| s.to_string())
                .collect()
        })
        .unwrap_or_default()
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;

    fn make_entity_config(dependencies: Vec<&str>) -> HashMap<String, JsonValue> {
        let mut config = HashMap::new();
        config.insert(
            "dependencies".to_string(),
            json!(dependencies.iter().map(|s| s.to_string()).collect::<Vec<_>>()),
        );
        config
    }

    #[test]
    fn test_compute_order_no_dependencies() {
        let mut entities = HashMap::new();
        entities.insert("A".to_string(), make_entity_config(vec![]));
        entities.insert("B".to_string(), make_entity_config(vec![]));
        entities.insert("C".to_string(), make_entity_config(vec![]));

        let order = compute_extraction_order(&entities).unwrap();

        assert_eq!(order.len(), 3);
        assert!(order.contains(&"A".to_string()));
        assert!(order.contains(&"B".to_string()));
        assert!(order.contains(&"C".to_string()));
    }

    #[test]
    fn test_compute_order_linear_chain() {
        let mut entities = HashMap::new();
        entities.insert("A".to_string(), make_entity_config(vec![]));
        entities.insert("B".to_string(), make_entity_config(vec!["A"]));
        entities.insert("C".to_string(), make_entity_config(vec!["B"]));

        let order = compute_extraction_order(&entities).unwrap();

        assert_eq!(order, vec!["A", "B", "C"]);
    }

    #[test]
    fn test_compute_order_diamond_dependency() {
        // A (root)
        // ├── B (depends on A)
        // ├── C (depends on A)
        // └── D (depends on B and C)
        let mut entities = HashMap::new();
        entities.insert("A".to_string(), make_entity_config(vec![]));
        entities.insert("B".to_string(), make_entity_config(vec!["A"]));
        entities.insert("C".to_string(), make_entity_config(vec!["A"]));
        entities.insert("D".to_string(), make_entity_config(vec!["B", "C"]));

        let order = compute_extraction_order(&entities).unwrap();

        assert_eq!(order.len(), 4);
        // A must be first
        assert_eq!(order[0], "A");
        // B and C must come before D
        let b_pos = order.iter().position(|x| x == "B").unwrap();
        let c_pos = order.iter().position(|x| x == "C").unwrap();
        let d_pos = order.iter().position(|x| x == "D").unwrap();
        assert!(b_pos < d_pos);
        assert!(c_pos < d_pos);
    }

    #[test]
    fn test_compute_order_circular_dependency() {
        let mut entities = HashMap::new();
        entities.insert("A".to_string(), make_entity_config(vec!["B"]));
        entities.insert("B".to_string(), make_entity_config(vec!["A"]));

        let result = compute_extraction_order(&entities);

        assert!(result.is_err());
        let err = result.unwrap_err();
        assert!(err.contains("Circular dependency"));
    }

    #[test]
    fn test_compute_order_missing_dependency() {
        let mut entities = HashMap::new();
        entities.insert("A".to_string(), make_entity_config(vec!["B"]));

        let result = compute_extraction_order(&entities);

        assert!(result.is_err());
        let err = result.unwrap_err();
        assert!(err.contains("depends on undefined entity"));
    }

    #[test]
    fn test_extract_dependencies_empty() {
        let config = HashMap::new();
        let deps = extract_dependencies(&config);
        assert_eq!(deps, Vec::<String>::new());
    }

    #[test]
    fn test_extract_dependencies_with_deps() {
        let mut config = HashMap::new();
        config.insert(
            "dependencies".to_string(),
            json!(["EntityA", "EntityB"]),
        );

        let deps = extract_dependencies(&config);

        assert_eq!(deps, vec!["EntityA", "EntityB"]);
    }

    #[test]
    fn test_parser_config_from_entities() {
        let mut entities = HashMap::new();
        entities.insert("A".to_string(), make_entity_config(vec![]));
        entities.insert("B".to_string(), make_entity_config(vec!["A"]));

        let config = ParserConfig::from_entities(entities).unwrap();

        assert_eq!(config.extraction_order, vec!["A", "B"]);
        assert!(config.has_entity("A"));
        assert!(config.has_entity("B"));
        assert!(!config.has_entity("C"));
    }

    #[test]
    fn test_parser_config_get_entity() {
        let mut entities = HashMap::new();
        entities.insert("TestEntity".to_string(), make_entity_config(vec![]));

        let config = ParserConfig::from_entities(entities).unwrap();

        assert!(config.get_entity("TestEntity").is_some());
        assert!(config.get_entity("MissingEntity").is_none());
    }

    #[test]
    fn test_parser_config_entity_names() {
        let mut entities = HashMap::new();
        entities.insert("A".to_string(), make_entity_config(vec![]));
        entities.insert("B".to_string(), make_entity_config(vec![]));

        let config = ParserConfig::from_entities(entities).unwrap();

        let names = config.entity_names();
        assert_eq!(names.len(), 2);
        assert!(names.contains(&&"A".to_string()));
        assert!(names.contains(&&"B".to_string()));
    }
}
