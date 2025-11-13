# NATS JetStream Integration Plan for Parser Binary

## Overview

Integrate NATS JetStream into the parser binary codegen to create a reliable, scalable ingestion system with:
- Async message publishing (HTTP still accepts POST, but writes to NATS)
- Separate worker component that consumes from NATS and writes to DB
- Durable message persistence in JetStream
- Automatic retries and dead-letter queues
- Full testability via cURL in local Docker Compose

## Architecture

### Current Flow (Synchronous)
```
Client → HTTP POST → Parser Binary → PostgreSQL → HTTP 200
```

### New Flow (Async with NATS)
```
Client → HTTP POST → Parser Binary API → NATS JetStream → HTTP 202 Accepted
                                              ↓
                                    Worker Pods Subscribe
                                              ↓
                                        PostgreSQL
```

### Components

1. **Parser Binary API** (Existing, Modified)
   - HTTP server (Axum)
   - Publishes messages to NATS JetStream
   - Returns 202 Accepted + message_id
   - Provides status endpoint

2. **Worker Binary** (New)
   - Subscribes to NATS JetStream stream
   - Parses messages
   - Writes to PostgreSQL
   - ACKs/NAKs messages

3. **NATS JetStream** (Infrastructure)
   - Durable stream storage
   - Message ordering
   - At-least-once delivery
   - Consumer management

## Code Generation Changes

### 1. New Cargo Dependencies

Add to generated `Cargo.toml`:

```toml
[dependencies]
# Existing dependencies...
axum = "0.7"
tokio = { version = "1.0", features = ["full"] }
serde = { version = "1.0", features = ["derive"] }
serde_json = "1.0"
diesel = { version = "2.3", features = ["postgres", "r2d2", "chrono"] }

# NEW: NATS JetStream dependencies
async-nats = "0.35"  # Official async Rust NATS client
uuid = { version = "1.0", features = ["v4", "serde"] }
tracing = "0.1"
tracing-subscriber = "0.3"

[features]
default = ["nats"]
nats = ["async-nats"]  # Optional feature flag
```

### 2. Message Envelope Structure

Generate `src/message_envelope.rs`:

```rust
use serde::{Deserialize, Serialize};
use uuid::Uuid;
use chrono::{DateTime, Utc};

/// Envelope wraps raw message with metadata
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MessageEnvelope {
    /// Unique message ID for tracking
    pub message_id: Uuid,

    /// Raw message body (before parsing)
    pub body: String,

    /// Entity type hint (from URL path)
    pub entity_type: Option<String>,

    /// Timestamp when message was received
    pub received_at: DateTime<Utc>,

    /// Retry count
    #[serde(default)]
    pub retry_count: u32,

    /// Source IP or identifier
    pub source: Option<String>,
}

impl MessageEnvelope {
    pub fn new(body: String, entity_type: Option<String>) -> Self {
        Self {
            message_id: Uuid::new_v4(),
            body,
            entity_type,
            received_at: Utc::now(),
            retry_count: 0,
            source: None,
        }
    }
}

/// Response returned to client
#[derive(Debug, Serialize)]
pub struct IngestionResponse {
    pub message_id: String,
    pub status: IngestionStatus,
    pub timestamp: DateTime<Utc>,
}

#[derive(Debug, Serialize)]
#[serde(rename_all = "lowercase")]
pub enum IngestionStatus {
    Accepted,   // Queued in NATS
    Persisted,  // Written to DB (for sync mode)
    Failed,     // Validation or other error
}
```

### 3. NATS Client Configuration

Generate `src/nats_client.rs`:

```rust
use async_nats::jetstream;
use std::time::Duration;

#[derive(Clone)]
pub struct NatsConfig {
    pub url: String,
    pub stream_name: String,
    pub max_age: Duration,
    pub max_bytes: i64,
}

impl Default for NatsConfig {
    fn default() -> Self {
        Self {
            url: std::env::var("NATS_URL")
                .unwrap_or_else(|_| "nats://localhost:4222".to_string()),
            stream_name: "MESSAGES".to_string(),
            max_age: Duration::from_secs(24 * 60 * 60), // 24 hours
            max_bytes: 1024 * 1024 * 1024, // 1GB
        }
    }
}

pub struct NatsClient {
    client: async_nats::Client,
    jetstream: jetstream::Context,
    stream_name: String,
}

impl NatsClient {
    pub async fn connect(config: NatsConfig) -> Result<Self, async_nats::Error> {
        // Connect to NATS
        let client = async_nats::connect(&config.url).await?;
        tracing::info!("Connected to NATS at {}", config.url);

        // Get JetStream context
        let jetstream = jetstream::new(client.clone());

        // Create or get stream
        let _stream = jetstream
            .get_or_create_stream(jetstream::stream::Config {
                name: config.stream_name.clone(),
                subjects: vec!["messages.>".to_string()],
                max_age: config.max_age,
                max_bytes: config.max_bytes,
                storage: jetstream::stream::StorageType::File,
                num_replicas: 1,
                ..Default::default()
            })
            .await?;

        tracing::info!("JetStream stream '{}' ready", config.stream_name);

        Ok(Self {
            client,
            jetstream,
            stream_name: config.stream_name,
        })
    }

    pub async fn publish_message(
        &self,
        envelope: &MessageEnvelope,
    ) -> Result<(), Box<dyn std::error::Error>> {
        let subject = format!("messages.ingest.{}",
            envelope.entity_type.as_deref().unwrap_or("default"));

        let payload = serde_json::to_vec(envelope)?;

        // Publish with JetStream (durable, acknowledged)
        let ack = self.jetstream
            .publish(subject, payload.into())
            .await?;

        // Wait for acknowledgment
        ack.await?;

        tracing::debug!(
            "Published message {} to JetStream",
            envelope.message_id
        );

        Ok(())
    }

    pub fn jetstream(&self) -> &jetstream::Context {
        &self.jetstream
    }
}
```

### 4. Modified HTTP Handlers

Generate `src/handlers.rs` (modified):

```rust
use axum::{
    extract::{Path, State},
    http::StatusCode,
    response::Json,
};
use crate::{
    message_envelope::{MessageEnvelope, IngestionResponse, IngestionStatus},
    nats_client::NatsClient,
    parsers::parse_message,
    models::MessageStatus,
    database::DbPool,
    error::AppError,
};
use std::sync::Arc;
use uuid::Uuid;
use chrono::Utc;

pub struct AppState {
    pub nats: Option<Arc<NatsClient>>,
    pub db_pool: DbPool,
}

/// Ingest a message via NATS JetStream
pub async fn ingest_message(
    State(state): State<Arc<AppState>>,
    body: String,
) -> Result<(StatusCode, Json<IngestionResponse>), AppError> {
    let envelope = MessageEnvelope::new(body, None);
    let message_id = envelope.message_id;

    // If NATS is enabled, use async mode
    if let Some(nats) = &state.nats {
        // Publish to JetStream
        nats.publish_message(&envelope).await?;

        // Return 202 Accepted
        Ok((
            StatusCode::ACCEPTED,
            Json(IngestionResponse {
                message_id: message_id.to_string(),
                status: IngestionStatus::Accepted,
                timestamp: Utc::now(),
            }),
        ))
    } else {
        // Fallback: synchronous mode (direct to DB)
        let parsed = parse_message(&envelope.body)?;
        // Insert to DB...

        Ok((
            StatusCode::OK,
            Json(IngestionResponse {
                message_id: message_id.to_string(),
                status: IngestionStatus::Persisted,
                timestamp: Utc::now(),
            }),
        ))
    }
}

/// Check message status
pub async fn check_status(
    State(state): State<Arc<AppState>>,
    Path(message_id): Path<Uuid>,
) -> Result<Json<MessageStatus>, AppError> {
    use diesel::prelude::*;
    use crate::schema::message_status::dsl;

    let mut conn = state.db_pool.get()?;

    // Query status from database
    let status = dsl::message_status
        .filter(dsl::message_id.eq(message_id))
        .first::<MessageStatus>(&mut conn)
        .optional()?;

    match status {
        Some(s) => Ok(Json(s)),
        None => Ok(Json(MessageStatus {
            message_id,
            status: "unknown".to_string(),
            received_at: Utc::now(),
            processed_at: None,
            error: Some("Message not found".to_string()),
        })),
    }
}

/// Health check endpoint
pub async fn health_check() -> &'static str {
    "OK"
}
```

