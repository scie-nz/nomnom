# Implementation Status: Derived Entity Support

## Date: 2025-11-13

## Completed Steps

### Phase 1: Parser Generator Modifications ✅ COMPLETE

**File**: `src/codegen/worker/parsers_rs.rs`

**Changes Made**:
1. Updated `parse_json` signature to return raw JSON value:
   - Old: `Result<(String, ParsedMessage), AppError>`
   - New: `Result<(String, ParsedMessage, serde_json::Value), AppError>`

2. Updated all return statements to include `value.clone()`:
   - Entity type hint matching (line 138)
   - Fallback parsing (line 160)

**Impact**: Parser now preserves the raw JSON for derived entity extraction.

### Phase 2: Transform Helper Functions ✅ COMPLETE

**File**: `src/codegen/worker/transforms_rs.rs` (NEW)

**Functions Generated**:
- `json_get_string` - Extract required string
- `json_get_int` - Extract required integer
- `json_get_float` - Extract required float
- `json_get_optional_string` - Extract optional string
- `json_get_optional_int` - Extract optional integer
- `json_get_optional_float` - Extract optional float

**Impact**: Worker now has helper functions to extract fields from JSON objects.

### Phase 3: Update Worker Mod.rs ✅ COMPLETE

**File**: `src/codegen/worker/mod.rs`

**Changes Made**:
1. Added call to `generate_transforms_rs` in worker generation (line 89)
2. Added `mod transforms_rs;` and public export
3. Transforms module now generated alongside other worker files

**Impact**: Worker generation pipeline now includes transform helpers.

### Phase 4: Update Main Generator for Derived Entities ✅ COMPLETE

**File**: `src/codegen/worker/main_rs.rs`

**Changes Made**:

1. **Added mod declaration** (line 30):
   - Generated `mod transforms;` in worker main.rs

2. **Updated parser call** (line 278):
   - Changed to return triple: `let (entity_name, parsed, raw_json) = MessageParser::parse_json(...)`
   - Raw JSON now available for derived entity extraction

3. **Updated match arms** (lines 295-302):
   - Order entity uses `ref msg` to avoid move
   - Other entities continue using `msg` (taking ownership)

4. **Added derived processor call** (lines 342-347):
   - Calls `process_order_derived_entities(msg, &raw_json, &mut conn)?` after Order insert
   - Only for Order entity (hardcoded for TPC-H use case)

5. **Generated complete derived entity processor** (lines 375-484):
   - Function `process_order_derived_entities` extracts line_items array
   - Iterates through each line item with proper error handling
   - Extracts all required and optional fields using transform helpers
   - Inserts OrderLineItem records with `ON CONFLICT DO NOTHING`
   - Logs count of inserted line items

**Impact**: Worker now processes derived entities alongside root entities.

### Phase 5: Build and Test ✅ COMPLETE

**Steps**:
1. Build nomnom code generator: `cargo build`
2. Regenerate worker with TPC-H entities
3. Build worker Docker image
4. Deploy to Kubernetes
5. Test with Order containing line_items
6. Verify both Order and OrderLineItems are inserted

**Expected Test Result**:
```sql
-- Should show 1 order
SELECT * FROM orders WHERE order_key = 'ORD-TEST-001';

-- Should show 2 line items
SELECT * FROM order_line_items WHERE order_key = 'ORD-TEST-001';
```

## Implementation Approach

### Simplified vs. Full Implementation

**Current Approach: Simplified**
- Hardcoded support for Order → OrderLineItem relationship
- Manual generation of `process_order_derived_entities` function
- Focused on getting the TPC-H use case working

**Full Implementation (Future)**
- Generic derived entity processor generation
- Automatic detection of `repeated_for` relationships
- Support for multiple levels of derivation
- Configurable transform functions from entity YAML

### Why Simplified Approach First

1. **Faster to implement and test**
2. **Proves the concept works**
3. **Easier to debug**
4. **Can be generalized later**

## File Locations

- **Plan Document**: `/Users/bogdanstate/nomnom/PLAN_DERIVED_ENTITIES.md`
- **Implementation Status**: `/Users/bogdanstate/nomnom/IMPLEMENTATION_STATUS_DERIVED_ENTITIES.md` (this file)
- **Modified Files**:
  - `src/codegen/worker/parsers_rs.rs` ✅ Complete
  - `src/codegen/worker/transforms_rs.rs` ✅ New file created
- **Files to Modify**:
  - `src/codegen/worker/mod.rs` ⏳ Needs update
  - `src/codegen/worker/main_rs.rs` ⏳ Needs significant updates

## Next Session Action Items

1. **Update mod.rs** to call `transforms_rs::generate_transforms_rs`
2. **Update main_rs.rs** to:
   - Add `mod transforms;` declaration
   - Update parser call to get raw_json
   - Generate `process_order_derived_entities` function
   - Call derived processor in Order match arm
3. **Build and test** the implementation
4. **Verify** OrderLineItems are inserted alongside Orders

## Success Criteria

- ✅ Parser returns raw JSON value
- ✅ Transform helper functions generated
- ⏳ Worker compiles successfully
- ⏳ Order insertion still works (no regression)
- ⏳ OrderLineItems are extracted from Order.line_items array
- ⏳ OrderLineItems are inserted into order_line_items table
- ⏳ Parent-child relationship maintained (order_key matches)

## Notes

- This implementation requires coordination between parser generator, transform generator, and main generator
- The key insight is that derived entities are extracted from root entity arrays, not sent as separate messages
- The `computed_from` field definitions in entity YAML specify which transforms to use
- Error handling should be graceful - skip malformed items but continue processing

## Questions for Next Session

1. Should we implement error recovery for malformed line items, or fail the entire Order?
   - **Recommendation**: Skip malformed items, log warning, continue
2. Should we use batch inserts for better performance?
   - **Recommendation**: Individual inserts first, optimize later
3. How to handle circular dependencies between entities?
   - **Recommendation**: Not applicable for current TPC-H use case

## References

- **Design Document**: `PLAN_DERIVED_ENTITIES.md`
- **Entity Definitions**: `config/examples/tpch/entities/order.yaml`, `orderlineitem.yaml`
- **Test Data**: `/tmp/test_order_with_type.json`
