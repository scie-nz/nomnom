# Debug Plan: Message Flow Issue

## Date: 2025-11-13

## Problem Statement

Messages sent to the ingestion API via curl are not being processed by the worker. Specifically:
- Messages are accepted by the API (200 OK response)
- Messages do not appear in `message_status` table
- Worker logs are not accessible via `kubectl logs`
- Cannot verify end-to-end derived entity processing

## Known Working Components

✅ Worker builds successfully (0 errors)
✅ Worker deploys to Kubernetes and runs (no crashes)
✅ Worker connects to NATS (2 connections active)
✅ Database schema is correct (`orders`, `order_line_items` tables exist)
✅ NATS stream exists and has messages (4 total messages reported)

## Known Issues

❌ New messages not appearing in `message_status` table
❌ Worker logs inaccessible via `kubectl logs`
❌ Message count in NATS not increasing when sending new messages
❌ Cannot verify derived entity processing

## Debugging Steps

### Step 1: Verify NATS Stream Configuration

**Goal**: Ensure NATS stream, subjects, and consumers are configured correctly

**Commands**:
```bash
# Check stream info
kubectl exec -n nomnom-dev nomnom-nats-0 -- nats stream info MESSAGES

# List consumers on the stream
kubectl exec -n nomnom-dev nomnom-nats-0 -- nats consumer list MESSAGES

# Get consumer details
kubectl exec -n nomnom-dev nomnom-nats-0 -- nats consumer info MESSAGES workers
```

**What to Look For**:
- Stream name: `MESSAGES`
- Subject pattern: Should match what ingestion API publishes to
- Consumer name: `workers`
- Consumer filter subject: Should match stream subjects
- Pending messages: Number of unprocessed messages
- Delivered messages: Number of messages sent to consumer

### Step 2: Verify Ingestion API Publishing Behavior

**Goal**: Confirm which NATS subject the ingestion API publishes to

**Commands**:
```bash
# Check ingestion API logs for publishing activity
kubectl logs -n nomnom-dev deployment/nomnom-ingestion-api --tail=50

# Port-forward and send test message with verbose output
kubectl port-forward -n nomnom-dev svc/nomnom-ingestion-api 8080:8080 &
curl -v -X POST http://localhost:8080/ingest \
  -H "Content-Type: application/json" \
  -d @/tmp/test_order_with_lineitems.json
```

**What to Look For**:
- Log lines indicating NATS publish operations
- Subject name used for publishing (e.g., `messages.Order`, `MESSAGES.Order`, etc.)
- Error messages related to NATS connection or publishing
- Response body from API (should contain message ID or status)

### Step 3: Verify Worker Consumer Subscription

**Goal**: Confirm worker is subscribed to correct NATS subject

**Commands**:
```bash
# Check worker environment variables
kubectl describe pod -n nomnom-dev -l app=nomnom-worker

# Check worker startup behavior
kubectl get pod -n nomnom-dev -l app=nomnom-worker -o yaml | grep -A 20 "env:"

# Look for NATS connection logs in worker
kubectl exec -n nomnom-dev deployment/nomnom-worker -- cat /proc/1/fd/1 2>/dev/null || echo "stdout not accessible"
```

**What to Look For**:
- `NATS_URL`: Should be `nats://nomnom-nats-client:4222`
- `NATS_STREAM`: Should be `MESSAGES`
- `NATS_CONSUMER`: Should be `workers`
- Any startup logs indicating consumer binding

### Step 4: Check Message Status Table Writes

**Goal**: Verify if ingestion API is writing to `message_status` table

**Commands**:
```bash
# Port-forward to PostgreSQL
kubectl port-forward -n nomnom-dev svc/nomnom-postgres 5432:5432 &

# Query message_status table
psql postgresql://postgres:postgres@localhost:5432/nomnom -c "
  SELECT id, entity_type, status, created_at, updated_at
  FROM message_status
  ORDER BY created_at DESC
  LIMIT 10;
"

# Count total messages
psql postgresql://postgres:postgres@localhost:5432/nomnom -c "
  SELECT status, COUNT(*)
  FROM message_status
  GROUP BY status;
"
```

**What to Look For**:
- Recent `message_status` entries with timestamps matching test sends
- Status progression: `pending` → `processing` → `completed` or `failed`
- Any entries stuck in `pending` status

### Step 5: Enable Debug Logging

**Goal**: Add temporary debug output to see worker behavior

**Options**:

**Option A: Run Worker Locally**
```bash
# Build worker locally with debug output
cd /tmp/tpch-worker-derived2

# Set environment variables
export RUST_LOG=debug
export NATS_URL=nats://localhost:4222  # Port-forwarded
export NATS_STREAM=MESSAGES
export NATS_CONSUMER=workers
export DATABASE_URL=postgresql://postgres:postgres@localhost:5432/nomnom

# Port-forward NATS
kubectl port-forward -n nomnom-dev svc/nomnom-nats-client 4222:4222 &

# Port-forward PostgreSQL
kubectl port-forward -n nomnom-dev svc/nomnom-postgres 5432:5432 &

# Run worker
cargo run
```

