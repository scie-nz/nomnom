# Real-Time Dashboard Implementation Plan

## Overview

Build a code-generated real-time dashboard that visualizes streaming data and database inserts as they happen. The dashboard will automatically adapt to entity configurations defined in YAML files.

**Key Features:**
- 🌐 **Database Agnostic:** Works with PostgreSQL, MySQL, and MariaDB
- ⚡ **Near Real-Time:** 500ms polling (configurable 100ms-5s)
- 🎨 **Auto-Generated:** All code generated from entity YAML configs
- 🔧 **Zero Config:** Works out-of-the-box with smart defaults
- 🐳 **Docker Native:** Runs as docker-compose services
- 📊 **Rich Visualizations:** Live tables, statistics, filters, export

**Technical Approach:**
Uses a polling-based architecture with a `db_events` table populated by database triggers. FastAPI backend polls for new events and broadcasts via WebSocket to React frontend. Simple, reliable, and works across all major SQL databases.

## Architecture

```
┌─────────────────────────────────────────────────────────────────┐
│                         Browser (React)                         │
│  - Live table of recent inserts                                 │
│  - Entity statistics (created/found counts)                     │
│  - Auto-scroll, filtering, color-coding                         │
└────────────────────┬────────────────────────────────────────────┘
                     │ WebSocket
┌────────────────────▼────────────────────────────────────────────┐
│                    FastAPI Backend                              │
│  - WebSocket server broadcasting to all clients                 │
│  - Background task polling db_events table (100-500ms)          │
│  - When new events found, broadcast via WebSocket               │
│  - Health checks, stats endpoints                               │
└────────────────────┬────────────────────────────────────────────┘
                     │ Polling SQL query every 100-500ms
┌────────────────────▼────────────────────────────────────────────┐
│                Database (PostgreSQL / MySQL / MariaDB)          │
│                                                                 │
│  Entity Tables          db_events (changelog)                   │
│  ┌─────────────┐        ┌──────────────────────┐              │
│  │   orders    │──────▶ │ id (auto-increment)  │              │
│  │order_line_  │        │ entity (varchar)     │              │
│  │  items      │──────▶ │ event_type (varchar) │              │
│  │  customers  │        │ payload (JSON)       │              │
│  │  products   │──────▶ │ created_at (timestamp)│             │
│  └─────────────┘        └──────────────────────┘              │
│       ▲                                                          │
│       │ Triggers on INSERT populate db_events                   │
└─────────────────────────────────────────────────────────────────┘
```

## Key Design Principles

1. **Code Generation First**: Dashboard components auto-generated from entity YAML configs
2. **Entity-Aware**: Dashboard knows about all persistent entities dynamically
3. **Database Agnostic**: Works with PostgreSQL, MySQL, and MariaDB
4. **Polling-Based**: Simple, reliable approach using events table (100-500ms polling)
5. **Zero Config**: Works out-of-the-box after `./build.sh`
6. **Docker Native**: Runs as a service in docker-compose
7. **Development Friendly**: Hot-reload for both backend and frontend

## Components to Code-Generate

### 1. Database Migration (SQL) - Events Table + Triggers
**Generated File:** `dashboard/migrations/001_create_events_table.sql`

**What to Generate:**

#### Events Table (database-agnostic)
```sql
-- Auto-generated events/changelog table
CREATE TABLE IF NOT EXISTS db_events (
    id BIGINT AUTO_INCREMENT PRIMARY KEY,  -- MySQL/MariaDB
    -- id BIGSERIAL PRIMARY KEY,            -- PostgreSQL variant
    entity VARCHAR(100) NOT NULL,
    event_type VARCHAR(20) NOT NULL,
    payload JSON NOT NULL,
    created_at TIMESTAMP DEFAULT CURRENT_TIMESTAMP,
    INDEX idx_created_at (created_at),
    INDEX idx_entity (entity)
);
```

