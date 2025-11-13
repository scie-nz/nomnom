# Plan: Implement Derived Entity Processing in Worker

## Executive Summary

The worker currently only processes **root entities** (e.g., Order) and completely ignores **derived entities** (e.g., OrderLineItem). This means that when an Order message with `line_items` array is received, the Order is inserted into the database, but the OrderLineItems are never extracted or persisted.

**Impact**: Missing data in the database - only parent entities are stored, not their child/derived entities.

## Problem Statement

### Current Behavior

When the worker receives this message:
```json
{
  "entity_type": "Order",
  "order_key": "ORD-002",
  "line_items": [
    {"line_number": 1, "part_key": "PART-100", "quantity": 20, ...},
    {"line_number": 2, "part_key": "PART-200", "quantity": 15, ...}
  ]
}
```

**What happens**:
1. ✅ Order is parsed and inserted into `orders` table
2. ❌ OrderLineItems are **ignored** - `order_line_items` table remains empty

### Root Cause

In `/Users/bogdanstate/nomnom/src/codegen/worker/main_rs.rs:282-289`:

```rust
for entity in entities {
    // Only include root entities that are persistent
    if !entity.is_root() || !entity.is_persistent() || entity.is_abstract {
        continue;  // ← Skips ALL derived entities!
    }
    // ...generate match arm for root entity only
}
```

This loop generates `match` arms **only for root entities**. Derived entities are skipped entirely.

## Entity Configuration Analysis

### Order Entity (Root)
```yaml
entity:
  name: Order
  source_type: root
  fields:
    - name: line_items
      type: List[Object]  # Array of line items
```

### OrderLineItem Entity (Derived)
```yaml
entity:
  name: OrderLineItem
  source_type: derived
  repeated_for:
    entity: Order           # Parent entity
    field: line_items       # Array field in parent
    each_known_as: item     # Variable name for each item

  fields:
    # Copy from parent Order
    - name: order_key
      computed_from:
        transform: copy_field
        sources:
          - source: Order
            field: order_key

    # Extract from JSON item object
    - name: line_number
      computed_from:
        transform: json_get_int
        sources:
          - item
        args:
          field: "line_number"

    - name: part_key
      computed_from:
        transform: json_get_string
        sources:
          - item
        args:
          field: "part_key"
    # ... more fields
```

## Required Changes

### Phase 1: Extend Parser to Store Root Entity Data

**File**: `src/codegen/worker/parsers_rs.rs`

**Change**: When parsing a root entity, also preserve the raw JSON value for use by derived entities.

Currently, the parser returns:
```rust
Ok((entity_name, ParsedMessage))
```

Should return:
```rust
Ok((entity_name, ParsedMessage, raw_json_value))
```

This allows derived entity processors to access the parent's array fields.

### Phase 2: Generate Derived Entity Extractors

**File**: `src/codegen/worker/main_rs.rs`

**Current flow** (process_message function):
```rust
match parsed {
    ParsedMessage::Order(msg) => {
        // Insert Order
        diesel::sql_query("INSERT INTO orders ...").execute(&mut conn)?;
        Ok(())
    }
}
```

**Required flow**:
```rust
match parsed {
    ParsedMessage::Order(msg) => {
        // 1. Insert Order
        diesel::sql_query("INSERT INTO orders ...").execute(&mut conn)?;

        // 2. Process derived entities
        process_order_derived_entities(&msg, &raw_json, &mut conn)?;

        Ok(())
    }
}
```

### Phase 3: Implement Transform Functions

**File**: `src/codegen/worker/transforms_rs.rs` (new file)

Generate helper functions for each transform type:

```rust
// Extract string field from JSON object
fn json_get_string(obj: &serde_json::Value, field: &str) -> Result<String, AppError> {
    obj.get(field)
        .and_then(|v| v.as_str())
        .map(|s| s.to_string())
        .ok_or_else(|| AppError::InvalidField(field.to_string()))
}

// Extract optional integer field
fn json_get_optional_int(obj: &serde_json::Value, field: &str) -> Option<i32> {
    obj.get(field)
        .and_then(|v| v.as_i64())
        .map(|x| x as i32)
}

// Copy field from parent entity
fn copy_field_string(parent_value: &str) -> String {
    parent_value.to_string()
}

// ... more transforms
```

### Phase 4: Generate Derived Entity Processors

**File**: `src/codegen/worker/main_rs.rs`

For each root entity that has derived entities, generate a processor function:

