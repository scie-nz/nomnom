# NATS JetStream Subject Pattern Overlap - Bug Fix

## Problem Summary

The ingestion API code generator created overlapping NATS JetStream subject patterns, causing the ingestion API to crash on startup with error:
```
Error { kind: JetStream(Error { code: 400, err_code: ErrorCode(10065),
description: Some("subjects overlap with an existing stream") })
```

## Root Cause

File: `/Users/bogdanstate/nomnom/src/codegen/ingestion_server/nats_client_rs.rs`

**Before Fix (Line 61):**
- Main stream: `"messages.>"` - Matches ALL messages.* subjects
- DLQ stream: `"messages.dlq.>"` - Matches messages.dlq.* subjects (subset of main)

NATS JetStream rejects overlapping subject patterns between streams.

## Solution

Changed main stream pattern to be non-overlapping:
- Main stream: `"messages.ingest.>"` - Only ingestion messages
- DLQ stream: `"messages.dlq.>"` - Only DLQ messages

This aligns with the existing `publish_message` function (line 100) which already uses:
```rust
format!("messages.ingest.{}", entity_type)
```

## Implementation

**File Modified:** `/Users/bogdanstate/nomnom/src/codegen/ingestion_server/nats_client_rs.rs:61`

```rust
// Before
subjects: vec!["messages.>".to_string()],

// After
subjects: vec!["messages.ingest.>".to_string()],
```

## Additional Fixes Completed

### 1. NATS Storage Limit
- **Issue**: NATS JetStream configured with 1GB storage, but two streams reserved 2GB total
- **Fix**: Increased `max_file` from `1Gi` to `5Gi` in NATS configmap
- **File**: Updated via `kubectl patch configmap nomnom-nats-config`
- **Verification**: NATS logs show "Max Storage: 5.00 GB"

### 2. Worker Table Creation
- **Issue**: Worker only created tables for root entities, not derived entities
- **Fix**: Changed entity filter in code generator from `!entity.is_root()` to `!entity.is_persistent()`
- **File**: `/Users/bogdanstate/nomnom/src/codegen/worker/database_rs.rs`
- **Result**: Worker now creates both `order_line_items` and `orders` tables

### 3. Dashboard Deployment
- **Issue**: Backend port mismatch (3000 vs 8080), frontend OOMKilled
- **Fix**: Updated Helm values.yaml with correct ports and memory limits
- **File**: `/Users/bogdanstate/nomnom/nomnom-helm/values.yaml`
- **Status**: Dashboard backend and frontend running successfully (1/1 Ready)

## Next Steps

1. **Regenerate Ingestion API** with fixed code generator
2. **Create Dev Dockerfile** for ingestion API (similar to dashboard) to speed up iteration
3. **Build and Deploy** new ingestion API image
4. **Test End-to-End** data flow:
   - Send message to ingestion API
   - Verify NATS JetStream receives it
   - Verify worker processes and inserts to database
   - Verify dashboard shows real-time updates

## Current System Status

✅ **NATS**: Running with 5GB storage, fresh streams
✅ **PostgreSQL**: Running with all required tables
✅ **Worker**: Running, tables created successfully
✅ **Dashboard**: Backend and frontend both 1/1 Running
❌ **Ingestion API (new)**: Needs regeneration with fix
✅ **Ingestion API (old)**: Still running with old code

## Testing Plan

After deploying fixed ingestion API:

1. Scale old ingestion API to 0 replicas
2. Verify new ingestion API starts successfully
3. Port-forward ingestion API: `kubectl port-forward -n nomnom-dev svc/nomnom-ingestion-api 8080:8080`
4. Send test OrderLineItem via curl
5. Check NATS streams for message
6. Verify worker logs show processing
7. Query PostgreSQL for inserted data
8. Check dashboard for real-time updates

## Performance Optimization

**Dev Image Strategy** (recommended for ingestion API):
- Current: Full Rust rebuild takes 3-5 minutes
- Solution: Create `Dockerfile.dev` that skips cargo-chef layers
- Benefit: Rebuilds in ~30 seconds for code changes
- Implementation: Copy approach from dashboard Dockerfile.dev

Example structure:
```dockerfile
FROM rust:alpine
RUN apk add --no-cache musl-dev postgresql-dev
WORKDIR /build
COPY . .
RUN cargo build --release
# ... runtime stage
```
