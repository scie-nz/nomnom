# Plan: Complete End-to-End Deployment with All Fixes

## Date
2025-11-12

## Executive Summary

The dashboard frontend and backend are now fully functional and deployed to Kubernetes. However, the worker and ingestion API are using old images from before our code generation fixes. This plan outlines:

1. Dashboard fixes that were applied (to be persisted in Helm chart)
2. Steps to generate and deploy updated worker and ingestion API
3. Complete end-to-end testing procedure

## Current Deployment Status

| Component | Status | Image | Notes |
|-----------|--------|-------|-------|
| **Dashboard Frontend** | ✅ FUNCTIONAL | `nomnom-dashboard-frontend:functional` | React + Vite, 256Mi memory, port 5173 |
| **Dashboard Backend** | ✅ FUNCTIONAL | `nomnom-dashboard-backend:functional` | Axum Rust, 1Gi memory, port 8080 |
| **Worker** | ❌ OLD IMAGE | `nomnom-worker:latest` | Needs regeneration with table fixes |
| **Ingestion API** | ❌ OLD IMAGE | `nomnom-ingestion-api:latest` | Needs regeneration with NATS routes |
| **NATS** | ✅ RUNNING | - | JetStream ready |
| **PostgreSQL** | ✅ RUNNING | - | No tables (waiting for worker) |

## Dashboard Fixes Applied

### 1. Backend Readiness Probe Fix

**Problem**: Container port was 3000, but application listens on 8080

**Fix Applied**:
```bash
kubectl patch deployment nomnom-dashboard-backend -n nomnom-dev \
  --type='json' \
  -p='[{"op": "replace", "path": "/spec/template/spec/containers/0/ports/0/containerPort", "value":8080}]'
```

**Helm Chart Update Required**:
```yaml
# nomnom-helm/values.yaml
dashboard:
  backend:
    containerPort: 8080  # Changed from 3000
    service:
      port: 8080         # Changed from 3000
```

### 2. Frontend Memory Limit Fix

**Problem**: Frontend OOMKilled with 64Mi limit (npm dev server needs more)

**Fix Applied**:
```bash
kubectl patch deployment nomnom-dashboard-frontend -n nomnom-dev \
  --type='json' \
  -p='[{"op": "replace", "path": "/spec/template/spec/containers/0/resources/limits/memory", "value":"256Mi"}]'
```

**Helm Chart Update Required**:
```yaml
# nomnom-helm/values.yaml
dashboard:
  frontend:
    resources:
      limits:
        memory: 256Mi  # Changed from 64Mi
      requests:
        memory: 128Mi
```

### 3. Backend Memory Limit

**Problem**: Backend OOMKilled with default limits (debug builds need more memory)

**Fix Applied**:
```bash
helm upgrade nomnom nomnom-helm -n nomnom-dev \
  --set dashboard.backend.resources.limits.memory=1Gi
```

**Helm Chart Update Required**:
```yaml
# nomnom-helm/values.yaml
dashboard:
  backend:
    resources:
      limits:
        memory: 1Gi    # Changed from 128Mi for debug builds
        cpu: 500m
      requests:
        memory: 256Mi
        cpu: 100m
```

### 4. Dashboard Image Tags

**Current**: Using `functional` tag for both frontend and backend

**Helm Chart Update Required**:
```yaml
# nomnom-helm/values.yaml
dashboard:
  backend:
    image:
      repository: localhost:5001/nomnom-dashboard-backend
      tag: functional  # Or set to 'latest' after regenerating
      pullPolicy: Always
  frontend:
    image:
      repository: localhost:5001/nomnom-dashboard-frontend
      tag: functional  # Or set to 'latest' after regenerating
      pullPolicy: Always
```

## Code Generation Fixes Applied

### Worker Database Generation Fix

**File**: `src/codegen/worker/database_rs.rs`

**Change** (lines 68-77):
```rust
// BEFORE:
for entity in entities {
    if !entity.is_root() || !entity.is_persistent() || entity.is_abstract {
        continue;
    }
```

**After**:
```rust
for entity in entities {
    // Skip entities without persistence or abstract entities
    if !entity.is_persistent() || entity.is_abstract {
        continue;
    }

    // Skip reference entities (read from external sources)
    if entity.source_type.to_lowercase() == "reference" {
        continue;
    }
```

**Impact**:
- ✅ Creates tables for ALL persistent entities (root + derived)
- ✅ Includes `orders` AND `order_line_items` tables
- ✅ Auto-generates `id SERIAL PRIMARY KEY`
- ✅ Creates UNIQUE constraints for unicity_fields
- ✅ Creates indices for performance

## Phase 1: Update Helm Chart with Dashboard Fixes

### Step 1.1: Locate Helm Chart