```rust
fn process_order_derived_entities(
    order: &OrderMessage,
    raw_json: &serde_json::Value,
    conn: &mut PgConnection,
) -> Result<(), AppError> {
    // Extract line_items array from raw JSON
    let line_items = raw_json
        .get("line_items")
        .and_then(|v| v.as_array())
        .ok_or_else(|| AppError::InvalidFormat("Missing line_items array".to_string()))?;

    // Process each line item
    for item in line_items {
        // Extract fields using transforms
        let order_key = order.order_key.clone(); // copy from parent
        let line_number = json_get_int(item, "line_number")?;
        let part_key = json_get_string(item, "part_key")?;
        let supplier_key = json_get_optional_string(item, "supplier_key");
        let quantity = json_get_int(item, "quantity")?;
        let extended_price = json_get_float(item, "extended_price")?;
        // ... more fields

        // Insert OrderLineItem
        diesel::sql_query(
            r#"INSERT INTO order_line_items
               (order_key, line_number, part_key, supplier_key, quantity, extended_price, ...)
               VALUES ($1, $2, $3, $4, $5, $6, ...)
               ON CONFLICT DO NOTHING"#
        )
        .bind::<Text, _>(&order_key)
        .bind::<Integer, _>(&line_number)
        .bind::<Text, _>(&part_key)
        .bind::<Nullable<Text>, _>(&supplier_key)
        .bind::<Integer, _>(&quantity)
        .bind::<Double, _>(&extended_price)
        // ... more binds
        .execute(conn)?;
    }

    tracing::info!("Inserted {} OrderLineItems for Order {}", line_items.len(), order.order_key);
    Ok(())
}
```

## Implementation Steps

### Step 1: Modify Parser Generator to Preserve Raw JSON

**File**: `src/codegen/worker/parsers_rs.rs`

**Changes**:
1. Update `parse_json` signature to return raw JSON value:
   ```rust
   pub fn parse_json(json_str: &str, entity_type_hint: Option<&str>)
       -> Result<(String, ParsedMessage, serde_json::Value), AppError>
   ```

2. Return the parsed JSON object along with the message:
   ```rust
   if let Ok(msg) = Self::parse_order(obj) {
       return Ok(("Order".to_string(), ParsedMessage::Order(msg), value.clone()));
   }
   ```

### Step 2: Generate Transform Functions Module

**New File**: `src/codegen/worker/transforms_rs.rs`

**Purpose**: Generate a transforms.rs module with helper functions for all transform types used by derived entities.

**Algorithm**:
1. Scan all derived entities
2. Collect all unique `computed_from.transform` values
3. Generate helper function for each transform type:
   - `json_get_string`, `json_get_int`, `json_get_float`
   - `json_get_optional_string`, `json_get_optional_int`, `json_get_optional_float`
   - `copy_field` (various types)

### Step 3: Build Derived Entity Relationship Map

**File**: `src/codegen/worker/mod.rs` (or main_rs.rs)

**Purpose**: Build a map of root entities → their derived entities

```rust
// Map: parent entity name → list of derived entities
let derived_map: HashMap<String, Vec<&EntityDef>> = entities
    .iter()
    .filter(|e| e.is_derived() && e.is_persistent())
    .filter_map(|e| {
        e.repeated_for.as_ref().map(|rf| (rf.entity.clone(), e))
    })
    .fold(HashMap::new(), |mut acc, (parent, derived)| {
        acc.entry(parent).or_insert_with(Vec::new).push(derived);
        acc
    });
```

### Step 4: Generate Derived Entity Processor Functions

**File**: `src/codegen/worker/main_rs.rs`

**For each root entity that has derived entities**:

Generate a function like:
```rust
fn process_{root_entity_lower}_derived_entities(
    {root_entity_lower}: &{RootEntity}Message,
    raw_json: &serde_json::Value,
    conn: &mut PgConnection,
) -> Result<(), AppError>
```

**Function body**:
1. For each derived entity with `repeated_for` pointing to this root:
   - Extract array field from raw JSON
   - Iterate through array items
   - For each item:
     - Extract fields using transforms
     - Insert into database

### Step 5: Update Match Arms to Call Derived Processors

**File**: `src/codegen/worker/main_rs.rs`

**Current**:
```rust
match parsed {
    ParsedMessage::Order(msg) => {
        diesel::sql_query("INSERT INTO orders ...").execute(&mut conn)?;
        Ok(())
    }
}
```

**Updated**:
```rust
match (parsed, raw_json) {  // ← Now pattern match on both
    (ParsedMessage::Order(msg), raw_json) => {
        // Insert root entity
        diesel::sql_query("INSERT INTO orders ...").execute(&mut conn)?;

        // Process derived entities (if any)
        if has_derived_entities!("Order") {  // ← Check at codegen time
            process_order_derived_entities(&msg, &raw_json, &mut conn)?;
        }

        Ok(())
    }
}
```

### Step 6: Update process_message to Handle Raw JSON

**File**: `src/codegen/worker/main_rs.rs`

**Change**:
```rust
// Before:
let (entity_name, parsed) = MessageParser::parse_json(&envelope.body, envelope.entity_type.as_deref())?;

// After:
let (entity_name, parsed, raw_json) = MessageParser::parse_json(&envelope.body, envelope.entity_type.as_deref())?;
```