#### Triggers for Each Entity

**PostgreSQL Example:**
```sql
-- Auto-generated trigger for Order
CREATE OR REPLACE FUNCTION log_order_insert()
RETURNS trigger AS $$
BEGIN
  INSERT INTO db_events (entity, event_type, payload)
  VALUES ('Order', 'insert', row_to_json(NEW)::json);
  RETURN NEW;
END;
$$ LANGUAGE plpgsql;

CREATE TRIGGER order_insert_event
AFTER INSERT ON orders
FOR EACH ROW EXECUTE FUNCTION log_order_insert();
```

**MySQL/MariaDB Example:**
```sql
-- Auto-generated trigger for Order
DELIMITER $$
CREATE TRIGGER order_insert_event
AFTER INSERT ON orders
FOR EACH ROW
BEGIN
  INSERT INTO db_events (entity, event_type, payload)
  VALUES (
    'Order',
    'insert',
    JSON_OBJECT(
      'order_key', NEW.order_key,
      'customer_key', NEW.customer_key,
      'total_price', NEW.total_price,
      'order_date', NEW.order_date
      -- ... all fields from field_overrides
    )
  );
END$$
DELIMITER ;
```

**Generation Logic:**
- Detect database type from config (PostgreSQL vs MySQL/MariaDB)
- Generate appropriate SQL dialect for events table
- Loop through all entities with `persistence.database` config
- Skip abstract entities and reference entities (optional)
- Generate one trigger per persistent entity
- For MySQL: Build JSON_OBJECT with all fields from field_overrides
- For PostgreSQL: Use row_to_json(NEW) for simplicity
- Use snake_case table names from `conformant_table`

### 2. FastAPI Backend (`dashboard/backend/main.py`)

**What to Generate:**

#### Entity Configuration
```python
# Auto-generated from YAML configs
import hashlib

def entity_color(name: str) -> str:
    """Generate consistent color from entity name hash"""
    colors = [
        "#3b82f6", "#10b981", "#f59e0b", "#ef4444",
        "#8b5cf6", "#ec4899", "#14b8a6", "#f97316"
    ]
    hash_val = int(hashlib.md5(name.encode()).hexdigest(), 16)
    return colors[hash_val % len(colors)]

ENTITIES = [
    {
        "name": "Order",
        "table": "orders",
        "color": entity_color("Order"),  # Consistent hash-based
        "icon": "📦",
        "fields": ["order_key", "customer_key", "total_price", "order_date"],
    },
    {
        "name": "OrderLineItem",
        "table": "order_line_items",
        "color": entity_color("OrderLineItem"),
        "icon": "📄",
        "fields": ["order_key", "line_number", "part_key", "quantity"],
    },
    # ... more entities
]

# Configuration
POLL_INTERVAL = 0.5  # 500ms polling interval
MAX_EVENTS_PER_POLL = 100
```

#### Database Polling Background Task
```python
# Auto-generated polling task
import asyncio
from collections import defaultdict
from databases import Database

database = Database(DATABASE_URL)
last_event_id = 0
connected_clients = set()

async def poll_events():
    """Background task that polls db_events table"""
    global last_event_id

    while True:
        try:
            # Query for new events since last poll
            query = """
                SELECT id, entity, event_type, payload, created_at
                FROM db_events
                WHERE id > :last_id
                ORDER BY id ASC
                LIMIT :limit
            """

            new_events = await database.fetch_all(
                query,
                {"last_id": last_event_id, "limit": MAX_EVENTS_PER_POLL}
            )

            # Broadcast new events to all connected WebSocket clients
            if new_events:
                for event in new_events:
                    message = {
                        "entity": event["entity"],
                        "event_type": event["event_type"],
                        "data": event["payload"],
                        "timestamp": event["created_at"].isoformat(),
                    }

                    # Broadcast to all connected clients
                    disconnected = set()
                    for client in connected_clients:
                        try:
                            await client.send_json(message)
                        except Exception:
                            disconnected.add(client)

                    # Remove disconnected clients
                    connected_clients.difference_update(disconnected)

                # Update last_event_id to the highest ID we've seen
                last_event_id = new_events[-1]["id"]

            await asyncio.sleep(POLL_INTERVAL)

        except Exception as e:
            print(f"Polling error: {e}")
            await asyncio.sleep(POLL_INTERVAL)

@app.on_event("startup")
async def startup():
    await database.connect()
    asyncio.create_task(poll_events())

@app.on_event("shutdown")
async def shutdown():
    await database.disconnect()
```

