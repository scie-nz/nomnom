# Pre-Kubernetes Deployment Improvements

**Goal**: Add critical missing features before Kubernetes deployment to ensure production-ready setup.

**Timeline**: ~4-6 hours of implementation
**Priority**: High - Required for proper K8s deployment

## Overview

Before deploying to Kubernetes, we need to implement missing features that are essential for reliable production operation:

1. **Readiness endpoint** - Critical for K8s readiness probes
2. **Message status tracking** - Important for observability and debugging
3. **Dead letter queue** - Essential for handling persistent failures
4. **Configurable parameters** - Needed for flexible deployment

## Feature 1: Add `/ready` Endpoint

### Why It's Important

- Kubernetes distinguishes between **liveness** (is the app alive?) and **readiness** (is it ready to serve traffic?)
- `/health` checks database connectivity, which might be slow during startup
- `/ready` should be lightweight and return quickly
- Traffic should only route to pods that are truly ready

### Implementation

**Files to Modify:**
- `src/codegen/ingestion_server/handlers_rs.rs` - Add ready handler
- `src/codegen/ingestion_server/main_rs.rs` - Add route

**Code Changes:**

1. **Add ready handler** in `handlers_rs.rs`:
   ```rust
   /// Readiness check endpoint (lightweight)
   #[utoipa::path(
       get,
       path = "/ready",
       responses(
           (status = 200, description = "Service is ready to accept traffic")
       )
   )]
   pub async fn ready_check() -> impl IntoResponse {
       // Lightweight check - just verify the app is running
       // Don't check database here (that's for /health)
       Json(serde_json::json!({
           "ready": true,
           "timestamp": chrono::Utc::now().to_rfc3339()
       }))
   }
   ```

2. **Add route** in `main_rs.rs`:
   ```rust
   .route("/ready", get(handlers::ready_check))
   ```

3. **Update OpenAPI spec** - The `#[utoipa::path]` annotation will auto-update it

**Testing:**
```bash
# Should return 200 immediately without database checks
curl http://localhost:8080/ready

# Expected response:
# {"ready": true, "timestamp": "2025-11-10T05:30:00Z"}
```

**Success Criteria:**
- [x] `/ready` returns 200 in <10ms (no DB queries)
- [x] `/health` still checks database connectivity
- [x] OpenAPI spec includes new endpoint
- [x] Can be used for K8s readiness probe

**Estimated Time:** 30 minutes

---

## Feature 2: Message Status Tracking

### Why It's Important

- Enables clients to check processing status of submitted messages
- Critical for debugging failed messages
- Provides audit trail for data pipeline
- Current `/ingest/status/{message_id}` returns placeholder

### Database Schema

**New table: `message_status`**

```sql
CREATE TABLE IF NOT EXISTS message_status (
    message_id UUID PRIMARY KEY,
    entity_type VARCHAR(50) NOT NULL,
    status VARCHAR(20) NOT NULL,  -- 'accepted', 'processing', 'completed', 'failed'
    received_at TIMESTAMP NOT NULL,
    processed_at TIMESTAMP,
    retry_count INTEGER DEFAULT 0,
    error_message TEXT,
    source VARCHAR(255),
    created_at TIMESTAMP DEFAULT NOW(),
    updated_at TIMESTAMP DEFAULT NOW()
);

CREATE INDEX idx_message_status_received_at ON message_status(received_at);
CREATE INDEX idx_message_status_status ON message_status(status);
```

### Implementation

**Phase 1: Database Setup**

