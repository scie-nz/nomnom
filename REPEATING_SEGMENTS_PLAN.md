# Repeating Segments Code Generation Fix

## Problem Statement

The nomnom code generator doesn't properly handle repeating HL7 segments (like DG1 for diagnoses and PR1 for procedures). Currently:

1. Segments are extracted as `Vec<String>` correctly
2. But the generator sets all derived entity fields to `None` with TODO comments
3. No loop is generated to iterate over the segment vector
4. Result: Zero records inserted into database for repeating segments

## Requirements

1. **Parent entities must have a repetition field** - Add to all entities if missing
2. **At most one parent can be repeated** - Validation must fail codegen if violated
3. **Generate loop constructs** - Properly iterate over repeating segments
4. **Extract fields within loops** - Parse each segment individually

## Implementation Plan

### Phase 1: Schema Changes and Validation

#### 1.1 Add Repetition Field to All Parent Entities

**Files to modify:**
- Entity YAML configs in `/home/bogdan/claude-code/ingestion/config/entities/*.yaml`

**Changes:**
- Add `repetition: singleton` or `repetition: repeated` to all entities
- For transient entities derived from single HL7 segments (PID, PV1, MSH, EVN): `singleton`
- For transient entities derived from repeating segments (DG1, PR1): `repeated`
- For derived/permanent entities: Check parent repetition to determine

**Example:**
```yaml
# config/entities/diagnosis.yaml
name: Diagnosis
source_type: hl7v2_segment
repetition: repeated  # <-- ADD THIS
segment_type: DG1
fields:
  - name: diagnosis_code
    # ...
```

#### 1.2 Add YAML Schema Validation

**File to modify:** `/home/bogdan/claude-code/nomnom/src/codegen/types.rs`

**Location:** Add validation to `EntityDef` deserialization

**Problem:** Serde silently ignores unknown fields by default. If users add `repetition_type` instead of `repetition`, it will be ignored, causing silent failures.

**Solution:** Use `#[serde(deny_unknown_fields)]` on structs to fail fast on typos.

**Changes needed:**

```rust
// Add to EntityDef struct (line ~291)
#[derive(Debug, Clone, Serialize, Deserialize, Default)]
#[serde(deny_unknown_fields)]  // <-- ADD THIS
pub struct EntityDef {
    // ... existing fields
}

// Add to FieldDef struct (search for "pub struct FieldDef")
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]  // <-- ADD THIS
pub struct FieldDef {
    // ... existing fields
}

// Add to PersistenceConfig struct
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]  // <-- ADD THIS
pub struct PersistenceConfig {
    // ... existing fields
}

// Add to DatabaseConfig struct
#[derive(Debug, Clone, Deserialize, Serialize)]
#[serde(deny_unknown_fields)]  // <-- ADD THIS
pub struct DatabaseConfig {
    // ... existing fields
}

// Add to ParentDef struct (line ~263)
#[derive(Debug, Clone, Deserialize, Serialize, Default)]
#[serde(deny_unknown_fields)]  // <-- ADD THIS
pub struct ParentDef {
    // ... existing fields
}
```

**Expected error message when typo detected:**
```
Error: unknown field `repetition_type`, expected one of `name`, `source_type`, `repetition`, `parent`, `parents`, ...
```

**Benefits:**
- Catches typos immediately during codegen (not at runtime)
- Prevents silent configuration errors
- Makes valid field names discoverable through error messages

#### 1.3 Add Parent Repetition Validation

**File to modify:** `/home/bogdan/claude-code/nomnom/src/codegen/worker/main_rs.rs`

**Location:** Add validation function before `generate_derived_entity_processing`

**New function:**
```rust
fn validate_parent_repetition(
    derived_entity: &EntityDef,
    entities_by_name: &HashMap<String, &EntityDef>,
) -> Result<(), String> {
    let parents = match &derived_entity.parent {
        Some(parent_name) => vec![parent_name.clone()],
        None => derived_entity.parents.iter().map(|p| p.parent_type.clone()).collect(),
    };

    let mut repeated_parents: Vec<String> = Vec::new();

    for parent_name in &parents {
        if let Some(parent_entity) = entities_by_name.get(parent_name) {
            if parent_entity.repetition.as_ref().map(|r| r == "repeated").unwrap_or(false) {
                repeated_parents.push(parent_name.clone());
            }
        }
    }

    if repeated_parents.len() > 1 {
        return Err(format!(
            "Entity '{}' has multiple repeated parents: {:?}. Only one parent can be repeated.",
            derived_entity.name,
            repeated_parents
        ));
    }

    Ok(())
}
```