#### WebSocket Handler
```python
# Auto-generated WebSocket endpoint
from fastapi import WebSocket

@app.websocket("/ws")
async def websocket_endpoint(websocket: WebSocket):
    await websocket.accept()
    connected_clients.add(websocket)

    try:
        # Keep connection alive and handle client messages
        while True:
            # Wait for client messages (ping/pong, filters, etc.)
            data = await websocket.receive_text()
            # Handle client commands if needed
    except Exception as e:
        print(f"WebSocket error: {e}")
    finally:
        connected_clients.discard(websocket)
```

#### Stats Endpoint
```python
# Auto-generated stats endpoint
@app.get("/api/stats")
async def get_stats():
    stats = {}
    for entity in ENTITIES:
        count = await database.fetch_val(
            f"SELECT COUNT(*) FROM {entity['table']}"
        )
        stats[entity['name']] = {"total": count}
    return stats

@app.get("/api/entities")
async def get_entities():
    """Return entity metadata for frontend"""
    return ENTITIES
```

**Generation Logic:**
- Read entity YAML configs
- Extract persistent entities
- Generate ENTITIES config array with hash-based colors
- Auto-select first 4-5 fields from field_overrides for each entity
- Generate polling background task with configurable interval
- Generate WebSocket connection manager
- Generate stats queries for all tables

### 3. React Frontend Components

**Generated File:** `dashboard/frontend/src/generated/entities.ts`

**TypeScript Entity Config:**
```typescript
// Auto-generated entity configuration
export interface Entity {
  name: string;
  table: string;
  channel: string;
  color: string;
  icon: string;
  fields: Field[];
}

export interface Field {
  name: string;
  type: "string" | "integer" | "float" | "datetime";
  display: boolean;  // Show in table?
}

export const ENTITIES: Entity[] = [
  {
    name: "Order",
    table: "orders",
    channel: "order_insert",
    color: "#3b82f6",
    icon: "📦",
    fields: [
      { name: "order_key", type: "string", display: true },
      { name: "customer_key", type: "string", display: true },
      { name: "total_price", type: "float", display: true },
      { name: "order_date", type: "string", display: true },
      // ... more fields from field_overrides
    ]
  },
  // ... more entities
];
```

**Generated File:** `dashboard/frontend/src/components/EntityTable.tsx`

**Dynamic Table Component:**
```tsx
// Auto-generated based on entity fields
export function EntityTable({ entity, records }) {
  return (
    <table>
      <thead>
        <tr>
          <th>Timestamp</th>
          {entity.fields.filter(f => f.display).map(field => (
            <th key={field.name}>{field.name}</th>
          ))}
        </tr>
      </thead>
      <tbody>
        {records.map(record => (
          <tr>
            <td>{record.timestamp}</td>
            {entity.fields.filter(f => f.display).map(field => (
              <td key={field.name}>
                {formatField(record[field.name], field.type)}
              </td>
            ))}
          </tr>
        ))}
      </tbody>
    </table>
  );
}
```

**Generation Logic:**
- Read entity YAML configs
- Extract field_overrides for each entity
- Map YAML field types to TypeScript types
- Generate Entity interface and ENTITIES array
- Select key fields to display (first 4-5 fields, configurable)

### 4. Docker Configuration

