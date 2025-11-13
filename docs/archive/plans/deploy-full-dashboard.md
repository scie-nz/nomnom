# Plan: Deploy Full Functional Dashboard

## Date
2025-11-12

## Executive Summary

The nomnom system has **fully functional code generation** for all components. However, the current Kubernetes deployment is using a **placeholder frontend** instead of the generated React application. This plan outlines how to regenerate and deploy the complete, functional dashboard.

## Current State Analysis

### What's Currently Deployed

| Component | Status | Details |
|-----------|--------|---------|
| **Dashboard Backend** | ✅ FUNCTIONAL | Axum Rust server with WebSocket, REST API, database polling |
| **Worker** | ✅ FUNCTIONAL | NATS consumer, database insertion, DLQ routing, retry logic |
| **Ingestion API** | ✅ FUNCTIONAL | NATS publisher, HTTP endpoints for message ingestion |
| **Dashboard Frontend** | ❌ PLACEHOLDER | Static HTML page instead of React app |

### Root Cause

The `test-helm-kind.sh` script (lines 192-193) builds a generic placeholder frontend from `Dockerfile.frontend` instead of the **generated React dashboard**.

```bash
# Current (WRONG):
docker build -f Dockerfile.frontend -t localhost:5001/nomnom-dashboard-frontend:latest .

# Should be:
# Generate dashboard first, THEN build from generated code
```

### What Exists But Isn't Deployed

The code generation system creates a **complete React dashboard** with:
- Real-time WebSocket connection to backend
- Entity cards displaying table data
- Connection status indicators
- TypeScript + Vite + Tailwind CSS
- Auto-generated entity metadata
- Configurable record limits

**Generated Code Location:** `/tmp/nomnom-dashboard-test/frontend/`

## Architecture Overview

```
┌─────────────────────────────────────────────────────────────┐
│                         User Browser                         │
│                    http://localhost:8081                     │
└──────────────────────────┬──────────────────────────────────┘
                           │
                           ▼
┌─────────────────────────────────────────────────────────────┐
│              Dashboard Frontend (React + Vite)               │
│  - Real-time WebSocket connection                            │
│  - Entity cards with scrollable tables                       │
│  - Connection status (green/red dot)                         │
│  - Displays last 10 records per entity                       │
└──────────────────────────┬──────────────────────────────────┘
                           │ WebSocket (/ws) + REST (/api/*)
                           ▼
┌─────────────────────────────────────────────────────────────┐
│            Dashboard Backend (Axum Rust Server)              │
│  - Direct table polling (500ms intervals)                    │
│  - WebSocket broadcast to all clients                        │
│  - REST endpoints: /api/entities, /api/stats, /api/health   │
│  - Initial data loading on WebSocket connect                 │
└──────────────────────────┬──────────────────────────────────┘
                           │ SQL queries
                           ▼
┌─────────────────────────────────────────────────────────────┐
│                    PostgreSQL Database                       │
│  Tables:                                                      │
│  - orders (root entity)                                      │
│  - order_line_items (derived entity)                         │
│  - message_status (tracking)                                 │
└─────────────────────────────────────────────────────────────┘
                           ▲
                           │ INSERT operations
┌──────────────────────────┴──────────────────────────────────┐
│                    Worker (NATS Consumer)                    │
│  - Consumes messages from NATS JetStream                     │
│  - Parses JSON payloads                                      │
│  - Inserts into entity tables                                │
│  - DLQ routing on max retries                                │
│  - Tracks status in message_status table                     │
└──────────────────────────┬──────────────────────────────────┘
                           │ Pull messages
                           ▼
┌─────────────────────────────────────────────────────────────┐
│              NATS JetStream (Message Broker)                 │
│  Stream: MESSAGES                                            │
│  Consumer: workers (durable, explicit ACK)                   │
│  DLQ: messages.dlq.{entity_type}                             │
└──────────────────────────┬──────────────────────────────────┘
                           ▲
                           │ Publish messages
┌──────────────────────────┴──────────────────────────────────┐
│              Ingestion API (NATS Publisher)                  │
│  POST /ingest/{entity_type}                                  │
│  - Validates JSON payload                                    │
│  - Wraps in MessageEnvelope (UUID, timestamp, source)        │
│  - Publishes to NATS stream                                  │
└─────────────────────────────────────────────────────────────┘
```

