/// FastAPI backend generation for real-time dashboard.

use super::utils::{DatabaseType, generate_entity_display_config, DashboardConfig};
use crate::codegen::EntityDef;
use std::path::Path;
use std::error::Error;

/// Generate FastAPI backend code
pub fn generate_backend(
    entities: &[EntityDef],
    output_dir: &Path,
    _config_dir: &str,
    db_type: DatabaseType,
) -> Result<(), Box<dyn Error>> {
    let config = DashboardConfig::default();

    // Generate main.py
    generate_main_py(entities, output_dir, db_type, &config)?;

    // Generate requirements.txt
    generate_requirements_txt(output_dir, db_type)?;

    // Generate config.py (entity configurations)
    generate_config_py(entities, output_dir)?;

    Ok(())
}

/// Generate main.py with FastAPI app, polling, and WebSocket
fn generate_main_py(
    entities: &[EntityDef],
    output_dir: &Path,
    db_type: DatabaseType,
    config: &DashboardConfig,
) -> Result<(), Box<dyn Error>> {
    let main_file = output_dir.join("main.py");
    let mut output = std::fs::File::create(&main_file)?;

    use std::io::Write;

    writeln!(output, "# Auto-generated FastAPI backend for real-time dashboard")?;
    writeln!(output, "# Database type: {}\n", db_type.as_str())?;

    // Imports
    writeln!(output, "import asyncio")?;
    writeln!(output, "import os")?;
    writeln!(output, "from dotenv import load_dotenv")?;
    writeln!(output, "from fastapi import FastAPI, WebSocket")?;
    writeln!(output, "from fastapi.middleware.cors import CORSMiddleware")?;
    writeln!(output, "from databases import Database")?;
    writeln!(output, "from config import ENTITIES\n")?;
    writeln!(output, "# Load environment variables from .env file")?;
    writeln!(output, "load_dotenv()\n")?;

    // FastAPI app setup
    writeln!(output, "app = FastAPI(title=\"Real-Time Dashboard API\")\n")?;

    // CORS middleware
    writeln!(output, "# CORS middleware for frontend")?;
    writeln!(output, "app.add_middleware(")?;
    writeln!(output, "    CORSMiddleware,")?;
    writeln!(output, "    allow_origins=[\"*\"],  # In production, specify frontend URL")?;
    writeln!(output, "    allow_credentials=True,")?;
    writeln!(output, "    allow_methods=[\"*\"],")?;
    writeln!(output, "    allow_headers=[\"*\"],")?;
    writeln!(output, ")\n")?;

    // Database connection
    writeln!(output, "# Database connection")?;
    writeln!(output, "DATABASE_URL = os.getenv(\"DATABASE_URL\")")?;
    writeln!(output, "if not DATABASE_URL:")?;
    writeln!(output, "    raise ValueError(")?;
    writeln!(output, "        \"DATABASE_URL environment variable not set. \"")?;
    writeln!(output, "        \"Please create a .env file with DATABASE_URL=postgresql://user:pass@host:port/db\"")?;
    writeln!(output, "    )")?;
    writeln!(output, "database = Database(DATABASE_URL)\n")?;

    // Global state
    writeln!(output, "# Global state")?;
    writeln!(output, "last_event_id = 0")?;
    writeln!(output, "connected_clients = set()\n")?;

    // Configuration
    writeln!(output, "# Configuration")?;
    writeln!(output, "POLL_INTERVAL = {}  # seconds", config.polling_interval_ms as f32 / 1000.0)?;
    writeln!(output, "MAX_EVENTS_PER_POLL = {}\n", config.max_events_per_poll)?;

    // Polling background task
    generate_polling_task(&mut output)?;

    // WebSocket endpoint
    generate_websocket_endpoint(&mut output)?;

    // API endpoints
    generate_api_endpoints(&mut output)?;

    // Startup/shutdown
    writeln!(output, "\n@app.on_event(\"startup\")")?;
    writeln!(output, "async def startup():")?;
    writeln!(output, "    await database.connect()")?;
    writeln!(output, "    asyncio.create_task(poll_events())\n")?;

    writeln!(output, "@app.on_event(\"shutdown\")")?;
    writeln!(output, "async def shutdown():")?;
    writeln!(output, "    await database.disconnect()")?;

    println!("cargo:rerun-if-changed={}", main_file.display());
    Ok(())
}