```bash
cd /Users/bogdanstate/nomnom
find . -name "values.yaml" -path "*/nomnom-helm/*"
```

### Step 1.2: Update values.yaml

Edit the Helm chart values file with all dashboard fixes:

```yaml
dashboard:
  backend:
    image:
      repository: localhost:5001/nomnom-dashboard-backend
      tag: functional
      pullPolicy: Always

    containerPort: 8080  # CHANGED from 3000

    service:
      type: NodePort
      port: 8080  # CHANGED from 3000
      targetPort: 8080  # CHANGED from 3000

    resources:
      limits:
        cpu: 500m
        memory: 1Gi  # CHANGED from 128Mi
      requests:
        cpu: 100m
        memory: 256Mi

  frontend:
    image:
      repository: localhost:5001/nomnom-dashboard-frontend
      tag: functional
      pullPolicy: Always

    containerPort: 5173

    service:
      type: NodePort
      port: 5173
      targetPort: 5173

    resources:
      limits:
        cpu: 500m
        memory: 256Mi  # CHANGED from 64Mi
      requests:
        cpu: 100m
        memory: 128Mi  # CHANGED from 32Mi
```

### Step 1.3: Apply Helm Chart Updates

```bash
helm upgrade nomnom nomnom-helm -n nomnom-dev --wait --timeout 3m
```

### Step 1.4: Verify Dashboard Still Works

```bash
kubectl get pods -n nomnom-dev
# Both dashboard pods should be 1/1 Running

# Test dashboard access
curl http://localhost:8081
```

## Phase 2: Generate and Deploy Updated Worker

### Step 2.1: Build nomnom Binary

```bash
cd /Users/bogdanstate/nomnom
export RUSTFLAGS="-L /opt/homebrew/opt/libpq/lib"
cargo build --bin nomnom
```

### Step 2.2: Generate Worker Code

```bash
WORKER_DIR="/tmp/nomnom-worker-fixed-$(date +%s)"

./target/debug/nomnom generate-worker \
  --entities config/examples/tpch/entities \
  --output "$WORKER_DIR" \
  --nats-url "nats://nats:4222"

echo "Worker generated in: $WORKER_DIR"
```

### Step 2.3: Verify Generated Code

```bash
# Check that database.rs includes table creation for both entities
grep -A 5 "order_line_items" "$WORKER_DIR/src/database.rs"
grep -A 5 "orders" "$WORKER_DIR/src/database.rs"

# Verify UNIQUE constraints are present
grep "CONSTRAINT" "$WORKER_DIR/src/database.rs"
```

### Step 2.4: Build Worker Docker Image

```bash
cd "$WORKER_DIR"

docker build -t localhost:5001/nomnom-worker:latest .
docker push localhost:5001/nomnom-worker:latest
```

### Step 2.5: Deploy Worker

```bash
kubectl delete pod -n nomnom-dev -l app.kubernetes.io/component=worker

# Wait for new pod
sleep 10
kubectl get pods -n nomnom-dev -l app.kubernetes.io/component=worker
```

### Step 2.6: Verify Tables Created

```bash
# Check worker logs
kubectl logs -n nomnom-dev -l app.kubernetes.io/component=worker --tail=50

# Verify tables exist
kubectl exec -n nomnom-dev nomnom-postgres-0 -- \
  psql -U postgres -c "\dt"

# Should see:
# - orders
# - order_line_items
# - message_status
```

## Phase 3: Generate and Deploy Updated Ingestion API

### Step 3.1: Generate Ingestion API Code

```bash
INGESTION_DIR="/tmp/nomnom-ingestion-api-fixed-$(date +%s)"

./target/debug/nomnom generate-ingestion-api \
  --entities config/examples/tpch/entities \
  --output "$INGESTION_DIR" \
  --nats-url "nats://nats:4222"

echo "Ingestion API generated in: $INGESTION_DIR"
```

### Step 3.2: Verify Generated Routes

```bash
# Check main.rs for ingestion routes
grep -A 10 "ingest" "$INGESTION_DIR/src/main.rs"

# Should see routes like:
# POST /ingest/Order
# POST /ingest/OrderLineItem
```

### Step 3.3: Build Ingestion API Docker Image

```bash
cd "$INGESTION_DIR"

docker build -t localhost:5001/nomnom-ingestion-api:latest .
docker push localhost:5001/nomnom-ingestion-api:latest
```

### Step 3.4: Deploy Ingestion API

```bash
kubectl delete pod -n nomnom-dev -l app.kubernetes.io/component=ingestion-api

# Wait for new pod
sleep 10
kubectl get pods -n nomnom-dev -l app.kubernetes.io/component=ingestion-api
```

### Step 3.5: Verify API Routes

```bash
# Check ingestion API logs
kubectl logs -n nomnom-dev -l app.kubernetes.io/component=ingestion-api --tail=50

# Should see routes registered
```

