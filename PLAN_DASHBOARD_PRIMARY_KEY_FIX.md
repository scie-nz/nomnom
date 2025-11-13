# Plan: Fix Dashboard Code Generation to Support Dynamic Primary Keys

## Problem Statement

**Current Issue**: Dashboard backend code generation hardcodes `id` as the primary key in SQL queries, but the worker generates tables using natural keys (e.g., `order_key`) as primary keys.

**Impact**:
- Dashboard crashes with "column 'id' does not exist" errors
- Real-time polling fails
- Initial data loading fails
- No data visible in dashboard UI

## Root Cause Analysis

**Worker behavior** (src/codegen/worker/database_rs.rs:105):
```rust
// Add primary key (first field typically)
if !field_lines.is_empty() {
    field_lines[0] = format!("{} PRIMARY KEY", field_lines[0]);
}
```
- Uses **first field** from `persistence.field_overrides` as PRIMARY KEY
- For Order entity: `order_key` becomes the primary key
- For OrderLineItem: would be the first field

**Dashboard behavior** (src/codegen/dashboard/axum_backend.rs):
- Hardcodes `id` in SQL queries at multiple locations
- Assumes auto-increment integer surrogate keys
- Incompatible with natural key strategy

## Solution Approach

### Phase 1: Update Entity Configuration ✅ (Partially Complete)

**Status**: EntityConfig struct updated with `primary_key` field

**Remaining Work**:
1. Extract primary key in polling task spawning loop
2. Pass primary key to `poll_entity_table()` function
3. Pass primary key in WebSocket initial data loading

### Phase 2: Update Generated Code Templates

**Files to modify**: `src/codegen/dashboard/axum_backend.rs`

#### 2.1: Update `start_all_polling_tasks()` Generation (Lines 303-326)

**Current**:
```rust
writeln!(output, "            poll_entity_table(")?;
writeln!(output, "                \"{}\".to_string(),", table_name)?;
writeln!(output, "                \"{}\".to_string(),", entity_name)?;
writeln!(output, "                state,")?;
```

**Change to**:
```rust
// Extract primary key for this entity
let primary_key = if let Some(ref persistence) = entity.persistence {
    if let Some(first_field) = persistence.field_overrides.first() {
        first_field.name.to_lowercase()
    } else {
        "id".to_string()
    }
} else {
    "id".to_string()
};

writeln!(output, "            poll_entity_table(")?;
writeln!(output, "                \"{}\".to_string(),", table_name)?;
writeln!(output, "                \"{}\".to_string(),", entity_name)?;
writeln!(output, "                \"{}\".to_string(),", primary_key)?;  // NEW
writeln!(output, "                state,")?;
```

#### 2.2: Update `poll_entity_table()` Signature (Lines 332-338)

**Current**:
```rust
writeln!(output, "async fn poll_entity_table(")?;
writeln!(output, "    table: String,")?;
writeln!(output, "    entity_name: String,")?;
writeln!(output, "    state: AppState,")?;
```

**Change to**:
```rust
writeln!(output, "async fn poll_entity_table(")?;
writeln!(output, "    table: String,")?;
writeln!(output, "    entity_name: String,")?;
writeln!(output, "    primary_key: String,")?;  // NEW
writeln!(output, "    state: AppState,")?;
```

#### 2.3: Update Poll Query Generation (Lines 340-348)

**Current**:
```rust
writeln!(output, "        let query = format!(")?;
writeln!(output, "            \"SELECT * FROM {{}} WHERE id > $1 ORDER BY id ASC LIMIT $2\",")?;
writeln!(output, "            table")?;
```

**Change to**:
```rust
writeln!(output, "        let query = format!(")?;
writeln!(output, "            \"SELECT * FROM {{}} WHERE {{}} > $1 ORDER BY {{}} ASC LIMIT $2\",")?;
writeln!(output, "            table, primary_key, primary_key")?;
```

#### 2.4: Update Max ID Extraction (Lines 350-356)

**Current**:
```rust
writeln!(output, "                    if let Some(max_id) = rows.iter()")?;
writeln!(output, "                        .filter_map(|row| row.try_get::<i64, _>(\"id\").ok())")?;
writeln!(output, "                        .max()")?;
```

**Challenge**: Primary key might be VARCHAR (e.g., `order_key`), not integer

**Solution**: Use String comparison for VARCHAR keys
```rust
writeln!(output, "                    // Try to get max value from primary key column")?;
writeln!(output, "                    // Note: For string keys, we track last seen value")?;
writeln!(output, "                    if let Some(last_row) = rows.last() {{")?;
writeln!(output, "                        // Try integer first, fall back to string")?;
writeln!(output, "                        if let Ok(pk_val) = last_row.try_get::<i64, _>(&primary_key) {{")?;
writeln!(output, "                            state.last_ids.write().await.insert(table.clone(), pk_val);")?;
writeln!(output, "                        }} else if let Ok(pk_str) = last_row.try_get::<String, _>(&primary_key) {{")?;
writeln!(output, "                            // For string keys, use hash as tracking ID")?;
writeln!(output, "                            let hash_id = pk_str.bytes().fold(0i64, |acc, b| acc.wrapping_mul(31).wrapping_add(b as i64));")?;
writeln!(output, "                            state.last_ids.write().await.insert(table.clone(), hash_id);")?;
writeln!(output, "                        }}")?;
writeln!(output, "                    }}")?;
```