### 5. Worker Binary

Generate new `worker/src/main.rs`:

```rust
use async_nats::jetstream;
use diesel::prelude::*;
use diesel::r2d2::{self, ConnectionManager};
use futures::StreamExt;
use std::time::Duration;
use tracing::{info, error, warn};

mod message_envelope;
mod parsers;
mod models;
mod database;
mod schema;
mod error;

use message_envelope::MessageEnvelope;

type DbPool = r2d2::Pool<ConnectionManager<PgConnection>>;

#[derive(Clone)]
struct WorkerConfig {
    nats_url: String,
    database_url: String,
    stream_name: String,
    consumer_name: String,
    batch_size: usize,
}

impl Default for WorkerConfig {
    fn default() -> Self {
        Self {
            nats_url: std::env::var("NATS_URL")
                .unwrap_or_else(|_| "nats://localhost:4222".to_string()),
            database_url: std::env::var("DATABASE_URL")
                .expect("DATABASE_URL must be set"),
            stream_name: "MESSAGES".to_string(),
            consumer_name: "workers".to_string(),
            batch_size: 10,
        }
    }
}

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    // Initialize tracing
    tracing_subscriber::fmt()
        .with_env_filter(
            std::env::var("RUST_LOG").unwrap_or_else(|_| "info".to_string())
        )
        .init();

    let config = WorkerConfig::default();

    // Connect to database
    let db_pool = database::create_pool(&config.database_url)?;
    info!("Connected to database");

    // Connect to NATS
    let client = async_nats::connect(&config.nats_url).await?;
    info!("Connected to NATS at {}", config.nats_url);

    let jetstream = jetstream::new(client);

    // Get or create consumer
    let stream = jetstream.get_stream(&config.stream_name).await?;

    let consumer = stream
        .get_or_create_consumer(
            &config.consumer_name,
            jetstream::consumer::pull::Config {
                durable_name: Some(config.consumer_name.clone()),
                ack_policy: jetstream::consumer::AckPolicy::Explicit,
                max_deliver: 3, // Retry up to 3 times
                ack_wait: Duration::from_secs(30),
                filter_subject: "messages.>".to_string(),
                ..Default::default()
            },
        )
        .await?;

    info!("Consumer '{}' ready", config.consumer_name);

    // Process messages in batches
    let mut messages = consumer
        .batch()
        .max_messages(config.batch_size)
        .messages()
        .await?;

    info!("Worker started, waiting for messages...");

    while let Some(msg) = messages.next().await {
        match msg {
            Ok(msg) => {
                match process_message(msg, &db_pool).await {
                    Ok(_) => {
                        // Success - acknowledge message
                        if let Err(e) = msg.ack().await {
                            error!("Failed to ack message: {}", e);
                        }
                    }
                    Err(e) => {
                        error!("Failed to process message: {}", e);

                        // Check retry count
                        if msg.info().unwrap().num_delivered >= 3 {
                            // Max retries exceeded - send to DLQ or log
                            warn!("Message exceeded max retries, terminating");
                            if let Err(e) = msg.term().await {
                                error!("Failed to terminate message: {}", e);
                            }
                        } else {
                            // NAK for retry
                            if let Err(e) = msg.nak().await {
                                error!("Failed to nak message: {}", e);
                            }
                        }
                    }
                }
            }
            Err(e) => {
                error!("Error receiving message: {}", e);
            }
        }
    }

    Ok(())
}

async fn process_message(
    msg: jetstream::Message,
    pool: &DbPool,
) -> Result<(), Box<dyn std::error::Error>> {
    // Deserialize envelope
    let envelope: MessageEnvelope = serde_json::from_slice(&msg.payload)?;

    info!(
        "Processing message {} (attempt {})",
        envelope.message_id,
        msg.info().unwrap().num_delivered
    );

    // Parse message
    let parsed = parsers::parse_message(&envelope.body)?;

    // Get DB connection
    let mut conn = pool.get()?;

    // Begin transaction
    conn.transaction(|conn| {
        // Insert parsed data
        match parsed {
            parsers::ParsedMessage::Order(order) => {
                diesel::insert_into(schema::orders::table)
                    .values(&order)
                    .execute(conn)?;
            }
            parsers::ParsedMessage::OrderLineItem(item) => {
                diesel::insert_into(schema::order_line_items::table)
                    .values(&item)
                    .execute(conn)?;
            }
        }

        // Update message status
        let status = models::MessageStatus {
            message_id: envelope.message_id,
            status: "processed".to_string(),
            received_at: envelope.received_at,
            processed_at: Some(chrono::Utc::now()),
            error: None,
        };

        diesel::insert_into(schema::message_status::table)
            .values(&status)
            .execute(conn)?;

        Ok(())
    })?;

    info!("Successfully processed message {}", envelope.message_id);

    Ok(())
}
```