## Phase 4: End-to-End Testing

### Step 4.1: Verify All Pods Running

```bash
kubectl get pods -n nomnom-dev

# Expected:
# nomnom-dashboard-backend-*     1/1  Running
# nomnom-dashboard-frontend-*    1/1  Running
# nomnom-worker-*                1/1  Running
# nomnom-ingestion-api-*         1/1  Running
# nomnom-nats-0                  1/1  Running
# nomnom-postgres-0              1/1  Running
```

### Step 4.2: Test Order Ingestion

```bash
# Send test order
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

# Should return: {"message_id": "...", "status": "published"}
```

### Step 4.3: Verify Message Flow

**Check NATS**:
```bash
kubectl exec -n nomnom-dev nomnom-nats-0 -- \
  nats stream info MESSAGES

# Should show 1 message
```

**Check Worker Logs**:
```bash
kubectl logs -n nomnom-dev -l app.kubernetes.io/component=worker --tail=20

# Should see:
# - "Received message..."
# - "INSERT INTO orders..."
# - "Message processed successfully"
```

**Check Database**:
```bash
kubectl exec -n nomnom-dev nomnom-postgres-0 -- \
  psql -U postgres -c "SELECT * FROM orders WHERE order_key = 'O-TEST-001';"

# Should return the inserted order
```

### Step 4.4: Verify Dashboard Real-Time Update

**Check Dashboard Backend Logs**:
```bash
kubectl logs -n nomnom-dev -l app.kubernetes.io/component=dashboard-backend --tail=20

# Should see:
# - "Detected new rows in orders: 1"
# - "Broadcasting to N clients"
```

**Check Dashboard Frontend**:
```bash
# Open browser to http://localhost:8081
# Should see:
# - Connection status: Green dot (Connected)
# - Orders card with 1 record showing:
#   - order_key: O-TEST-001
#   - customer_key: C-001
#   - order_status: P
#   - total_price: 12345.67
#   - ...
```

### Step 4.5: Test OrderLineItem Ingestion

```bash
# Send test line item
curl -X POST http://localhost:8080/ingest/OrderLineItem \
  -H "Content-Type: application/json" \
  -d '{
    "order_key": "O-TEST-001",
    "line_number": 1,
    "part_key": "P-12345",
    "supplier_key": "S-001",
    "quantity": 10,
    "extended_price": 1234.56,
    "discount": 0.05,
    "tax": 0.08,
    "return_flag": "N",
    "line_status": "O",
    "ship_date": "2024-11-15",
    "commit_date": "2024-11-14",
    "receipt_date": "2024-11-16"
  }'
```

**Verify in Dashboard**:
- OrderLineItems card should appear
- Should show 1 record with line_number: 1

### Step 4.6: Performance Test

```bash
# Send 10 orders in rapid succession
for i in {1..10}; do
  curl -X POST http://localhost:8080/ingest/Order \
    -H "Content-Type: application/json" \
    -d "{
      \"order_key\": \"O-TEST-$(printf "%03d" $i)\",
      \"customer_key\": \"C-001\",
      \"order_status\": \"P\",
      \"total_price\": $((12345 + i)),
      \"order_date\": \"2024-11-12\",
      \"order_priority\": \"1-URGENT\"
    }"

  sleep 0.1
done

# Dashboard should show all 10 orders within 5 seconds
```

## Phase 5: Update Helm Chart for Worker and Ingestion API

### Step 5.1: Update values.yaml for Worker

```yaml
worker:
  image:
    repository: localhost:5001/nomnom-worker
    tag: latest
    pullPolicy: Always

  env:
    - name: DATABASE_URL
      value: "postgresql://postgres:postgres@postgres:5432/postgres"
    - name: NATS_URL
      value: "nats://nats:4222"

  resources:
    limits:
      cpu: 500m
      memory: 512Mi
    requests:
      cpu: 100m
      memory: 256Mi
```

### Step 5.2: Update values.yaml for Ingestion API

```yaml
ingestionApi:
  image:
    repository: localhost:5001/nomnom-ingestion-api
    tag: latest
    pullPolicy: Always

  containerPort: 8080

  service:
    type: NodePort
    port: 8080
    targetPort: 8080

  env:
    - name: NATS_URL
      value: "nats://nats:4222"

  resources:
    limits:
      cpu: 500m
      memory: 256Mi
    requests:
      cpu: 100m
      memory: 128Mi
```

### Step 5.3: Apply Full Helm Chart

```bash
helm upgrade nomnom nomnom-helm -n nomnom-dev --wait --timeout 5m
```

## Verification Checklist

### Infrastructure
- [ ] All pods are 1/1 Running
- [ ] No pod restarts in last 5 minutes
- [ ] All services have endpoints

