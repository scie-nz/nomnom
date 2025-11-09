//! Auto-generated parser binary from nomnom entity configurations
//!
//! This binary reads file paths from stdin and outputs:
//! - JSON Lines: One entity per line in JSON format
//! - SQL: Database queries with inlined values (dry-run mode)
//!
//! Generated code is 100% config-driven - no hardcoded business logic.

#![allow(unused_imports)]
#![allow(dead_code)]

use std::io::{self, BufRead};
use std::error::Error;
use clap::Parser;
use std::collections::HashMap;
use serde_json::Value;
use std::path::Path;

// Import from the library crate (lib name is _rust)
use _rust::generated::*;

// Database imports for --execute-db mode
use _rust::db::{Database, DatabaseConfig, Pool, operations::GetOrCreate};
use _rust::models::*;

// Note: Transforms are now injected directly into generated.rs
// No registry needed - entity constructors call transform functions directly


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

/// CLI arguments for parser binary
#[derive(Parser, Debug)]
#[command(name = "parser")]
#[command(about = "Parse message files and output entities in JSON/SQL format", long_about = None)]
struct Cli {
    /// Show only JSON output (skip SQL)
    #[arg(long)]
    json_only: bool,

    /// Show only SQL output (skip JSON)
    #[arg(long)]
    sql_only: bool,

    /// Show both JSON and SQL (default)
    #[arg(long)]
    dry_run: bool,

    /// Enable lineage tracking (adds SHA and parent references to JSON output)
    #[arg(long)]
    lineage: bool,

    /// Display ASCII tree visualization of entity lineage
    #[arg(long)]
    show_lineage: bool,

    /// Lineage tree format: compact (default) or detailed
    #[arg(long, default_value = "compact")]
    lineage_format: String,

    /// Execute statements directly against database (requires DATABASE_URL env var)
    #[arg(long)]
    execute_db: bool,

    /// Show verbose output (detailed execution logs)
    #[arg(long, short)]
    verbose: bool,
}

/// Results from parsing a single file
#[derive(Debug)]
struct ParseResults {
    order: OrderCore,
    order_line_item: Vec<OrderLineItemCore>,
}

/// Statistics from database execution
#[derive(Debug, Default)]
struct ExecutionStats {
    order_created: usize,
    order_found: usize,
    order_line_item_created: usize,
    order_line_item_found: usize,
}

fn main() -> Result<(), Box<dyn Error>> {
    let cli = Cli::parse();

    // Determine output mode
    let show_json = !cli.sql_only && !cli.execute_db;
    let show_sql = !cli.json_only && !cli.execute_db;

    // If no flags specified, --dry-run is default (show both)
    let show_json = if cli.dry_run { true } else { show_json };
    let show_sql = if cli.dry_run { true } else { show_sql };

    // Parse lineage format
    let lineage_format = if cli.lineage_format == "detailed" {
        LineageFormat::Detailed
    } else {
        LineageFormat::Compact
    };

    // Initialize database connection pool if --execute-db is set
    let db_pool: Option<Pool> = if cli.execute_db {
        let db_url = std::env::var("DATABASE_URL")
            .expect("DATABASE_URL must be set for --execute-db mode");

        if cli.verbose {
            eprintln!("Connecting to database: {}", db_url);
        }

        let database = Database::new(&db_url)
            .expect("Failed to connect to database");

        if cli.verbose {
            eprintln!("Database connection established");
        }

        Some(database.pool().clone())
    } else {
        None
    };

    // Read file paths from stdin (one per line)
    let stdin = io::stdin();
    for line in stdin.lock().lines() {
        let file_path = line?;

        // Process file
        match process_file(
            &file_path,
            show_json,
            show_sql,
            cli.lineage,
            cli.show_lineage,
            lineage_format,
            db_pool.as_ref(),
            cli.verbose,
        ) {
            Ok(_) => {},
            Err(e) => {
                eprintln!("Error processing file '{}': {}", file_path, e);
                // Continue to next file
            }
        }
    }

    Ok(())
}

