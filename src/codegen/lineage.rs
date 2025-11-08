//! Lineage tracing code generation for parser binary.
//!
//! This module generates code for:
//! - SHA-based entity identity tracking
//! - Parent-child lineage relationships
//! - ASCII tree visualization
//!
//! Usage: Add `--lineage` flag to parser to enable lineage tracking


/// Generate lineage-related data structures and functions
pub fn generate_lineage_code() -> String {
    let mut code = String::new();

    code.push_str(&generate_lineage_types());
    code.push_str(&generate_sha_computation());
    code.push_str(&generate_lineage_tracker());
    code.push_str(&generate_tree_builder());
    code.push_str(&generate_ascii_renderer());

    code
}

/// Generate lineage metadata types
fn generate_lineage_types() -> String {
    r#"
// ============================================================================
// Lineage Tracking Types
// ============================================================================

use sha2::{Sha256, Digest};
use std::collections::BTreeMap;
use serde_json::json;
use serde::{Serialize, Deserialize};

/// Lineage metadata for a single entity instance
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct LineageMetadata {
    /// SHA-256 hash (truncated to 64-bit for readability)
    pub sha: String,
    /// References to parent entities
    pub parent_shas: Vec<ParentReference>,
    /// ISO 8601 timestamp when entity was derived
    pub derived_at: String,
    /// Database lookup result (if applicable)
    #[serde(skip_serializing_if = "Option::is_none")]
    pub database_lookup: Option<DatabaseLookupResult>,
}

/// Reference to a parent entity
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ParentReference {
    #[serde(rename = "type")]
    pub entity_type: String,
    pub sha: String,
}

/// Result of database lookup for persistent entities
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum DatabaseLookupResult {
    FoundExisting,
    CreatedNew,
    NotApplicable,  // Transient entity (no database)
}

/// Entity identity for SHA computation
#[derive(Debug, Clone, Serialize)]
struct EntityIdentity {
    entity_type: String,
    fields: BTreeMap<String, Value>,
    parent_shas: Vec<String>,
    // Note: derived_at NOT included - we want deterministic SHAs
}

/// Node in the lineage tree
#[derive(Debug, Clone)]
struct LineageNode {
    entity_type: String,
    sha: String,
    parent_shas: Vec<String>,
    is_repeated: bool,
    is_permanent: bool,  // Track if entity is permanent (vs transient)
    instance_count: usize,
    database_lookup: Option<DatabaseLookupResult>,
    fields: Option<BTreeMap<String, Value>>,  // For detailed rendering
}

/// Tree structure for visualization
#[derive(Debug, Clone)]
struct LineageTree {
    node: LineageNode,
    children: Vec<LineageTree>,
}

/// Lineage tree rendering format
#[derive(Debug, Clone, Copy)]
pub enum LineageFormat {
    Compact,
    Detailed,
}

"#.to_string()
}

/// Generate SHA computation function
fn generate_sha_computation() -> String {
    r#"
// ============================================================================
// SHA Computation
// ============================================================================

/// Compute SHA-256 hash for an entity instance
fn compute_entity_sha(identity: &EntityIdentity) -> String {
    // Serialize to canonical JSON (BTreeMap ensures field ordering)
    let canonical = serde_json::to_string(identity)
        .expect("Failed to serialize entity identity");

    // Compute SHA-256
    let mut hasher = Sha256::new();
    hasher.update(canonical.as_bytes());
    let result = hasher.finalize();

    // Convert to hex and truncate to 16 chars (64 bits)
    let full_hex = format!("{:x}", result);
    full_hex[..16].to_string()
}

/// Get current timestamp in milliseconds since epoch
fn current_time_ms() -> i64 {
    use std::time::{SystemTime, UNIX_EPOCH};
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .expect("Time went backwards")
        .as_millis() as i64
}

/// Format timestamp as ISO 8601
fn format_timestamp(ms: i64) -> String {
    use chrono::{DateTime, Utc, TimeZone};
    let dt = Utc.timestamp_millis_opt(ms).unwrap();
    dt.to_rfc3339()
}

"#.to_string()
}

