# Axum Dashboard Design Document

## Overview

This document describes the migration of the nomnom real-time dashboard from FastAPI to Axum, and from event-based polling to table-based polling.

## Current Architecture (FastAPI + db_events)

### Components
- **SQL Triggers**: Populate `db_events` table on INSERT to entity tables
- **FastAPI Backend**: Polls `db_events` table, broadcasts via WebSocket
- **React Frontend**: Displays real-time updates via WebSocket connection

### Data Flow
```
INSERT into orders
  ↓ (trigger)
INSERT into db_events
  ↓ (polling)
FastAPI backend detects new event
  ↓ (WebSocket)
React frontend receives update
```

### Issues
1. New Axum ingestion server doesn't populate `db_events` table
2. Python/Rust language split adds complexity
3. Requires SQL triggers to be set up correctly
4. Event table can grow unbounded

## New Architecture (Axum + Table Polling)

### Components
- **Axum Backend**: Directly polls entity tables, broadcasts via WebSocket
- **React Frontend**: Same frontend, connects to Axum backend

### Data Flow
```
INSERT into orders
  ↓ (direct polling)
Axum backend detects new row
  ↓ (WebSocket)
React frontend receives update
```

### Benefits
1. Single language stack (Rust)
2. No SQL triggers required
3. No event table overhead
4. Simpler deployment (single binary)
5. Better performance (native Rust vs Python)

## Technical Design

### Backend Architecture

#### Core Components

```rust
// State management
struct DashboardState {
    // Track last seen ID per entity table
    last_ids: Arc<RwLock<HashMap<String, i64>>>,
    // Connected WebSocket clients
    clients: Arc<RwLock<HashSet<Arc<Mutex<WebSocket>>>>>,
    // Database connection pool
    pool: PgPool,
}

// Entity configuration
struct EntityConfig {
    name: String,
    table: String,
    color: String,
    icon: String,
    fields: Vec<String>,
    max_records: usize,
}
```

#### Polling Strategy

**Per-Entity Polling Task**:
```rust
async fn poll_entity_table(
    table: String,
    entity_name: String,
    state: Arc<DashboardState>,
) {
    loop {
        // Get last seen ID for this table
        let last_id = state.last_ids.read().await
            .get(&table).copied().unwrap_or(0);

        // Query for new records
        let new_records = sqlx::query(&format!(
            "SELECT * FROM {} WHERE id > $1 ORDER BY id ASC LIMIT 100",
            table
        ))
        .bind(last_id)
        .fetch_all(&state.pool)
        .await?;

        if !new_records.is_empty() {
            // Update last seen ID
            let max_id = new_records.iter()
                .filter_map(|r| r.try_get::<i64, _>("id").ok())
                .max()
                .unwrap_or(last_id);

            state.last_ids.write().await.insert(table.clone(), max_id);

            // Broadcast to all clients
            for record in new_records {
                let message = json!({
                    "entity": entity_name,
                    "event_type": "insert",
                    "data": record_to_json(record),
                    "timestamp": Utc::now().to_rfc3339(),
                });

                broadcast_to_clients(&state.clients, message).await;
            }
        }

        tokio::time::sleep(Duration::from_millis(500)).await;
    }
}
```

#### API Endpoints

**REST Endpoints**:
- `GET /api/entities` - Return entity metadata (name, table, fields, colors)
- `GET /api/stats` - Return row counts per entity table
- `GET /api/health` - Health check with connected client count

**WebSocket Endpoint**:
- `WebSocket /ws` - Real-time updates stream
  - Client connects, added to clients set
  - Receives JSON messages with entity updates
  - Auto-removed on disconnect

#### Message Format

WebSocket messages sent to frontend:
```json
{
  "entity": "Order",
  "event_type": "insert",
  "data": {
    "id": 123,
    "order_key": "ORDER-000001",
    "customer_key": "CUST-0774",
    "total_price": 68895.3,
    "order_date": "2024-10-29",
    "order_priority": "1-URGENT"
  },
  "timestamp": "2025-11-09T15:30:45Z"
}
```