/// Process a single file path
fn process_file(
    file_path: &str,
    show_json: bool,
    show_sql: bool,
    enable_lineage: bool,
    show_lineage: bool,
    lineage_format: LineageFormat,
    db_pool: Option<&Pool>,
    verbose: bool,
) -> Result<(), Box<dyn Error>> {
    // Create root entity from file path (no registry - transforms are injected)
    let order = OrderCore::from_string(file_path)?;

    // Initialize lineage tracker if needed
    let mut lineage_tracker = if enable_lineage || show_lineage {
        Some(LineageTracker::new())
    } else {
        None
    };

    // Extract all entities (pass ownership of root)
    let (results, entity_shas) = extract_all_entities(order, lineage_tracker.as_mut())?;

    // Show lineage tree if requested
    if show_lineage {
        if let Some(ref tracker) = lineage_tracker {
            let tree = tracker.render_tree(lineage_format);
            eprintln!("{}\\n", tree);
        }
        // When showing lineage, suppress JSON/SQL output
        return Ok(());
    }

    // Execute to database if requested
    if let Some(pool) = db_pool {
        let stats = execute_to_database(&results, pool, verbose)?;

        if verbose {
            eprintln!("✓ Database execution complete:");
            eprintln!("  - Order: {} created, {} found", stats.order_created, stats.order_found);
            eprintln!("  - OrderLineItem: {} created, {} found", stats.order_line_item_created, stats.order_line_item_found);
        } else {
            eprintln!("✓ Database execution successful");
        }

        // When executing to database, suppress JSON/SQL output
        return Ok(());
    }

    // Output JSON if requested
    if show_json {
        output_json_entities(&results, lineage_tracker.as_ref(), &entity_shas, enable_lineage)?;
    }

    // Output SQL if requested
    if show_sql {
        output_sql_statements(&results)?;
    }

    Ok(())
}

/// Extract all entities from root entity
fn extract_all_entities(
    order: OrderCore,
    mut lineage_tracker: Option<&mut LineageTracker>,
) -> Result<(ParseResults, HashMap<String, String>), Box<dyn Error>> {
    let mut entity_shas: HashMap<String, String> = HashMap::new();

    if let Some(tracker) = lineage_tracker.as_mut() {
        let root_fields = entity_to_fields(&order);
        let root_sha = tracker.compute_sha("Order", &root_fields, &[], true);
        entity_shas.insert("Order".to_string(), root_sha);
    }

    let order_line_item = OrderLineItemCore::from_parent_repeated(&order)?;
    if let Some(tracker) = lineage_tracker.as_mut() {
        for (idx, item) in order_line_item.iter().enumerate() {
            let fields = entity_to_fields(item);
            let parent_shas: Vec<String> = vec![
                entity_shas.get("Order").cloned().unwrap_or_default(),
            ];
            let sha = tracker.compute_sha("OrderLineItem", &fields, &parent_shas, true);
            entity_shas.insert(format!("OrderLineItem[{}]", idx), sha);
        }
    }

    Ok((ParseResults {
        order,
        order_line_item,
    }, entity_shas))
}

/// Output all entities as JSON Lines
fn output_json_entities(
    results: &ParseResults,
    lineage_tracker: Option<&LineageTracker>,
    entity_shas: &HashMap<String, String>,
    enable_lineage: bool,
) -> Result<(), Box<dyn Error>> {
    {
        let mut json = serde_json::json!({
            "entity_type": "Order",
            "data": serde_json::to_value(&results.order)?,
        });
        if enable_lineage {
            if let Some(tracker) = lineage_tracker {
                if let Some(sha) = entity_shas.get("Order") {
                    let lineage = tracker.create_metadata(sha);
                    json["lineage"] = serde_json::to_value(&lineage)?;
                }
            }
        }
        println!("{}", serde_json::to_string(&json)?);
    }
    for (i, entity) in results.order_line_item.iter().enumerate() {
        let mut json = serde_json::json!({
            "entity_type": format!("OrderLineItem[{}]", i),
            "data": serde_json::to_value(entity)?,
        });
        if enable_lineage {
            if let Some(tracker) = lineage_tracker {
                let sha_key = format!("OrderLineItem[{}]", i);
                if let Some(sha) = entity_shas.get(&sha_key) {
                    let lineage = tracker.create_metadata(sha);
                    json["lineage"] = serde_json::to_value(&lineage)?;
                }
            }
        }
        println!("{}", serde_json::to_string(&json)?);
    }
    Ok(())
}

