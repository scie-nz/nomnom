# Current NATS Setup - Verified Configuration

**Date**: 2025-11-10
**Status**: ✅ Fully Tested and Working

## Services Running

```
✓ NATS JetStream     (nats:latest)          ports: 4222 (client), 8222 (monitoring)
✓ PostgreSQL         (postgres:16-alpine)   port: 5432
✓ Ingestion API      (custom Alpine)        port: 8080
✓ Worker             (custom Alpine)        (no HTTP ports)
```

## Ingestion API Endpoints

### Available Endpoints (from OpenAPI spec)

1. **`GET /health`** ✅
   ```json
   {
     "status": "healthy",
     "database": "connected",
     "entities": ["Order"],
     "version": "0.1.0"
   }
   ```

2. **`POST /ingest/message`** ✅
   - **Content-Type**: `application/json`
   - **Request Body**: JSON object matching entity schema
   - **Response**: 202 Accepted
   ```json
   {
     "message_id": "uuid",
     "status": "accepted",
     "timestamp": "ISO8601"
   }
   ```
   - **Example**:
   ```bash
   curl -X POST http://localhost:8080/ingest/message \
     -H "Content-Type: application/json" \
     -d '{
       "order_key": "123",
       "customer_key": "CUST-001",
       "order_status": "O",
       "total_price": 99.99,
       "order_date": "2024-01-01",
       "order_priority": "1-URGENT",
       "clerk": "Clerk#001",
       "ship_priority": 0,
       "comment": "Test order"
     }'
   ```

3. **`POST /ingest/batch`** ✅
   - **Content-Type**: `application/json`
   - **Request Body**: Multiple JSON objects, one per line
   - **Response**: Batch processing result with counts

4. **`GET /ingest/status/{message_id}`** ⚠️ Partial
   - Returns placeholder response
   - **TODO**: Requires message_status table implementation

5. **`GET /stats`** ✅
   - Returns basic stats (currently placeholder)
   ```json
   {
     "total_messages_processed": 0,
     "uptime_seconds": 0
   }
   ```

6. **`GET /swagger-ui`** ✅
   - HTTP 303 redirect to Swagger UI
   - OpenAPI spec available at `/api-docs/openapi.json`

### Missing Endpoints

❌ **`/ready`** - Not implemented (returns 404)
- **TODO**: Add readiness probe endpoint separate from health check

## NATS JetStream Endpoints

### Monitoring Endpoints (port 8222)

1. **`GET /varz`** ✅ - Server variables and stats
2. **`GET /healthz`** ✅ - Health check
   ```json
   {"status": "ok"}
   ```
3. **`GET /jsz`** ✅ - JetStream stats
   ```json
   {
     "streams": 1,
     "consumers": 1,
     "messages": 6,
     "bytes": 2727
   }
   ```

### Configuration

- **Stream**: `MESSAGES`
- **Subjects**: `messages.>`
- **Consumer**: `workers` (durable, pull-based)
- **Storage**: File-based (persisted in `/tmp/nats/jetstream`)
- **Max Age**: 24h
- **Max Bytes**: 1GB

## Worker Configuration

### Environment Variables

```bash
DATABASE_URL=postgresql://nomnom:nomnom@postgres:5432/nomnom
NATS_URL=nats://nats:4222
NATS_STREAM=MESSAGES
NATS_CONSUMER=workers  # (auto-created if not exists)
RUST_LOG=info
```

### Hardcoded Configuration (in generated code)

```rust
max_messages: 10         // Batch size per fetch
max_deliver: 3           // Max delivery attempts before DLQ
ack_policy: Explicit     // Manual ACK/NAK
filter_subject: "messages.ingest.>"
```

### Behavior

- ✅ Auto-creates database tables on startup
- ✅ Consumes messages from NATS JetStream
- ✅ Parses JSON message bodies
- ✅ Inserts into PostgreSQL with conflict handling
- ✅ ACKs successful processing
- ✅ NAKs failed processing (for retry)
- ❌ No HTTP endpoints (pure consumer)
- ❌ No health checks (not a HTTP service)