1. **Add to worker's `database_rs.rs`** - Generate table creation:
   ```rust
   // In ensure_tables() function, add:
   writeln!(output, "    // Create message_status table")?;
   writeln!(output, "    diesel::sql_query(r#\"")?;
   writeln!(output, "        CREATE TABLE IF NOT EXISTS message_status (")?;
   writeln!(output, "            message_id UUID PRIMARY KEY,")?;
   writeln!(output, "            entity_type VARCHAR(50) NOT NULL,")?;
   writeln!(output, "            status VARCHAR(20) NOT NULL,")?;
   writeln!(output, "            received_at TIMESTAMP NOT NULL,")?;
   writeln!(output, "            processed_at TIMESTAMP,")?;
   writeln!(output, "            retry_count INTEGER DEFAULT 0,")?;
   writeln!(output, "            error_message TEXT,")?;
   writeln!(output, "            source VARCHAR(255)")?;
   writeln!(output, "        )")?;
   writeln!(output, "    \"#)")?;
   writeln!(output, "    .execute(conn)?;\n")?;
   ```

**Phase 2: Ingestion API Updates**

2. **Update `handlers_rs.rs`** - Record message status on ingestion:
   ```rust
   pub async fn ingest_message(
       State(state): State<Arc<AppState>>,
       body: String,
   ) -> Result<(StatusCode, Json<IngestionResponse>), AppError> {
       // ... existing code ...

       // Create message envelope
       let envelope = MessageEnvelope::new(body, entity_type.clone());

       // Publish to NATS JetStream
       state.nats.publish_message(&envelope).await?;

       // NEW: Record status in database
       let mut conn = state.db_pool.get()?;
       diesel::sql_query(
           "INSERT INTO message_status (message_id, entity_type, status, received_at)
            VALUES ($1, $2, $3, $4)"
       )
       .bind::<diesel::sql_types::Uuid, _>(&envelope.message_id)
       .bind::<diesel::sql_types::Text, _>(entity_type.as_deref().unwrap_or("unknown"))
       .bind::<diesel::sql_types::Text, _>("accepted")
       .bind::<diesel::sql_types::Timestamp, _>(&envelope.received_at.naive_utc())
       .execute(&mut conn)?;

       // ... return response ...
   }
   ```

3. **Update status check handler** in `handlers_rs.rs`:
   ```rust
   pub async fn check_status(
       State(state): State<Arc<AppState>>,
       Path(message_id): Path<String>,
   ) -> Result<Json<serde_json::Value>, AppError> {
       let uuid = Uuid::parse_str(&message_id)?;
       let mut conn = state.db_pool.get()?;

       let result: Option<(String, String, chrono::NaiveDateTime, Option<chrono::NaiveDateTime>)> =
           diesel::sql_query(
               "SELECT entity_type, status, received_at, processed_at
                FROM message_status WHERE message_id = $1"
           )
           .bind::<diesel::sql_types::Uuid, _>(&uuid)
           .get_result(&mut conn)
           .optional()?;

       match result {
           Some((entity_type, status, received_at, processed_at)) => {
               Ok(Json(serde_json::json!({
                   "message_id": message_id,
                   "entity_type": entity_type,
                   "status": status,
                   "received_at": received_at,
                   "processed_at": processed_at
               })))
           }
           None => Err(AppError::ValidationError("Message not found".to_string()))
       }
   }
   ```

**Phase 3: Worker Updates**