/// Generate LineageTracker implementation
fn generate_lineage_tracker() -> String {
    r#"
// ============================================================================
// Lineage Tracker
// ============================================================================

/// Tracks entity lineage during parsing
struct LineageTracker {
    entities: HashMap<String, LineageNode>,
    sha_to_entity: HashMap<String, String>,  // SHA -> entity_type
    root_sha: Option<String>,
}

impl LineageTracker {
    fn new() -> Self {
        Self {
            entities: HashMap::new(),
            sha_to_entity: HashMap::new(),
            root_sha: None,
        }
    }

    /// Compute SHA for an entity and store its lineage
    fn compute_sha(
        &mut self,
        entity_type: &str,
        fields: &BTreeMap<String, Value>,
        parent_shas: &[String],
        is_permanent: bool,
    ) -> String {
        // Filter out empty parent SHAs
        let filtered_parent_shas: Vec<String> = parent_shas.iter()
            .filter(|s| !s.is_empty())
            .cloned()
            .collect();

        let identity = EntityIdentity {
            entity_type: entity_type.to_string(),
            fields: fields.clone(),
            parent_shas: filtered_parent_shas.clone(),
        };

        let sha = compute_entity_sha(&identity);

        // Store lineage node
        self.entities.insert(sha.clone(), LineageNode {
            entity_type: entity_type.to_string(),
            sha: sha.clone(),
            parent_shas: filtered_parent_shas.clone(),
            is_repeated: false,
            is_permanent,
            instance_count: 1,
            database_lookup: None,
            fields: Some(fields.clone()),
        });

        self.sha_to_entity.insert(sha.clone(), entity_type.to_string());

        // Track root entity
        if filtered_parent_shas.is_empty() {
            self.root_sha = Some(sha.clone());
        }

        sha
    }

    /// Compute SHA after database lookup (may have different data)
    fn compute_sha_after_db_lookup(
        &mut self,
        entity_type: &str,
        fields: &BTreeMap<String, Value>,
        parent_shas: &[String],
        is_permanent: bool,
        lookup_result: DatabaseLookupResult,
    ) -> String {
        let sha = self.compute_sha(entity_type, fields, parent_shas, is_permanent);

        if let Some(node) = self.entities.get_mut(&sha) {
            node.database_lookup = Some(lookup_result);
        }

        sha
    }

    /// Create lineage metadata for an entity
    fn create_metadata(&self, sha: &str) -> LineageMetadata {
        let node = self.entities.get(sha)
            .expect(&format!("SHA not found in lineage tracker: {}", sha));

        LineageMetadata {
            sha: sha.to_string(),
            parent_shas: node.parent_shas.iter()
                .filter(|parent_sha| !parent_sha.is_empty())  // Filter out empty SHAs
                .map(|parent_sha| {
                    let parent_type = self.sha_to_entity.get(parent_sha)
                        .expect(&format!("Parent SHA not found: {}", parent_sha));
                    ParentReference {
                        entity_type: parent_type.clone(),
                        sha: parent_sha.clone(),
                    }
                })
                .collect(),
            derived_at: format_timestamp(current_time_ms()),
            database_lookup: node.database_lookup.clone(),
        }
    }
}

"#.to_string()
}

/// Generate tree building functions
fn generate_tree_builder() -> String {
    r#"
// ============================================================================
// Tree Building
// ============================================================================

impl LineageTracker {
    /// Build lineage tree from root entity
    fn build_tree(&self) -> Option<LineageTree> {
        let root_sha = self.root_sha.as_ref()?;
        Some(self.build_subtree(root_sha))
    }

    /// Recursively build tree from a given node
    fn build_subtree(&self, sha: &str) -> LineageTree {
        let node = self.entities.get(sha)
            .expect(&format!("SHA not found: {}", sha))
            .clone();

        let children = self.find_children(sha);

        LineageTree { node, children }
    }

    /// Find all children of a given parent SHA
    fn find_children(&self, parent_sha: &str) -> Vec<LineageTree> {
        let mut children = Vec::new();

        for (sha, node) in &self.entities {
            if node.parent_shas.contains(&parent_sha.to_string()) {
                children.push(self.build_subtree(sha));
            }
        }

        // Sort by entity type for consistent output
        children.sort_by(|a, b| a.node.entity_type.cmp(&b.node.entity_type));

        children
    }
}

"#.to_string()
}