/// Output SQL statements for permanent entities
fn output_sql_statements(results: &ParseResults) -> Result<(), Box<dyn Error>> {
    println!("-- ========================================");
    println!("-- SQL Statements (Dry-Run Mode)");
    println!("-- ========================================");
    println!();
    for (i, entity) in results.order_line_item.iter().enumerate() {
        output_order_line_item_sql(entity, Some(i))?;
    }
    output_order_sql(&results.order, None)?;
    Ok(())
}

/// Output SQL for OrderLineItem entity
fn output_order_line_item_sql(entity: &OrderLineItemCore, index: Option<usize>) -> Result<(), Box<dyn Error>> {
    println!("-- ========================================");
    if let Some(i) = index {
        println!("-- OrderLineItem[{}]", i);
    } else {
        println!("-- OrderLineItem");
    }
    println!("-- Table: order_line_items");
    println!("-- Unicity fields: order_key, line_number");
    println!("-- ========================================");
    println!();
    println!("SELECT * FROM order_line_items");
    println!("WHERE");
    println!("      order_key {}", sql_cmp_string(&entity.order_key));
    println!("  AND line_number {}", sql_cmp(&entity.line_number));
    println!("LIMIT 1;");
    println!();
    println!("-- If not found:");
    println!("INSERT INTO order_line_items (order_key, line_number, part_key, supplier_key, quantity, extended_price, discount, tax, return_flag, line_status, ship_date, commit_date, receipt_date)");
    println!("VALUES");
    print!("  (");
    print!("{}", sql_opt_string(&entity.order_key));
    print!(", ");
    print!("{}", sql_opt(&entity.line_number));
    print!(", ");
    print!("{}", sql_opt_string(&entity.part_key));
    print!(", ");
    print!("{}", sql_opt_string_option(&entity.supplier_key));
    print!(", ");
    print!("{}", sql_opt(&entity.quantity));
    print!(", ");
    print!("{}", sql_opt(&entity.extended_price));
    print!(", ");
    print!("{}", sql_opt_option(&entity.discount));
    print!(", ");
    print!("{}", sql_opt_option(&entity.tax));
    print!(", ");
    print!("{}", sql_opt_string_option(&entity.return_flag));
    print!(", ");
    print!("{}", sql_opt_string_option(&entity.line_status));
    print!(", ");
    print!("{}", sql_opt_string_option(&entity.ship_date));
    print!(", ");
    print!("{}", sql_opt_string_option(&entity.commit_date));
    print!(", ");
    print!("{}", sql_opt_string_option(&entity.receipt_date));
    println!(")");
    println!(";");
    println!();
    Ok(())
}

/// Output SQL for Order entity
fn output_order_sql(entity: &OrderCore, index: Option<usize>) -> Result<(), Box<dyn Error>> {
    println!("-- ========================================");
    if let Some(i) = index {
        println!("-- Order[{}]", i);
    } else {
        println!("-- Order");
    }
    println!("-- Table: orders");
    println!("-- Unicity fields: order_key");
    println!("-- ========================================");
    println!();
    println!("SELECT * FROM orders");
    println!("WHERE");
    println!("      order_key {}", sql_cmp_string(&entity.order_key));
    println!("LIMIT 1;");
    println!();
    println!("-- If not found:");
    println!("INSERT INTO orders (order_key, customer_key, order_status, total_price, order_date, order_priority, clerk, ship_priority, comment, line_items)");
    println!("VALUES");
    print!("  (");
    print!("{}", sql_opt_string(&entity.order_key));
    print!(", ");
    print!("{}", sql_opt_string(&entity.customer_key));
    print!(", ");
    print!("{}", sql_opt_string(&entity.order_status));
    print!(", ");
    print!("{}", sql_opt(&entity.total_price));
    print!(", ");
    print!("{}", sql_opt_string(&entity.order_date));
    print!(", ");
    print!("{}", sql_opt_string_option(&entity.order_priority));
    print!(", ");
    print!("{}", sql_opt_string_option(&entity.clerk));
    print!(", ");
    print!("{}", sql_opt_option(&entity.ship_priority));
    print!(", ");
    print!("{}", sql_opt_string_option(&entity.comment));
    print!(", ");
    print!("{}", sql_opt_json_array(&entity.line_items));
    println!(")");
    println!(";");
    println!();
    Ok(())
}

