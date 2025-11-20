/// Dependency graph for entity processing order
///
/// Builds a directed acyclic graph (DAG) from entity definitions
/// and computes topological processing order.

use crate::codegen::EntityDef;
use std::collections::{HashMap, HashSet, VecDeque};
use std::error::Error;

#[derive(Debug, Clone)]
pub struct DependencyNode {
    pub entity: String,
    pub depends_on: Vec<String>,
    pub level: usize,
}

#[derive(Debug)]
pub struct DependencyGraph {
    pub nodes: HashMap<String, DependencyNode>,
    pub levels: Vec<Vec<String>>,
}

impl DependencyGraph {
    /// Build dependency graph from entity definitions
    pub fn build(entities: &[EntityDef]) -> Result<Self, Box<dyn Error>> {
        let mut nodes = HashMap::new();

        // First pass: collect all entities and their direct dependencies
        for entity in entities {
            // Skip root and abstract entities
            if entity.is_root() || entity.is_abstract {
                continue;
            }

            let mut depends_on = Vec::new();

            // Get dependencies from derivation.source_entities
            if let Some(ref derivation) = entity.derivation {
                if let Some(ref source_entities) = derivation.source_entities {
                    match source_entities {
                        serde_yaml::Value::Mapping(map) => {
                            for (_key, value) in map {
                                match value {
                                    // Simple string format: alias: EntityName
                                    serde_yaml::Value::String(source_entity_name) => {
                                        // Only exclude abstract entities
                                        if let Some(source_entity) = entities.iter().find(|e| &e.name == source_entity_name) {
                                            if !source_entity.is_abstract {
                                                depends_on.push(source_entity_name.clone());
                                            }
                                        }
                                    }
                                    // Detailed object format: alias: {entity: EntityName, ancillary: bool}
                                    serde_yaml::Value::Mapping(obj) => {
                                        if let Some(serde_yaml::Value::String(source_entity_name)) =
                                            obj.get(&serde_yaml::Value::String("entity".to_string())) {
                                            // Only exclude abstract entities
                                            if let Some(source_entity) = entities.iter().find(|e| &e.name == source_entity_name) {
                                                if !source_entity.is_abstract {
                                                    depends_on.push(source_entity_name.clone());
                                                }
                                            }
                                        }
                                    }
                                    _ => {}
                                }
                            }
                        }
                        serde_yaml::Value::String(s) => {
                            // Only exclude abstract entities
                            if let Some(source_entity) = entities.iter().find(|e| &e.name == s) {
                                if !source_entity.is_abstract {
                                    depends_on.push(s.clone());
                                }
                            }
                        }
                        serde_yaml::Value::Sequence(seq) => {
                            for item in seq {
                                if let serde_yaml::Value::String(s) = item {
                                    // Only exclude abstract entities
                                    if let Some(source_entity) = entities.iter().find(|e| &e.name == s) {
                                        if !source_entity.is_abstract {
                                            depends_on.push(s.clone());
                                        }
                                    }
                                }
                            }
                        }
                        _ => {}
                    }
                }
            }

            // Also check parents field
            if !entity.parents.is_empty() {
                for parent in &entity.parents {
                    // Only exclude abstract entities
                    if let Some(parent_entity) = entities.iter().find(|e| &e.name == &parent.parent_type) {
                        if !parent_entity.is_abstract && !depends_on.contains(&parent.parent_type) {
                            depends_on.push(parent.parent_type.clone());
                        }
                    }
                }
            } else if let Some(ref parent_name) = entity.parent {
                // Only exclude abstract entities
                if let Some(parent_entity) = entities.iter().find(|e| &e.name == parent_name) {
                    if !parent_entity.is_abstract && !depends_on.contains(parent_name) {
                        depends_on.push(parent_name.clone());
                    }
                }
            }

            nodes.insert(
                entity.name.clone(),
                DependencyNode {
                    entity: entity.name.clone(),
                    depends_on,
                    level: 0, // Will be computed
                },
            );
        }

        // Compute levels using topological sort
        let levels = Self::compute_levels(&nodes)?;

        // Update node levels
        for (level_num, level_entities) in levels.iter().enumerate() {
            for entity_name in level_entities {
                if let Some(node) = nodes.get_mut(entity_name) {
                    node.level = level_num;
                }
            }
        }

        Ok(DependencyGraph { nodes, levels })
    }