#### 2.5: Update WebSocket Initial Data Query (Lines ~462)

**Current**:
```rust
writeln!(output, "        let query = format!(")?;
writeln!(output, "            \"SELECT * FROM {{}} ORDER BY id DESC LIMIT $1\",")?;
writeln!(output, "            entity.table")?;
```

**Change to**:
```rust
writeln!(output, "        let query = format!(")?;
writeln!(output, "            \"SELECT * FROM {{}} ORDER BY {{}} DESC LIMIT $1\",")?;
writeln!(output, "            entity.table, entity.primary_key")?;
```

### Phase 3: Handle String vs Integer Primary Keys

**Challenge**: The dashboard uses `HashMap<String, i64>` to track last seen IDs, but string primary keys can't be directly stored as i64.

**Options**:

**Option A**: Change to `HashMap<String, String>` (RECOMMENDED)
- More flexible, supports any primary key type
- Requires updating AppState definition in main.rs generation
- Comparison logic changes from `>` to string comparison

**Option B**: Use hash of string keys (CURRENT APPROACH)
- Keeps existing i64 storage
- Risk of collisions (low but nonzero)
- Simpler migration path

**Recommendation**: Start with Option B (hash), document Option A for future improvement

### Phase 4: Testing Strategy

#### 4.1: Regenerate Dashboard
```bash
cd /Users/bogdanstate/nomnom
./target/debug/nomnom generate-dashboard \
  --entities config/examples/tpch/entities \
  --output /tmp/tpch-dashboard-fixed \
  --database postgresql \
  --backend axum
```

#### 4.2: Verify Generated Code
Check `/tmp/tpch-dashboard-fixed/src/`:
- ✅ `config.rs`: EntityConfig includes `primary_key: "order_key"`
- ✅ `polling.rs`: poll_entity_table accepts primary_key parameter
- ✅ `polling.rs`: SQL uses dynamic primary key in WHERE/ORDER BY
- ✅ `websocket.rs`: Initial query uses entity.primary_key

#### 4.3: Build and Test
```bash
cd /tmp/tpch-dashboard-fixed
docker build -f Dockerfile.backend.dev -t localhost:5001/nomnom-dashboard-backend:fixed .
docker push localhost:5001/nomnom-dashboard-backend:fixed
kubectl set image deployment/nomnom-dashboard-backend dashboard-backend=localhost:5001/nomnom-dashboard-backend:fixed -n nomnom-dev
```

#### 4.4: Verify Functionality
```bash
# Check pod starts without crashes
kubectl get pods -n nomnom-dev | grep dashboard-backend

# Check logs for successful polling
kubectl logs -f <dashboard-pod> -n nomnom-dev

# Expected: No more "column 'id' does not exist" errors
```

## Implementation Order

1. ✅ **EntityConfig struct** - Add primary_key field (DONE)
2. ✅ **EntityConfig instantiation** - Extract primary_key from entity (DONE)
3. **polling.rs spawning** - Pass primary_key to poll_entity_table()
4. **poll_entity_table signature** - Add primary_key parameter
5. **Poll SQL query** - Use dynamic primary_key in WHERE/ORDER BY
6. **Max ID tracking** - Handle string vs integer keys (use hash)
7. **websocket.rs** - Use entity.primary_key in initial data query
8. **Regenerate and test** - Build, deploy, verify

## Potential Issues & Mitigations

### Issue 1: Hash Collisions with String Keys
**Mitigation**: Use crypto hash (SHA256 first 8 bytes) instead of simple multiplicative hash

### Issue 2: OrderLineItem Table Missing
**Separate Issue**: Worker only creates tables for root entities, not derived
**Fix Required**: Update worker code generation to create derived entity tables

### Issue 3: Performance with String Keys
**Impact**: String comparison slower than integer
**Mitigation**: Add index on primary key column (already done by PRIMARY KEY constraint)

## Success Criteria

- [ ] Dashboard backend starts without CrashLoopBackOff
- [ ] Logs show successful polling: "Starting polling for table: orders"
- [ ] No "column 'id' does not exist" errors
- [ ] WebSocket initial data loads successfully
- [ ] Real-time updates appear when worker processes messages
- [ ] All entities (Order, OrderLineItem, Customer, Product) visible in dashboard

## Estimated Effort

- Code changes: ~30-40 lines across 6 locations
- Testing: Regenerate → Build (3-4 min) → Deploy → Verify
- Total time: **15-20 minutes** (assuming no unexpected issues)

## Files to Modify

1. `src/codegen/dashboard/axum_backend.rs` - Main code generation template
   - Lines 303-326: Polling task spawning
   - Lines 332-338: Function signature
   - Lines 340-348: SQL query generation
   - Lines 350-356: Max ID tracking
   - Lines ~462: WebSocket initial query

## Notes

- This fix addresses the immediate crash issue
- Future enhancement: Support composite primary keys
- Consider migrating to `HashMap<String, String>` in next major version