### 6. Database Schema Addition

Add message status tracking table to schema generation:

```sql
-- Generated migration for message_status table
CREATE TABLE message_status (
    message_id UUID PRIMARY KEY,
    status VARCHAR(50) NOT NULL,
    received_at TIMESTAMP NOT NULL,
    processed_at TIMESTAMP,
    error TEXT,
    retry_count INTEGER DEFAULT 0,
    created_at TIMESTAMP DEFAULT CURRENT_TIMESTAMP
);

CREATE INDEX idx_message_status_received ON message_status(received_at);
CREATE INDEX idx_message_status_status ON message_status(status);
```

### 7. Environment Variables

Add to generated `.env.example`:

```bash
# NATS Configuration
NATS_URL=nats://localhost:4222
NATS_STREAM=MESSAGES
NATS_CONSUMER=workers

# Database
DATABASE_URL=postgresql://user:password@localhost:5432/dbname

# Server
PORT=8080
HOST=0.0.0.0

# Logging
RUST_LOG=info

# Feature flags
ENABLE_NATS=true  # Set to false for synchronous mode
```

## Local Testing with Docker Compose

### docker-compose.nats.yml

```yaml
version: '3.8'

services:
  # NATS JetStream server
  nats:
    image: nats:2.10-alpine
    ports:
      - "4222:4222"   # Client connections
      - "8222:8222"   # HTTP monitoring
      - "6222:6222"   # Cluster routing
    command:
      - "-js"                    # Enable JetStream
      - "-sd"                    # Enable JetStream store directory
      - "/data"                  # Data directory
      - "-m"                     # Enable monitoring
      - "8222"                   # Monitoring port
    volumes:
      - nats-data:/data
    networks:
      - nomnom
    healthcheck:
      test: ["CMD", "wget", "--spider", "http://localhost:8222/healthz"]
      interval: 10s
      timeout: 5s
      retries: 5

  # PostgreSQL database
  postgres:
    image: postgres:17-alpine
    environment:
      POSTGRES_DB: tpch
      POSTGRES_USER: postgres
      POSTGRES_PASSWORD: devpassword
    ports:
      - "5432:5432"
    volumes:
      - postgres-data:/var/lib/postgresql/data
      - ./schema.sql:/docker-entrypoint-initdb.d/01-schema.sql
    networks:
      - nomnom
    healthcheck:
      test: ["CMD-SHELL", "pg_isready -U postgres"]
      interval: 10s
      timeout: 5s
      retries: 5

  # Ingestion API (publishes to NATS)
  ingestion-api:
    build:
      context: .
      dockerfile: Dockerfile.ingestion
    ports:
      - "8080:8080"
    environment:
      NATS_URL: nats://nats:4222
      DATABASE_URL: postgresql://postgres:devpassword@postgres:5432/tpch
      PORT: 8080
      HOST: 0.0.0.0
      RUST_LOG: info
      ENABLE_NATS: "true"
    depends_on:
      nats:
        condition: service_healthy
      postgres:
        condition: service_healthy
    networks:
      - nomnom
    restart: unless-stopped

  # Worker (consumes from NATS, writes to DB)
  worker:
    build:
      context: .
      dockerfile: Dockerfile.worker
    environment:
      NATS_URL: nats://nats:4222
      DATABASE_URL: postgresql://postgres:devpassword@postgres:5432/tpch
      RUST_LOG: info
    depends_on:
      nats:
        condition: service_healthy
      postgres:
        condition: service_healthy
    networks:
      - nomnom
    restart: unless-stopped
    # Scale workers: docker compose up --scale worker=3

  # NATS monitoring UI (optional)
  nats-surveyor:
    image: natsio/nats-surveyor:latest
    ports:
      - "7777:7777"
    command:
      - "-s"
      - "http://nats:8222"
      - "-p"
      - "7777"
    depends_on:
      - nats
    networks:
      - nomnom

volumes:
  nats-data:
  postgres-data:

networks:
  nomnom:
    driver: bridge
```