## Code Generation Structure

### Files to Modify

1. **`src/codegen/worker/parsers_rs.rs`**
   - Update `parse_json` return type to include raw JSON value
   - Modify all return statements to include `value.clone()`

2. **`src/codegen/worker/main_rs.rs`**
   - Update `process_message` to get raw JSON from parser
   - Update match arms to use `(parsed, raw_json)` tuple
   - Generate derived entity processor functions
   - Call derived processors after inserting root entities

3. **`src/codegen/worker/transforms_rs.rs`** (NEW)
   - Generate transform helper functions
   - Include in worker's `mod.rs`

4. **`src/codegen/worker/mod.rs`**
   - Export transforms module
   - Build derived entity relationship map

## Transform Function Mapping

Map entity field `computed_from.transform` to Rust function:

| Transform | Rust Type | Function Name | Required/Optional |
|-----------|-----------|---------------|-------------------|
| `json_get_string` | `String` | `json_get_string(obj, field)` | Required |
| `json_get_optional_string` | `Option<String>` | `json_get_optional_string(obj, field)` | Optional |
| `json_get_int` | `i32` | `json_get_int(obj, field)` | Required |
| `json_get_optional_int` | `Option<i32>` | `json_get_optional_int(obj, field)` | Optional |
| `json_get_float` | `f64` | `json_get_float(obj, field)` | Required |
| `json_get_optional_float` | `Option<f64>` | `json_get_optional_float(obj, field)` | Optional |
| `copy_field` | `T` | Direct copy from parent | N/A |

## Testing Strategy

### Unit Tests
1. Test transform functions with various inputs
2. Test derived entity processor with sample JSON
3. Test error handling for missing fields

### Integration Tests
1. Send Order with line_items via ingestion API
2. Verify Order inserted into `orders` table
3. Verify OrderLineItems inserted into `order_line_items` table
4. Verify correct parent-child relationships (order_key matches)

### Test Cases

**Test 1: Order with 2 line items**
```json
{
  "entity_type": "Order",
  "order_key": "ORD-TEST-001",
  "customer_key": "CUST-001",
  "total_price": 2500.75,
  "line_items": [
    {"line_number": 1, "part_key": "PART-100", "quantity": 20},
    {"line_number": 2, "part_key": "PART-200", "quantity": 15}
  ]
}
```

**Expected**:
- 1 row in `orders` with order_key = "ORD-TEST-001"
- 2 rows in `order_line_items` with order_key = "ORD-TEST-001"

**Test 2: Order with empty line_items**
```json
{
  "entity_type": "Order",
  "order_key": "ORD-TEST-002",
  "line_items": []
}
```

**Expected**:
- 1 row in `orders`
- 0 rows in `order_line_items`
- No errors

**Test 3: Order with missing optional fields**
```json
{
  "line_items": [
    {"line_number": 1, "part_key": "PART-100", "quantity": 20, "discount": null}
  ]
}
```

**Expected**:
- OrderLineItem inserted with `discount = NULL`

## Error Handling

