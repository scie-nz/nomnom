# Message Flow Debug Findings

## Date: 2025-11-13

## Problem Summary

Messages sent to the ingestion API are not being processed by the worker. The message flow is broken between API and worker.

## Root Cause Identified

**Stream Configuration Mismatch** between Ingestion API and Worker

### Ingestion API Configuration
File: `src/codegen/ingestion_server/nats_client_rs.rs`

- **Line 61**: Stream subjects = `["messages.ingest.>"]`
- **Line 100-101**: Publishes to subject = `"messages.ingest.{entity_type}"` (e.g., `messages.ingest.Order`)

### Worker Configuration
File: Generated in `/tmp/tpch-worker-derived2/src/main.rs`

- **Line 85**: Stream subjects = `["messages.>"]` ⚠️ TOO BROAD
- **Line 103**: Consumer filter = `"messages.ingest.>"` ✅ CORRECT

### Why This Breaks

1. **Conflicting Stream Definitions**: When both services try to create the `MESSAGES` stream with different subject patterns, NATS JetStream will:
   - Keep the first configuration that was created
   - Reject subsequent attempts to modify the stream config
   - Cause silent failures or errors in logs

2. **Race Condition**: Whichever service (API or worker) starts first will create the stream with its configuration. The other service will then fail to reconfigure it.

3. **Impact**:
   - If **API starts first**: Stream has subjects `["messages.ingest.>"]` - Worker's attempt to set `["messages.>"]` fails
   - If **Worker starts first**: Stream has subjects `["messages.>"]` - API publishes work, but the broader pattern may cause issues with DLQ routing

## Evidence

1. **Database Query**:
   ```sql
   SELECT message_id, entity_type, status, received_at
   FROM message_status
   ORDER BY received_at DESC LIMIT 10;
   ```

   **Result**: Only 3 old messages from Nov 12, 18:19 UTC. No new messages appearing.

2. **Ingestion API Logs**: Empty (no output via `kubectl logs`)

3. **Port Conflicts**: Multiple processes listening on port 8080 (PIDs: 31694, 85565, 90057)

4. **Worker Status**: Running successfully for 133+ minutes with no crashes

5. **NATS Connections**: 2 connections active (API + Worker confirmed connected)

## Fix Required

### Location
File: `src/codegen/worker/main_rs.rs`
Function: `generate_main_rs()`
Approximate line: Line generating stream configuration

### Change Needed

**BEFORE (Incorrect)**:
```rust
subjects: vec!["messages.>".to_string()],
```

**AFTER (Correct)**:
```rust
subjects: vec!["messages.ingest.>".to_string()],
```

### Why This Fixes It

1. **Consistent Configuration**: Both API and Worker will create/use streams with identical subject patterns
2. **No Race Condition**: Either service can create the stream first, and the other will successfully use it
3. **Proper Isolation**: The `messages.ingest.>` pattern only captures ingestion messages, not DLQ messages (`messages.dlq.>`)
4. **Consumer Filter Still Works**: The worker's consumer filter `"messages.ingest.>"` will continue to work correctly

## Implementation Steps

1. **Modify Worker Generator**:
   ```rust
   // In src/codegen/worker/main_rs.rs
   // Find the stream configuration generation
   // Change:
   writeln!(output, "            subjects: vec![\"messages.>\".to_string()],")?;
   // To:
   writeln!(output, "            subjects: vec![\"messages.ingest.>\".to_string()],")?;
   ```

2. **Regenerate Worker**:
   ```bash
   cd /Users/bogdanstate/nomnom
   cargo build
   ./target/debug/nomnom generate-worker \
     --entity-dir config/examples/tpch/entities \
     --output-dir /tmp/tpch-worker-fixed
   ```

3. **Rebuild Docker Image**:
   ```bash
   cd /tmp/tpch-worker-fixed
   docker build -t localhost:5001/nomnom-worker:fixed .
   docker push localhost:5001/nomnom-worker:fixed
   ```

4. **Delete NATS Stream** (to clear old configuration):
   ```bash
   # Use NATS CLI or restart NATS pod with fresh storage
   kubectl delete pod nomnom-nats-0 -n nomnom-dev
   kubectl delete pvc data-nomnom-nats-0 -n nomnom-dev
   ```

5. **Deploy Fixed Worker**:
   ```bash
   kubectl set image -n nomnom-dev deployment/nomnom-worker \
     worker=localhost:5001/nomnom-worker:fixed
   ```

6. **Restart Ingestion API** (optional, to recreate stream):
   ```bash
   kubectl rollout restart -n nomnom-dev deployment/nomnom-ingestion-api
   ```

7. **Test Message Flow**:
   ```bash
   curl -X POST http://localhost:8080/ingest/message \
     -H "Content-Type: application/json" \
     -d @/tmp/test_order_with_lineitems.json

   # Check message_status table
   kubectl exec -n nomnom-dev nomnom-postgres-0 -- \
     psql -U postgres -d nomnom -c \
     "SELECT * FROM message_status ORDER BY received_at DESC LIMIT 1;"

   # Check orders table
   kubectl exec -n nomnom-dev nomnom-postgres-0 -- \
     psql -U postgres -d nomnom -c \
     "SELECT * FROM orders WHERE order_key = 'ORD-DERIVED-TEST-001';"

   # Check order_line_items table
   kubectl exec -n nomnom-dev nomnom-postgres-0 -- \
     psql -U postgres -d nomnom -c \
     "SELECT * FROM order_line_items WHERE order_key = 'ORD-DERIVED-TEST-001';"
   ```

## Success Criteria After Fix

- ✅ Message appears in `message_status` table with status `accepted`
- ✅ Status updates to `processing`, then `completed`
- ✅ Order record inserted into `orders` table
- ✅ OrderLineItems (2) inserted into `order_line_items` table
- ✅ Worker logs show "Inserted 2 OrderLineItems for Order ORD-DERIVED-TEST-001"
- ✅ Ingestion API logs show "Message {uuid} queued for processing"

## Related Files

- Ingestion API NATS Client: `src/codegen/ingestion_server/nats_client_rs.rs`
- Worker Generator: `src/codegen/worker/main_rs.rs`
- Generated Worker: `/tmp/tpch-worker-derived2/src/main.rs`
- Test Message: `/tmp/test_order_with_lineitems.json`

## Debugging History

- Checked NATS stream configuration (NATS CLI not available in pod)
- Reviewed ingestion API code generation - found subject pattern `messages.ingest.>`
- Reviewed worker code generation - found subject pattern `messages.>` (mismatch!)
- Checked database - confirmed no new messages being written
- Verified both services are running and connected to NATS
- Identified root cause: Stream configuration conflict

## Next Steps

1. Apply the fix to `src/codegen/worker/main_rs.rs`
2. Regenerate and redeploy worker
3. Clear NATS stream state
4. Test end-to-end message flow
5. Verify derived entity processing works correctly