### Testing with cURL

```bash
# 1. Start all services
docker compose -f docker-compose.nats.yml up -d

# 2. Wait for services to be healthy
docker compose -f docker-compose.nats.yml ps

# 3. Ingest a message (via HTTP - NATS is transparent)
curl -X POST http://localhost:8080/api/ingest/message \
  -H "Content-Type: text/plain" \
  -d "O|123|456|F|123.45|2024-01-01|urgent|clerk1|1|comment"

# Response (202 Accepted):
{
  "message_id": "550e8400-e29b-41d4-a716-446655440000",
  "status": "accepted",
  "timestamp": "2024-01-01T12:00:00Z"
}

# 4. Check message status
curl http://localhost:8080/api/ingest/status/550e8400-e29b-41d4-a716-446655440000

# Response:
{
  "message_id": "550e8400-e29b-41d4-a716-446655440000",
  "status": "processed",
  "received_at": "2024-01-01T12:00:00Z",
  "processed_at": "2024-01-01T12:00:01Z",
  "error": null
}

# 5. Send batch of messages
for i in {1..100}; do
  curl -X POST http://localhost:8080/api/ingest/message \
    -H "Content-Type: text/plain" \
    -d "O|$i|456|F|123.45|2024-01-01|urgent|clerk1|1|comment" &
done
wait

# 6. Check NATS stats (monitoring UI)
open http://localhost:7777

# Or via CLI
docker exec -it nomnom-nats-1 nats stream info MESSAGES

# 7. Check worker logs
docker compose -f docker-compose.nats.yml logs -f worker

# 8. Scale workers
docker compose -f docker-compose.nats.yml up -d --scale worker=5

# 9. Query database directly
docker exec -it nomnom-postgres-1 psql -U postgres -d tpch -c "SELECT COUNT(*) FROM orders;"
```

### NATS CLI Testing

```bash
# Install NATS CLI
brew install nats-io/nats-tools/nats  # macOS
# or download from https://github.com/nats-io/natscli

# Connect to NATS
nats --server=localhost:4222

# View streams
nats stream ls
nats stream info MESSAGES

# View consumers
nats consumer ls MESSAGES
nats consumer info MESSAGES workers

# Manually publish a message (bypass HTTP)
echo '{"message_id":"test-123","body":"O|999|888|F|99.99|2024-01-01|test|test|1|test","received_at":"2024-01-01T00:00:00Z","retry_count":0}' | \
  nats pub messages.ingest.test

# Monitor messages
nats sub "messages.>"

# View consumer lag
watch -n 1 'nats consumer info MESSAGES workers'
```

## Code Generation Implementation

### nomnom CLI Changes

Add new command and options:

```rust
// src/cli.rs

#[derive(Parser)]
pub struct GenerateIngestionCmd {
    /// Enable NATS JetStream integration
    #[arg(long, default_value = "true")]
    pub enable_nats: bool,

    /// Generate worker binary
    #[arg(long, default_value = "true")]
    pub generate_worker: bool,

    /// NATS stream name
    #[arg(long, default_value = "MESSAGES")]
    pub nats_stream: String,

    // ... existing fields
}

impl GenerateIngestionCmd {
    pub fn run(&self) -> Result<()> {
        // Load entities...

        // Generate API server
        codegen::generate_ingestion_api(
            &entities,
            &self.output,
            &config,
        )?;

        // Generate worker if enabled
        if self.generate_worker {
            codegen::generate_ingestion_worker(
                &entities,
                &self.output.join("worker"),
                &config,
            )?;
        }

        // Generate docker-compose with NATS
        codegen::generate_docker_compose_nats(
            &self.output,
            &config,
        )?;

        Ok(())
    }
}
```

### File Generator Functions

```rust
// src/codegen/ingestion_server/nats.rs

pub fn generate_nats_client(output_dir: &Path) -> Result<()> {
    let file_path = output_dir.join("src/nats_client.rs");
    let mut file = File::create(&file_path)?;

    writeln!(file, "{}", NATS_CLIENT_TEMPLATE)?;
    Ok(())
}

pub fn generate_message_envelope(output_dir: &Path) -> Result<()> {
    let file_path = output_dir.join("src/message_envelope.rs");
    let mut file = File::create(&file_path)?;

    writeln!(file, "{}", MESSAGE_ENVELOPE_TEMPLATE)?;
    Ok(())
}

pub fn generate_worker_main(
    entities: &[EntityDef],
    output_dir: &Path,
) -> Result<()> {
    let worker_dir = output_dir.join("worker");
    std::fs::create_dir_all(&worker_dir.join("src"))?;

    let file_path = worker_dir.join("src/main.rs");
    let mut file = File::create(&file_path)?;

    writeln!(file, "{}", WORKER_MAIN_TEMPLATE)?;
    // ... generate parser match arms for each entity

    Ok(())
}
```

## Testing Strategy

### Unit Tests

```rust
#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn test_nats_publish() {
        let config = NatsConfig::default();
        let client = NatsClient::connect(config).await.unwrap();

        let envelope = MessageEnvelope::new(
            "O|1|2|F|100.0|2024-01-01|urgent|clerk|1|test".to_string(),
            Some("order".to_string()),
        );

        client.publish_message(&envelope).await.unwrap();
    }

    #[tokio::test]
    async fn test_worker_processing() {
        // Spin up test NATS server
        // Publish test message
        // Verify it gets processed
    }
}
```

### Integration Tests

```bash
#!/bin/bash
# test-nats-integration.sh

set -e

echo "Starting services..."
docker compose -f docker-compose.nats.yml up -d

echo "Waiting for services..."
sleep 10

echo "Testing ingestion..."
RESPONSE=$(curl -s -X POST http://localhost:8080/api/ingest/message \
  -H "Content-Type: text/plain" \
  -d "O|123|456|F|123.45|2024-01-01|urgent|clerk1|1|comment")

MESSAGE_ID=$(echo $RESPONSE | jq -r '.message_id')
echo "Message ID: $MESSAGE_ID"

echo "Waiting for processing..."
sleep 2

echo "Checking status..."
STATUS=$(curl -s http://localhost:8080/api/ingest/status/$MESSAGE_ID)
echo "Status: $STATUS"

# Verify status is "processed"
if echo $STATUS | jq -e '.status == "processed"' > /dev/null; then
    echo "✅ Test passed: Message was processed"
else
    echo "❌ Test failed: Message status is not 'processed'"
    exit 1
fi

echo "Cleaning up..."
docker compose -f docker-compose.nats.yml down

echo "✅ All tests passed!"
```

### Load Testing