### Code Generation

#### File Structure
```
dashboard/
├── Cargo.toml                 # Generated with dependencies
├── src/
│   ├── main.rs               # Axum server entry point
│   ├── config.rs             # Entity configurations
│   ├── polling.rs            # Table polling logic
│   ├── websocket.rs          # WebSocket handlers
│   └── api.rs                # REST API endpoints
├── frontend/                 # React frontend (unchanged)
│   ├── src/
│   │   ├── App.tsx
│   │   └── components/
│   └── package.json
└── .env.example              # DATABASE_URL template
```

#### Generator Modules

**New Module**: `/src/codegen/dashboard/axum_backend.rs`

Functions:
- `generate_backend()` - Main entry point
- `generate_cargo_toml()` - Dependencies (axum, tokio, sqlx, etc.)
- `generate_main_rs()` - Server setup, route registration
- `generate_config_rs()` - Entity metadata from YAML
- `generate_polling_rs()` - Per-table polling tasks
- `generate_websocket_rs()` - WebSocket connection handling
- `generate_api_rs()` - REST endpoints

**Updated Module**: `/src/codegen/dashboard/mod.rs`

Add:
```rust
mod axum_backend;

pub enum BackendType {
    FastAPI,  // Existing
    Axum,     // New
}

pub fn generate_all(
    entities: &[EntityDef],
    output_dir: &Path,
    config_dir: &str,
    db_type: DatabaseType,
    backend_type: BackendType,  // New parameter
) -> Result<(), Box<dyn Error>>
```

### Entity Filtering

**Include ALL persistent entities** (both root and derived):
```rust
for entity in entities {
    if !entity.is_persistent() || entity.is_abstract {
        continue;  // Skip non-persistent and abstract
    }
    if entity.source_type.to_lowercase() == "reference" {
        continue;  // Skip reference data
    }

    // Include both:
    // - Root entities (Order) - can be directly ingested
    // - Derived entities (OrderLineItem) - extracted from root entities
    generate_entity_config(entity);
}
```

This is correct because:
- Root entities (Order) are ingested via API → show in dashboard
- Derived entities (OrderLineItem) are extracted and persisted → show in dashboard
- Both are real data in the database that changes over time

### Dependencies

**Cargo.toml** (generated):
```toml
[package]
name = "dashboard"
version = "0.1.0"
edition = "2021"

[dependencies]
axum = { version = "0.7", features = ["ws"] }
tokio = { version = "1", features = ["full"] }
sqlx = { version = "0.7", features = ["runtime-tokio-rustls", "postgres", "json"] }
serde = { version = "1.0", features = ["derive"] }
serde_json = "1.0"
tower = "0.4"
tower-http = { version = "0.5", features = ["cors"] }
tracing = "0.1"
tracing-subscriber = "0.3"
chrono = { version = "0.4", features = ["serde"] }
dotenv = "0.15"
```

### Frontend Integration

The React frontend remains **unchanged** - it already expects:
- WebSocket at `/ws`
- REST endpoints at `/api/entities`, `/api/stats`, `/api/health`
- Message format with `entity`, `event_type`, `data`, `timestamp`

The Axum backend implements the exact same API contract.

## Implementation Plan

### Phase 1: Core Axum Backend Generator (Priority 1)
1. Create `/src/codegen/dashboard/axum_backend.rs`
2. Implement `generate_backend()` function
3. Generate basic Axum server structure:
   - `main.rs` - Server setup
   - `config.rs` - Entity metadata

**Estimated Code**: ~200 lines in generator, ~150 lines generated

### Phase 2: Polling & WebSocket (Priority 1)
1. Generate polling tasks per entity table
2. Generate WebSocket connection handler
3. Generate broadcast logic to all clients