/// Generate ASCII rendering functions
fn generate_ascii_renderer() -> String {
    r#"
// ============================================================================
// ASCII Tree Rendering
// ============================================================================

// ANSI color codes
const RESET: &str = "\x1b[0m";
const BOLD: &str = "\x1b[1m";
const DIM: &str = "\x1b[2m";
const GREEN: &str = "\x1b[32m";
const CYAN: &str = "\x1b[36m";
const YELLOW: &str = "\x1b[33m";
const MAGENTA: &str = "\x1b[35m";
const BLUE: &str = "\x1b[34m";

impl LineageTracker {
    /// Render lineage tree in specified format
    fn render_tree(&self, format: LineageFormat) -> String {
        match self.build_tree() {
            Some(tree) => match format {
                LineageFormat::Compact => self.render_compact(&tree, "", true),
                LineageFormat::Detailed => self.render_detailed(&tree, "", true),
            },
            None => "No lineage tree available\n".to_string(),
        }
    }

    /// Render compact tree format
    fn render_compact(&self, tree: &LineageTree, prefix: &str, is_last: bool) -> String {
        let mut output = String::new();

        let connector = if is_last { "└─ " } else { "├─ " };

        // Entity type with color and marker
        let (entity_marker, entity_color) = if tree.node.is_permanent {
            ("💾 ", CYAN)  // Permanent entities: disk icon + cyan
        } else {
            ("⚡ ", YELLOW)  // Transient entities: lightning + yellow
        };

        // Database lookup annotation
        let db_annotation = match &tree.node.database_lookup {
            Some(DatabaseLookupResult::FoundExisting) => format!(" {}🔍 DB: found{}", MAGENTA, RESET),
            Some(DatabaseLookupResult::CreatedNew) => format!(" {}✨ DB: created{}", GREEN, RESET),
            _ => String::new(),
        };

        // Truncate SHA for display
        let sha_short = &tree.node.sha[..8.min(tree.node.sha.len())];

        output.push_str(&format!(
            "{}{}{}{}{}{} {}[{}]{}{}{}\n",
            DIM, prefix, RESET,
            connector,
            entity_marker,
            entity_color, tree.node.entity_type,
            DIM, sha_short, RESET,
            db_annotation
        ));

        // Render children
        let new_prefix = format!("{}{}", prefix, if is_last { "   " } else { "│  " });

        for (i, child) in tree.children.iter().enumerate() {
            let is_last_child = i == tree.children.len() - 1;
            output.push_str(&self.render_compact(child, &new_prefix, is_last_child));
        }

        output
    }

    /// Render detailed tree format (includes field values)
    fn render_detailed(&self, tree: &LineageTree, prefix: &str, is_last: bool) -> String {
        let mut output = String::new();

        let connector = if is_last { "└─ " } else { "├─ " };
        let sha_short = &tree.node.sha[..8.min(tree.node.sha.len())];

        // Entity header
        output.push_str(&format!(
            "{}{}{} [{}]\n",
            prefix,
            connector,
            tree.node.entity_type,
            sha_short
        ));

        // Field details
        let field_prefix = format!("{}{}  ", prefix, if is_last { " " } else { "│" });

        if let Some(fields) = &tree.node.fields {
            for (key, value) in fields {
                let value_str = match value {
                    Value::String(s) => s.clone(),
                    _ => value.to_string(),
                };
                // Truncate long values
                let display_value = if value_str.len() > 50 {
                    format!("{}...", &value_str[..47])
                } else {
                    value_str
                };
                output.push_str(&format!("{}│  {}: {}\n", field_prefix, key, display_value));
            }
        }

        // Database annotation
        if let Some(ref lookup) = tree.node.database_lookup {
            output.push_str(&format!("{}│  database_lookup: {:?}\n", field_prefix, lookup));
        }

        output.push_str(&format!("{}│  derived_at: {}\n", field_prefix, format_timestamp(current_time_ms())));
        output.push_str(&format!("{}│\n", field_prefix));

        // Render children
        let new_prefix = format!("{}{}", prefix, if is_last { "   " } else { "│  " });

        for (i, child) in tree.children.iter().enumerate() {
            let is_last_child = i == tree.children.len() - 1;
            output.push_str(&self.render_detailed(child, &new_prefix, is_last_child));
        }

        output
    }
}

"#.to_string()
}

/// Generate helper to convert entity to field map
pub fn generate_entity_to_fields_helper() -> String {
    r#"
/// Convert entity to BTreeMap for SHA computation
fn entity_to_fields<T: Serialize>(entity: &T) -> BTreeMap<String, Value> {
    let json = serde_json::to_value(entity)
        .expect("Failed to serialize entity");

    match json {
        Value::Object(map) => {
            // Filter out lineage field if present (we don't want recursive lineage)
            map.into_iter()
                .filter(|(k, _)| k != "lineage")
                .collect()
        }
        _ => BTreeMap::new(),
    }
}

"#.to_string()
}
