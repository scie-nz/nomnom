#!/usr/bin/env rust-script
//! Generate real-time dashboard for TPCH example
//!
//! Run with: cargo run --example generate_dashboard

use std::path::Path;

fn main() -> Result<(), Box<dyn std::error::Error>> {
    println!("🎨 Generating real-time dashboard for TPCH example...\n");

    // Load entities from YAML configs
    let entities = nomnom::codegen::load_entities("entities")?;

    println!("📋 Loaded {} entities", entities.len());
    for entity in &entities {
        if entity.is_persistent() {
            println!("  ✓ {} (persistent)", entity.name);
        }
    }
    println!();

    // Generate dashboard with PostgreSQL backend
    nomnom::codegen::dashboard::generate_all(
        &entities,
        Path::new("dashboard"),
        "entities",
        nomnom::codegen::dashboard::DatabaseType::PostgreSQL,
    )?;

    println!("\n✨ Dashboard generated successfully!");
    println!("📁 Output directory: dashboard/");
    println!("\n📖 Next steps:");
    println!("  1. Review generated SQL: cat dashboard/migrations/001_create_events_table.sql");
    println!("  2. Run migrations: cd dashboard/migrations && ./run.sh");
    println!("  3. Install frontend deps: cd dashboard/frontend && npm install");
    println!("  4. Start services: docker compose -f docker-compose.yml -f dashboard/docker-compose.dashboard.yml up");

    Ok(())
}