4. **Update worker's `main_rs.rs`** - Update status on processing:
   ```rust
   fn process_message(
       payload: &[u8],
       pool: &database::DbPool,
   ) -> Result<(), AppError> {
       let envelope: MessageEnvelope = serde_json::from_slice(payload)?;
       let message_id = envelope.message_id;

       // Parse message body
       let (entity_name, parsed) = MessageParser::parse_json(&envelope.body)?;

       // Get database connection
       let mut conn = pool.get()?;

       // Update status to 'processing'
       diesel::sql_query(
           "UPDATE message_status SET status = $1 WHERE message_id = $2"
       )
       .bind::<Text, _>("processing")
       .bind::<diesel::sql_types::Uuid, _>(&message_id)
       .execute(&mut conn)
       .ok(); // Ignore errors - status tracking is optional

       // Insert into database based on entity type
       let result = match parsed {
           ParsedMessage::Order(msg) => {
               // ... existing INSERT code ...
           }
       };

       // Update status to 'completed' or 'failed'
       match result {
           Ok(_) => {
               diesel::sql_query(
                   "UPDATE message_status
                    SET status = $1, processed_at = NOW()
                    WHERE message_id = $2"
               )
               .bind::<Text, _>("completed")
               .bind::<diesel::sql_types::Uuid, _>(&message_id)
               .execute(&mut conn)
               .ok();
           }
           Err(ref e) => {
               diesel::sql_query(
                   "UPDATE message_status
                    SET status = $1, error_message = $2, retry_count = retry_count + 1
                    WHERE message_id = $3"
               )
               .bind::<Text, _>("failed")
               .bind::<Text, _>(&format!("{:?}", e))
               .bind::<diesel::sql_types::Uuid, _>(&message_id)
               .execute(&mut conn)
               .ok();
           }
       }

       result
   }
   ```

**Testing:**
```bash
# 1. Send a message
RESPONSE=$(curl -s -X POST http://localhost:8080/ingest/message \
  -H "Content-Type: application/json" \
  -d '{"order_key":"STATUS-TEST-1","customer_key":"CUST-001",...}')

MESSAGE_ID=$(echo $RESPONSE | jq -r '.message_id')

# 2. Check status immediately (should be 'accepted' or 'processing')
curl http://localhost:8080/ingest/status/$MESSAGE_ID

# 3. Wait a moment, check again (should be 'completed')
sleep 1
curl http://localhost:8080/ingest/status/$MESSAGE_ID

# 4. Check database directly
docker compose exec postgres psql -U nomnom -d nomnom \
  -c "SELECT * FROM message_status WHERE message_id = '$MESSAGE_ID';"
```

**Success Criteria:**
- [x] `message_status` table auto-created by worker
- [x] Ingestion API records 'accepted' status on message submission
- [x] Worker updates status to 'processing' when consuming
- [x] Worker updates status to 'completed' on success
- [x] Worker updates status to 'failed' with error message on failure
- [x] `/ingest/status/{message_id}` returns real data
- [x] Status transitions: accepted → processing → completed/failed

**Estimated Time:** 2 hours

---

## Feature 3: Dead Letter Queue (DLQ)

### Why It's Important

- Messages that fail after max retries should not be lost
- DLQ enables manual inspection and reprocessing
- Critical for data integrity and debugging
- Currently, messages disappear after 3 failed delivery attempts

### Implementation Strategy

**Option A: NATS Native DLQ** (Recommended)
- Use NATS JetStream's built-in max_deliver and consumer filtering
- Failed messages automatically routed to DLQ stream
- Simpler, leverages NATS features

**Option B: Database DLQ**
- Store failed messages in PostgreSQL table
- More complex but gives full control
- Better for complex retry logic

**We'll implement Option A** - NATS native approach

### NATS Configuration

**Phase 1: Create DLQ Stream**

1. **Update ingestion server's `nats_client_rs.rs`**:
   ```rust
   pub async fn initialize_nats(nats_url: &str, stream_name: &str) -> Result<NatsClient, Box<dyn Error>> {
       let client = async_nats::connect(nats_url).await?;
       let jetstream = jetstream::new(client.clone());

       // Create main stream (existing)
       let stream = jetstream.get_or_create_stream(/* ... */).await?;

       // NEW: Create DLQ stream for failed messages
       let dlq_stream = jetstream.get_or_create_stream(jetstream::stream::Config {
           name: format!("{}_DLQ", stream_name),
           subjects: vec![format!("messages.dlq.>")],
           max_age: Duration::from_secs(7 * 24 * 60 * 60), // 7 days
           max_bytes: 1024 * 1024 * 1024, // 1GB
           storage: jetstream::stream::StorageType::File,
           num_replicas: 1,
           ..Default::default()
       }).await?;

       tracing::info!("DLQ stream '{}' ready", format!("{}_DLQ", stream_name));

       // ...
   }
   ```