## Implementation Plan

### Phase 1: Regenerate Dashboard with Fixed Code

**Objective:** Generate fresh dashboard code with all table generation fixes

**Steps:**

1. **Build nomnom binary with fixes**
   ```bash
   export RUSTFLAGS="-L /opt/homebrew/opt/libpq/lib"
   cargo build --bin nomnom
   ```

2. **Generate complete dashboard**
   ```bash
   OUTPUT_DIR="/tmp/nomnom-full-dashboard-$(date +%s)"

   ./target/debug/nomnom generate-dashboard \
     --entities config/examples/tpch/entities \
     --output "$OUTPUT_DIR" \
     --database postgresql \
     --backend axum
   ```

3. **Verify generated structure**
   ```bash
   ls -la "$OUTPUT_DIR"
   # Should contain:
   # - backend/  (Axum Rust server)
   # - frontend/ (React + Vite app)
   # - Dockerfile.backend
   # - Dockerfile.frontend
   # - docker-compose.yml
   ```

### Phase 2: Build Docker Images

**Objective:** Build production-ready Docker images from generated code

**Backend Image:**
```bash
cd "$OUTPUT_DIR"

docker build -f Dockerfile.backend \
  -t localhost:5001/nomnom-dashboard-backend:functional \
  .

docker push localhost:5001/nomnom-dashboard-backend:functional
```

**Frontend Image:**
```bash
cd "$OUTPUT_DIR"

docker build -f Dockerfile.frontend \
  -t localhost:5001/nomnom-dashboard-frontend:functional \
  .

docker push localhost:5001/nomnom-dashboard-frontend:functional
```

### Phase 3: Update Kubernetes Deployment

**Objective:** Deploy functional frontend and backend to Kubernetes

**Option A: Helm Upgrade with New Images**
```bash
helm upgrade nomnom nomnom-helm \
  -n nomnom-dev \
  --reuse-values \
  --set dashboard.backend.image.tag=functional \
  --set dashboard.backend.resources.limits.memory=1Gi \
  --set dashboard.frontend.image.tag=functional \
  --wait --timeout 3m
```

**Option B: Update test-helm-kind.sh Script**

Modify `test-helm-kind.sh` to generate dashboard before building:

```bash
# Around line 120, add BEFORE building frontend:
if [ "$DEV_MODE" = true ]; then
    log_info "Generating full dashboard with nomnom CLI..."

    # Build nomnom with fixes
    export RUSTFLAGS="-L /opt/homebrew/opt/libpq/lib"
    cargo build --bin nomnom

    # Generate dashboard
    DASHBOARD_DIR="/tmp/kind-dashboard-$(date +%s)"
    ./target/debug/nomnom generate-dashboard \
      --entities config/examples/tpch/entities \
      --output "$DASHBOARD_DIR" \
      --database postgresql \
      --backend axum

    # Build backend from generated code
    docker build -f "$DASHBOARD_DIR/Dockerfile.backend" \
      -t localhost:${REGISTRY_PORT}/nomnom-dashboard-backend:latest \
      "$DASHBOARD_DIR"
    docker push localhost:${REGISTRY_PORT}/nomnom-dashboard-backend:latest

    # Build frontend from generated code
    docker build -f "$DASHBOARD_DIR/Dockerfile.frontend" \
      -t localhost:${REGISTRY_PORT}/nomnom-dashboard-frontend:latest \
      "$DASHBOARD_DIR"
    docker push localhost:${REGISTRY_PORT}/nomnom-dashboard-frontend:latest
fi
```

### Phase 4: Verification

**Objective:** Confirm all components are functional

**4.1 Check Pod Status**
```bash
kubectl get pods -n nomnom-dev
# All pods should be Running with 1/1 READY
```

**4.2 Verify Backend WebSocket**
```bash
# Backend should expose /ws endpoint
kubectl logs -n nomnom-dev -l app.kubernetes.io/component=dashboard-backend --tail=20
# Should see: "Dashboard server listening on http://0.0.0.0:8080"
```