**Call site:** In `generate_derived_entity_processing` at the start:
```rust
pub fn generate_derived_entity_processing(
    output: &mut String,
    project_config: &ProjectConfig,
    derived_entity: &EntityDef,
    entities_by_name: &HashMap<String, &EntityDef>,
    transforms: &HashMap<String, Transform>,
) -> Result<(), Box<dyn std::error::Error>> {
    // NEW: Validate parent repetition
    validate_parent_repetition(derived_entity, entities_by_name)
        .map_err(|e| format!("Validation error: {}", e))?;

    // ... rest of function
}
```

### Phase 2: Code Generator Modifications

#### 2.1 Detect Repeating Parent Entity

**File:** `/home/bogdan/claude-code/nomnom/src/codegen/worker/main_rs.rs`

**Location:** Within `generate_derived_entity_processing`, after validation

**New logic:**
```rust
// Determine if this entity has a repeating parent
let (has_repeating_parent, repeating_parent_name, repeating_source_name) = {
    let parents = match &derived_entity.parent {
        Some(parent_name) => vec![parent_name.clone()],
        None => derived_entity.parents.clone().unwrap_or_default(),
    };

    let mut repeating_info = (false, String::new(), String::new());

    for parent_name in parents {
        if let Some(parent_entity) = entities_by_name.get(&parent_name) {
            if parent_entity.repetition.as_ref().map(|r| r == "repeated").unwrap_or(false) {
                // Found repeating parent - determine segment type
                let source_name = if parent_entity.source_type == "hl7v2_segment" {
                    // Use segment_type if available, otherwise derive from entity name
                    parent_entity.segment_type.clone()
                        .unwrap_or_else(|| parent_name.to_uppercase())
                } else {
                    parent_name.clone()
                };

                repeating_info = (true, parent_name.clone(), source_name);
                break;
            }
        }
    }

    repeating_info
};
```

#### 2.2 Generate Loop for Repeating Segments

**Location:** Replace the TODO comment at line 593

**Current code:**
```rust
writeln!(output, "    // Process {} entities", derived_entity.name)?;
// For now, support simple single-entity extraction (not repeating)
// TODO: Add support for repeating derived entities (e.g., from DG1_segments)
writeln!(output, "    // Extract fields from root entity data")?;
```

**New code:**
```rust
writeln!(output, "    // Process {} entities", derived_entity.name)?;

if has_repeating_parent {
    // Generate loop over segments
    let segments_var = format!("{}_segments",
        repeating_source_name.to_uppercase());
    let segment_var = format!("{}_segment",
        repeating_source_name.to_lowercase());

    writeln!(output, "")?;
    writeln!(output, "    // Loop over each {} segment and insert {} records",
        repeating_source_name, derived_entity.name)?;
    writeln!(output, "    for {} in &{} {{", segment_var, segments_var)?;

    // Field extraction will happen inside this loop
    // (see Phase 2.3)

} else {
    writeln!(output, "    // Extract fields from root entity data")?;
}
```

#### 2.3 Extract Fields Within Loop

**Location:** Field instantiation section (currently lines 600-650)

**Modify the field extraction logic to:**

1. **For fields from repeating parent:** Extract from `segment_var` using `extract_from_hl7_segment`
2. **For fields from other parents:** Extract normally (outside loop or cloned inside)

**Example generated code for Diagnosis entity:**

```rust
// Loop over each DG1 segment and insert diagnosis records
for dg1_segment in &hl_7_v_2_message_DG1_segments {
    // Extract diagnosis fields from this specific DG1 segment
    let diagnosis_code = extract_from_hl7_segment(&Some(dg1_segment.clone()), "DG1.3.1").unwrap_or(None);
    let diagnosis_code_text = extract_from_hl7_segment(&Some(dg1_segment.clone()), "DG1.3.2").unwrap_or(None);
    let diagnosis_code_system = extract_from_hl7_segment(&Some(dg1_segment.clone()), "DG1.3.3").unwrap_or(None);
    let diagnosis_datetime = extract_from_hl7_segment(&Some(dg1_segment.clone()), "DG1.5").unwrap_or(None);
    let diagnosis_type_code = extract_from_hl7_segment(&Some(dg1_segment.clone()), "DG1.6").unwrap_or(None);
    let diagnosis_priority = extract_from_hl7_segment(&Some(dg1_segment.clone()), "DG1.15").unwrap_or(None);
    let diagnosing_clinician_id = extract_from_hl7_segment(&Some(dg1_segment.clone()), "DG1.16.1").unwrap_or(None);

    // Extract common fields from other parents (these are same for all iterations)
    let facility_id = filename_f_facilityId.clone();
    let message_date = filename_f_date.clone();
    let event_timestamp = event_type_event_timestamp.clone();
    let patient_identifier = patient_account_patient_identifier.clone();

    // INSERT happens here (once per segment)
    diesel::sql_query(
        r#"INSERT INTO patient_diagnosis_conformant (...) VALUES (...) ON CONFLICT (...) DO NOTHING"#
    )
    .bind::<Nullable<Text>, _>(&facility_id)
    .bind::<Nullable<Text>, _>(&diagnosis_code)
    // ... other binds
    .execute(conn)?;
}
```