    /// Compute processing levels using topological sort (Kahn's algorithm)
    fn compute_levels(
        nodes: &HashMap<String, DependencyNode>,
    ) -> Result<Vec<Vec<String>>, Box<dyn Error>> {
        // Build reverse dependency map (who depends on this entity)
        let mut reverse_deps: HashMap<String, Vec<String>> = HashMap::new();
        let mut in_degree: HashMap<String, usize> = HashMap::new();

        for (entity_name, node) in nodes {
            // Only count dependencies on entities that are in the graph (non-root, non-abstract)
            let valid_deps: Vec<&String> = node.depends_on.iter()
                .filter(|dep| nodes.contains_key(*dep))
                .collect();

            in_degree.insert(entity_name.clone(), valid_deps.len());

            for dep in valid_deps {
                reverse_deps
                    .entry(dep.clone())
                    .or_insert_with(Vec::new)
                    .push(entity_name.clone());
            }
        }

        // Find all entities with no valid dependencies (in_degree == 0)
        let mut queue: VecDeque<String> = in_degree
            .iter()
            .filter(|(_, &degree)| degree == 0)
            .map(|(name, _)| name.clone())
            .collect();

        let mut levels: Vec<Vec<String>> = Vec::new();
        let mut processed = HashSet::new();

        // Process level by level
        while !queue.is_empty() {
            let mut current_level = Vec::new();

            // Process all entities at current level
            let level_size = queue.len();
            for _ in 0..level_size {
                if let Some(entity_name) = queue.pop_front() {
                    current_level.push(entity_name.clone());
                    processed.insert(entity_name.clone());

                    // Reduce in-degree for dependents
                    if let Some(dependents) = reverse_deps.get(&entity_name) {
                        for dependent in dependents {
                            if let Some(degree) = in_degree.get_mut(dependent) {
                                *degree -= 1;
                                if *degree == 0 {
                                    queue.push_back(dependent.clone());
                                }
                            }
                        }
                    }
                }
            }

            if !current_level.is_empty() {
                levels.push(current_level);
            }
        }

        // Check for cycles
        if processed.len() != nodes.len() {
            let unprocessed: Vec<String> = nodes
                .keys()
                .filter(|k| !processed.contains(*k))
                .cloned()
                .collect();
            return Err(format!(
                "Circular dependency detected involving entities: {:?}",
                unprocessed
            )
            .into());
        }

        Ok(levels)
    }

    /// Get entities at a specific level
    pub fn get_level(&self, level: usize) -> Option<&Vec<String>> {
        self.levels.get(level)
    }

    /// Get total number of levels
    pub fn num_levels(&self) -> usize {
        self.levels.len()
    }

    /// Get all entities in processing order (flattened levels)
    pub fn processing_order(&self) -> Vec<String> {
        self.levels.iter().flatten().cloned().collect()
    }

    /// Check if an entity depends on another entity (directly or indirectly)
    pub fn depends_on(&self, entity: &str, dependency: &str) -> bool {
        if let Some(node) = self.nodes.get(entity) {
            if node.depends_on.contains(&dependency.to_string()) {
                return true;
            }
            // Check transitive dependencies
            for dep in &node.depends_on {
                if self.depends_on(dep, dependency) {
                    return true;
                }
            }
        }
        false
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_simple_dependency_chain() {
        // Create mock entities:
        // A depends on nothing
        // B depends on A
        // C depends on B
        let mut nodes = HashMap::new();

        nodes.insert(
            "A".to_string(),
            DependencyNode {
                entity: "A".to_string(),
                depends_on: vec![],
                level: 0,
            },
        );

        nodes.insert(
            "B".to_string(),
            DependencyNode {
                entity: "B".to_string(),
                depends_on: vec!["A".to_string()],
                level: 0,
            },
        );

        nodes.insert(
            "C".to_string(),
            DependencyNode {
                entity: "C".to_string(),
                depends_on: vec!["B".to_string()],
                level: 0,
            },
        );

        let levels = DependencyGraph::compute_levels(&nodes).unwrap();

        assert_eq!(levels.len(), 3);
        assert_eq!(levels[0], vec!["A"]);
        assert_eq!(levels[1], vec!["B"]);
        assert_eq!(levels[2], vec!["C"]);
    }

    #[test]
    fn test_parallel_dependencies() {
        // Create mock entities:
        // A, B depend on nothing
        // C depends on both A and B
        let mut nodes = HashMap::new();

        nodes.insert(
            "A".to_string(),
            DependencyNode {
                entity: "A".to_string(),
                depends_on: vec![],
                level: 0,
            },
        );

        nodes.insert(
            "B".to_string(),
            DependencyNode {
                entity: "B".to_string(),
                depends_on: vec![],
                level: 0,
            },
        );

        nodes.insert(
            "C".to_string(),
            DependencyNode {
                entity: "C".to_string(),
                depends_on: vec!["A".to_string(), "B".to_string()],
                level: 0,
            },
        );

        let levels = DependencyGraph::compute_levels(&nodes).unwrap();

        assert_eq!(levels.len(), 2);
        assert_eq!(levels[0].len(), 2); // A and B at level 0
        assert!(levels[0].contains(&"A".to_string()));
        assert!(levels[0].contains(&"B".to_string()));
        assert_eq!(levels[1], vec!["C"]); // C at level 1
    }

    #[test]
    fn test_circular_dependency_detection() {
        // Create circular dependency: A -> B -> C -> A
        let mut nodes = HashMap::new();

        nodes.insert(
            "A".to_string(),
            DependencyNode {
                entity: "A".to_string(),
                depends_on: vec!["C".to_string()],
                level: 0,
            },
        );

        nodes.insert(
            "B".to_string(),
            DependencyNode {
                entity: "B".to_string(),
                depends_on: vec!["A".to_string()],
                level: 0,
            },
        );

        nodes.insert(
            "C".to_string(),
            DependencyNode {
                entity: "C".to_string(),
                depends_on: vec!["B".to_string()],
                level: 0,
            },
        );

        let result = DependencyGraph::compute_levels(&nodes);
        assert!(result.is_err());
        assert!(result.unwrap_err().to_string().contains("Circular dependency"));
    }
}
