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

use crate::codegen::types::{EntityDef, DatabaseConfig};
use crate::codegen::utils::to_snake_case;
use crate::codegen::ProjectBuildConfig;
use crate::codegen::lineage::{generate_lineage_code, generate_entity_to_fields_helper};
use std::collections::{HashMap, HashSet, VecDeque};
use std::fs;
use std::path::Path;
use regex::Regex;

// Transform registry is now provided by hl7utils
// No need to discover or generate transforms here

/// Generate the complete parser binary source code
pub fn generate_parser_binary(
    config: &ProjectBuildConfig,
    entities: &[EntityDef],
) -> Result<String, String> {
    // 1. Find root entity (NO HARDCODING!)
    let root_entity = entities.iter()
        .find(|e| e.is_root())
        .ok_or("No root entity found in entity configurations. Expected entity with type: root")?;

    // 2. Compute extraction order (topological sort)
    let extraction_order = compute_extraction_order(entities, root_entity)?;

    // 3. Identify permanent entities (have persistence config with database, not abstract)
    let permanent_entities: Vec<&EntityDef> = entities.iter()
        .filter(|e| e.is_persistent() && !e.is_abstract)
        .collect();

    // 4. Generate code sections
    let mut code = String::new();
    code.push_str(&generate_header());
    code.push_str(&generate_imports());
    code.push_str(&generate_lineage_code());
    code.push_str(&generate_entity_to_fields_helper());
    code.push_str(&generate_cli_struct());
    code.push_str(&generate_parse_results_struct(&extraction_order));
    code.push_str(&generate_main_function(root_entity));
    code.push_str(&generate_extraction_function(root_entity, &extraction_order));
    code.push_str(&generate_json_output_function(&extraction_order));
    code.push_str(&generate_sql_output_function(&permanent_entities));
    code.push_str(&generate_sql_helpers());

    Ok(code)
}

/// Compute topological ordering of entities for extraction
fn compute_extraction_order(
    entities: &[EntityDef],
    root_entity: &EntityDef,
) -> Result<Vec<EntityDef>, String> {
    // Build entity name -> entity map
    let entity_map: HashMap<String, EntityDef> = entities.iter()
        .map(|e| (e.name.clone(), e.clone()))
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

    // Check for cycles
    if result.len() != entities.len() {
        return Err(format!(
            "Circular dependency detected in entity graph. Extracted {} of {} entities",
            result.len(),
            entities.len()
        ));
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
fn generate_imports() -> String {
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
    code.push_str("// Note: Transforms are now injected directly into generated.rs\n");
    code.push_str("// No registry needed - entity constructors call transform functions directly\n");
    code.push_str("\n");

    code
}

/// Generate CLI argument parser
fn generate_cli_struct() -> String {
    r#"/// CLI arguments for parser binary
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
}

"#.to_string()
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

        let field_type = if let Some(repetition) = &entity.repetition {
            if repetition == "repeated" {
                format!("Vec<{}>", type_name)
            } else {
                type_name
            }
        } else {
            type_name
        };

        code.push_str(&format!("    {}: {},\n", field_name, field_type));
    }

    code.push_str("}\n\n");
    code
}

/// Generate main function
fn generate_main_function(root_entity: &EntityDef) -> String {
    let root_snake = to_snake_case(&root_entity.name);
    let root_core = format!("{}Core", root_entity.name);

    format!(r#"fn main() -> Result<(), Box<dyn Error>> {{
    let cli = Cli::parse();

    // Determine output mode
    let show_json = !cli.sql_only;
    let show_sql = !cli.json_only;

    // If no flags specified, --dry-run is default (show both)
    let show_json = if cli.dry_run {{ true }} else {{ show_json }};
    let show_sql = if cli.dry_run {{ true }} else {{ show_sql }};

    // Parse lineage format
    let lineage_format = if cli.lineage_format == "detailed" {{
        LineageFormat::Detailed
    }} else {{
        LineageFormat::Compact
    }};

    // Read file paths from stdin (one per line)
    let stdin = io::stdin();
    for line in stdin.lock().lines() {{
        let file_path = line?;

        // Process file
        match process_file(&file_path, show_json, show_sql, cli.lineage, cli.show_lineage, lineage_format) {{
            Ok(_) => {{}},
            Err(e) => {{
                eprintln!("Error processing file '{{}}': {{}}", file_path, e);
                // Continue to next file
            }}
        }}
    }}

    Ok(())
}}