### Database
- [ ] `orders` table exists with correct schema
- [ ] `order_line_items` table exists with correct schema
- [ ] `message_status` table exists
- [ ] UNIQUE constraints are present
- [ ] Indices are created

### NATS
- [ ] MESSAGES stream exists
- [ ] workers consumer exists
- [ ] DLQ subjects configured

### Dashboard Backend
- [ ] Listening on port 8080
- [ ] WebSocket endpoint `/ws` accessible
- [ ] REST endpoints `/api/entities`, `/api/stats`, `/api/health` working
- [ ] Polling both tables every 500ms
- [ ] Memory usage stable under 1Gi

### Dashboard Frontend
- [ ] Vite dev server running on port 5173
- [ ] Accessible at http://localhost:8081
- [ ] Shows "Connected" status (green dot)
- [ ] Displays entity cards for both tables
- [ ] Real-time updates visible within 1 second

### Worker
- [ ] Consuming from NATS JetStream
- [ ] Creating tables on startup
- [ ] Inserting records into database
- [ ] ACKing messages successfully
- [ ] No errors in logs

### Ingestion API
- [ ] Routes configured for all entities
- [ ] POST /ingest/Order returns message_id
- [ ] POST /ingest/OrderLineItem returns message_id
- [ ] Publishing to NATS successfully
- [ ] No 404 errors

### End-to-End Flow
- [ ] API accepts JSON payload
- [ ] Message published to NATS
- [ ] Worker consumes and inserts to DB
- [ ] Dashboard detects new row
- [ ] Frontend displays record within 1 second
- [ ] Multiple concurrent requests handled correctly

## Rollback Plan

If issues occur:

### Rollback Dashboard
```bash
helm upgrade nomnom nomnom-helm -n nomnom-dev \
  --set dashboard.backend.image.tag=latest \
  --set dashboard.frontend.image.tag=latest
```

### Rollback Worker
```bash
# Use previous working image
kubectl set image deployment/nomnom-worker -n nomnom-dev \
  worker=localhost:5001/nomnom-worker:previous
```

### Rollback Ingestion API
```bash
kubectl set image deployment/nomnom-ingestion-api -n nomnom-dev \
  ingestion-api=localhost:5001/nomnom-ingestion-api:previous
```

### Full Cluster Reset
```bash
./test-helm-kind.sh --dev
```

## File Locations

### Generated Code
```
/tmp/nomnom-full-dashboard-1762943219/    # Dashboard (DEPLOYED)
/tmp/nomnom-worker-fixed-*/                # Worker (TO DEPLOY)
/tmp/nomnom-ingestion-api-fixed-*/         # Ingestion API (TO DEPLOY)
```

### Code Generation Sources
```
/Users/bogdanstate/nomnom/src/codegen/
├── dashboard/
│   ├── axum_backend.rs
│   ├── react_frontend.rs
│   └── docker.rs
├── worker/
│   ├── main_rs.rs
│   ├── database_rs.rs  # FIXED: persistent entity filter
│   ├── parsers_rs.rs
│   └── cargo_toml.rs
└── ingestion_server/
    ├── main_rs.rs
    ├── handlers_rs.rs
    ├── nats_client_rs.rs
    └── cargo_toml.rs
```

### Helm Chart
```
nomnom-helm/
├── Chart.yaml
├── values.yaml  # UPDATE WITH ALL FIXES
└── templates/
    ├── dashboard-backend.yaml
    ├── dashboard-frontend.yaml
    ├── worker.yaml
    └── ingestion-api.yaml
```

## Success Criteria

1. ✅ Dashboard frontend displays real-time data
2. ✅ Dashboard backend polling all persistent entity tables
3. ✅ Worker creates tables for ALL persistent entities (root + derived)
4. ✅ Ingestion API accepts messages for all entity types
5. ✅ Complete message flow: API → NATS → Worker → DB → Dashboard → Browser
6. ✅ All fixes persisted in Helm chart
7. ✅ No manual kubectl patches required after Helm deployment
8. ✅ System handles 100+ messages/sec throughput

## Next Steps After Completion

1. **Generate and Deploy Worker** following Phase 2
2. **Generate and Deploy Ingestion API** following Phase 3
3. **Test End-to-End Flow** following Phase 4
4. **Update Helm Chart** following Phase 5
5. **Verify All Checklist Items** in Verification Checklist
6. **Document for Production** if all tests pass

## Estimated Timeline

- Phase 1 (Helm Chart Update): 15 minutes
- Phase 2 (Worker): 20 minutes
- Phase 3 (Ingestion API): 20 minutes
- Phase 4 (E2E Testing): 30 minutes
- Phase 5 (Final Helm Update): 15 minutes

**Total**: ~1.5-2 hours for complete deployment and testing
