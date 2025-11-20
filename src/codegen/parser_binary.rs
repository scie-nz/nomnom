//! Parser binary code generation.
//!
//! This module generates a standalone Rust binary that:
//! 1. Reads file paths from stdin (one per line)
//! 2. Extracts all entities from each file
//! 3. Outputs JSON Lines format (one entity per line)
//! 4. Outputs SQL statements with inlined values (dry-run mode)
//!
//! Key principle: **NO HARDCODED FORMAT LOGIC**
//! Everything is read from entity YAML configurations:
//! - Root entity discovery (type: root)
//! - Extraction order (computed from dependency graph)
//! - Persistence config (from entity.persistence.database)
//! - Unicity fields (from persistence.database.unicity_fields)

use crate::codegen::types::EntityDef;
use crate::codegen::utils::to_snake_case;
use crate::codegen::ProjectBuildConfig;
use crate::codegen::lineage::{generate_lineage_code, generate_entity_to_fields_helper};
use std::collections::{HashMap, VecDeque};

// Transform registry is now provided by hl7utils
// No need to discover or generate transforms here

/// Generate the complete parser binary source code
pub fn generate_parser_binary(
    _config: &ProjectBuildConfig,
    entities: &[EntityDef],
) -> Result<String, String> {
    // 1. Filter out reference entities - they're not processed by the parser
    // Reference entities are pre-loaded data (e.g., Customer, Product tables)
    let processable_entities: Vec<&EntityDef> = entities.iter()
        .filter(|e| e.source_type.to_lowercase() != "reference")
        .collect();

    if processable_entities.is_empty() {
        return Err("No processable entities found. All entities are reference data.".to_string());
    }

    // 2. Find root entity from processable entities
    let root_entity = processable_entities.iter()
        .find(|e| e.is_root())
        .ok_or("No root entity found in entity configurations. Expected at least one entity with type: root")?;

    // 3. Compute extraction order (topological sort - only processable entities)
    let extraction_order = compute_extraction_order(&processable_entities, root_entity)?;

    // 3. Identify permanent entities (have persistence config with database, not abstract)
    let permanent_entities: Vec<&EntityDef> = entities.iter()
        .filter(|e| e.is_persistent(entities) && !e.is_abstract)
        .collect();

    // Check if database support is needed (any persistent entities exist)
    let has_database_support = !permanent_entities.is_empty();

    // 4. Generate code sections
    let mut code = String::new();
    code.push_str(&generate_header());
    code.push_str(&generate_imports(has_database_support));
    code.push_str(&generate_lineage_code());
    code.push_str(&generate_entity_to_fields_helper());
    code.push_str(&generate_cli_struct(has_database_support));
    code.push_str(&generate_parse_results_struct(&extraction_order));
    if has_database_support {
        code.push_str(&generate_execution_stats_struct(&extraction_order, entities));
    }
    code.push_str(&generate_main_function(root_entity, has_database_support, &extraction_order, entities));
    code.push_str(&generate_extraction_function(root_entity, &extraction_order, entities));
    code.push_str(&generate_json_output_function(&extraction_order));
    code.push_str(&generate_sql_output_function(&permanent_entities, entities));
    if has_database_support {
        code.push_str(&generate_execute_to_database_function(&extraction_order, &permanent_entities, entities));
    }
    code.push_str(&generate_sql_helpers());

    Ok(code)
}

/// Compute topological ordering of entities for extraction
fn compute_extraction_order(
    entities: &[&EntityDef],
    root_entity: &EntityDef,
) -> Result<Vec<EntityDef>, String> {
    // Build entity name -> entity map
    let entity_map: HashMap<String, EntityDef> = entities.iter()
        .map(|e| (e.name.clone(), (*e).clone()))
        .collect();

    // Build dependency graph (entity -> parents)
    let mut graph: HashMap<String, Vec<String>> = HashMap::new();
    let mut in_degree: HashMap<String, usize> = HashMap::new();

    for entity in entities {
        in_degree.insert(entity.name.clone(), 0);
        graph.insert(entity.name.clone(), vec![]);
    }

    for entity in entities {
        let parents = entity.get_parents();
        for parent in parents {
            if !entity_map.contains_key(&parent) {
                return Err(format!(
                    "Entity '{}' references unknown parent '{}'",
                    entity.name, parent
                ));
            }
            // Add edge: parent -> entity (parent must be extracted before entity)
            graph.get_mut(&parent).unwrap().push(entity.name.clone());
            *in_degree.get_mut(&entity.name).unwrap() += 1;
        }
    }

    // Topological sort (Kahn's algorithm)
    let mut queue: VecDeque<String> = VecDeque::new();
    let mut result: Vec<EntityDef> = Vec::new();

    // Start with root entity (has no parents)
    queue.push_back(root_entity.name.clone());

    while let Some(entity_name) = queue.pop_front() {
        let entity = entity_map.get(&entity_name)
            .ok_or(format!("Entity '{}' not found in entity map", entity_name))?;
        result.push(entity.clone());

        // Process children
        if let Some(children) = graph.get(&entity_name) {
            for child in children {
                let degree = in_degree.get_mut(child).unwrap();
                *degree -= 1;
                if *degree == 0 {
                    queue.push_back(child.clone());
                }
            }
        }
    }

    // Check for cycles or unreachable entities
    if result.len() != entities.len() {
        // Find which entities were not processed
        let processed_names: std::collections::HashSet<_> = result.iter()
            .map(|e| e.name.as_str())
            .collect();
        let unprocessed: Vec<_> = entities.iter()
            .filter(|e| !processed_names.contains(e.name.as_str()))
            .map(|e| &e.name)
            .collect();

        // Check if unprocessed entities have dependencies
        let mut has_dependencies = false;
        for entity in entities {
            if !processed_names.contains(entity.name.as_str()) {
                let parents = entity.get_parents();
                if !parents.is_empty() {
                    has_dependencies = true;
                    break;
                }
            }
        }

        if has_dependencies {
            return Err(format!(
                "Circular dependency detected in entity graph.\nUnprocessed entities: {:?}\nThese entities form a dependency cycle.",
                unprocessed
            ));
        } else {
            return Err(format!(
                "Unreachable entities detected in graph.\nUnprocessed entities: {:?}\nThese entities are not connected to the root entity '{}' and cannot be extracted.\nNote: Parser binary only supports single-root entity graphs.",
                unprocessed,
                root_entity.name
            ));
        }
    }

    Ok(result)
}