**Phase 2: Update Worker Consumer**

2. **Update worker's `main_rs.rs`** - Route failed messages to DLQ:
   ```rust
   // In main() function, update consumer config:
   let consumer = stream
       .get_or_create_consumer(
           &consumer_name,
           jetstream::consumer::pull::Config {
               durable_name: Some(consumer_name.clone()),
               ack_policy: jetstream::consumer::AckPolicy::Explicit,
               max_deliver: 3,  // Try 3 times
               filter_subject: "messages.ingest.>".to_string(),
               // NEW: Send to DLQ after max_deliver
               deliver_policy: jetstream::consumer::DeliverPolicy::All,
               ..Default::default()
           }
       )
       .await?;

   // In process_message error handling:
   Err(e) => {
       tracing::error!("Failed to process message: {:?}", e);

       // Get delivery count from message metadata
       let delivery_count = msg.info()
           .map(|info| info.deliver_count)
           .unwrap_or(1);

       if delivery_count >= 3 {
           // Max retries reached - send to DLQ
           tracing::warn!("Message {} failed after {} attempts, sending to DLQ",
                          envelope.message_id, delivery_count);

           // Republish to DLQ stream
           state.nats.publish(
               &format!("messages.dlq.{}", entity_name),
               payload.into()
           ).await?;

           // ACK the original message (remove from main queue)
           msg.ack().await?;

           // Update status
           diesel::sql_query(
               "UPDATE message_status
                SET status = $1, error_message = $2
                WHERE message_id = $3"
           )
           .bind::<Text, _>("dlq")
           .bind::<Text, _>(&format!("Failed after {} attempts: {:?}", delivery_count, e))
           .bind::<diesel::sql_types::Uuid, _>(&envelope.message_id)
           .execute(&mut conn)
           .ok();
       } else {
           // NAK for retry
           msg.ack_with(jetstream::AckKind::Nak(None)).await?;
       }
   }
   ```

**Phase 3: DLQ Monitoring Tools**

3. **Add DLQ inspection scripts**:

   Create `scripts/inspect-dlq.sh`:
   ```bash
   #!/bin/bash
   # Inspect dead letter queue

   NATS_POD=$(kubectl get pods -n nomnom -l app=nats -o jsonpath='{.items[0].metadata.name}')

   echo "=== DLQ Stream Info ==="
   kubectl exec -it -n nomnom $NATS_POD -- \
     nats stream info MESSAGES_DLQ

   echo ""
   echo "=== DLQ Messages ==="
   kubectl exec -it -n nomnom $NATS_POD -- \
     nats stream view MESSAGES_DLQ
   ```

   Create `scripts/reprocess-dlq.sh`:
   ```bash
   #!/bin/bash
   # Reprocess messages from DLQ

   NATS_POD=$(kubectl get pods -n nomnom -l app=nats -o jsonpath='{.items[0].metadata.name}')

   echo "Reprocessing DLQ messages..."
   kubectl exec -it -n nomnom $NATS_POD -- \
     nats consumer add MESSAGES_DLQ reprocessor \
       --filter "messages.dlq.>" \
       --deliver all \
       --replay instant
   ```

**Testing:**
```bash
# 1. Send a message that will fail (invalid data)
curl -X POST http://localhost:8080/ingest/message \
  -H "Content-Type: application/json" \
  -d '{"order_key":"DLQ-TEST","customer_key":null,"order_status":"INVALID",...}'

# 2. Check worker logs - should see 3 retry attempts
docker compose logs worker --tail 20

# 3. Check DLQ stream
docker compose exec nats nats stream info MESSAGES_DLQ

# 4. View DLQ messages
docker compose exec nats nats stream view MESSAGES_DLQ

# 5. Check message status
curl http://localhost:8080/ingest/status/$MESSAGE_ID
# Should show status: "dlq"
```