**Code generator changes needed:**

```rust
// Within generate_derived_entity_processing
for field in &derived_entity.fields {
    let field_name = format!("{}_{}",
        derived_entity.name.to_lowercase(),
        field.name.to_lowercase());

    if has_repeating_parent {
        // Determine if this field comes from the repeating parent
        let from_repeating_parent = field.source.as_ref()
            .map(|src| src.starts_with(&repeating_source_name))
            .unwrap_or(false);

        if from_repeating_parent {
            // Extract from current segment in loop
            let hl7_path = field.source.as_ref()
                .map(|s| s.as_str())
                .unwrap_or("");

            writeln!(output,
                "        let {}__{} = extract_from_hl7_segment(&Some({}.clone()), \"{}\").unwrap_or(None);",
                derived_entity.name.to_lowercase(),
                field.name.to_lowercase(),
                segment_var,
                hl7_path
            )?;
        } else {
            // Clone from parent entity extracted before loop
            // (implementation depends on transform type)
        }
    } else {
        // Singleton behavior (current logic)
    }
}
```

#### 2.4 Generate INSERT Within Loop

**Location:** INSERT generation section (currently lines 792-849)

**Changes:**
- Wrap INSERT statement with proper indentation if inside loop
- Close loop after INSERT

**New code structure:**
```rust
if has_repeating_parent {
    // INSERT is already inside the loop from Phase 2.2
    // Just need proper indentation (8 spaces instead of 4)

    // Generate INSERT with 8-space indentation
    writeln!(output, "        diesel::sql_query(")?;
    writeln!(output, "            r#\"INSERT INTO {} ({}) VALUES ({}) ON CONFLICT ({}) DO NOTHING\"#",
        table_name, columns_list, placeholders, unicity_fields)?;
    writeln!(output, "        )")?;

    // Generate binds with 8-space indentation
    for (i, field) in fields_to_insert.iter().enumerate() {
        writeln!(output, "        .bind::<Nullable<Text>, _>(&{})", field)?;
    }

    writeln!(output, "        .execute(conn)?;")?;
    writeln!(output, "    }}")?; // Close the for loop

} else {
    // Singleton INSERT (current logic with 4-space indentation)
}
```

### Phase 3: Testing Strategy

#### 3.1 Unit Tests for Validation

**File:** Create `/home/bogdan/claude-code/nomnom/src/codegen/tests/validation_tests.rs`

**Tests:**

1. **YAML validation tests:**
   - **Test unknown field fails:** Entity YAML with typo like `repetition_type` should fail with clear error
   - **Test valid fields succeed:** Entity YAML with all valid fields should deserialize successfully
   - **Test nested unknown fields fail:** Unknown field in `persistence` or `fields` should fail

2. **Parent repetition validation tests:**
   - **Test multiple repeated parents fail:** Entity with two repeated parents should fail validation
   - **Test single repeated parent succeeds:** Entity with one repeated parent should pass
   - **Test no repeated parents succeeds:** Entity with all singleton parents should pass

**Example test code:**

```rust
#[cfg(test)]
mod validation_tests {
    use super::*;

    #[test]
    fn test_unknown_field_in_entity() {
        let yaml = r#"
entity:
  name: TestEntity
  source_type: hl7v2_segment
  repetition_type: singleton  # TYPO - should be 'repetition'
  fields: []
"#;
        let result: Result<EntitySpec, _> = serde_yaml::from_str(yaml);
        assert!(result.is_err());
        assert!(result.unwrap_err().to_string().contains("unknown field"));
    }

    #[test]
    fn test_valid_entity_fields() {
        let yaml = r#"
entity:
  name: TestEntity
  source_type: hl7v2_segment
  repetition: singleton
  fields: []
"#;
        let result: Result<EntitySpec, _> = serde_yaml::from_str(yaml);
        assert!(result.is_ok());
    }
}

#### 3.2 Integration Tests

**File:** `/home/bogdan/claude-code/ingestion/tests/rust/test_diesel_end_to_end.py`

**Test case:**
```python
def test_repeating_segments_diagnosis_procedure():
    """Test that multiple DG1 and PR1 segments create multiple records."""

    # Use existing test message with 3 diagnoses and 2 procedures
    message_path = "../tests/data/canonical_hl7v2/KONZA[NJ][100104]MSH31[URG]MSH7[20211006]PD[20221012111950904]RD[20221012111950849]"

    # Ingest message
    result = ingest_hl7_message(message_path)

    # Query database
    with SessionLocal() as session:
        # Should have exactly 3 diagnosis records
        diagnoses = session.query(PatientDiagnosisConformant).all()
        assert len(diagnoses) == 3

        # Verify diagnosis codes
        diagnosis_codes = {d.diagnosis_code for d in diagnoses}
        assert diagnosis_codes == {'I10', 'E11.9', 'J44.1'}

        # Should have exactly 2 procedure records
        procedures = session.query(PatientProcedureConformant).all()
        assert len(procedures) == 2

        # Verify procedure codes
        procedure_codes = {p.procedure_code for p in procedures}
        assert procedure_codes == {'33533', '43239'}