/// Generate file header with documentation
fn generate_header() -> String {
    "//! Auto-generated parser binary from nomnom entity configurations
//!
//! This binary reads file paths from stdin and outputs:
//! - JSON Lines: One entity per line in JSON format
//! - SQL: Database queries with inlined values (dry-run mode)
//!
//! Generated code is 100% config-driven - no hardcoded business logic.

#![allow(unused_imports)]
#![allow(dead_code)]

".to_string()
}

/// Generate imports
fn generate_imports(has_database_support: bool) -> String {
    let mut code = String::new();

    // Basic imports
    code.push_str("use std::io::{self, BufRead};\n");
    code.push_str("use std::error::Error;\n");
    code.push_str("use clap::Parser;\n");
    code.push_str("use std::collections::HashMap;\n");
    code.push_str("use serde_json::Value;\n");
    code.push_str("use std::path::Path;\n");
    code.push_str("\n");

    // Import from the library crate
    code.push_str("// Import from the library crate (lib name is _rust)\n");
    code.push_str("use _rust::generated::*;\n");
    code.push_str("\n");

    // Database imports for --execute-db mode
    if has_database_support {
        code.push_str("// Database imports for --execute-db mode\n");
        code.push_str("use _rust::db::{Database, DatabaseConfig, Pool, operations::GetOrCreate};\n");
        code.push_str("use _rust::models::*;\n");
        code.push_str("\n");
    }

    code.push_str("// Note: Transforms are now injected directly into generated.rs\n");
    code.push_str("// No registry needed - entity constructors call transform functions directly\n");
    code.push_str("\n");

    code
}