```javascript
// k6-load-test.js
import http from 'k6/http';
import { check, sleep } from 'k6';

export const options = {
  scenarios: {
    constant_load: {
      executor: 'constant-vus',
      vus: 50,
      duration: '5m',
    },
  },
  thresholds: {
    http_req_duration: ['p(95)<200'],  // 95% under 200ms (async!)
    http_req_failed: ['rate<0.01'],
  },
};

export default function() {
  const payload = `O|${__VU}|${__ITER}|F|123.45|2024-01-01|urgent|clerk|1|test`;

  const res = http.post(
    'http://localhost:8080/api/ingest/message',
    payload,
    { headers: { 'Content-Type': 'text/plain' } }
  );

  check(res, {
    'status is 202': (r) => r.status === 202,
    'has message_id': (r) => r.json('message_id') !== undefined,
  });

  sleep(0.1);
}
```

## Monitoring

### Prometheus Metrics

Add to generated code:

```rust
use prometheus::{Counter, Histogram, Registry};

lazy_static! {
    pub static ref MESSAGES_PUBLISHED: Counter = Counter::new(
        "messages_published_total",
        "Total messages published to NATS"
    ).unwrap();

    pub static ref MESSAGES_PROCESSED: Counter = Counter::new(
        "messages_processed_total",
        "Total messages processed by workers"
    ).unwrap();

    pub static ref PUBLISH_LATENCY: Histogram = Histogram::new(
        "nats_publish_latency_seconds",
        "NATS publish latency"
    ).unwrap();

    pub static ref PROCESSING_LATENCY: Histogram = Histogram::new(
        "message_processing_latency_seconds",
        "Message processing latency"
    ).unwrap();
}
```

### NATS Metrics Endpoint

NATS exposes Prometheus metrics at `:8222/metrics`:

```yaml
# Prometheus scrape config
scrape_configs:
  - job_name: 'nats'
    static_configs:
      - targets: ['localhost:8222']

  - job_name: 'ingestion-api'
    static_configs:
      - targets: ['localhost:8080']

  - job_name: 'worker'
    static_configs:
      - targets: ['localhost:9090']  # Add metrics endpoint to worker
```

## Migration Path

### Phase 1: Add NATS Support (Week 1)
- [ ] Generate NATS client code
- [ ] Generate message envelope
- [ ] Modify HTTP handlers to publish to NATS
- [ ] Add feature flag for NATS vs sync
- [ ] Test locally with docker-compose

### Phase 2: Worker Implementation (Week 2)
- [ ] Generate worker binary
- [ ] Add consumer logic
- [ ] Add retry and DLQ handling
- [ ] Add status tracking to DB
- [ ] Test worker processing

### Phase 3: Local Testing (Week 3)
- [ ] Create docker-compose setup
- [ ] Write integration tests
- [ ] Load testing with k6
- [ ] Performance tuning
- [ ] Documentation

### Phase 4: K8s Integration (Week 4)
- [ ] Update Helm charts for NATS
- [ ] Add KEDA autoscaling
- [ ] Deploy to kind
- [ ] E2E testing
- [ ] Production readiness review

## Success Criteria

- [ ] Generated code compiles without errors
- [ ] NATS connection establishes successfully
- [ ] Messages publish to JetStream
- [ ] Workers consume and process messages
- [ ] Status endpoint returns correct status
- [ ] HTTP API still works via cURL
- [ ] Docker Compose setup runs locally
- [ ] Load test achieves > 1000 msg/sec
- [ ] Zero message loss under normal conditions
- [ ] Graceful degradation under failure
- [ ] Monitoring dashboards show metrics
- [ ] Documentation is complete

## Benefits Summary

1. **Reliability**: Durable message storage, automatic retries
2. **Scalability**: Independent scaling of API and workers
3. **Performance**: Sub-100ms HTTP response times
4. **Observability**: Full message tracking and status
5. **Testability**: Still works with cURL, docker-compose
6. **K8s-Native**: KEDA autoscaling, cloud-portable
7. **Rust-Native**: async-nats is idiomatic, performant
8. **Cost-Effective**: NATS is lightweight, minimal overhead