/// Output SQL for Customer entity
fn output_customer_sql(entity: &CustomerCore, index: Option<usize>) -> Result<(), Box<dyn Error>> {
    println!("-- ========================================");
    if let Some(i) = index {
        println!("-- Customer[{}]", i);
    } else {
        println!("-- Customer");
    }
    println!("-- Table: customers");
    println!("-- Unicity fields: customer_key");
    println!("-- ========================================");
    println!();
    println!("SELECT * FROM customers");
    println!("WHERE");
    println!("      customer_key {}", sql_cmp_string(&entity.customer_key));
    println!("LIMIT 1;");
    println!();
    println!("-- If not found:");
    println!("INSERT INTO customers (customer_key, name, address, nation_key, phone, account_balance, market_segment, comment)");
    println!("VALUES");
    print!("  (");
    print!("{}", sql_opt_string(&entity.customer_key));
    print!(", ");
    print!("{}", sql_opt_string(&entity.name));
    print!(", ");
    print!("{}", sql_opt_string_option(&entity.address));
    print!(", ");
    print!("{}", sql_opt_string_option(&entity.nation_key));
    print!(", ");
    print!("{}", sql_opt_string_option(&entity.phone));
    print!(", ");
    print!("{}", sql_opt(&entity.account_balance));
    print!(", ");
    print!("{}", sql_opt_string_option(&entity.market_segment));
    print!(", ");
    print!("{}", sql_opt_string_option(&entity.comment));
    println!(")");
    println!(";");
    println!();
    Ok(())
}

/// Output SQL for Product entity
fn output_product_sql(entity: &ProductCore, index: Option<usize>) -> Result<(), Box<dyn Error>> {
    println!("-- ========================================");
    if let Some(i) = index {
        println!("-- Product[{}]", i);
    } else {
        println!("-- Product");
    }
    println!("-- Table: products");
    println!("-- Unicity fields: part_key");
    println!("-- ========================================");
    println!();
    println!("SELECT * FROM products");
    println!("WHERE");
    println!("      part_key {}", sql_cmp_string(&entity.part_key));
    println!("LIMIT 1;");
    println!();
    println!("-- If not found:");
    println!("INSERT INTO products (part_key, name, manufacturer, brand, product_type, size, container, retail_price, comment)");
    println!("VALUES");
    print!("  (");
    print!("{}", sql_opt_string(&entity.part_key));
    print!(", ");
    print!("{}", sql_opt_string(&entity.name));
    print!(", ");
    print!("{}", sql_opt_string_option(&entity.manufacturer));
    print!(", ");
    print!("{}", sql_opt_string_option(&entity.brand));
    print!(", ");
    print!("{}", sql_opt_string_option(&entity.product_type));
    print!(", ");
    print!("{}", sql_opt_option(&entity.size));
    print!(", ");
    print!("{}", sql_opt_string_option(&entity.container));
    print!(", ");
    print!("{}", sql_opt(&entity.retail_price));
    print!(", ");
    print!("{}", sql_opt_string_option(&entity.comment));
    println!(")");
    println!(";");
    println!();
    Ok(())
}