**Success Criteria:**
- [x] DLQ stream auto-created on startup
- [x] Messages that fail 3 times route to DLQ
- [x] Original message ACKed after DLQ routing
- [x] Message status updated to 'dlq'
- [x] DLQ messages include error context
- [x] DLQ retention: 7 days
- [x] DLQ can be inspected via NATS CLI
- [x] DLQ messages can be reprocessed manually

**Estimated Time:** 2-3 hours

---

## Feature 4: Configurable Parameters

### Why It's Important

- Hardcoded values limit deployment flexibility
- Different environments need different settings (dev vs prod)
- Essential for Kubernetes ConfigMap usage
- Current hardcoded: batch size, consumer name, retry count

### Implementation

**Files to Modify:**
- `src/codegen/worker/main_rs.rs` - Add environment variable parsing
- `src/codegen/ingestion_server/main_rs.rs` - Add config parsing

**Worker Configuration:**

1. **Add to worker's `main_rs.rs`**:
   ```rust
   // After loading environment variables
   dotenv::dotenv().ok();

   // Get configuration from environment
   let nats_url = std::env::var("NATS_URL")
       .unwrap_or_else(|_| "nats://localhost:4222".to_string());
   let stream_name = std::env::var("NATS_STREAM")
       .unwrap_or_else(|_| "MESSAGES".to_string());
   let consumer_name = std::env::var("NATS_CONSUMER")
       .unwrap_or_else(|_| "workers".to_string());

   // NEW: Configurable parameters
   let batch_size: usize = std::env::var("WORKER_BATCH_SIZE")
       .unwrap_or_else(|_| "10".to_string())
       .parse()
       .unwrap_or(10);

   let max_deliver: i64 = std::env::var("WORKER_MAX_DELIVER")
       .unwrap_or_else(|_| "3".to_string())
       .parse()
       .unwrap_or(3);

   let poll_interval_ms: u64 = std::env::var("WORKER_POLL_INTERVAL_MS")
       .unwrap_or_else(|_| "100".to_string())
       .parse()
       .unwrap_or(100);

   tracing::info!(
       "Worker config: batch_size={}, max_deliver={}, poll_interval={}ms",
       batch_size, max_deliver, poll_interval_ms
   );

   // Use in consumer configuration
   let consumer = stream
       .get_or_create_consumer(
           &consumer_name,
           jetstream::consumer::pull::Config {
               durable_name: Some(consumer_name.clone()),
               ack_policy: jetstream::consumer::AckPolicy::Explicit,
               max_deliver,  // Use configured value
               filter_subject: "messages.ingest.>".to_string(),
               ..Default::default()
           }
       )
       .await?;

   // Use in message fetching
   let mut messages = consumer
       .fetch()
       .max_messages(batch_size)  // Use configured value
       .messages()
       .await?;

   // Use in polling interval
   tokio::time::sleep(Duration::from_millis(poll_interval_ms)).await;
   ```

2. **Update `.env.example`** in worker codegen:
   ```bash
   # Database connection
   DATABASE_URL=postgresql://user:password@localhost:5432/dbname

   # NATS Configuration
   NATS_URL=nats://localhost:4222
   NATS_STREAM=MESSAGES
   NATS_CONSUMER=workers

   # Worker Configuration
   WORKER_BATCH_SIZE=10              # Messages to fetch per batch
   WORKER_MAX_DELIVER=3              # Max delivery attempts before DLQ
   WORKER_POLL_INTERVAL_MS=100       # Milliseconds between batches

   # Logging
   RUST_LOG=info
   ```

**Ingestion API Configuration:**