**Option B: Add Debug Statements to Generated Code**
```bash
# Modify src/codegen/worker/main_rs.rs to add extra debug statements
# Regenerate worker with debug output
cd /Users/bogdanstate/nomnom
./target/debug/nomnom generate-worker \
  --entity-dir config/examples/tpch/entities \
  --output-dir /tmp/tpch-worker-debug

# Rebuild and redeploy
cd /tmp/tpch-worker-debug
docker build -t localhost:5001/nomnom-worker:debug .
docker push localhost:5001/nomnom-worker:debug
kubectl set image -n nomnom-dev deployment/nomnom-worker worker=localhost:5001/nomnom-worker:debug
```

### Step 6: Trace a Single Test Message

**Goal**: Follow one message through entire pipeline

**Process**:
1. Note current NATS message count
2. Note current `message_status` table count
3. Send single test message via curl
4. Verify message appears in NATS stream
5. Verify message appears in `message_status` table
6. Verify worker picks up message
7. Verify database insertions (orders, order_line_items)

**Commands**:
```bash
# 1. Baseline
kubectl exec -n nomnom-dev nomnom-nats-0 -- nats stream info MESSAGES | grep "Messages:"
psql postgresql://postgres:postgres@localhost:5432/nomnom -c "SELECT COUNT(*) FROM message_status;"

# 2. Send message
curl -X POST http://localhost:8080/ingest \
  -H "Content-Type: application/json" \
  -d @/tmp/test_order_with_lineitems.json

# 3. Check NATS
kubectl exec -n nomnom-dev nomnom-nats-0 -- nats stream info MESSAGES | grep "Messages:"

# 4. Check message_status
psql postgresql://postgres:postgres@localhost:5432/nomnom -c "
  SELECT * FROM message_status ORDER BY created_at DESC LIMIT 1;
"

# 5. Check orders table
psql postgresql://postgres:postgres@localhost:5432/nomnom -c "
  SELECT * FROM orders WHERE order_key = 'ORD-DERIVED-TEST-001';
"

# 6. Check order_line_items table
psql postgresql://postgres:postgres@localhost:5432/nomnom -c "
  SELECT * FROM order_line_items WHERE order_key = 'ORD-DERIVED-TEST-001';
"
```

## Hypothesis: Most Likely Root Causes

### Hypothesis 1: Subject Mismatch
**Probability**: HIGH

**Description**: Ingestion API publishes to one subject (e.g., `messages.Order`) but worker/stream expects different subject (e.g., `MESSAGES.Order`).

**How to Test**: Check ingestion API logs and NATS stream configuration for subject patterns.

**How to Fix**: Update either ingestion API or worker configuration to use consistent subject naming.

### Hypothesis 2: Consumer Not Auto-Created
**Probability**: MEDIUM

**Description**: Worker expects consumer `workers` to exist but it was never created or was deleted.

**How to Test**: Run `nats consumer list MESSAGES` to see if `workers` consumer exists.

**How to Fix**: Manually create consumer with correct filter subjects, or update worker to create consumer if not exists.

### Hypothesis 3: Message Status Table Not Written
**Probability**: MEDIUM

**Description**: Ingestion API accepts message but fails to write to `message_status` table before publishing to NATS.

**How to Test**: Check ingestion API logs for database errors; query `message_status` table after sending message.

**How to Fix**: Fix database connection or error handling in ingestion API.

### Hypothesis 4: Logging Configuration Issue
**Probability**: LOW (doesn't explain message flow issue)

**Description**: Worker logs not appearing in kubectl logs due to stdout/stderr configuration in container.

**How to Test**: Run worker locally or exec into container to check /proc/1/fd/1.

**How to Fix**: Update Dockerfile to ensure proper stdout/stderr handling.

## Success Criteria

Message flow is fixed when:
1. ✅ Message sent via curl appears in `message_status` table as `pending`
2. ✅ Worker picks up message (status changes to `processing`)
3. ✅ Order record inserted into `orders` table
4. ✅ OrderLineItems extracted and inserted into `order_line_items` table
5. ✅ Message status updated to `completed`
6. ✅ Worker logs show derived entity processing

## Files to Check

- `/Users/bogdanstate/nomnom/src/codegen/ingestion_server/handlers_rs.rs` - API message publishing logic
- `/tmp/tpch-worker-derived2/src/main.rs` - Worker NATS consumer setup
- Kubernetes manifests for NATS stream/consumer configuration
- Database schema verification

## Next Session Recovery

If debugging is interrupted, resume by:
1. Reading this plan
2. Checking which hypothesis was being tested
3. Continuing from Step 1 if starting fresh

## Notes

- Test environment uses Kubernetes with local registry
- NATS cluster is `nomnom-nats` in `nomnom-dev` namespace
- PostgreSQL is `nomnom-postgres` in `nomnom-dev` namespace
- Worker image tag: `localhost:5001/nomnom-worker:derived`
- Test message: `/tmp/test_order_with_lineitems.json`