/// Generate polling background task
fn generate_polling_task(output: &mut std::fs::File) -> Result<(), Box<dyn Error>> {
    use std::io::Write;

    writeln!(output, "async def poll_events():")?;
    writeln!(output, "    \"\"\"Background task that polls db_events table\"\"\"")?;
    writeln!(output, "    global last_event_id\n")?;

    writeln!(output, "    while True:")?;
    writeln!(output, "        try:")?;
    writeln!(output, "            # Query for new events since last poll")?;
    writeln!(output, "            query = \"\"\"")?;
    writeln!(output, "                SELECT id, entity, event_type, payload, created_at")?;
    writeln!(output, "                FROM db_events")?;
    writeln!(output, "                WHERE id > :last_id")?;
    writeln!(output, "                ORDER BY id ASC")?;
    writeln!(output, "                LIMIT :limit")?;
    writeln!(output, "            \"\"\"")?;
    writeln!(output)?;
    writeln!(output, "            new_events = await database.fetch_all(")?;
    writeln!(output, "                query,")?;
    writeln!(output, "                {{\"last_id\": last_event_id, \"limit\": MAX_EVENTS_PER_POLL}}")?;
    writeln!(output, "            )\n")?;

    writeln!(output, "            # Broadcast new events to all connected WebSocket clients")?;
    writeln!(output, "            if new_events:")?;
    writeln!(output, "                for event in new_events:")?;
    writeln!(output, "                    message = {{")?;
    writeln!(output, "                        \"entity\": event[\"entity\"],")?;
    writeln!(output, "                        \"event_type\": event[\"event_type\"],")?;
    writeln!(output, "                        \"data\": event[\"payload\"],")?;
    writeln!(output, "                        \"timestamp\": event[\"created_at\"].isoformat(),")?;
    writeln!(output, "                    }}\n")?;

    writeln!(output, "                    # Broadcast to all connected clients")?;
    writeln!(output, "                    disconnected = set()")?;
    writeln!(output, "                    for client in connected_clients:")?;
    writeln!(output, "                        try:")?;
    writeln!(output, "                            await client.send_json(message)")?;
    writeln!(output, "                        except Exception:")?;
    writeln!(output, "                            disconnected.add(client)\n")?;

    writeln!(output, "                    # Remove disconnected clients")?;
    writeln!(output, "                    connected_clients.difference_update(disconnected)\n")?;

    writeln!(output, "                # Update last_event_id to the highest ID we've seen")?;
    writeln!(output, "                last_event_id = new_events[-1][\"id\"]\n")?;

    writeln!(output, "            await asyncio.sleep(POLL_INTERVAL)\n")?;

    writeln!(output, "        except Exception as e:")?;
    writeln!(output, "            print(f\"Polling error: {{e}}\")")?;
    writeln!(output, "            await asyncio.sleep(POLL_INTERVAL)\n")?;

    Ok(())
}

/// Generate WebSocket endpoint
fn generate_websocket_endpoint(output: &mut std::fs::File) -> Result<(), Box<dyn Error>> {
    use std::io::Write;

    writeln!(output, "\n@app.websocket(\"/ws\")")?;
    writeln!(output, "async def websocket_endpoint(websocket: WebSocket):")?;
    writeln!(output, "    await websocket.accept()")?;
    writeln!(output, "    connected_clients.add(websocket)\n")?;

    writeln!(output, "    try:")?;
    writeln!(output, "        # Keep connection alive and handle client messages")?;
    writeln!(output, "        while True:")?;
    writeln!(output, "            # Wait for client messages (ping/pong, filters, etc.)")?;
    writeln!(output, "            data = await websocket.receive_text()")?;
    writeln!(output, "            # Handle client commands if needed")?;
    writeln!(output, "            # For now, just echo back")?;
    writeln!(output, "            # await websocket.send_text(f\"Echo: {{data}}\")")?;
    writeln!(output, "    except Exception as e:")?;
    writeln!(output, "        print(f\"WebSocket error: {{e}}\")")?;
    writeln!(output, "    finally:")?;
    writeln!(output, "        connected_clients.discard(websocket)\n")?;

    Ok(())
}

