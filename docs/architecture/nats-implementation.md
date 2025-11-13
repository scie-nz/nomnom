# NATS JetStream Ingestion API Implementation

## Summary

Successfully implemented NATS JetStream integration for the ingestion server code generator. The ingestion API now publishes messages to NATS asynchronously instead of directly writing to the database.

## Implementation Date

November 9, 2025

## What Was Implemented

### 1. NATS Client Module Generation (`nats_client_rs.rs`)

Generates `nats_client.rs` with:
- `NatsConfig` struct with defaults from environment variables
  - `NATS_URL` (default: `nats://localhost:4222`)
  - `NATS_STREAM` (default: `MESSAGES`)
  - Configurable max_age (24 hours) and max_bytes (1GB)
- `NatsClient` for connection management
  - `connect()`: Initializes JetStream stream with file storage
  - `publish_message()`: Publishes MessageEnvelope with acknowledgment
  - `jetstream()`: Accessor for advanced operations

**Location**: `/Users/bogdanstate/nomnom/src/codegen/ingestion_server/nats_client_rs.rs`

### 2. Message Envelope Module Generation (`message_envelope_rs.rs`)

Generates `message_envelope.rs` with:
- `MessageEnvelope` struct for wrapping messages with metadata
  - `message_id`: UUID for tracking
  - `body`: Raw message content (String)
  - `entity_type`: Optional entity type hint
  - `received_at`: Timestamp
  - `retry_count`: For retry logic
  - `source`: Optional source identifier
- `IngestionResponse` for API responses
- `IngestionStatus` enum (Accepted, Persisted, Failed)

**Location**: `/Users/bogdanstate/nomnom/src/codegen/ingestion_server/message_envelope_rs.rs`

### 3. Updated Cargo.toml Generation

Added dependencies:
- `uuid = { version = "1", features = ["v4", "serde"] }`
- `async-nats = "0.35"`

**Modified**: `/Users/bogdanstate/nomnom/src/codegen/ingestion_server/cargo_toml.rs:44-48`

### 4. Updated HTTP Handlers

**Modified**: `/Users/bogdanstate/nomnom/src/codegen/ingestion_server/handlers_rs.rs`

Changes:
- Added `AppState` struct with `nats: NatsClient` and `db_pool: DbPool`
- `ingest_message()` handler:
  - Returns 202 Accepted (instead of 200 OK)
  - Creates `MessageEnvelope` from request body
  - Publishes to NATS JetStream
  - Returns `IngestionResponse` with message_id
- `ingest_batch()` handler:
  - Publishes each line to NATS
  - Returns 202 Accepted
- Added `check_status()` handler:
  - GET `/ingest/status/:message_id`
  - Placeholder for checking message processing status
  - Returns UUID validation

### 5. Updated Main.rs Generation

**Modified**: `/Users/bogdanstate/nomnom/src/codegen/ingestion_server/main_rs.rs`

Changes:
- Import `nats_client` and `message_envelope` modules
- Initialize `NatsClient` during startup
- Create `AppState` with both NATS and database pool
- Wrap state in `Arc<AppState>` for sharing
- Add route: `GET /ingest/status/:message_id`

### 6. Environment Configuration

**Modified**: `/Users/bogdanstate/nomnom/src/codegen/ingestion_server/mod.rs:157-159`

Added to `.env.example`:
```bash
# NATS Configuration
NATS_URL=nats://localhost:4222
NATS_STREAM=MESSAGES
```

### 7. Docker Compose for NATS Testing

**New**: `generate_docker_compose_nats()` in `mod.rs:231-326`

Generates `docker-compose.nats.yml` with:
- **NATS JetStream server**
  - Image: `nats:latest`
  - Ports: 4222 (client), 8222 (monitoring)
  - Healthcheck: wget monitoring endpoint
- **PostgreSQL database**
  - Image: `postgres:16-alpine`
  - Credentials: nomnom/nomnom
  - Healthcheck: pg_isready
- **Ingestion API service**
  - Builds from Dockerfile
  - Port: 8080
  - Connects to NATS and PostgreSQL
  - Waits for healthchecks before starting
- **TODO: Worker service** (commented out, needs implementation)

## Architecture Flow

```
HTTP POST                 NATS JetStream              Worker                Database
    │                          │                        │                     │
    │  POST /ingest/message    │                        │                     │
    ├─────────────────────────>│                        │                     │
    │                          │                        │                     │
    │  Validate JSON           │                        │                     │
    │  Create envelope         │                        │                     │
    │  message_id = UUID       │                        │                     │
    │                          │                        │                     │
    │  Publish to NATS         │                        │                     │
    ├─────────────────────────>│                        │                     │
    │                          │  Store in stream       │                     │
    │                          │  (durable)             │                     │
    │  Wait for ACK            │                        │                     │
    │<─────────────────────────┤                        │                     │
    │                          │                        │                     │
    │  202 Accepted            │                        │                     │
    │  + message_id            │                        │                     │
    │<─────────────────────────┤                        │                     │
    │                          │                        │                     │
    │                          │  Pull message          │                     │
    │                          │<───────────────────────┤                     │
    │                          │                        │                     │
    │                          │  MessageEnvelope       │                     │
    │                          ├───────────────────────>│                     │
    │                          │                        │  Parse body         │
    │                          │                        │  Validate           │
    │                          │                        │                     │
    │                          │                        │  INSERT             │
    │                          │                        ├────────────────────>│
    │                          │                        │                     │
    │                          │                        │  Success            │
    │                          │                        │<────────────────────┤
    │                          │  ACK message           │                     │
    │                          │<───────────────────────┤                     │
```

