# NATS Architecture Components Clarification

## Component Breakdown

### Current Parser Binary (Before NATS)
```rust
// Generated from entities YAML
HTTP Server (Axum)           // Generic, reusable
    ↓
Parser Logic                 // Entity-specific, codegen'd
    ↓
Database Writer              // Entity-specific, codegen'd
```

## New Architecture with NATS

### Component 1: Parser Binary API (Generic - NOT Codegen'd)

**Purpose**: Lightweight HTTP ingestion endpoint that publishes to NATS

**Responsibility**:
- Accept HTTP POST requests
- Wrap body in MessageEnvelope (add UUID, timestamp)
- Publish to NATS JetStream
- Return 202 Accepted + message_id
- Provide status lookup endpoint

**Implementation**: Single reusable Rust binary (lives in `src/bin/nats-api.rs`)

```rust
// src/bin/nats-api.rs
// This is NOT generated - it's a generic component

use axum::{Router, routing::{post, get}, extract::State};
use async_nats::jetstream;

#[derive(Clone)]
struct AppState {
    nats: NatsClient,
    db_pool: DbPool,  // Only for status queries
}

#[tokio::main]
async fn main() {
    let nats = NatsClient::connect(NatsConfig::from_env()).await.unwrap();
    let db_pool = create_db_pool(&env::var("DATABASE_URL").unwrap());

    let app = Router::new()
        .route("/api/ingest/message", post(ingest_message))
        .route("/api/ingest/status/:id", get(check_status))
        .route("/health", get(health_check))
        .with_state(Arc::new(AppState { nats, db_pool }));

    // ... start server
}

async fn ingest_message(
    State(state): State<Arc<AppState>>,
    body: String,
) -> Result<Json<IngestionResponse>, AppError> {
    let envelope = MessageEnvelope {
        message_id: Uuid::new_v4(),
        body,
        received_at: Utc::now(),
        retry_count: 0,
    };

    // Publish to NATS (generic, no entity knowledge)
    state.nats.publish_message(&envelope).await?;

    Ok(Json(IngestionResponse {
        message_id: envelope.message_id.to_string(),
        status: IngestionStatus::Accepted,
        timestamp: envelope.received_at,
    }))
}
```

**Characteristics**:
- ✅ Generic (no entity-specific logic)
- ✅ Reusable across all nomnom projects
- ✅ Single binary, not generated
- ✅ Lightweight (~16MB Alpine image)
- ❌ Does NOT need codegen

---

### Component 2: Worker Binary (Entity-Specific - IS Codegen'd)

**Purpose**: Consume messages from NATS and persist to database

**Responsibility**:
- Subscribe to NATS JetStream stream
- Consume MessageEnvelope from queue
- **Parse message body** (entity-specific!)
- **Validate** (entity-specific!)
- **Write to database** (entity-specific!)
- ACK/NAK message

**Implementation**: Generated from entities YAML (reuse existing parser logic)

```rust
// worker/src/main.rs
// THIS IS GENERATED from entities YAML

use async_nats::jetstream;
mod parsers;  // GENERATED: entity-specific parsing
mod models;   // GENERATED: entity-specific models
mod database; // GENERATED: entity-specific DB writes
mod schema;   // GENERATED: Diesel schema

#[tokio::main]
async fn main() {
    let nats = connect_nats().await.unwrap();
    let db_pool = create_db_pool().unwrap();

    let jetstream = jetstream::new(nats);
    let consumer = jetstream
        .get_stream("MESSAGES")
        .await?
        .get_consumer("workers")
        .await?;

    let mut messages = consumer.messages().await?;

    while let Some(msg) = messages.next().await {
        match process_message(msg, &db_pool).await {
            Ok(_) => msg.ack().await?,
            Err(e) => msg.nak().await?,
        }
    }
}

async fn process_message(
    msg: jetstream::Message,
    pool: &DbPool,
) -> Result<()> {
    // Deserialize envelope (generic)
    let envelope: MessageEnvelope = serde_json::from_slice(&msg.payload)?;

    // ENTITY-SPECIFIC: Parse message body
    // This comes from generated parsers.rs
    let parsed = parsers::parse_message(&envelope.body)?;

    // ENTITY-SPECIFIC: Write to database
    // This comes from generated database.rs and models.rs
    let mut conn = pool.get()?;
    conn.transaction(|conn| {
        match parsed {
            ParsedMessage::Order(order) => {
                diesel::insert_into(schema::orders::table)
                    .values(&order)
                    .execute(conn)?;
            }
            ParsedMessage::OrderLineItem(item) => {
                diesel::insert_into(schema::order_line_items::table)
                    .values(&item)
                    .execute(conn)?;
            }
        }

        // Update message status table
        // ...

        Ok(())
    })
}
```

**Characteristics**:
- ❌ Entity-specific (has parsing logic for Orders, LineItems, etc.)
- ✅ Reuses existing codegen (parsers, models, database)
- ✅ Generated per project
- ✅ MUST be codegen'd

---

## Code Generation Strategy

### What to Generate