/// Generate CLI argument parser
fn generate_cli_struct(has_database_support: bool) -> String {
    let mut code = String::new();

    code.push_str(r#"/// CLI arguments for parser binary
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
"#);

    if has_database_support {
        code.push_str(r#"
    /// Execute statements directly against database (requires DATABASE_URL env var)
    #[arg(long)]
    execute_db: bool,

    /// Show verbose output (detailed execution logs)
    #[arg(long, short)]
    verbose: bool,
"#);
    }

    code.push_str("}\n\n");
    code
}

/// Generate ParseResults struct to hold all extracted entities
fn generate_parse_results_struct(extraction_order: &[EntityDef]) -> String {
    let mut code = String::new();

    code.push_str("/// Results from parsing a single file\n");
    code.push_str("#[derive(Debug)]\n");
    code.push_str("struct ParseResults {\n");

    for entity in extraction_order {
        // Skip abstract entities
        if entity.is_abstract {
            continue;
        }

        let field_name = to_snake_case(&entity.name);
        let type_name = format!("{}Core", entity.name);

        // Check if entity is repeated (either via repetition field or repeated_for)
        let is_repeated = entity.repetition.as_ref().map(|r| r == "repeated").unwrap_or(false)
            || entity.repeated_for.is_some();

        let field_type = if is_repeated {
            format!("Vec<{}>", type_name)
        } else {
            type_name
        };

        code.push_str(&format!("    {}: {},\n", field_name, field_type));
    }

    code.push_str("}\n\n");
    code
}

/// Generate ExecutionStats struct for database operations
fn generate_execution_stats_struct(extraction_order: &[EntityDef], all_entities: &[EntityDef]) -> String {
    let mut code = String::new();

    code.push_str("/// Statistics from database execution\n");
    code.push_str("#[derive(Debug, Default)]\n");
    code.push_str("struct ExecutionStats {\n");

    // Generate fields for each persistent, non-abstract, non-reference entity
    for entity in extraction_order {
        // Skip non-persistent or abstract entities
        if !entity.is_persistent(all_entities) || entity.is_abstract {
            continue;
        }

        // Skip reference entities
        if entity.source_type.to_lowercase() == "reference" {
            continue;
        }

        let field_name = to_snake_case(&entity.name);
        code.push_str(&format!("    {}_created: usize,\n", field_name));
        code.push_str(&format!("    {}_found: usize,\n", field_name));
    }

    code.push_str("}\n\n");
    code
}

/// Generate main function
fn generate_main_function(root_entity: &EntityDef, has_database_support: bool, extraction_order: &[EntityDef], all_entities: &[EntityDef]) -> String {
    let root_snake = to_snake_case(&root_entity.name);
    let root_core = format!("{}Core", root_entity.name);

    let mut code = String::new();

    // Main function signature and CLI parsing
    code.push_str("fn main() -> Result<(), Box<dyn Error>> {\n");
    code.push_str("    let cli = Cli::parse();\n\n");

    // Output mode determination
    if has_database_support {
        code.push_str("    // Determine output mode\n");
        code.push_str("    let show_json = !cli.sql_only && !cli.execute_db;\n");
        code.push_str("    let show_sql = !cli.json_only && !cli.execute_db;\n\n");
    } else {
        code.push_str("    // Determine output mode\n");
        code.push_str("    let show_json = !cli.sql_only;\n");
        code.push_str("    let show_sql = !cli.json_only;\n\n");
    }

    code.push_str("    // If no flags specified, --dry-run is default (show both)\n");
    code.push_str("    let show_json = if cli.dry_run { true } else { show_json };\n");
    code.push_str("    let show_sql = if cli.dry_run { true } else { show_sql };\n\n");

    code.push_str("    // Parse lineage format\n");
    code.push_str("    let lineage_format = if cli.lineage_format == \"detailed\" {\n");
    code.push_str("        LineageFormat::Detailed\n");
    code.push_str("    } else {\n");
    code.push_str("        LineageFormat::Compact\n");
    code.push_str("    };\n\n");

    // Database initialization if supported
    if has_database_support {
        code.push_str("    // Initialize database connection pool if --execute-db is set\n");
        code.push_str("    let db_pool: Option<Pool> = if cli.execute_db {\n");
        code.push_str("        let db_url = std::env::var(\"DATABASE_URL\")\n");
        code.push_str("            .expect(\"DATABASE_URL must be set for --execute-db mode\");\n\n");
        code.push_str("        if cli.verbose {\n");
        code.push_str("            eprintln!(\"Connecting to database: {}\", db_url);\n");
        code.push_str("        }\n\n");
        code.push_str("        let database = Database::new(&db_url)\n");
        code.push_str("            .expect(\"Failed to connect to database\");\n\n");
        code.push_str("        if cli.verbose {\n");
        code.push_str("            eprintln!(\"Database connection established\");\n");
        code.push_str("        }\n\n");
        code.push_str("        Some(database.pool().clone())\n");
        code.push_str("    } else {\n");
        code.push_str("        None\n");
        code.push_str("    };\n\n");
    }

    // Process files from stdin
    code.push_str("    // Read file paths from stdin (one per line)\n");
    code.push_str("    let stdin = io::stdin();\n");
    code.push_str("    for line in stdin.lock().lines() {\n");
    code.push_str("        let file_path = line?;\n\n");
    code.push_str("        // Process file\n");

    if has_database_support {
        code.push_str("        match process_file(\n");
        code.push_str("            &file_path,\n");
        code.push_str("            show_json,\n");
        code.push_str("            show_sql,\n");
        code.push_str("            cli.lineage,\n");
        code.push_str("            cli.show_lineage,\n");
        code.push_str("            lineage_format,\n");
        code.push_str("            db_pool.as_ref(),\n");
        code.push_str("            cli.verbose,\n");
        code.push_str("        ) {\n");
    } else {
        code.push_str("        match process_file(&file_path, show_json, show_sql, cli.lineage, cli.show_lineage, lineage_format) {\n");
    }

    code.push_str("            Ok(_) => {},\n");
    code.push_str("            Err(e) => {\n");
    code.push_str("                eprintln!(\"Error processing file '{}': {}\", file_path, e);\n");
    code.push_str("                // Continue to next file\n");
    code.push_str("            }\n");
    code.push_str("        }\n");
    code.push_str("    }\n\n");
    code.push_str("    Ok(())\n");
    code.push_str("}\n\n");

    // process_file function
    code.push_str("/// Process a single file path\n");
    code.push_str("fn process_file(\n");
    code.push_str("    file_path: &str,\n");
    code.push_str("    show_json: bool,\n");
    code.push_str("    show_sql: bool,\n");
    code.push_str("    enable_lineage: bool,\n");
    code.push_str("    show_lineage: bool,\n");
    code.push_str("    lineage_format: LineageFormat,\n");
    if has_database_support {
        code.push_str("    db_pool: Option<&Pool>,\n");
        code.push_str("    verbose: bool,\n");
    }
    code.push_str(") -> Result<(), Box<dyn Error>> {\n");
    code.push_str(&format!("    // Create root entity from file path (no registry - transforms are injected)\n"));
    code.push_str(&format!("    let {} = {}::from_string(file_path)?;\n\n", root_snake, root_core));

    code.push_str("    // Initialize lineage tracker if needed\n");
    code.push_str("    let mut lineage_tracker = if enable_lineage || show_lineage {\n");
    code.push_str("        Some(LineageTracker::new())\n");
    code.push_str("    } else {\n");
    code.push_str("        None\n");
    code.push_str("    };\n\n");

    code.push_str("    // Extract all entities (pass ownership of root)\n");
    code.push_str(&format!("    let (results, entity_shas) = extract_all_entities({}, lineage_tracker.as_mut())?;\n\n", root_snake));

    code.push_str("    // Show lineage tree if requested\n");
    code.push_str("    if show_lineage {\n");
    code.push_str("        if let Some(ref tracker) = lineage_tracker {\n");
    code.push_str("            let tree = tracker.render_tree(lineage_format);\n");
    code.push_str("            eprintln!(\"{}\\\\n\", tree);\n");
    code.push_str("        }\n");
    code.push_str("        // When showing lineage, suppress JSON/SQL output\n");
    code.push_str("        return Ok(());\n");
    code.push_str("    }\n\n");

    // Database execution if supported
    if has_database_support {
        code.push_str("    // Execute to database if requested\n");
        code.push_str("    if let Some(pool) = db_pool {\n");
        code.push_str("        let stats = execute_to_database(&results, pool, verbose)?;\n\n");
        code.push_str("        if verbose {\n");
        code.push_str("            eprintln!(\"✓ Database execution complete:\");\n");

        // Generate verbose output for each persistent, non-abstract, non-reference entity
        for entity in extraction_order {
            // Skip non-persistent or abstract entities
            if !entity.is_persistent(all_entities) || entity.is_abstract {
                continue;
            }

            // Skip reference entities
            if entity.source_type.to_lowercase() == "reference" {
                continue;
            }

            let var_name = to_snake_case(&entity.name);
            code.push_str(&format!(
                "            eprintln!(\"  - {}: {{}} created, {{}} found\", stats.{}_created, stats.{}_found);\n",
                entity.name, var_name, var_name
            ));
        }

        code.push_str("        } else {\n");
        code.push_str("            eprintln!(\"✓ Database execution successful\");\n");
        code.push_str("        }\n\n");
        code.push_str("        // When executing to database, suppress JSON/SQL output\n");
        code.push_str("        return Ok(());\n");
        code.push_str("    }\n\n");
    }

    code.push_str("    // Output JSON if requested\n");
    code.push_str("    if show_json {\n");
    code.push_str("        output_json_entities(&results, lineage_tracker.as_ref(), &entity_shas, enable_lineage)?;\n");
    code.push_str("    }\n\n");

    code.push_str("    // Output SQL if requested\n");
    code.push_str("    if show_sql {\n");
    code.push_str("        output_sql_statements(&results)?;\n");
    code.push_str("    }\n\n");

    code.push_str("    Ok(())\n");
    code.push_str("}\n\n");

    code
}

/// Generate entity extraction function
fn generate_extraction_function(
    root_entity: &EntityDef,
    extraction_order: &[EntityDef],
    all_entities: &[EntityDef],
) -> String {
    let root_snake = to_snake_case(&root_entity.name);
    let root_type = format!("{}Core", root_entity.name);

    let mut code = String::new();
    code.push_str("/// Extract all entities from root entity\n");
    code.push_str("fn extract_all_entities(\n");
    code.push_str(&format!("    {}: {},\n", root_snake, root_type));
    code.push_str("    mut lineage_tracker: Option<&mut LineageTracker>,\n");
    code.push_str(") -> Result<(ParseResults, HashMap<String, String>), Box<dyn Error>> {\n");

    // Initialize SHA storage if lineage tracking is enabled
    code.push_str("    let mut entity_shas: HashMap<String, String> = HashMap::new();\n\n");

    // Compute SHA for root entity if lineage tracking is enabled
    code.push_str("    if let Some(tracker) = lineage_tracker.as_mut() {\n");
    code.push_str(&format!("        let root_fields = entity_to_fields(&{});\n", root_snake));
    let root_is_permanent = root_entity.is_persistent(all_entities);
    code.push_str(&format!("        let root_sha = tracker.compute_sha(\"{}\", &root_fields, &[], {});\n", root_entity.name, root_is_permanent));
    code.push_str(&format!("        entity_shas.insert(\"{}\".to_string(), root_sha);\n", root_entity.name));
    code.push_str("    }\n\n");

    // Generate extraction calls for each entity in topological order
    for entity in extraction_order {
        if entity.is_root() {
            continue; // Root already provided as parameter
        }

        // Skip abstract entities (they don't have constructors)
        if entity.is_abstract {
            continue;
        }

        let var_name = to_snake_case(&entity.name);
        let type_name = format!("{}Core", entity.name);
        let parents = entity.get_parents();

        // Skip if no parents (should not happen after topological sort)
        if parents.is_empty() {
            code.push_str(&format!("    // WARNING: Entity {} has no parent!\n", entity.name));
            continue;
        }

        // Determine extraction method based on repetition and parents
        let is_repeated = entity.repetition.as_ref().map(|r| r == "repeated").unwrap_or(false)
            || entity.repeated_for.is_some();
        let has_single_parent = parents.len() == 1;

        if is_repeated && has_single_parent {
            // Repeated entity with single parent: Vec<EntityCore>
            // Use from_parent_repeated which returns Vec<Self>
            let parent_var = to_snake_case(&parents[0]);
            code.push_str(&format!(
                "    let {} = {}::from_parent_repeated(&{})?;\n",
                var_name, type_name, parent_var
            ));

            // Compute SHA for each item in repeated entity
            code.push_str("    if let Some(tracker) = lineage_tracker.as_mut() {\n");
            code.push_str(&format!("        for (idx, item) in {}.iter().enumerate() {{\n", var_name));
            code.push_str("            let fields = entity_to_fields(item);\n");
            code.push_str("            let parent_shas: Vec<String> = vec![\n");
            code.push_str(&format!("                entity_shas.get(\"{}\").cloned().unwrap_or_default(),\n", parents[0]));
            code.push_str("            ];\n");
            let is_permanent = entity.is_persistent(all_entities);
            code.push_str(&format!("            let sha = tracker.compute_sha(\"{}\", &fields, &parent_shas, {});\n", entity.name, is_permanent));
            code.push_str(&format!("            entity_shas.insert(format!(\"{}[{{}}]\", idx), sha);\n", entity.name));
            code.push_str("        }\n");
            code.push_str("    }\n");
        } else if is_repeated && !has_single_parent {
            // Repeated entity with multiple parents
            // One parent must be repeated - iterate over it
            // Some other parents might also be repeated (same_segment_as relationship)

            // Find the primary repeated parent (transient entity like Procedure)
            let mut repeated_parent_idx = None;
            for (i, parent) in parents.iter().enumerate() {
                let parent_entity = extraction_order.iter().find(|e| &e.name == parent);
                if let Some(p) = parent_entity {
                    if p.repetition.as_ref().map(|r| r == "repeated").unwrap_or(false) {
                        repeated_parent_idx = Some(i);
                        break;
                    }
                }
            }

            if let Some(repeated_idx) = repeated_parent_idx {
                // Check if any other parents are also repeated (same_segment_as case)
                let mut other_repeated_parents = Vec::new();
                for (i, parent) in parents.iter().enumerate() {
                    if i != repeated_idx {
                        let parent_entity = extraction_order.iter().find(|e| &e.name == parent);
                        if let Some(p) = parent_entity {
                            if p.repetition.as_ref().map(|r| r == "repeated").unwrap_or(false) {
                                other_repeated_parents.push(i);
                            }
                        }
                    }
                }

                let repeated_parent_var = to_snake_case(&parents[repeated_idx]);
                code.push_str(&format!("    let mut {} = Vec::new();\n", var_name));

                // Use indexed iteration if there are other repeated parents
                if !other_repeated_parents.is_empty() {
                    code.push_str(&format!("    for (idx, {}) in {}.iter().enumerate() {{\n",
                        to_snake_case(&format!("{}_item", parents[repeated_idx])),
                        repeated_parent_var
                    ));
                } else {
                    code.push_str(&format!("    for {} in &{} {{\n",
                        to_snake_case(&format!("{}_item", parents[repeated_idx])),
                        repeated_parent_var
                    ));
                }

                code.push_str(&format!("        let item = {}::from_sources(",  type_name));
                for (i, parent) in parents.iter().enumerate() {
                    if i > 0 {
                        code.push_str(", ");
                    }
                    if i == repeated_idx {
                        // Use the loop item variable
                        code.push_str(&to_snake_case(&format!("{}_item", parents[i])));
                    } else if other_repeated_parents.contains(&i) {
                        // Other repeated parent - access by index
                        let parent_var = to_snake_case(parent);
                        code.push_str(&format!("&{}[idx]", parent_var));
                    } else {
                        // Non-repeated parent - use reference
                        let parent_var = to_snake_case(parent);
                        code.push_str(&format!("&{}", parent_var));
                    }
                }
                code.push_str(")?;\n");

                // Compute SHA for the repeated multi-parent entity (before pushing)
                code.push_str("\n        if let Some(tracker) = lineage_tracker.as_mut() {\n");
                code.push_str("            let fields = entity_to_fields(&item);\n");
                code.push_str("            let parent_shas: Vec<String> = vec![\n");
                for (i, parent) in parents.iter().enumerate() {
                    if i == repeated_idx {
                        // Primary repeated parent - use idx if available
                        if !other_repeated_parents.is_empty() {
                            code.push_str(&format!("                entity_shas.get(&format!(\"{}[{{}}]\", idx)).cloned().unwrap_or_default(),\n", parent));
                        } else {
                            // No index available, use position in vec
                            code.push_str(&format!("                entity_shas.get(&format!(\"{}[{{}}]\", {}.len() - 1)).cloned().unwrap_or_default(),\n", parent, var_name));
                        }
                    } else if other_repeated_parents.contains(&i) {
                        // Other repeated parent
                        code.push_str(&format!("                entity_shas.get(&format!(\"{}[{{}}]\", idx)).cloned().unwrap_or_default(),\n", parent));
                    } else {
                        // Singleton parent
                        code.push_str(&format!("                entity_shas.get(\"{}\").cloned().unwrap_or_default(),\n", parent));
                    }
                }
                code.push_str("            ];\n");
                let is_permanent = entity.is_persistent(all_entities);
                code.push_str(&format!("            let sha = tracker.compute_sha(\"{}\", &fields, &parent_shas, {});\n", entity.name, is_permanent));
                code.push_str(&format!("            entity_shas.insert(format!(\"{}[{{}}]\", {}.len()), sha);\n", entity.name, var_name));
                code.push_str("        }\n");

                code.push_str(&format!("        {}.push(item);\n", var_name));
                code.push_str("    }\n");
            } else {
                // Fallback: no repeated parent found, use from_sources
                code.push_str(&format!(
                    "    let {} = {}::from_sources(",
                    var_name, type_name
                ));
                for (i, parent) in parents.iter().enumerate() {
                    if i > 0 {
                        code.push_str(", ");
                    }
                    let parent_var = to_snake_case(parent);
                    code.push_str(&format!("&{}", parent_var));
                }
                code.push_str(")?;\n");
            }
        } else {
            // Singleton entity: use from_sources
            // from_sources always takes (parent1, ..., parentN, registry) and returns Result<Self, String>
            code.push_str(&format!(
                "    let {} = {}::from_sources(",
                var_name, type_name
            ));
            for (i, parent) in parents.iter().enumerate() {
                if i > 0 {
                    code.push_str(", ");
                }
                let parent_var = to_snake_case(parent);
                code.push_str(&format!("&{}", parent_var));
            }
            code.push_str(")?;\n");

            // Compute SHA for singleton entity if lineage tracking is enabled
            code.push_str("    if let Some(tracker) = lineage_tracker.as_mut() {\n");
            code.push_str(&format!("        let fields = entity_to_fields(&{});\n", var_name));

            // Build parent SHA vector by looking up each parent's SHA
            code.push_str("        let parent_shas: Vec<String> = vec![\n");
            for parent in &parents {
                code.push_str(&format!("            entity_shas.get(\"{}\").cloned().unwrap_or_default(),\n", parent));
            }
            code.push_str("        ];\n");

            let is_permanent = entity.is_persistent(all_entities);
            code.push_str(&format!("        let sha = tracker.compute_sha(\"{}\", &fields, &parent_shas, {});\n", entity.name, is_permanent));
            code.push_str(&format!("        entity_shas.insert(\"{}\".to_string(), sha);\n", entity.name));
            code.push_str("    }\n");
        }
    }

    // Build ParseResults struct
    code.push_str("\n    Ok((ParseResults {\n");
    for entity in extraction_order {
        // Skip abstract entities
        if entity.is_abstract {
            continue;
        }
        let var_name = to_snake_case(&entity.name);
        code.push_str(&format!("        {},\n", var_name));
    }
    code.push_str("    }, entity_shas))\n");
    code.push_str("}\n\n");

    code
}

/// Generate JSON output function
fn generate_json_output_function(extraction_order: &[EntityDef]) -> String {
    let mut code = String::new();

    code.push_str("/// Output all entities as JSON Lines\n");
    code.push_str("fn output_json_entities(\n");
    code.push_str("    results: &ParseResults,\n");
    code.push_str("    lineage_tracker: Option<&LineageTracker>,\n");
    code.push_str("    entity_shas: &HashMap<String, String>,\n");
    code.push_str("    enable_lineage: bool,\n");
    code.push_str(") -> Result<(), Box<dyn Error>> {\n");

    for entity in extraction_order {
        // Skip abstract entities
        if entity.is_abstract {
            continue;
        }

        let var_name = to_snake_case(&entity.name);
        let entity_name = &entity.name;
        let is_repeated = entity.repetition.as_ref().map(|r| r == "repeated").unwrap_or(false)
            || entity.repeated_for.is_some();

        if is_repeated {
            // Repeated entity: output with index
            code.push_str(&format!(
                "    for (i, entity) in results.{}.iter().enumerate() {{\n",
                var_name
            ));
            code.push_str("        let mut json = serde_json::json!({\n");
            code.push_str(&format!(
                "            \"entity_type\": format!(\"{}[{{}}]\", i),\n",
                entity_name
            ));
            code.push_str("            \"data\": serde_json::to_value(entity)?,\n");
            code.push_str("        });\n");

            // Add lineage metadata if enabled
            code.push_str("        if enable_lineage {\n");
            code.push_str("            if let Some(tracker) = lineage_tracker {\n");
            code.push_str(&format!("                let sha_key = format!(\"{}[{{}}]\", i);\n", entity_name));
            code.push_str("                if let Some(sha) = entity_shas.get(&sha_key) {\n");
            code.push_str("                    let lineage = tracker.create_metadata(sha);\n");
            code.push_str("                    json[\"lineage\"] = serde_json::to_value(&lineage)?;\n");
            code.push_str("                }\n");
            code.push_str("            }\n");
            code.push_str("        }\n");

            code.push_str("        println!(\"{}\", serde_json::to_string(&json)?);\n");
            code.push_str("    }\n");
        } else {
            // Singleton entity
            code.push_str("    {\n");
            code.push_str("        let mut json = serde_json::json!({\n");
            code.push_str(&format!("            \"entity_type\": \"{}\",\n", entity_name));
            code.push_str(&format!("            \"data\": serde_json::to_value(&results.{})?,\n", var_name));
            code.push_str("        });\n");

            // Add lineage metadata if enabled
            code.push_str("        if enable_lineage {\n");
            code.push_str("            if let Some(tracker) = lineage_tracker {\n");
            code.push_str(&format!("                if let Some(sha) = entity_shas.get(\"{}\") {{\n", entity_name));
            code.push_str("                    let lineage = tracker.create_metadata(sha);\n");
            code.push_str("                    json[\"lineage\"] = serde_json::to_value(&lineage)?;\n");
            code.push_str("                }\n");
            code.push_str("            }\n");
            code.push_str("        }\n");

            code.push_str("        println!(\"{}\", serde_json::to_string(&json)?);\n");
            code.push_str("    }\n");
        }
    }

    code.push_str("    Ok(())\n");
    code.push_str("}\n\n");
    code
}

/// Generate SQL output function
fn generate_sql_output_function(permanent_entities: &[&EntityDef], all_entities: &[EntityDef]) -> String {
    let mut code = String::new();

    code.push_str("/// Output SQL statements for permanent entities\n");
    code.push_str("fn output_sql_statements(results: &ParseResults) -> Result<(), Box<dyn Error>> {\n");
    code.push_str("    println!(\"-- ========================================\");\n");
    code.push_str("    println!(\"-- SQL Statements (Dry-Run Mode)\");\n");
    code.push_str("    println!(\"-- ========================================\");\n");
    code.push_str("    println!();\n");

    // Filter out reference entities - they're not in ParseResults
    for entity in permanent_entities {
        // Skip reference entities
        if entity.source_type.to_lowercase() == "reference" {
            continue;
        }

        let function_name = format!("output_{}_sql", to_snake_case(&entity.name));
        let var_name = to_snake_case(&entity.name);
        let is_repeated = entity.repetition.as_ref().map(|r| r == "repeated").unwrap_or(false)
            || entity.repeated_for.is_some();

        if is_repeated {
            code.push_str(&format!(
                "    for (i, entity) in results.{}.iter().enumerate() {{\n",
                var_name
            ));
            code.push_str(&format!("        {}(entity, Some(i))?;\n", function_name));
            code.push_str("    }\n");
        } else {
            code.push_str(&format!("    {}(&results.{}, None)?;\n", function_name, var_name));
        }
    }

    code.push_str("    Ok(())\n");
    code.push_str("}\n\n");

    // Generate SQL output function for each permanent entity
    for entity in permanent_entities {
        code.push_str(&generate_entity_sql_function(entity, all_entities));
    }

    code
}

/// Get SQL helper function name for a field type
fn get_sql_opt_function(field_type: &str, nullable: bool) -> &'static str {
    match (field_type, nullable) {
        ("String", false) => "sql_opt_string",
        ("String", true) => "sql_opt_string_option",
        ("List[Object]" | "List[Json]", _) => "sql_opt_json_array",
        (_, true) => "sql_opt_option",
        (_, false) => "sql_opt",
    }
}

/// Get SQL comparison function name for a field type
fn get_sql_cmp_function(field_type: &str, nullable: bool) -> &'static str {
    match (field_type, nullable) {
        ("String", false) => "sql_cmp_string",
        ("String", true) => "sql_cmp_string_option",
        (_, true) => "sql_cmp_option",
        (_, false) => "sql_cmp",
    }
}