**Estimated Code**: ~250 lines in generator, ~200 lines generated

### Phase 3: REST API Endpoints (Priority 2)
1. Generate `/api/entities` endpoint
2. Generate `/api/stats` endpoint
3. Generate `/api/health` endpoint

**Estimated Code**: ~100 lines in generator, ~80 lines generated

### Phase 4: Integration & CLI (Priority 2)
1. Update `dashboard::generate_all()` to accept `BackendType`
2. Add CLI flag: `--backend axum` or `--backend fastapi`
3. Update example `generate_dashboard.rs`

**Estimated Code**: ~50 lines changes

### Phase 5: Testing (Priority 1)
1. Generate dashboard for tpch example
2. Start Axum dashboard server
3. Send test data via ingestion server
4. Verify real-time updates in browser
5. Load test with many concurrent clients

### Phase 6: Documentation (Priority 3)
1. Update dashboard generation docs
2. Add Axum backend README
3. Update docker-compose for Axum option
4. Migration guide from FastAPI to Axum

## Comparison: FastAPI vs Axum

| Aspect | FastAPI (Current) | Axum (Proposed) |
|--------|------------------|-----------------|
| Language | Python | Rust |
| Event Source | db_events table | Direct table polling |
| Dependencies | Python runtime, uvicorn | Single binary |
| Setup | Requires SQL triggers | No triggers needed |
| Performance | ~5k req/s | ~50k req/s |
| Memory | ~100MB | ~10MB |
| Deployment | Docker + Python | Single binary |
| Type Safety | Runtime (Pydantic) | Compile time (Rust) |

## Migration Path

For existing FastAPI dashboards:

1. **Keep FastAPI running**: No immediate breaking changes
2. **Generate Axum version**: Run `nomnom generate-dashboard --backend axum`
3. **Test in parallel**: Run both backends, compare behavior
4. **Switch frontend**: Update WebSocket URL when ready
5. **Decommission FastAPI**: Remove Python backend

The frontend is backend-agnostic - same React code works with both.

## Open Questions

1. **Backward Compatibility**: Should we keep FastAPI generator or deprecate it?
   - **Recommendation**: Keep both, make Axum the default for new projects

2. **Database Support**: Should we support MySQL/MariaDB in Axum version?
   - **Recommendation**: Start with PostgreSQL only, add MySQL later

3. **Pagination**: Should polling use pagination for large tables?
   - **Recommendation**: Yes, use `LIMIT 100` per poll, configurable

4. **Filtering**: Should WebSocket clients be able to filter by entity?
   - **Recommendation**: Phase 2 feature, not MVP

5. **Metrics**: Should we expose Prometheus metrics?
   - **Recommendation**: Phase 2 feature, not MVP

## Success Criteria

✅ Axum dashboard generator creates working server
✅ Server polls all persistent entity tables
✅ WebSocket broadcasts updates in real-time
✅ Frontend displays updates (no changes needed)
✅ Performance: handles 1000+ inserts/sec
✅ Performance: supports 100+ concurrent WebSocket clients
✅ Single binary deployment
✅ No SQL triggers required

## Timeline Estimate

- **Phase 1**: Core generator - 2-3 hours
- **Phase 2**: Polling & WebSocket - 3-4 hours
- **Phase 3**: REST endpoints - 1-2 hours
- **Phase 4**: Integration - 1 hour
- **Phase 5**: Testing - 2-3 hours
- **Phase 6**: Documentation - 1-2 hours

**Total**: 10-15 hours of development time

## References

- Axum documentation: https://docs.rs/axum/latest/axum/
- Axum WebSocket example: https://github.com/tokio-rs/axum/blob/main/examples/websockets
- SQLx documentation: https://docs.rs/sqlx/latest/sqlx/
- Current FastAPI generator: `/src/codegen/dashboard/fastapi_backend.rs`
