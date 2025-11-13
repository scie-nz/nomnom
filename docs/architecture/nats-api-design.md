# Generic NATS API Binary

## Overview

Created a **generic, reusable NATS API binary** that is **NOT codegen'd**. This binary can be used across all nomnom projects without modification.

## Location

`src/bin/nats-api.rs`

## Architecture

```
┌─────────────────────────────────────────────────────────┐
│  Generic NATS API Binary (NOT codegen'd)                │
│  - One binary for all projects                          │
│  - No entity knowledge                                   │
│  - Just wraps JSON and publishes to NATS               │
└──────────────┬──────────────────────────────────────────┘
               │ Publishes MessageEnvelope
               ▼
┌─────────────────────────────────────────────────────────┐
│  NATS JetStream                                          │
│  - Stream: MESSAGES                                      │
│  - Durable storage                                       │
└──────────────┬──────────────────────────────────────────┘
               │ Consumed by
               ▼
┌─────────────────────────────────────────────────────────┐
│  Worker Binary (IS codegen'd, entity-specific)          │
│  - Parses message body                                   │
│  - Writes to database                                    │
│  - TODO: Implement worker codegen                        │
└─────────────────────────────────────────────────────────┘
```

## What It Does

1. **Accepts HTTP POST** at `/ingest/message` and `/ingest/batch`
2. **Validates JSON** (basic validation only, no entity knowledge)
3. **Wraps in MessageEnvelope** with UUID, timestamp, metadata
4. **Publishes to NATS JetStream**
5. **Returns 202 Accepted** with `message_id` for tracking

## Endpoints

### POST /ingest/message
Ingest a single message.

**Request:**
```bash
curl -X POST http://localhost:8080/ingest/message \
  -H "Content-Type: application/json" \
  -d '{"order_id": "123", "total": 100.50}'
```

**Response (202 Accepted):**
```json
{
  "message_id": "550e8400-e29b-41d4-a716-446655440000",
  "status": "accepted",
  "timestamp": "2025-11-09T21:27:00Z"
}
```

### POST /ingest/batch
Ingest multiple messages (newline-delimited JSON).

**Request:**
```bash
curl -X POST http://localhost:8080/ingest/batch \
  -H "Content-Type: text/plain" \
  -d $'{"order_id": "1", "total": 10}\n{"order_id": "2", "total": 20}'
```

**Response (202 Accepted):**
```json
{
  "status": "success",
  "processed": 2,
  "inserted": 2,
  "failed": 0,
  "errors": [],
  "duration_ms": 12
}
```

### GET /ingest/status/:message_id
Check processing status (placeholder, requires worker implementation).

**Request:**
```bash
curl http://localhost:8080/ingest/status/550e8400-e29b-41d4-a716-446655440000
```

**Response:**
```json
{
  "message_id": "550e8400-e29b-41d4-a716-446655440000",
  "status": "accepted",
  "message": "Status tracking requires message_status table (to be implemented by worker)"
}
```

### GET /health
Health check endpoint.

**Response:**
```json
{
  "status": "healthy",
  "service": "nats-api",
  "version": "0.1.0"
}
```

## Configuration

Environment variables:

```bash
# NATS connection
NATS_URL=nats://localhost:4222
NATS_STREAM=MESSAGES

# Server
PORT=8080
```

## Building and Running

### Build the binary:
```bash
cargo build --bin nats-api --release
```

### Run locally:
```bash
export NATS_URL=nats://localhost:4222
export NATS_STREAM=MESSAGES
export PORT=8080

./target/release/nats-api
```

### Run with Docker:
```dockerfile
FROM rust:alpine as builder
WORKDIR /build
COPY . .
RUN cargo build --bin nats-api --release

FROM alpine:3.19
COPY --from=builder /build/target/release/nats-api /app/
CMD ["/app/nats-api"]
```

## Supporting Modules

### src/nats/mod.rs
Main NATS module

### src/nats/message_envelope.rs
```rust
pub struct MessageEnvelope {
    pub message_id: Uuid,
    pub body: String,
    pub entity_type: Option<String>,
    pub received_at: DateTime<Utc>,
    pub retry_count: u32,
    pub source: Option<String>,
}
```

### src/nats/client.rs
```rust
pub struct NatsClient {
    // Connects to NATS JetStream
    // Publishes MessageEnvelope
    // Waits for ACK
}
```

## Comparison to Old Architecture

### ❌ OLD: `src/codegen/ingestion_server`
- Generated per project
- Entity-specific parsing logic
- Direct database writes
- Synchronous processing
- Tightly coupled

### ✅ NEW: `src/bin/nats-api.rs`
- Single binary for all projects
- No entity knowledge
- Publishes to NATS
- Async processing
- Decoupled

## What Remains

### 1. Worker Binary Codegen
Create `src/codegen/worker/` that generates:
- NATS consumer loop
- Entity-specific parsers (reuse from old ingestion_server)
- Entity-specific models (reuse from old ingestion_server)
- Entity-specific database writes (reuse from old ingestion_server)

### 2. Docker Images
- **nats-api**: Ship as generic image `ghcr.io/yourorg/nomnom-nats-api:v1.0`
- **worker**: Build per project from generated code

### 3. Kubernetes Deployment
- Deploy nats-api as Deployment (same image everywhere)
- Deploy workers with KEDA autoscaling
- NATS JetStream as StatefulSet

## Testing

To test the nats-api binary:

```bash
# Start NATS locally
docker run -p 4222:4222 -p 8222:8222 nats:latest -js -m 8222

# Run nats-api
cargo run --bin nats-api

# In another terminal, ingest a message
curl -X POST http://localhost:8080/ingest/message \
  -H "Content-Type: application/json" \
  -d '{"test": "message"}'

# Check NATS monitoring
curl http://localhost:8222/jsz
```

## Benefits

✅ **Reusable**: One binary for all nomnom projects
✅ **Simple**: ~200 lines of code, easy to maintain
✅ **Generic**: No entity-specific logic
✅ **Fast**: Lightweight HTTP → NATS publisher
✅ **Scalable**: Horizontal scaling, no state
✅ **Observable**: NATS monitoring built-in
✅ **Testable**: cURL works, no complex setup

## Files Created

1. `src/nats/mod.rs` - NATS module
2. `src/nats/message_envelope.rs` - Message envelope types
3. `src/nats/client.rs` - NATS client
4. `src/bin/nats-api.rs` - Generic API binary
5. Updated `Cargo.toml` - Added dependencies and nats-api binary
6. Updated `src/lib.rs` - Exported NATS types

## Next Steps

1. **Create worker codegen** (`src/codegen/worker/`)
2. **Deprecate `src/codegen/ingestion_server`** (or keep for sync mode users)
3. **Test end-to-end** (nats-api → NATS → worker → database)
4. **Create Docker images** and Helm charts
5. **Document deployment** strategy