1. **Worker Binary**: YES, generate from entities
   - `worker/src/main.rs` - NATS consumer loop (template)
   - `worker/src/parsers.rs` - Entity parsing (REUSE from current parser)
   - `worker/src/models.rs` - Entity models (REUSE from current parser)
   - `worker/src/database.rs` - DB writes (REUSE from current parser)
   - `worker/src/schema.rs` - Diesel schema (REUSE from current parser)
   - `worker/Cargo.toml` - Dependencies (generate)

2. **Parser API Binary**: NO, ship as generic binary
   - Lives in `src/bin/nats-api.rs`
   - Compiled once, reused everywhere
   - Configured via environment variables
   - No entity knowledge

### Generator Changes

```rust
// src/cli.rs

impl GenerateIngestionCmd {
    pub fn run(&self) -> Result<()> {
        let entities = load_entities(&self.entities)?;

        if self.enable_nats {
            // Option 1: Generate worker from existing parser
            codegen::generate_nats_worker(
                &entities,
                &self.output.join("worker"),
            )?;

            // Option 2: Copy generic NATS API binary
            // (or document that user should use nomnom/nats-api binary)
            println!("
                NATS API Binary:
                  Use the generic nats-api binary from nomnom:
                  docker pull ghcr.io/yourorg/nomnom-nats-api:latest

                  Or build from source:
                  cargo build --bin nats-api --release
            ");
        } else {
            // Generate traditional synchronous parser
            codegen::generate_parser_binary(&entities, &self.output)?;
        }

        Ok(())
    }
}
```

### File Structure After Generation

```
project/
├── worker/                    # GENERATED (entity-specific)
│   ├── Cargo.toml
│   └── src/
│       ├── main.rs            # NATS consumer + message processing
│       ├── parsers.rs         # REUSED from current parser codegen
│       ├── models.rs          # REUSED from current parser codegen
│       ├── database.rs        # REUSED from current parser codegen
│       └── schema.rs          # REUSED from current parser codegen
│
└── docker-compose.nats.yml    # GENERATED

# NATS API lives in nomnom repo (not generated):
# nomnom/src/bin/nats-api.rs
```

## Deployment Architecture

```
┌─────────────────────────────────────────────────────────┐
│  NATS API Pod (Generic Binary)                          │
│  - ghcr.io/yourorg/nomnom-nats-api:latest              │
│  - Same image for all projects                          │
│  - Configured via env vars                              │
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
│  Worker Pods (Project-Specific, Generated)              │
│  - Built from generated worker/ directory                │
│  - Contains entity-specific parsing logic                │
│  - Scales with KEDA based on queue depth                 │
└──────────────┬──────────────────────────────────────────┘
               │ Writes to
               ▼
┌─────────────────────────────────────────────────────────┐
│  PostgreSQL                                              │
└─────────────────────────────────────────────────────────┘
```

## Benefits of This Approach

1. **Separation of Concerns**:
   - API: Generic message queueing (reusable)
   - Worker: Entity-specific processing (generated)

2. **Code Reuse**:
   - Worker reuses ALL existing parser codegen
   - No duplication of parsing logic

3. **Simplicity**:
   - NATS API is dead simple, doesn't need generation
   - Worker is just "parser without HTTP server"

4. **Testing**:
   - Can test NATS API independently (curl works!)
   - Can test worker with mock NATS messages

5. **Deployment Flexibility**:
   - Deploy same NATS API image everywhere
   - Each project has its own worker image

## Docker Images

### NATS API (Generic, One Image for All)
```dockerfile
# Built once in nomnom repo
FROM rust:alpine as builder
WORKDIR /build
COPY . .
RUN cargo build --bin nats-api --release

FROM alpine:3.19
COPY --from=builder /build/target/release/nats-api /app/
USER appuser:appuser
EXPOSE 8080
CMD ["/app/nats-api"]
```

Tag: `ghcr.io/yourorg/nomnom-nats-api:v1.0`
Size: ~16MB

### Worker (Project-Specific, Generated per Project)
```dockerfile
# Built in each generated project
FROM rust:alpine as builder
WORKDIR /build
COPY worker/ .
RUN cargo build --release

FROM alpine:3.19
COPY --from=builder /build/target/release/worker /app/
USER appuser:appuser
CMD ["/app/worker"]
```

Tag: `myproject/tpch-worker:latest`
Size: ~24MB

## Helm Chart Structure

```yaml
# values.yaml
ingestion:
  api:
    image:
      # Generic NATS API (same for all)
      repository: ghcr.io/yourorg/nomnom-nats-api
      tag: v1.0

  worker:
    image:
      # Project-specific worker (generated)
      repository: myproject/tpch-worker
      tag: latest
```

## Summary

| Component | Codegen? | Entity-Specific? | Reusable? | Source |
|-----------|----------|------------------|-----------|---------|
| NATS API | ❌ No | ❌ No | ✅ Yes | `nomnom/src/bin/nats-api.rs` |
| Worker | ✅ Yes | ✅ Yes | ❌ No | Generated from entities |

**Key Insight**: The NATS API is so simple and generic that it doesn't need codegen. The Worker is just the current parser binary with NATS consumer instead of HTTP server.