**4.3 Test Frontend Connection**
```bash
# Access dashboard
open http://localhost:8081

# Should see:
# - "Connected" status (green dot)
# - Entity cards for "orders" and "order_line_items"
# - "No data available" or actual records if data exists
```

**4.4 Test End-to-End Data Flow**
```bash
# 1. Send test message to ingestion API
curl -X POST http://localhost:8080/ingest/Order \
  -H "Content-Type: application/json" \
  -d '{
    "order_key": "O-TEST-001",
    "customer_key": "C-001",
    "order_status": "P",
    "total_price": 12345.67,
    "order_date": "2024-11-12",
    "order_priority": "1-URGENT"
  }'

# 2. Check worker logs (should process message)
kubectl logs -n nomnom-dev -l app.kubernetes.io/component=worker --tail=30
# Should see: INSERT INTO orders ... executed

# 3. Check dashboard backend logs (should detect new row)
kubectl logs -n nomnom-dev -l app.kubernetes.io/component=dashboard-backend --tail=30
# Should see: Broadcasting to N clients

# 4. Check dashboard frontend
# Should see new order appear in the "orders" card within 500ms
```

### Phase 5: Configure Backend Service Port

**Issue:** Backend is listening on port 8080 but service expects 3000

**Fix Backend Configuration:**

Option A: Update generated backend to use port 3000
```rust
// In generated src/main.rs, change:
let addr = SocketAddr::from(([0, 0, 0, 0], 8080));  // BEFORE

// To:
let port = env::var("PORT").unwrap_or_else(|_| "3000".to_string());
let addr = SocketAddr::from(([0, 0, 0, 0], port.parse::<u16>().unwrap()));
```

Option B: Update Helm service to match backend port
```bash
helm upgrade nomnom nomnom-helm \
  -n nomnom-dev \
  --reuse-values \
  --set dashboard.backend.service.port=8080 \
  --set dashboard.backend.containerPort=8080
```

## Expected Results

### Before Fix
```
Dashboard Frontend:  ❌ Placeholder HTML page
Dashboard Backend:   ✅ Working but no frontend
Worker:              ✅ Processing messages
Database:            ✅ Tables created with all fixes
```

### After Fix
```
Dashboard Frontend:  ✅ React app with real-time updates
Dashboard Backend:   ✅ WebSocket + REST API fully functional
Worker:              ✅ Processing messages → database → dashboard
Database:            ✅ Tables with proper schema
End-to-End Flow:     ✅ API → NATS → Worker → DB → Dashboard → Browser
```

### Dashboard Features

**Connection Status:**
- Green dot: Connected to backend WebSocket
- Red dot: Disconnected, attempting reconnection

**Entity Cards:**
- One card per entity (orders, order_line_items)
- Displays last 10 records (configurable)
- Shows all fields from database schema
- Scrollable table with headers

**Real-Time Updates:**
- New records appear within 500ms of insertion
- Updates broadcast to all connected clients
- Maintains client-side state with max records cap

**API Endpoints:**
- `GET /api/entities` - List all entities with metadata
- `GET /api/stats` - Get row counts per entity
- `GET /api/health` - Health check endpoint
- `WS /ws` - WebSocket connection for real-time updates

## Testing Checklist

### Functional Tests

- [ ] Dashboard frontend loads without errors
- [ ] WebSocket connection established (green status)
- [ ] Entity cards displayed for both tables
- [ ] Send test message via ingestion API
- [ ] Worker processes message successfully
- [ ] New record appears in dashboard within 1 second
- [ ] Multiple browser tabs receive same updates
- [ ] Disconnecting backend shows red status in frontend
- [ ] Reconnecting restores data and shows green status

### Performance Tests

- [ ] Dashboard handles 100+ records per entity
- [ ] Backend memory usage stays under 1GB
- [ ] Worker processes messages at >100/sec throughput
- [ ] Frontend remains responsive with 10+ entities
- [ ] WebSocket broadcasts complete in <50ms

### Error Handling Tests