**Generated File:** `dashboard/Dockerfile.backend`
```dockerfile
FROM python:3.11-slim

WORKDIR /app

COPY backend/requirements.txt .
RUN pip install --no-cache-dir -r requirements.txt

COPY backend/ .

CMD ["uvicorn", "main:app", "--host", "0.0.0.0", "--port", "8000", "--reload"]
```

**Generated File:** `dashboard/Dockerfile.frontend`
```dockerfile
FROM node:20-alpine

WORKDIR /app

COPY frontend/package.json frontend/package-lock.json ./
RUN npm install

COPY frontend/ .

CMD ["npm", "run", "dev", "--", "--host", "0.0.0.0"]
```

**Update:** `docker-compose.yml`
```yaml
services:
  # ... existing postgres, etc.

  dashboard-backend:
    build:
      context: ./dashboard
      dockerfile: Dockerfile.backend
    ports:
      - "8000:8000"
    environment:
      - DATABASE_URL=${DATABASE_URL}
    depends_on:
      - postgres
    volumes:
      - ./dashboard/backend:/app  # Hot reload

  dashboard-frontend:
    build:
      context: ./dashboard
      dockerfile: Dockerfile.frontend
    ports:
      - "5173:5173"
    depends_on:
      - dashboard-backend
    volumes:
      - ./dashboard/frontend:/app  # Hot reload
      - /app/node_modules  # Prevent overwrite
```

## Code Generation Integration

### New Nomnom Module: `src/codegen/dashboard/`

Create new code generation module with these files:

```
src/codegen/dashboard/
├── mod.rs              # Main entry point
├── sql_triggers.rs     # Generate PostgreSQL triggers
├── fastapi_backend.rs  # Generate FastAPI main.py
├── react_frontend.rs   # Generate React components
└── docker.rs           # Generate Dockerfiles
```

### Integration Point: `build.rs` or New CLI Command

**Option 1: Add to existing build.rs**
```rust
// After Diesel generation
println!("Generating dashboard...");
dashboard::generate_all(
    &entities,
    &config_dir,
    &output_dir.join("dashboard"),
)?;
```

**Option 2: New CLI command (preferred)**
```bash
# User runs after initial setup
nomnom generate-dashboard --config entities/ --output dashboard/
```

This keeps dashboard generation separate and optional.

### Update `src/codegen/mod.rs`
```rust
pub mod diesel;
pub mod parser_binary;
pub mod dashboard;  // NEW

// Re-export
pub use dashboard::generate_all as generate_dashboard;
```

## Implementation Steps

### Phase 1: Code Generation Foundation (Core Infrastructure)

**Tasks:**
1. Create `src/codegen/dashboard/mod.rs` module structure
2. Add entity YAML parsing helper (reuse existing parsing logic)
3. Implement color/icon assignment logic (cycling through predefined sets)
4. Create template rendering utilities (or use simple string formatting)

**Deliverable:** Foundation for generating dashboard code

---

### Phase 2: PostgreSQL Triggers Generation

**Tasks:**
1. Implement `src/codegen/dashboard/sql_triggers.rs`
2. Generate `CREATE FUNCTION notify_X_insert()` for each entity
3. Generate `CREATE TRIGGER X_insert_trigger` for each entity
4. Handle table name mapping (conformant_table from YAML)
5. Generate migration runner script

**Output Files:**
- `dashboard/migrations/001_create_triggers.sql`
- `dashboard/migrations/run.sh` (executes migrations)

**Test:**
```bash
./dashboard/migrations/run.sh
# Verify triggers exist in PostgreSQL
docker compose exec postgres psql -U tpch_user -d tpch_db -c "\df notify_*"
```

---

### Phase 3: FastAPI Backend Generation

**Tasks:**
1. Implement `src/codegen/dashboard/fastapi_backend.rs`
2. Generate ENTITIES config from YAML
3. Generate WebSocket endpoint with LISTEN setup
4. Generate `/api/stats` endpoint
5. Generate `/api/entities` metadata endpoint
6. Generate `requirements.txt` with dependencies
7. Generate `main.py` entry point