/// Execute entities to database using Diesel
fn execute_to_database(
    results: &ParseResults,
    pool: &Pool,
    verbose: bool,
) -> Result<ExecutionStats, Box<dyn Error>> {
    use diesel::prelude::*;

    let mut stats = ExecutionStats::default();
    let mut conn = pool.get()
        .map_err(|e| format!("Failed to get database connection: {}", e))?;

    // Execute in transaction for atomicity
    conn.transaction::<_, Box<dyn Error>, _>(|conn| {
        // Process Order (singleton)
        if verbose {
            eprintln!("Inserting Order: {:?}", results.order.order_key);
        }

        let new_item: NewOrder = (&results.order).into();

        use _rust::schema::orders::dsl::*;
        let existing = orders
            .filter(order_key.eq(&new_item.order_key))
            .first::<Order>(conn)
            .optional()?;

        match existing {
            Some(_) => {
                if verbose {
                    eprintln!("  ✓ Found existing");
                }
                stats.order_found += 1;
            }
            None => {
                // Insert new record
                diesel::insert_into(orders)
                    .values(new_item)
                    .execute(conn)?;

                if verbose {
                    eprintln!("  ✓ Created new");
                }
                stats.order_created += 1;
            }
        }

        // Process OrderLineItem (repeated)
        if verbose {
            eprintln!("Inserting {} line items...", results.order_line_item.len());
        }

        for (idx, item_core) in results.order_line_item.iter().enumerate() {
            let new_item: NewOrderLineItem = item_core.into();

            if verbose {
                eprintln!("  - Item {}: order_key={:?}, line_number={:?}",
                    idx + 1, new_item.order_key, new_item.line_number);
            }

            use _rust::schema::order_line_items::dsl::*;
            let existing = order_line_items
                .filter(order_key.eq(&new_item.order_key))
                .filter(line_number.eq(&new_item.line_number))
                .first::<OrderLineItem>(conn)
                .optional()?;

            match existing {
                Some(_) => {
                    if verbose {
                        eprintln!("    ✓ Found existing");
                    }
                    stats.order_line_item_found += 1;
                }
                None => {
                    // Insert new record
                    diesel::insert_into(order_line_items)
                        .values(new_item)
                        .execute(conn)?;

                    if verbose {
                        eprintln!("    ✓ Created new");
                    }
                    stats.order_line_item_created += 1;
                }
            }
        }

        Ok(stats)
    })
}

/// Format any value as SQL literal
fn sql_opt<T: std::fmt::Display>(value: &T) -> String {
    format!("{}", value)
}

/// Format Option<T> as SQL value
fn sql_opt_option<T: std::fmt::Display>(opt: &Option<T>) -> String {
    match opt {
        Some(v) => format!("{}", v),
        None => "NULL".to_string(),
    }
}

/// Format String as SQL string literal
fn sql_opt_string(s: &String) -> String {
    format!("'{}'", sql_escape(s))
}

/// Format Option<String> as SQL string literal
fn sql_opt_string_option(opt: &Option<String>) -> String {
    match opt {
        Some(s) => format!("'{}'", sql_escape(s)),
        None => "NULL".to_string(),
    }
}

/// Format Vec<serde_json::Value> as SQL (JSON array)
fn sql_opt_json_array(arr: &Vec<serde_json::Value>) -> String {
    match serde_json::to_string(arr) {
        Ok(json) => format!("'{}'", sql_escape(&json)),
        Err(_) => "NULL".to_string(),
    }
}

/// Format String as SQL comparison (field = 'value')
fn sql_cmp_string(s: &String) -> String {
    format!("= '{}'", sql_escape(s))
}

/// Format Option<String> as SQL comparison (field = 'value' OR field IS NULL)
fn sql_cmp_string_option(opt: &Option<String>) -> String {
    match opt {
        Some(s) => format!("= '{}'", sql_escape(s)),
        None => "IS NULL".to_string(),
    }
}

/// Format numeric value as SQL comparison
fn sql_cmp<T: std::fmt::Display>(value: &T) -> String {
    format!("= {}", value)
}

/// Format Option<T> as SQL comparison
fn sql_cmp_option<T: std::fmt::Display>(opt: &Option<T>) -> String {
    match opt {
        Some(v) => format!("= {}", v),
        None => "IS NULL".to_string(),
    }
}

/// Escape string for SQL (single quotes)
fn sql_escape(s: &str) -> String {
    s.replace('\'', "''")
}