- [ ] Invalid JSON to ingestion API returns 400 error
- [ ] Worker handles malformed messages (DLQ routing)
- [ ] Dashboard shows error banner on backend failure
- [ ] Frontend reconnects after backend restart
- [ ] Worker retries failed database insertions

## Rollback Plan

If issues occur after deployment:

```bash
# 1. Revert to previous images
helm upgrade nomnom nomnom-helm \
  -n nomnom-dev \
  --reuse-values \
  --set dashboard.backend.image.tag=latest \
  --set dashboard.frontend.image.tag=latest

# 2. Or redeploy entire stack
./test-helm-kind.sh --dev

# 3. Check pod status
kubectl get pods -n nomnom-dev
kubectl logs -n nomnom-dev -l app.kubernetes.io/component=dashboard-frontend
```

## File Locations Reference

### Code Generation Templates
```
/Users/bogdanstate/nomnom/src/codegen/
├── dashboard/
│   ├── axum_backend.rs      # Backend generator
│   ├── fastapi_backend.rs   # Python alternative
│   ├── react_frontend.rs    # Frontend generator
│   ├── docker.rs            # Dockerfile generation
│   └── sql_triggers.rs      # Migration generation
└── worker/
    ├── main_rs.rs           # Worker main loop
    ├── database_rs.rs       # Database operations (FIXED)
    ├── cargo_toml.rs        # Cargo.toml generation
    └── parsers_rs.rs        # Message parsing
```

### Generated Code Examples
```
/tmp/nomnom-dashboard-test/   # Latest generated dashboard
├── backend/
│   ├── src/
│   │   ├── main.rs          # Axum server
│   │   ├── config.rs        # Entity metadata
│   │   ├── polling.rs       # Table polling logic
│   │   ├── websocket.rs     # WebSocket handler
│   │   └── api.rs           # REST endpoints
│   └── Cargo.toml
└── frontend/
    ├── src/
    │   ├── App.tsx          # Main React component
    │   ├── components/
    │   │   ├── Dashboard.tsx    # Dashboard layout
    │   │   └── EntityCard.tsx   # Entity table display
    │   ├── hooks/
    │   │   └── useRealtimeData.ts  # WebSocket hook
    │   └── generated/
    │       └── entities.ts   # Auto-generated metadata
    ├── package.json
    └── vite.config.ts
```

### Worker Code
```
/tmp/worker-all-fixes/        # Latest generated worker (with all fixes)
├── src/
│   ├── main.rs              # NATS consumer + processing loop
│   ├── database.rs          # Table creation + INSERT operations
│   ├── models.rs            # Data models
│   ├── parsers.rs           # JSON parsing
│   └── error.rs             # Error types
├── Cargo.toml
└── Dockerfile
```

## Next Steps Priority

1. **IMMEDIATE:** Regenerate dashboard with current fixes
2. **HIGH:** Deploy functional React frontend
3. **MEDIUM:** Test end-to-end message flow
4. **LOW:** Add authentication/authorization layer

## Long-Term Improvements

### Frontend Enhancements
- Filtering and search within entity cards
- Pagination for large datasets
- Export to CSV/JSON
- Dark mode support
- Configurable refresh rates
- Entity relationship visualization

### Backend Enhancements
- Authentication with JWT
- Rate limiting per client
- Message compression (gzip)
- Persistence snapshots for large tables
- Query optimization with prepared statements
- Metrics endpoint (Prometheus format)

### Worker Enhancements
- Configurable batch processing
- Dead letter queue UI
- Message replay functionality
- Poison message detection
- Performance metrics tracking
- Horizontal scaling with KEDA

## Conclusion

The nomnom system has a **complete, production-ready code generation infrastructure**. The current issue is simply that the placeholder frontend was deployed instead of the generated React application. Following this plan will result in a fully functional real-time dashboard displaying data from both the `orders` and `order_line_items` tables, with all components working together seamlessly.

**Estimated Time:** 30-45 minutes to regenerate, build, and deploy
**Risk Level:** Low (backend and worker are already functional)
**Impact:** High (transforms placeholder into functional real-time dashboard)