**Output Files:**
- `dashboard/backend/main.py`
- `dashboard/backend/requirements.txt`
- `dashboard/backend/config.py` (generated entity config)

**Dependencies (requirements.txt):**
```
fastapi==0.109.0
uvicorn[standard]==0.27.0
databases[postgresql]==0.9.0  # For PostgreSQL
databases[mysql]==0.9.0        # For MySQL/MariaDB
python-dotenv==1.0.0
```

**Note:** The `databases` library provides async database access for multiple backends (PostgreSQL, MySQL, SQLite) with a unified API.

**Test:**
```bash
cd dashboard/backend
pip install -r requirements.txt
uvicorn main:app --reload
# Visit http://localhost:8000/docs (FastAPI auto-docs)
```

---

### Phase 4: React Frontend Generation

**Tasks:**
1. Implement `src/codegen/dashboard/react_frontend.rs`
2. Generate `src/generated/entities.ts` TypeScript config
3. Generate base React app structure (if not exists)
4. Generate `EntityCard` component for each entity type
5. Generate WebSocket hook (`useRealtimeData.ts`)
6. Generate main Dashboard layout
7. Generate `package.json` with dependencies

**Output Files:**
- `dashboard/frontend/src/generated/entities.ts`
- `dashboard/frontend/src/components/EntityCard.tsx`
- `dashboard/frontend/src/hooks/useRealtimeData.ts`
- `dashboard/frontend/src/App.tsx`
- `dashboard/frontend/package.json`
- `dashboard/frontend/index.html`
- `dashboard/frontend/vite.config.ts`

**Dependencies (package.json):**
```json
{
  "dependencies": {
    "react": "^18.2.0",
    "react-dom": "^18.2.0"
  },
  "devDependencies": {
    "@vitejs/plugin-react": "^4.2.1",
    "typescript": "^5.3.3",
    "vite": "^5.0.11"
  }
}
```

**Test:**
```bash
cd dashboard/frontend
npm install
npm run dev
# Visit http://localhost:5173
```

---

### Phase 5: Docker Integration

**Tasks:**
1. Implement `src/codegen/dashboard/docker.rs`
2. Generate `dashboard/Dockerfile.backend`
3. Generate `dashboard/Dockerfile.frontend`
4. Generate docker-compose service definitions
5. Update existing `docker-compose.yml` (or create `docker-compose.dashboard.yml`)
6. Generate startup script `dashboard/start.sh`

**Output Files:**
- `dashboard/Dockerfile.backend`
- `dashboard/Dockerfile.frontend`
- `docker-compose.dashboard.yml` (merged into main compose file)

**Test:**
```bash
docker compose up dashboard-backend dashboard-frontend
# Verify services start
# Visit http://localhost:5173
```

---

### Phase 6: Build Integration & CLI

**Tasks:**
1. Add dashboard generation to nomnom CLI or create new command
2. Update `build.sh` to optionally generate dashboard
3. Create `dashboard/build.sh` script
4. Add `--with-dashboard` flag to build process
5. Document dashboard generation in README

**Commands:**
```bash
# Option 1: Automatic during build
./build.sh --with-dashboard

# Option 2: Separate command
nomnom generate dashboard --config entities/ --output dashboard/

# Option 3: Manual trigger
./dashboard/generate.sh
```

**Test:**
```bash
./build.sh --with-dashboard
docker compose up -d
# Visit http://localhost:5173
# Run parser: python3 generate_test_data.py | ./target/debug/record_parser --execute-db
# Watch real-time updates in dashboard
```

---

### Phase 7: Frontend Features & Polish