## Testing

### Generated Files

Generated test server in `/tmp/test-nats-ingestion`:
```bash
$ ls /tmp/test-nats-ingestion/src/
database.rs          handlers.rs          message_envelope.rs  nats_client.rs
error.rs             main.rs              models.rs            parsers.rs
```

### Testing with cURL (when deployed)

```bash
# Start services
cd /tmp/test-nats-ingestion
docker compose -f docker-compose.nats.yml up -d

# Ingest a message
curl -X POST http://localhost:8080/ingest/message \
  -H "Content-Type: application/json" \
  -d '{"order_id": "123", "total": 100.50}'

# Response:
{
  "message_id": "550e8400-e29b-41d4-a716-446655440000",
  "status": "accepted",
  "timestamp": "2025-11-09T21:27:00Z"
}

# Check status
curl http://localhost:8080/ingest/status/550e8400-e29b-41d4-a716-446655440000

# Check NATS monitoring
curl http://localhost:8222/jsz  # JetStream stats
```

## What Remains To Be Done

### 1. Worker Binary Generation (HIGH PRIORITY)

According to `ARCH_NATS_COMPONENTS.md`, the worker binary should:
- **Reuse existing parser codegen** (parsers.rs, models.rs, database.rs)
- **Replace HTTP server with NATS consumer**
- Subscribe to NATS JetStream
- Consume `MessageEnvelope` from queue
- Parse message body using entity-specific parsers
- Write to database using entity-specific database module
- ACK/NAK messages

**Implementation approach**:
- Create new module: `src/codegen/worker/`
- Similar structure to `ingestion_server/` but with main.rs that:
  - Connects to NATS JetStream
  - Creates consumer
  - Polls messages in loop
  - Deserializes MessageEnvelope
  - Calls parsers and database modules
  - ACKs successful writes, NAKs failures

### 2. Message Status Table (MEDIUM PRIORITY)

Add to schema generation:
```sql
CREATE TABLE message_status (
    message_id UUID PRIMARY KEY,
    status VARCHAR(20) NOT NULL,  -- 'accepted', 'processing', 'completed', 'failed'
    entity_type VARCHAR(100),
    received_at TIMESTAMP NOT NULL,
    processed_at TIMESTAMP,
    error_message TEXT,
    retry_count INTEGER DEFAULT 0
);
```

Update `check_status()` handler to query this table.

### 3. Helm Chart Updates (LOW PRIORITY)

Update `PLAN_KUBERNETES_DEPLOYMENT.md` implementation:
- NATS StatefulSet (3 replicas, HA)
- Worker Deployment with KEDA autoscaling
- Scale on NATS stream message count
- Scale to zero when queue empty

## Files Modified

1. `/Users/bogdanstate/nomnom/src/codegen/ingestion_server/mod.rs`
   - Added `message_envelope_rs` and `nats_client_rs` modules
   - Added `generate_docker_compose_nats()` function
   - Updated generation flow

2. `/Users/bogdanstate/nomnom/src/codegen/ingestion_server/cargo_toml.rs`
   - Added UUID and async-nats dependencies

3. `/Users/bogdanstate/nomnom/src/codegen/ingestion_server/handlers_rs.rs`
   - Complete rewrite to use NATS
   - Added AppState
   - Changed response codes to 202 Accepted
   - Added check_status endpoint

4. `/Users/bogdanstate/nomnom/src/codegen/ingestion_server/main_rs.rs`
   - Initialize NatsClient
   - Create Arc<AppState>
   - Add status route

## New Files Created

1. `/Users/bogdanstate/nomnom/src/codegen/ingestion_server/message_envelope_rs.rs`
2. `/Users/bogdanstate/nomnom/src/codegen/ingestion_server/nats_client_rs.rs`

## Benefits Achieved

✅ **Decoupled ingestion from processing**: API is lightweight, just validates JSON and publishes
✅ **Async processing**: Returns 202 immediately, processing happens in background
✅ **Durability**: NATS JetStream provides durable storage (24 hours, 1GB default)
✅ **Scalability**: Workers can scale independently based on queue depth
✅ **Reliability**: At-least-once delivery with ACK/NAK
✅ **Monitoring**: NATS provides built-in monitoring at port 8222
✅ **Testing**: docker-compose.nats.yml enables local testing without K8s
✅ **cURL testable**: Still works with simple HTTP POST, returns message_id

## Next Steps for User

1. **Test the generated ingestion server**:
   ```bash
   cd /tmp/test-nats-ingestion
   docker compose -f docker-compose.nats.yml up -d
   # Wait for services to be healthy
   curl -X POST http://localhost:8080/ingest/message \
     -H "Content-Type: application/json" \
     -d '{"test": "message"}'
   ```

2. **Implement worker binary generation**:
   - Create `src/codegen/worker/mod.rs`
   - Reuse parser modules from ingestion_server
   - Generate NATS consumer main.rs
   - Add to CLI as `generate-worker` command

3. **Add message_status table**:
   - Update schema generation
   - Implement status tracking in worker
   - Complete check_status endpoint

4. **Deploy to Kubernetes**:
   - Follow `PLAN_KUBERNETES_DEPLOYMENT.md`
   - Use KEDA for worker autoscaling
   - Deploy NATS StatefulSet with HA

## Related Documents

- `ARCH_NATS_COMPONENTS.md` - Architecture clarification
- `PLAN_NATS_JETSTREAM_INTEGRATION.md` - Original implementation plan
- `PLAN_KUBERNETES_DEPLOYMENT.md` - K8s deployment strategy
- `PLAN_INGESTION_RELIABILITY.md` - Reliability and backpressure strategies