## Database Schema

### Tables Created

**`orders`** (auto-created by worker on startup)

```sql
CREATE TABLE IF NOT EXISTS orders (
    order_key      VARCHAR(255) NOT NULL PRIMARY KEY,
    customer_key   VARCHAR(255) NOT NULL,
    order_status   VARCHAR(1)   NOT NULL,
    total_price    DOUBLE PRECISION NOT NULL,
    order_date     VARCHAR(255) NOT NULL,
    order_priority VARCHAR(255),
    clerk          VARCHAR(255),
    ship_priority  INTEGER,
    comment        TEXT
);
```

### Connection Details

```
Host: postgres-service (in K8s) or postgres (in Docker Compose)
Port: 5432
Database: nomnom
User: nomnom
Password: nomnom (dev only)
```

## Message Flow

### Successful Processing

```
1. Client → POST /ingest/message
2. API validates JSON format ✓
3. API wraps in MessageEnvelope with UUID ✓
4. API publishes to NATS subject "messages.ingest.<entity>" ✓
5. API returns 202 Accepted with message_id ✓
6. Worker fetches batch from NATS ✓
7. Worker deserializes envelope ✓
8. Worker parses message body (entity-specific) ✓
9. Worker validates all required fields ✓
10. Worker inserts into PostgreSQL ✓
11. Worker ACKs message to NATS ✓
```

### Failed Processing

```
1-6. Same as successful flow
7. Worker encounters error (parse/validation/DB)
8. Worker logs error ✓
9. Worker NAKs message to NATS ✓
10. NATS redelivers message (up to max_deliver times) ✓
11. After max_deliver, message goes to... TODO: DLQ not implemented
```

## Differences from Original Plan

### ✅ What Matches the Plan

- NATS JetStream architecture
- Async processing with workers
- Auto-scaling approach (KEDA ready)
- StatefulSet for NATS (needs PVC)
- StatefulSet for PostgreSQL
- Deployment for API (stateless)
- Deployment for Workers (stateless)

### 🔧 What's Different

| Plan Says | Actually Is | Impact on K8s |
|-----------|-------------|---------------|
| `/api/ingest/message` | `/ingest/message` | ⚠️ Update Ingress paths |
| Pipe-delimited messages | JSON messages | ⚠️ Update docs/examples |
| Worker has `/health`, `/ready` | Worker has no HTTP server | ⚠️ Remove health checks from worker deployment |
| NATS healthcheck `/healthz` | Healthcheck unreliable | ⚠️ Use `service_started` not `service_healthy` |
| Schema via initdb | Schema via worker auto-create | ✅ Simpler deployment |
| Separate `/ready` endpoint | No `/ready` (only `/health`) | ⚠️ Add `/ready` or use `/health` for both probes |

### ❌ Not Yet Implemented

| Feature | Status | Priority |
|---------|--------|----------|
| `/ready` endpoint | Missing | High (needed for readiness probe) |
| Message status tracking | Placeholder only | Medium |
| Dead letter queue | Not implemented | Medium |
| Metrics endpoint | Placeholder only | Low |
| Batch size configuration | Hardcoded (10) | Low |
| Consumer name configuration | Hardcoded ("workers") | Low |

## Environment Variables Needed for K8s

### Ingestion API

```yaml
env:
  - name: DATABASE_URL
    valueFrom:
      secretKeyRef:
        name: postgres-secret
        key: connection-string
  - name: NATS_URL
    value: "nats://nats-service:4222"
  - name: NATS_STREAM
    value: "MESSAGES"
  - name: RUST_LOG
    value: "info"
```

### Worker

```yaml
env:
  - name: DATABASE_URL
    valueFrom:
      secretKeyRef:
        name: postgres-secret
        key: connection-string
  - name: NATS_URL
    value: "nats://nats-service:4222"
  - name: NATS_STREAM
    value: "MESSAGES"
  - name: NATS_CONSUMER  # Optional - defaults to "workers"
    value: "workers"
  - name: RUST_LOG
    value: "info"
```

## Health Check Configuration