**Tasks:**
1. Implement auto-scroll to latest records
2. Add entity filtering (show/hide specific entities)
3. Add search/filter within records
4. Add pause/resume streaming button
5. Add export to CSV functionality
6. Add dark mode toggle
7. Add connection status indicator
8. Add statistics cards (totals, rates)

**Enhancements:**
- Smooth animations for new rows
- Color-coded entity types
- Timestamp formatting
- Number formatting (e.g., currency for prices)
- Responsive design (mobile-friendly)

---

### Phase 8: Testing & Documentation

**Tasks:**
1. End-to-end test: Generate data → See in dashboard
2. Test with multiple concurrent browser clients
3. Test reconnection after backend restart
4. Load testing (1000+ inserts/second)
5. Write dashboard user guide
6. Add screenshots to README
7. Document customization options

**Test Scenarios:**
- Single client: Insert 100 orders, verify all appear
- Multi-client: 3 browsers, verify all receive updates
- Reconnection: Restart backend, verify clients reconnect
- Performance: 10,000 inserts, verify no lag/memory leak

---

## File Structure (After Generation)

```
config/examples/tpch/
├── dashboard/
│   ├── backend/
│   │   ├── main.py              # Generated FastAPI app
│   │   ├── config.py            # Generated entity config
│   │   └── requirements.txt     # Generated deps
│   ├── frontend/
│   │   ├── src/
│   │   │   ├── generated/
│   │   │   │   └── entities.ts  # Generated entity types
│   │   │   ├── components/
│   │   │   │   ├── EntityCard.tsx
│   │   │   │   ├── Dashboard.tsx
│   │   │   │   └── Stats.tsx
│   │   │   ├── hooks/
│   │   │   │   └── useRealtimeData.ts
│   │   │   ├── App.tsx
│   │   │   └── main.tsx
│   │   ├── package.json         # Generated
│   │   └── vite.config.ts
│   ├── migrations/
│   │   ├── 001_create_triggers.sql  # Generated
│   │   └── run.sh
│   ├── Dockerfile.backend       # Generated
│   ├── Dockerfile.frontend      # Generated
│   └── README.md               # Generated usage docs
└── docker-compose.yml          # Updated with dashboard services
```

## Configuration Options

Allow users to customize dashboard via YAML:

**`entities/dashboard.yaml`:** (optional, with smart defaults)
```yaml
dashboard:
  # Database configuration (auto-detected from nomnom.yaml if not specified)
  database:
    type: postgresql  # or mysql, mariadb
    url: ${DATABASE_URL}  # Environment variable

  # Polling configuration
  polling:
    interval_ms: 500        # Polling interval (100-5000ms)
    max_events_per_poll: 100
    cleanup_after_days: 7   # Auto-delete old events from db_events table

  # Server ports
  ports:
    frontend: 5173
    backend: 8000

  # Entity display customization (optional - auto-generated if not specified)
  entities:
    - name: Order
      color: "#3b82f6"      # Override hash-based color
      icon: "📦"
      display_fields: ["order_key", "customer_key", "total_price", "order_date"]
      max_records: 500      # Records to keep in browser memory

    - name: OrderLineItem
      # If not specified, uses auto-generated defaults:
      # - color: hash-based
      # - icon: default emoji
      # - display_fields: first 4-5 from field_overrides
      # - max_records: 500

  # Feature flags
  features:
    auto_scroll: true
    dark_mode: true
    export_csv: true
    statistics: true
    filters: true
```

**Smart Defaults:**
- Database type auto-detected from `nomnom.yaml` or Diesel connection
- Entity colors hash-based for consistency
- Display fields: first 4-5 from `field_overrides`
- Icons: auto-assigned based on entity name patterns
- All features enabled by default

This allows per-project customization without changing code generators.

## Success Criteria

✅ **Code Generation:**
- Dashboard code fully generated from entity YAML configs
- No manual edits needed after generation
- Regeneration preserves user customizations (via config file)