3. **Add to ingestion server's `main_rs.rs`**:
   ```rust
   // After loading environment variables
   let nats_url = std::env::var("NATS_URL")
       .unwrap_or_else(|_| "nats://localhost:4222".to_string());
   let stream_name = std::env::var("NATS_STREAM")
       .unwrap_or_else(|_| "MESSAGES".to_string());

   // NEW: Configurable stream settings
   let max_age_hours: u64 = std::env::var("NATS_MAX_AGE_HOURS")
       .unwrap_or_else(|_| "24".to_string())
       .parse()
       .unwrap_or(24);

   let max_bytes: i64 = std::env::var("NATS_MAX_BYTES")
       .unwrap_or_else(|_| "1073741824".to_string())  // 1GB
       .parse()
       .unwrap_or(1073741824);

   // Use in stream configuration
   let stream = jetstream.get_or_create_stream(jetstream::stream::Config {
       name: stream_name.clone(),
       subjects: vec!["messages.>".to_string()],
       max_age: Duration::from_secs(max_age_hours * 60 * 60),
       max_bytes,
       storage: jetstream::stream::StorageType::File,
       num_replicas: 1,
       ..Default::default()
   }).await?;
   ```

**Testing:**
```bash
# Test with custom configuration
docker compose down
docker compose up -d --build

# Override environment variables
docker compose exec worker env WORKER_BATCH_SIZE=50 WORKER_MAX_DELIVER=5 /app/worker

# Verify configuration in logs
docker compose logs worker | grep "Worker config"
# Should show: Worker config: batch_size=50, max_deliver=5, poll_interval=100ms
```

**Success Criteria:**
- [x] All hardcoded values replaced with env vars
- [x] Sensible defaults if env vars not set
- [x] Configuration logged on startup
- [x] `.env.example` documents all variables
- [x] K8s ConfigMap can override all settings

**Estimated Time:** 1 hour

---

## Testing Plan

### Integration Testing

After implementing all features:

1. **End-to-End Flow Test**
   ```bash
   # 1. Start services
   cd /private/tmp/nomnom-nats-test/ingestion-server
   docker compose -f docker-compose.test.yml up -d --build

   # 2. Test readiness
   curl http://localhost:8080/ready
   # Expected: {"ready": true, ...}

   # 3. Send valid message
   MSG_ID=$(curl -s -X POST http://localhost:8080/ingest/message \
     -H "Content-Type: application/json" \
     -d '{"order_key":"INT-TEST-1",...}' | jq -r '.message_id')

   # 4. Check status progression
   curl http://localhost:8080/ingest/status/$MSG_ID
   # Expected: status "accepted" or "processing"

   sleep 2
   curl http://localhost:8080/ingest/status/$MSG_ID
   # Expected: status "completed"

   # 5. Verify in database
   docker compose exec postgres psql -U nomnom -d nomnom \
     -c "SELECT * FROM orders WHERE order_key = 'INT-TEST-1';"

   # 6. Verify message status table
   docker compose exec postgres psql -U nomnom -d nomnom \
     -c "SELECT * FROM message_status WHERE message_id = '$MSG_ID';"
   ```

2. **DLQ Flow Test**
   ```bash
   # 1. Send invalid message (will fail processing)
   BAD_MSG_ID=$(curl -s -X POST http://localhost:8080/ingest/message \
     -H "Content-Type: application/json" \
     -d '{"order_key":"DLQ-TEST","customer_key":null}' | jq -r '.message_id')

   # 2. Watch worker logs for retries
   docker compose logs worker -f
   # Should see 3 retry attempts

   # 3. Check DLQ after retries
   docker compose exec nats nats stream info MESSAGES_DLQ
   # Should show 1 message

   # 4. Check message status
   curl http://localhost:8080/ingest/status/$BAD_MSG_ID
   # Expected: status "dlq"
   ```

3. **Configuration Test**
   ```bash
   # Test with custom worker config
   docker compose stop worker
   docker compose run -e WORKER_BATCH_SIZE=5 \
                       -e WORKER_MAX_DELIVER=2 \
                       worker

   # Verify logs show custom config
   ```