/// Generate SQL output function for a single entity
fn generate_entity_sql_function(entity: &EntityDef, all_entities: &[EntityDef]) -> String {
    let db_config = entity.get_database_config(all_entities)
        .expect("Permanent entity must have database config");

    let function_name = format!("output_{}_sql", to_snake_case(&entity.name));
    let entity_type = format!("{}Core", entity.name);
    let table_name = &db_config.conformant_table;
    let unicity_fields = &db_config.unicity_fields;

    let mut code = String::new();

    code.push_str(&format!("/// Output SQL for {} entity\n", entity.name));
    code.push_str(&format!(
        "fn {}(entity: &{}, index: Option<usize>) -> Result<(), Box<dyn Error>> {{\n",
        function_name, entity_type
    ));

    // Header
    code.push_str("    println!(\"-- ========================================\");\n");
    code.push_str(&format!(
        "    if let Some(i) = index {{\n        println!(\"-- {}[{{}}]\", i);\n    }} else {{\n        println!(\"-- {}\");\n    }}\n",
        entity.name, entity.name
    ));
    code.push_str(&format!(
        "    println!(\"-- Table: {}\");\n",
        table_name
    ));
    code.push_str(&format!(
        "    println!(\"-- Unicity fields: {}\");\n",
        unicity_fields.join(", ")
    ));
    code.push_str("    println!(\"-- ========================================\");\n");
    code.push_str("    println!();\n");

    // SELECT query
    code.push_str(&format!("    println!(\"SELECT * FROM {}\");\n", table_name));
    code.push_str("    println!(\"WHERE\");\n");

    for (i, field_name) in unicity_fields.iter().enumerate() {
        let separator = if i == 0 { "      " } else { "  AND " };
        // Find field definition to get type
        let field = entity.fields.iter().find(|f| &f.name == field_name);
        let cmp_fn = if let Some(f) = field {
            get_sql_cmp_function(&f.field_type, f.nullable)
        } else {
            "sql_cmp_string_option" // fallback
        };
        code.push_str(&format!(
            "    println!(\"{}{} {{}}\", {}(&entity.{}));\n",
            separator, field_name, cmp_fn, field_name
        ));
    }

    code.push_str("    println!(\"LIMIT 1;\");\n");
    code.push_str("    println!();\n");

    // INSERT query
    code.push_str("    println!(\"-- If not found:\");\n");

    // Collect all field names (excluding auto-generated PK if configured)
    let mut insert_fields: Vec<(&String, &String, bool)> = Vec::new();
    for field in &entity.fields {
        // Skip auto-generated conformant ID
        if db_config.autogenerate_conformant_id
            && field.name == db_config.conformant_id_column {
            continue;
        }
        insert_fields.push((&field.name, &field.field_type, field.nullable));
    }

    code.push_str(&format!(
        "    println!(\"INSERT INTO {} ({})\");\n",
        table_name,
        insert_fields.iter().map(|(name, _, _)| name.as_str()).collect::<Vec<_>>().join(", ")
    ));
    code.push_str("    println!(\"VALUES\");\n");
    code.push_str("    print!(\"  (\");\n");

    for (i, (field_name, field_type, nullable)) in insert_fields.iter().enumerate() {
        if i > 0 {
            code.push_str("    print!(\", \");\n");
        }
        let opt_fn = get_sql_opt_function(field_type, *nullable);
        code.push_str(&format!("    print!(\"{{}}\", {}(&entity.{}));\n", opt_fn, field_name));
    }

    code.push_str("    println!(\")\");\n");
    code.push_str("    println!(\";\");\n");  // Add semicolon to complete SQL statement
    code.push_str("    println!();\n");

    code.push_str("    Ok(())\n");
    code.push_str("}\n\n");

    code
}