✅ **Functionality:**
- Real-time updates appear within 100ms of database insert
- Support 100+ concurrent viewers
- Handle 1000+ inserts/second without lag
- Graceful reconnection on backend restart

✅ **Developer Experience:**
- Single command to generate and start: `./build.sh --with-dashboard && docker compose up`
- Hot-reload for development
- Clear error messages

✅ **User Experience:**
- Clean, responsive UI
- Obvious connection status
- Easy filtering and search
- Export functionality

## Future Enhancements (Post-MVP)

1. **Historical View:** Query past inserts, not just live stream
2. **Alerts:** Configure alerts on thresholds (e.g., >100 orders/min)
3. **Metrics:** Integration with Prometheus/Grafana for long-term metrics
4. **Multi-Database:** Support multiple PostgreSQL instances
5. **Authentication:** Add login for production deployments
6. **Custom Queries:** Allow users to write custom SQL views in dashboard
7. **UPDATE/DELETE Tracking:** Extend beyond just INSERTs
8. **Playback Mode:** Replay historical data at accelerated speed

## Timeline Estimate

- **Phase 1-2:** 4-6 hours (Foundation + SQL triggers)
- **Phase 3:** 3-4 hours (FastAPI backend)
- **Phase 4:** 6-8 hours (React frontend)
- **Phase 5:** 2-3 hours (Docker integration)
- **Phase 6:** 2-3 hours (Build integration)
- **Phase 7:** 4-6 hours (Features & polish)
- **Phase 8:** 2-3 hours (Testing & docs)

**Total: ~25-35 hours** for full implementation

**MVP (Phases 1-6):** ~15-20 hours

## Design Decisions (Confirmed)

1. **Database Strategy:** ✅ Polling-based using events table (works with PostgreSQL, MySQL, MariaDB)
2. **Polling Interval:** ✅ 500ms default (configurable: 100ms-5000ms)
3. **Color/Icon Assignment:** ✅ Hash entity name to color for consistency across rebuilds
4. **Field Selection:** ✅ Auto-select first 4-5 fields from field_overrides
5. **Record Retention:** ✅ Cap at 500 records per entity (configurable via YAML)
6. **WebSocket Library:** ✅ FastAPI built-in WebSocket (simpler, no extra dependencies)
7. **Frontend Framework:** ✅ React (better ecosystem, more examples)
8. **TypeScript:** ✅ Required for type safety with generated code
9. **Styling:** ✅ Tailwind CSS (easy to generate utility classes)

### Why Polling Instead of LISTEN/NOTIFY?

**Advantages:**
- ✅ **Database Agnostic:** Works with PostgreSQL, MySQL, and MariaDB
- ✅ **Simpler Implementation:** No need for dedicated LISTEN connections
- ✅ **Easier to Debug:** Standard SQL queries, visible in logs
- ✅ **More Reliable:** No connection state to manage
- ✅ **Still Fast:** 500ms polling feels real-time for most use cases
- ✅ **Ordered Events:** Events table provides guaranteed ordering
- ✅ **Historical Access:** Can query past events easily

**Trade-offs:**
- ⚠️ 100-500ms delay vs instant with LISTEN/NOTIFY
- ⚠️ Periodic database load (mitigated by indexed queries)
- ✅ For typical insert rates (<100/sec), overhead is negligible

## Next Steps

1. ✅ **Plan Approved** - Design decisions confirmed
2. **Phase 1**: Set up code generation module structure
3. **Phase 2**: Implement SQL trigger generation
4. **Quick Test**: Manually verify triggers work with pg_notify
5. **Continue with Phases 3-8**

---

**Plan Version:** 2.0
**Created:** 2025-11-09
**Updated:** 2025-11-09
**Status:** Approved - Ready for Implementation

**Changelog:**
- v2.0: Changed from PostgreSQL LISTEN/NOTIFY to database-agnostic polling approach
- v1.1: Initial design decisions confirmed
- v1.0: Initial draft