```

#### 3.3 Manual Verification

**Steps:**
1. Update entity YAML configs with repetition field
2. Regenerate worker: `cd /home/bogdan/claude-code/nomnom && cargo build`
3. Run codegen: `./target/debug/nomnom generate-worker --config /home/bogdan/claude-code/ingestion/config/nomnom.yaml`
4. Inspect generated `/home/bogdan/claude-code/ingestion/hl7-nomnom-parser/worker/src/main.rs`
5. Verify:
   - Loop construct exists around lines 357-417
   - Field extraction uses `extract_from_hl7_segment` on loop variable
   - INSERT is inside loop with proper indentation
   - No TODO comments for diagnosis/procedure fields

### Phase 4: Migration Path

#### 4.1 Update Existing Configurations

**Files to update:**
- `/home/bogdan/claude-code/ingestion/config/entities/diagnosis.yaml`
- `/home/bogdan/claude-code/ingestion/config/entities/procedure.yaml`
- All other entity configs

**Script to verify completeness:**
```bash
# Find all entity configs without repetition field
grep -L "repetition:" config/entities/*.yaml
```

#### 4.2 Backward Compatibility

**Default behavior:** If `repetition` field is missing:
- Assume `singleton` for safety
- Log warning during codegen
- Don't fail (graceful degradation)

**Implementation in types.rs:**
```rust
impl EntityDef {
    pub fn get_repetition(&self) -> &str {
        self.repetition.as_ref()
            .map(|s| s.as_str())
            .unwrap_or_else(|| {
                eprintln!("WARNING: Entity '{}' missing repetition field, assuming singleton", self.name);
                "singleton"
            })
    }
}
```

## Implementation Order

1. **Day 1:**
   - Phase 1.1 - Add repetition field to all entity YAMLs
   - Phase 1.2 - Add `#[serde(deny_unknown_fields)]` to type definitions
   - Phase 1.3 - Add parent repetition validation logic
   - Test YAML validation with intentional typos

2. **Day 2:**
   - Phase 2.1-2.2 - Detect repeating parent and generate loop
   - Phase 2.3 - Field extraction within loop
   - Write unit tests for validation

3. **Day 3:**
   - Phase 2.4 - INSERT generation within loop
   - Phase 3.1-3.2 - Unit tests and integration tests
   - Phase 3.3 - Manual verification of generated code

4. **Day 4:**
   - Phase 4 - Migration and backward compatibility
   - Documentation updates
   - Final end-to-end testing

## Success Criteria

1. ✅ **YAML validation enabled:** All type structs have `#[serde(deny_unknown_fields)]`
2. ✅ **Typo detection works:** Entity YAML with unknown field fails with clear error message
3. ✅ **All entity configs have `repetition` field:** Every parent entity specifies singleton/repeated
4. ✅ **Validation fails if multiple parents are repeated:** Codegen errors on invalid configs
5. ✅ **Generated worker has `for segment in segments { ... }` loops:** For DG1 and PR1
6. ✅ **Diagnosis records inserted:** 3 records for test message (I10, E11.9, J44.1)
7. ✅ **Procedure records inserted:** 2 records for test message (33533, 43239)
8. ✅ **No TODO comments in generated code:** For repeating entities
9. ✅ **All tests pass:** Unit tests, integration tests, manual verification

## Risk Mitigation

**Risk:** Silent configuration errors (typos in YAML)
**Mitigation:**
- Add `#[serde(deny_unknown_fields)]` to all entity structs
- Codegen fails immediately with clear error message on unknown fields
- Prevents hours of debugging runtime issues from config typos

**Risk:** Breaking existing singleton entity processing
**Mitigation:**
- Keep singleton code path unchanged
- Use `if has_repeating_parent { ... } else { ... }` branching
- Test both singleton and repeated entities

**Risk:** Performance degradation with nested loops
**Mitigation:**
- Only one level of looping (validated by allowing only one repeated parent)
- Database uses ON CONFLICT for efficient deduplication

**Risk:** Complex parent relationships (e.g., repeated parent of repeated parent)
**Mitigation:**
- Validation prevents multiple repeated parents
- Flattens hierarchy to single loop level

**Risk:** Breaking changes from strict YAML validation
**Mitigation:**
- Review all existing entity YAMLs for unknown fields before enabling
- Update documentation with complete list of valid fields
- Error messages include list of expected field names for discoverability