### Automated Tests

Create test script: `test-pre-k8s-features.sh`

```bash
#!/bin/bash
set -e

echo "=== Testing Pre-K8s Features ==="

# Start services
docker compose -f docker-compose.test.yml up -d --build
sleep 10

# Test 1: Readiness endpoint
echo "Test 1: Readiness endpoint"
READY=$(curl -s http://localhost:8080/ready | jq -r '.ready')
if [ "$READY" != "true" ]; then
  echo "FAIL: Readiness check failed"
  exit 1
fi
echo "PASS"

# Test 2: Message status tracking
echo "Test 2: Message status tracking"
MSG_ID=$(curl -s -X POST http://localhost:8080/ingest/message \
  -H "Content-Type: application/json" \
  -d '{"order_key":"TEST-STATUS","customer_key":"CUST-001",...}' | jq -r '.message_id')

sleep 2
STATUS=$(curl -s http://localhost:8080/ingest/status/$MSG_ID | jq -r '.status')
if [ "$STATUS" != "completed" ]; then
  echo "FAIL: Message status not completed (got: $STATUS)"
  exit 1
fi
echo "PASS"

# Test 3: DLQ routing
echo "Test 3: DLQ routing"
# Send invalid message
BAD_MSG_ID=$(curl -s -X POST http://localhost:8080/ingest/message \
  -H "Content-Type: application/json" \
  -d '{"order_key":"DLQ-TEST"}' | jq -r '.message_id')

# Wait for retries
sleep 10

# Check DLQ
DLQ_COUNT=$(docker compose exec nats nats stream info MESSAGES_DLQ --json | jq -r '.state.messages')
if [ "$DLQ_COUNT" -lt 1 ]; then
  echo "FAIL: No messages in DLQ"
  exit 1
fi
echo "PASS"

# Test 4: Configurable parameters
echo "Test 4: Configurable parameters"
docker compose logs worker | grep "Worker config" | grep "batch_size"
if [ $? -ne 0 ]; then
  echo "FAIL: Configuration not logged"
  exit 1
fi
echo "PASS"

echo ""
echo "=== All Tests Passed ==="
```

**Success Criteria:**
- [x] All automated tests pass
- [x] Manual integration tests pass
- [x] No errors in logs during testing
- [x] Database state matches expected
- [x] NATS streams show correct message counts

**Estimated Time:** 1 hour

---

## Documentation Updates

### Files to Create/Update

1. **Update `CURRENT_SETUP_VERIFICATION.md`**
   - Mark missing features as implemented
   - Update endpoint documentation
   - Add configuration reference

2. **Update `TESTING_GUIDE.md`**
   - Add tests for new features
   - Document DLQ inspection procedures

3. **Create `CONFIGURATION_REFERENCE.md`**
   - Document all environment variables
   - Provide examples for different scenarios
   - Explain configuration trade-offs

4. **Update `.env.example` files**
   - Add all new configuration options
   - Provide sensible defaults
   - Add comments explaining each variable

**Estimated Time:** 30 minutes

---

## Code Generation Updates

### Files to Modify in nomnom Codebase

1. **`src/codegen/ingestion_server/handlers_rs.rs`**
   - Add `ready_check()` handler
   - Update `check_status()` with real implementation
   - Add status update in `ingest_message()`

2. **`src/codegen/ingestion_server/main_rs.rs`**
   - Add `/ready` route
   - Add configuration parsing
   - Add DLQ stream creation

3. **`src/codegen/ingestion_server/nats_client_rs.rs`**
   - Add DLQ stream initialization
   - Add helper methods for DLQ publishing

4. **`src/codegen/worker/main_rs.rs`**
   - Add configuration parsing
   - Use configured values for batch size, max_deliver, etc.
   - Add DLQ routing logic
   - Add message status updates