### Parse Errors
- If `line_items` array is missing → continue (don't fail the entire message)
- If individual item is malformed → log error, skip that item, continue with others
- If required field is missing → log error, skip that item

### Database Errors
- Use `ON CONFLICT DO NOTHING` for idempotent inserts
- If derived entity insert fails → log error but don't fail root entity insert

### Logging Strategy
```rust
tracing::info!("Inserted {} OrderLineItems for Order {}", count, order_key);
tracing::warn!("Skipping malformed line item at index {}: {}", index, error);
tracing::error!("Failed to insert OrderLineItem: {}", error);
```

## Performance Considerations

### Batch Inserts
For better performance with many derived entities, consider batch inserts:

```rust
// Instead of N individual INSERTs:
for item in items {
    INSERT INTO table VALUES (...);
}

// Use a single INSERT with multiple VALUES:
INSERT INTO table VALUES
    (...),
    (...),
    (...);
```

**Trade-off**: More complex error handling (all-or-nothing vs. partial success)

**Recommendation**: Start with individual inserts, optimize later if needed.

## Compatibility Considerations

### Backward Compatibility
- This change is purely additive - existing root entity processing unchanged
- Existing workers without this feature will continue to work (just won't process derived entities)

### Forward Compatibility
- If entity definitions change, regenerate worker
- Transform functions are generated based on entity definitions

## Success Criteria

1. ✅ Worker processes both root and derived entities
2. ✅ OrderLineItems are extracted from Order.line_items and inserted
3. ✅ Database contains complete data (parents + children)
4. ✅ Dashboard displays both Orders and OrderLineItems
5. ✅ No breaking changes to existing functionality
6. ✅ Error handling prevents partial failures from breaking the entire message

## Future Enhancements

### Multi-Level Derivation
Support derived entities from derived entities:
- Order → OrderLineItem → OrderLineItemShipment

### Complex Transforms
Support more sophisticated transforms:
- Aggregations (sum, count, avg)
- Joins across multiple sources
- Conditional logic

### Transform Validation
At code generation time, validate that:
- All referenced transforms exist
- Transform signatures match field types
- No circular dependencies

## Appendix: Generated Code Examples

### Example: transforms.rs
```rust
// Auto-generated transform functions
use crate::error::AppError;
use serde_json::Value;

pub fn json_get_string(obj: &Value, field: &str) -> Result<String, AppError> {
    obj.get(field)
        .and_then(|v| v.as_str())
        .map(|s| s.to_string())
        .ok_or_else(|| AppError::InvalidField(field.to_string()))
}

pub fn json_get_optional_string(obj: &Value, field: &str) -> Option<String> {
    obj.get(field)
        .and_then(|v| if v.is_null() { None } else { v.as_str().map(|s| s.to_string()) })
}

pub fn json_get_int(obj: &Value, field: &str) -> Result<i32, AppError> {
    obj.get(field)
        .and_then(|v| v.as_i64())
        .map(|x| x as i32)
        .ok_or_else(|| AppError::InvalidField(field.to_string()))
}

// ... more functions
```

### Example: Derived Entity Processor
```rust
fn process_order_derived_entities(
    order: &OrderMessage,
    raw_json: &serde_json::Value,
    conn: &mut PgConnection,
) -> Result<(), AppError> {
    use crate::transforms::*;

    // Get line_items array
    let line_items = match raw_json.get("line_items").and_then(|v| v.as_array()) {
        Some(items) => items,
        None => {
            tracing::debug!("No line_items array found for Order {}", order.order_key);
            return Ok(()); // Not an error - order might have no line items
        }
    };

    let mut inserted_count = 0;

    for (index, item) in line_items.iter().enumerate() {
        // Extract fields
        let order_key = order.order_key.clone();

        let line_number = match json_get_int(item, "line_number") {
            Ok(v) => v,
            Err(e) => {
                tracing::warn!("Skipping line item at index {}: {}", index, e);
                continue;
            }
        };

        let part_key = match json_get_string(item, "part_key") {
            Ok(v) => v,
            Err(e) => {
                tracing::warn!("Skipping line item at index {}: {}", index, e);
                continue;
            }
        };

        let supplier_key = json_get_optional_string(item, "supplier_key");
        let quantity = json_get_int(item, "quantity")?;
        let extended_price = json_get_float(item, "extended_price")?;
        let discount = json_get_optional_float(item, "discount");
        let tax = json_get_optional_float(item, "tax");
        let return_flag = json_get_optional_string(item, "return_flag");
        let line_status = json_get_optional_string(item, "line_status");
        let ship_date = json_get_optional_string(item, "ship_date");
        let commit_date = json_get_optional_string(item, "commit_date");
        let receipt_date = json_get_optional_string(item, "receipt_date");

        // Insert OrderLineItem
        diesel::sql_query(
            r#"INSERT INTO order_line_items
               (order_key, line_number, part_key, supplier_key, quantity, extended_price,
                discount, tax, return_flag, line_status, ship_date, commit_date, receipt_date)
               VALUES ($1, $2, $3, $4, $5, $6, $7, $8, $9, $10, $11, $12, $13)
               ON CONFLICT (order_key, line_number) DO NOTHING"#
        )
        .bind::<Text, _>(&order_key)
        .bind::<Integer, _>(&line_number)
        .bind::<Text, _>(&part_key)
        .bind::<Nullable<Text>, _>(&supplier_key)
        .bind::<Integer, _>(&quantity)
        .bind::<Double, _>(&extended_price)
        .bind::<Nullable<Double>, _>(&discount)
        .bind::<Nullable<Double>, _>(&tax)
        .bind::<Nullable<Text>, _>(&return_flag)
        .bind::<Nullable<Text>, _>(&line_status)
        .bind::<Nullable<Text>, _>(&ship_date)
        .bind::<Nullable<Text>, _>(&commit_date)
        .bind::<Nullable<Text>, _>(&receipt_date)
        .execute(conn)?;

        inserted_count += 1;
    }

    tracing::info!("Inserted {} OrderLineItems for Order {}", inserted_count, order.order_key);
    Ok(())
}
```

## Conclusion

This plan provides a comprehensive approach to implementing derived entity processing in the worker. The key insight is that derived entities are not sent as separate messages - they are extracted from arrays within root entity messages using transforms defined in the entity configuration.

By implementing this feature, the nomnom system will support the full entity relationship model, enabling complex data structures like Orders with nested LineItems to be properly persisted and queried.