/// Generate API endpoints
fn generate_api_endpoints(output: &mut std::fs::File) -> Result<(), Box<dyn Error>> {
    use std::io::Write;

    writeln!(output, "\n@app.get(\"/api/stats\")")?;
    writeln!(output, "async def get_stats():")?;
    writeln!(output, "    \"\"\"Get statistics for all entities\"\"\"")?;
    writeln!(output, "    stats = {{}}")?;
    writeln!(output, "    for entity in ENTITIES:")?;
    writeln!(output, "        count = await database.fetch_val(")?;
    writeln!(output, "            f\"SELECT COUNT(*) FROM {{entity['table']}}\"")?;
    writeln!(output, "        )")?;
    writeln!(output, "        stats[entity['name']] = {{\"total\": count}}")?;
    writeln!(output, "    return stats\n")?;

    writeln!(output, "@app.get(\"/api/entities\")")?;
    writeln!(output, "async def get_entities():")?;
    writeln!(output, "    \"\"\"Return entity metadata for frontend\"\"\"")?;
    writeln!(output, "    return ENTITIES\n")?;

    writeln!(output, "@app.get(\"/api/health\")")?;
    writeln!(output, "async def health_check():")?;
    writeln!(output, "    return {{\"status\": \"ok\", \"connected_clients\": len(connected_clients)}}")?;

    Ok(())
}

/// Generate config.py with entity configurations
fn generate_config_py(entities: &[EntityDef], output_dir: &Path) -> Result<(), Box<dyn Error>> {
    let config_file = output_dir.join("config.py");
    let mut output = std::fs::File::create(&config_file)?;

    use std::io::Write;

    writeln!(output, "# Auto-generated entity configuration")?;
    writeln!(output, "# This file is regenerated when entities change\n")?;

    writeln!(output, "ENTITIES = [")?;

    for entity in entities {
        if !entity.is_persistent(entities) || entity.is_abstract {
            continue;
        }

        if entity.source_type.to_lowercase() == "reference" {
            continue;
        }

        let display_config = generate_entity_display_config(entity, entities);

        writeln!(output, "    {{")?;
        writeln!(output, "        \"name\": \"{}\",", display_config.name)?;
        writeln!(output, "        \"table\": \"{}\",", display_config.table)?;
        writeln!(output, "        \"color\": \"{}\",", display_config.color)?;
        writeln!(output, "        \"icon\": \"{}\",", display_config.icon)?;
        write!(output, "        \"fields\": [")?;
        for (i, field) in display_config.display_fields.iter().enumerate() {
            if i > 0 {
                write!(output, ", ")?;
            }
            write!(output, "\"{}\"", field)?;
        }
        writeln!(output, "],")?;
        writeln!(output, "        \"max_records\": {},", display_config.max_records)?;
        writeln!(output, "    }},")?;
    }

    writeln!(output, "]")?;

    println!("cargo:rerun-if-changed={}", config_file.display());
    Ok(())
}

/// Generate requirements.txt
fn generate_requirements_txt(output_dir: &Path, db_type: DatabaseType) -> Result<(), Box<dyn Error>> {
    let req_file = output_dir.join("requirements.txt");
    let mut output = std::fs::File::create(&req_file)?;

    use std::io::Write;

    writeln!(output, "# Auto-generated requirements for FastAPI backend")?;
    writeln!(output, "fastapi==0.109.0")?;
    writeln!(output, "uvicorn[standard]==0.27.0")?;

    match db_type {
        DatabaseType::PostgreSQL => {
            writeln!(output, "databases[postgresql]==0.9.0")?;
        }
        DatabaseType::MySQL | DatabaseType::MariaDB => {
            writeln!(output, "databases[mysql]==0.9.0")?;
        }
    }

    writeln!(output, "python-dotenv==1.0.0")?;

    Ok(())
}