5. **`src/codegen/worker/database_rs.rs`**
   - Add `message_status` table creation

6. **`src/codegen/worker/mod.rs`**
   - Update `.env.example` generation

**After changes:**
```bash
# Regenerate both components
cd /Users/bogdanstate/nomnom
cargo build --release

# Regenerate ingestion server
./target/release/nomnom generate-ingestion-server \
  --entities config/examples/tpch/entities \
  --output /private/tmp/nomnom-nats-test/ingestion-server

# Regenerate worker
./target/release/nomnom generate-worker \
  --entities config/examples/tpch/entities \
  --output /private/tmp/nomnom-nats-test/worker

# Rebuild and test
cd /private/tmp/nomnom-nats-test/ingestion-server
docker compose -f docker-compose.test.yml up -d --build
```

---

## Timeline

| Feature | Estimated Time | Priority |
|---------|---------------|----------|
| 1. `/ready` endpoint | 30 min | Critical |
| 2. Message status tracking | 2 hours | High |
| 3. Dead letter queue | 2-3 hours | High |
| 4. Configurable parameters | 1 hour | Medium |
| Integration testing | 1 hour | Critical |
| Documentation | 30 min | Medium |
| **Total** | **6-7 hours** | |

### Recommended Order

1. **Phase 1** (Critical path - 30 min)
   - Add `/ready` endpoint
   - Test and verify

2. **Phase 2** (2 hours)
   - Implement message status tracking
   - Test status flow

3. **Phase 3** (2-3 hours)
   - Implement DLQ
   - Test failure scenarios

4. **Phase 4** (1 hour)
   - Add configurable parameters
   - Update documentation

5. **Phase 5** (1.5 hours)
   - Integration testing
   - Final verification

---

## Success Criteria

### Ready for Kubernetes Deployment When:

- [x] `/ready` endpoint returns 200 in <10ms
- [x] `/health` checks database connectivity
- [x] Message status tracking fully functional
- [x] DLQ captures failed messages after max retries
- [x] All hardcoded values replaced with env vars
- [x] Worker logs configuration on startup
- [x] All automated tests pass
- [x] Integration tests pass
- [x] Documentation updated
- [x] Docker Compose setup works end-to-end
- [x] No errors in logs during normal operation
- [x] Database schema matches expectations
- [x] NATS streams configured correctly

---

## Next Steps After Completion

1. **Update Kubernetes Plan**
   - Revise `PLAN_KUBERNETES_DEPLOYMENT.md` with verified config
   - Align with implemented features

2. **Generate Helm Charts**
   - Implement `generate-helm` command
   - Generate charts based on verified setup

3. **Test in kind**
   - Deploy to local Kubernetes cluster
   - Verify all features work in K8s environment

4. **Production Hardening**
   - Add security policies
   - Implement monitoring
   - Set up alerts

---

## Risk Mitigation

### Potential Issues

1. **Message status updates fail**
   - Mitigation: Make status updates optional (`.ok()` on errors)
   - Don't block message processing on status failures

2. **DLQ fills up**
   - Mitigation: Set 7-day retention, monitor size
   - Alert when DLQ exceeds threshold

3. **Configuration changes break existing deployments**
   - Mitigation: Maintain backwards compatibility with defaults
   - Document breaking changes clearly

4. **Performance impact of status tracking**
   - Mitigation: Use async inserts, batch updates
   - Add index on message_id for fast lookups

### Rollback Plan

If issues arise:

1. Keep existing Docker Compose setup working
2. Use feature flags to disable new features
3. Maintain backwards compatibility
4. Test with both old and new code paths

---

## Questions to Resolve

- [ ] Should we add a `/metrics` endpoint for Prometheus?
- [ ] Should DLQ messages expire after 7 days or be kept longer?
- [ ] Should we add batch status update endpoint?
- [ ] Should we add admin API to manually reprocess DLQ messages?