/// Process a single file path
fn process_file(
    file_path: &str,
    show_json: bool,
    show_sql: bool,
    enable_lineage: bool,
    show_lineage: bool,
    lineage_format: LineageFormat,
) -> Result<(), Box<dyn Error>> {{
    // Create root entity from file path (no registry - transforms are injected)
    let {root_snake} = {root_core}::from_string(file_path)?;

    // Initialize lineage tracker if needed
    let mut lineage_tracker = if enable_lineage || show_lineage {{
        Some(LineageTracker::new())
    }} else {{
        None
    }};

    // Extract all entities (pass ownership of root)
    let (results, entity_shas) = extract_all_entities({root_snake}, lineage_tracker.as_mut())?;

    // Show lineage tree if requested
    if show_lineage {{
        if let Some(ref tracker) = lineage_tracker {{
            let tree = tracker.render_tree(lineage_format);
            eprintln!("{{}}\\n", tree);
        }}
        // When showing lineage, suppress JSON/SQL output
        return Ok(());
    }}

    // Output JSON if requested
    if show_json {{
        output_json_entities(&results, lineage_tracker.as_ref(), &entity_shas, enable_lineage)?;
    }}

    // Output SQL if requested
    if show_sql {{
        output_sql_statements(&results)?;
    }}

    Ok(())
}}

"#)
}

/// Generate entity extraction function
fn generate_extraction_function(
    root_entity: &EntityDef,
    extraction_order: &[EntityDef],
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
    let root_is_permanent = root_entity.is_persistent();
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
        let is_repeated = entity.repetition.as_ref().map(|r| r == "repeated").unwrap_or(false);
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
            let is_permanent = entity.is_persistent();
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
                let is_permanent = entity.is_persistent();
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

            let is_permanent = entity.is_persistent();
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
        let is_repeated = entity.repetition.as_ref().map(|r| r == "repeated").unwrap_or(false);

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
fn generate_sql_output_function(permanent_entities: &[&EntityDef]) -> String {
    let mut code = String::new();

    code.push_str("/// Output SQL statements for permanent entities\n");
    code.push_str("fn output_sql_statements(results: &ParseResults) -> Result<(), Box<dyn Error>> {\n");
    code.push_str("    println!(\"-- ========================================\");\n");
    code.push_str("    println!(\"-- SQL Statements (Dry-Run Mode)\");\n");
    code.push_str("    println!(\"-- ========================================\");\n");
    code.push_str("    println!();\n");

    for entity in permanent_entities {
        let function_name = format!("output_{}_sql", to_snake_case(&entity.name));
        let var_name = to_snake_case(&entity.name);
        let is_repeated = entity.repetition.as_ref().map(|r| r == "repeated").unwrap_or(false);

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
        code.push_str(&generate_entity_sql_function(entity));
    }

    code
}

/// Generate SQL output function for a single entity
fn generate_entity_sql_function(entity: &EntityDef) -> String {
    let db_config = entity.get_database_config()
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

    for (i, field) in unicity_fields.iter().enumerate() {
        let separator = if i == 0 { "      " } else { "  AND " };
        code.push_str(&format!(
            "    println!(\"{}{} {{}}\", sql_cmp_opt(&entity.{}));\n",
            separator, field, field
        ));
    }

    code.push_str("    println!(\"LIMIT 1;\");\n");
    code.push_str("    println!();\n");

    // INSERT query
    code.push_str("    println!(\"-- If not found:\");\n");

    // Collect all field names (excluding auto-generated PK if configured)
    let mut insert_fields: Vec<String> = Vec::new();
    for field in &entity.fields {
        // Skip auto-generated conformant ID
        if db_config.autogenerate_conformant_id
            && field.name == db_config.conformant_id_column {
            continue;
        }
        insert_fields.push(field.name.clone());
    }

    code.push_str(&format!(
        "    println!(\"INSERT INTO {} ({})\");\n",
        table_name,
        insert_fields.join(", ")
    ));
    code.push_str("    println!(\"VALUES\");\n");
    code.push_str("    print!(\"  (\");\n");

    for (i, field) in insert_fields.iter().enumerate() {
        if i > 0 {
            code.push_str("    print!(\", \");\n");
        }
        code.push_str(&format!("    print!(\"{{}}\", sql_opt(&entity.{}));\n", field));
    }

    code.push_str("    println!(\")\");\n");
    code.push_str("    println!();\n");

    code.push_str("    Ok(())\n");
    code.push_str("}\n\n");

    code
}

/// Generate SQL helper functions
fn generate_sql_helpers() -> String {
    r#"/// Format Option<String> as SQL value ('value' or NULL)
fn sql_opt(opt: &Option<String>) -> String {
    match opt {
        Some(s) => format!("'{}'", sql_escape(s)),
        None => "NULL".to_string(),
    }
}

/// Format Option<String> as SQL comparison (field = 'value' OR field IS NULL)
fn sql_cmp_opt(opt: &Option<String>) -> String {
    match opt {
        Some(s) => format!("= '{}'", sql_escape(s)),
        None => "IS NULL".to_string(),
    }
}

/// Escape string for SQL (single quotes)
fn sql_escape(s: &str) -> String {
    s.replace('\'', "''")
}
"#.to_string()
}