### Ingestion API

```yaml
livenessProbe:
  httpGet:
    path: /health
    port: 8080
  initialDelaySeconds: 10
  periodSeconds: 10
  timeoutSeconds: 2
  failureThreshold: 3

readinessProbe:
  httpGet:
    path: /health  # TODO: change to /ready when implemented
    port: 8080
  initialDelaySeconds: 5
  periodSeconds: 5
  timeoutSeconds: 2
  failureThreshold: 2
```

### Worker

```yaml
# NO health checks - worker is not an HTTP service
# Kubernetes will monitor the process itself (restartPolicy: Always)
# Consider adding a sidecar for health monitoring if needed
```

### NATS

```yaml
# NO healthcheck in container
# Use service_started dependency instead of service_healthy
# NATS monitoring port 8222 available but not suitable for K8s probes
```

### PostgreSQL

```yaml
livenessProbe:
  exec:
    command:
      - pg_isready
      - -U
      - nomnom
  initialDelaySeconds: 30
  periodSeconds: 10

readinessProbe:
  exec:
    command:
      - pg_isready
      - -U
      - nomnom
  initialDelaySeconds: 5
  periodSeconds: 5
```

## KEDA Configuration

### Worker Autoscaling

```yaml
apiVersion: keda.sh/v1alpha1
kind: ScaledObject
metadata:
  name: ingestion-worker-scaler
spec:
  scaleTargetRef:
    name: ingestion-worker
  minReplicaCount: 1
  maxReplicaCount: 20
  cooldownPeriod: 300
  pollingInterval: 15
  triggers:
    - type: nats-jetstream
      metadata:
        natsServerMonitoringEndpoint: "nats-service.nomnom.svc.cluster.local:8222"
        account: "$G"
        stream: "MESSAGES"
        consumer: "workers"
        lagThreshold: "100"  # Scale up if lag > 100 messages per pod
        activationLagThreshold: "10"  # Wake from zero if lag > 10
```

## Testing Checklist

- [x] Ingestion API `/health` returns 200
- [x] Ingestion API accepts JSON messages
- [x] Messages published to NATS JetStream
- [x] Worker consumes messages from NATS
- [x] Worker creates database tables automatically
- [x] Worker inserts data into PostgreSQL
- [x] Worker ACKs successful processing
- [x] NATS monitoring endpoint `/varz` works
- [x] NATS health endpoint `/healthz` works
- [x] OpenAPI spec available at `/api-docs/openapi.json`
- [x] Swagger UI accessible (redirects from `/swagger-ui`)
- [ ] `/ready` endpoint (not yet implemented)
- [ ] Message status tracking (placeholder only)
- [ ] Worker NAK on failure triggers redelivery
- [ ] Dead letter queue for max retries

## Recommendations for K8s Deployment

### High Priority

1. **Add `/ready` endpoint** to ingestion API for proper readiness probes
2. **Update all Ingress paths** to remove `/api` prefix (actual: `/ingest/*`)
3. **Document JSON message format** (not pipe-delimited as in plan)
4. **Worker deployment**: Don't add health checks (no HTTP server)
5. **NATS dependencies**: Use `condition: service_started` (not `service_healthy`)

### Medium Priority

6. **Implement message status tracking** (requires new table + API updates)
7. **Add dead letter queue** for failed messages (after max retries)
8. **Make batch size configurable** via environment variable
9. **Add metrics endpoint** for Prometheus scraping
10. **Implement `/stats` with real data** (currently placeholder)

### Low Priority

11. Add tracing/correlation IDs across the pipeline
12. Implement consumer name configuration
13. Add Grafana dashboards for NATS metrics
14. Document scaling behavior under load

## Next Steps

1. ✅ Verify current setup (COMPLETE)
2. ⏭️  Update `PLAN_KUBERNETES_DEPLOYMENT.md` with corrections
3. ⏭️  Implement missing features (starting with `/ready` endpoint)
4. ⏭️  Generate Helm charts with verified configuration
5. ⏭️  Test in kind cluster
6. ⏭️  Load testing and performance tuning