/// Generate database execution function
fn generate_execute_to_database_function(
    extraction_order: &[EntityDef],
    permanent_entities: &[&EntityDef],
    all_entities: &[EntityDef],
) -> String {
    let mut code = String::new();

    code.push_str("/// Execute entities to database using Diesel\n");
    code.push_str("fn execute_to_database(\n");
    code.push_str("    results: &ParseResults,\n");
    code.push_str("    pool: &Pool,\n");
    code.push_str("    verbose: bool,\n");
    code.push_str(") -> Result<ExecutionStats, Box<dyn Error>> {\n");
    code.push_str("    use diesel::prelude::*;\n\n");
    code.push_str("    let mut stats = ExecutionStats::default();\n");
    code.push_str("    let mut conn = pool.get()\n");
    code.push_str("        .map_err(|e| format!(\"Failed to get database connection: {}\", e))?;\n\n");

    code.push_str("    // Execute in transaction for atomicity\n");
    code.push_str("    conn.transaction::<_, Box<dyn Error>, _>(|conn| {\n");

    // Collect all unique table names for imports
    let mut table_names = std::collections::HashSet::new();
    for entity in extraction_order {
        if entity.is_persistent(all_entities) && !entity.is_abstract && entity.source_type.to_lowercase() != "reference" {
            if let Some(db_config) = entity.get_database_config(all_entities) {
                table_names.insert(db_config.conformant_table.clone());
            }
        }
    }

    // Generate use statements for all unique tables
    for table_name in &table_names {
        code.push_str(&format!("        use _rust::schema::{};\n", table_name));
    }
    code.push_str("\n");

    // Process each persistent entity
    for entity in extraction_order {
        // Skip non-persistent or abstract entities
        if !entity.is_persistent(all_entities) || entity.is_abstract {
            continue;
        }

        // Skip reference entities
        if entity.source_type.to_lowercase() == "reference" {
            continue;
        }

        let var_name = to_snake_case(&entity.name);
        let type_name = format!("{}Core", entity.name);

        // If entity extends another, use parent's model types for database operations
        let base_entity_name = if let Some(ref parent_name) = entity.extends {
            parent_name.clone()
        } else {
            entity.name.clone()
        };

        let new_type_name = format!("New{}", base_entity_name);
        let model_type_name = base_entity_name;

        let db_config = entity.get_database_config(all_entities)
            .expect("Persistent entity must have database config");
        let table_name = &db_config.conformant_table;
        let unicity_fields = &db_config.unicity_fields;

        let is_repeated = entity.repetition.as_ref().map(|r| r == "repeated").unwrap_or(false)
            || entity.repeated_for.is_some();

        if is_repeated {
            // Process repeated entity (Vec<EntityCore>)
            code.push_str(&format!("        // Process {} (repeated)\n", entity.name));
            code.push_str("        if verbose {\n");
            code.push_str(&format!("            eprintln!(\"Inserting {{}} line items...\", results.{}.len());\n", var_name));
            code.push_str("        }\n\n");

            code.push_str(&format!("        for (idx, item_core) in results.{}.iter().enumerate() {{\n", var_name));
            code.push_str(&format!("            let new_item: {} = item_core.into();\n\n", new_type_name));

            code.push_str("            if verbose {\n");
            // Create verbose message with unicity fields
            let verbose_fields: Vec<String> = unicity_fields.iter()
                .map(|f| format!("{}={{:?}}", f))
                .collect();
            code.push_str(&format!("                eprintln!(\"  - Item {{}}: {}\",\n", verbose_fields.join(", ")));
            code.push_str("                    idx + 1");
            for field in unicity_fields {
                code.push_str(&format!(", new_item.{}", field));
            }
            code.push_str(");\n");
            code.push_str("            }\n\n");

            // Check if exists
            code.push_str(&format!("            let existing = {}::table\n", table_name));
            for (i, field) in unicity_fields.iter().enumerate() {
                if i == 0 {
                    code.push_str(&format!("                .filter({}::{}.eq(&new_item.{}))\n", table_name, field, field));
                } else {
                    code.push_str(&format!("                .filter({}::{}.eq(&new_item.{}))\n", table_name, field, field));
                }
            }
            code.push_str(&format!("                .first::<{}>(conn)\n", model_type_name));
            code.push_str("                .optional()?;\n\n");

            code.push_str("            match existing {\n");
            code.push_str("                Some(_) => {\n");
            code.push_str("                    if verbose {\n");
            code.push_str("                        eprintln!(\"    ✓ Found existing\");\n");
            code.push_str("                    }\n");
            code.push_str(&format!("                    stats.{}_found += 1;\n", var_name));
            code.push_str("                }\n");
            code.push_str("                None => {\n");

            // Insert new record
            code.push_str("                    // Insert new record\n");
            code.push_str(&format!("                    diesel::insert_into({}::table)\n", table_name));
            code.push_str("                        .values(new_item)\n");
            code.push_str("                        .execute(conn)?;\n\n");

            code.push_str("                    if verbose {\n");
            code.push_str("                        eprintln!(\"    ✓ Created new\");\n");
            code.push_str("                    }\n");
            code.push_str(&format!("                    stats.{}_created += 1;\n", var_name));
            code.push_str("                }\n");
            code.push_str("            }\n");
            code.push_str("        }\n\n");
        } else {
            // Process singleton entity
            code.push_str(&format!("        // Process {} (singleton)\n", entity.name));
            code.push_str("        if verbose {\n");
            code.push_str(&format!("            eprintln!(\"Inserting {}: {{:?}}\", results.{}.{});\n",
                entity.name, var_name, unicity_fields[0]));
            code.push_str("        }\n\n");

            code.push_str(&format!("        let new_item: {} = (&results.{}).into();\n\n", new_type_name, var_name));

            // Check if exists
            code.push_str(&format!("        let existing = {}::table\n", table_name));
            for (i, field) in unicity_fields.iter().enumerate() {
                if i == 0 {
                    code.push_str(&format!("            .filter({}::{}.eq(&new_item.{}))\n", table_name, field, field));
                } else {
                    code.push_str(&format!("            .filter({}::{}.eq(&new_item.{}))\n", table_name, field, field));
                }
            }
            code.push_str(&format!("            .first::<{}>(conn)\n", model_type_name));
            code.push_str("            .optional()?;\n\n");

            code.push_str("        match existing {\n");
            code.push_str("            Some(_) => {\n");
            code.push_str("                if verbose {\n");
            code.push_str("                    eprintln!(\"  ✓ Found existing\");\n");
            code.push_str("                }\n");
            code.push_str(&format!("                stats.{}_found += 1;\n", var_name));
            code.push_str("            }\n");
            code.push_str("            None => {\n");

            // Insert new record
            code.push_str("                // Insert new record\n");
            code.push_str(&format!("                diesel::insert_into({}::table)\n", table_name));
            code.push_str("                    .values(new_item)\n");
            code.push_str("                    .execute(conn)?;\n\n");

            code.push_str("                if verbose {\n");
            code.push_str("                    eprintln!(\"  ✓ Created new\");\n");
            code.push_str("                }\n");
            code.push_str(&format!("                stats.{}_created += 1;\n", var_name));
            code.push_str("            }\n");
            code.push_str("        }\n\n");
        }
    }

    code.push_str("        Ok(stats)\n");
    code.push_str("    })\n");
    code.push_str("}\n\n");

    code
}

/// Generate SQL helper functions
fn generate_sql_helpers() -> String {
    r#"/// Format any value as SQL literal
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
"#.to_string()
}
