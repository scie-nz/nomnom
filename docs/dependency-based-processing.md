# Design Document: Dependency-Based Entity Processing

## Problem Statement

The current entity processing architecture always starts from `Hl7v2MessageFile` and re-extracts all dependency entities for each derived entity. For example, when processing `Anesthesiologist`, the current code:

1. Extracts `Filename` fields from `Hl7v2MessageFile`
2. Extracts `Facility` fields from `Filename`
3. Extracts `Hl7v2Message` fields from `Hl7v2MessageFile`
4. Extracts `PatientVisit` fields from `Hl7v2Message`
5. Extracts `Procedure` fields from `Hl7v2Message` segments
6. Finally extracts `Anesthesiologist` fields from `Procedure`

This pattern is repeated for **every** persistent entity, causing massive redundant extraction work.

### Current Waste Example

For a message with 3 PR1 segments and 5 provider entities derived from procedures:
- `Filename` extracted 5 times (once per provider type)
- `Facility` extracted 5 times
- `Hl7v2Message` extracted 5 times
- `PatientVisit` extracted 5 times
- `Procedure` extracted 15 times (5 provider types × 3 segments)

**Result**: ~30 extractions instead of ~8.

## Proposed Solution

### Core Principle

**Each entity processor should accept only its direct source entities as parameters, not the root entity. Each entity is extracted exactly once and reused for all dependents.**

### Example Transformation

```rust
// ❌ Current (wasteful):
pub async fn process_anesthesiologist(
    hl7v2messagefile: &Hl7v2MessageFileMessage,
    raw_json: &serde_json::Value,
    conn: &mut DbConnection,
    jetstream: &Context,
) -> Result<(), AppError> {
    // Re-extracts Filename, Facility, Hl7v2Message, PatientVisit, Procedure
    // Even though these were already extracted for other entities
}

// ✅ Proposed (efficient):
pub fn extract(
    facility: &Facility,
    patient_visit: &PatientVisit,
    procedure: &Procedure,
) -> Result<Anesthesiologist, AppError> {
    // Uses already-extracted source entities
}

pub async fn persist(
    anesthesiologist: &Anesthesiologist,
    conn: &mut DbConnection,
) -> Result<(), AppError> {
    // Persists to database
}
```

## Architecture

### 1. Entity Structs

Each entity gets a generated struct containing its fields:

```rust
#[derive(Debug, Clone)]
pub struct Anesthesiologist {
    pub source: Option<String>,
    pub provcode: Option<String>,
    pub lname: Option<String>,
    pub fname: Option<String>,
    pub mname: Option<String>,
    pub suffix: Option<String>,
    pub unitname: Option<String>,
    pub service: Option<String>,
    pub prov_type: Option<String>,
    pub provrole: Option<String>,
    pub source_type: Option<String>,
    pub vault_based_id: Option<String>,
}

#[derive(Debug, Clone)]
pub struct Facility {
    pub conformant_facility_vault_based_id: Option<String>,
    pub code: Option<String>,
    pub source: Option<String>,
}

#[derive(Debug, Clone)]
pub struct Procedure {
    pub set_id: Option<String>,
    pub procedure_code: Option<String>,
    pub procedure_code_text: Option<String>,
    pub anesthesiologist_id: Option<String>,
    pub anesthesiologist_last_name: Option<String>,
    pub anesthesiologist_first_name: Option<String>,
    // ... all other fields
}
```

### 2. Dependency Graph Construction

Build a directed acyclic graph (DAG) of entity dependencies from YAML:

```
Hl7v2MessageFile
├─→ Filename
│   └─→ Facility
├─→ Hl7v2Message
    ├─→ EventType (transient)
    ├─→ MessageHeader (transient)
    ├─→ PatientIdentification (transient)
    ├─→ PatientVisit (transient)
    │   ├─→ Admitting (persistent, depends on: Facility, PatientVisit)
    │   └─→ Attending (persistent, depends on: Facility, PatientVisit)
    └─→ Procedure (transient, repeated)
        ├─→ Anesthesiologist (persistent, depends on: Facility, PatientVisit, Procedure)
        └─→ Surgeon (persistent, depends on: Facility, PatientVisit, Procedure)
```

### 3. Processing Order (Topological Sort)

Process entities in dependency order:

**Level 0**: Root entity
- `Hl7v2MessageFile`

**Level 1**: Direct descendants of root
- `Filename` (transient)
- `Hl7v2Message` (transient)

**Level 2**: Descendants of Level 1
- `Facility` (persistent, from Filename)
- `EventType` (transient, from Hl7v2Message)
- `PatientVisit` (transient, from Hl7v2Message)
- `Procedure` (transient, repeated, from Hl7v2Message)

**Level 3**: Descendants of Level 2
- `Anesthesiologist` (persistent, from Facility + PatientVisit + Procedure)
- `Surgeon` (persistent, from Facility + PatientVisit + Procedure)
- `Admitting` (persistent, from Facility + PatientVisit)

### 4. Entity Processor Structure

Each entity processor module provides two functions:

#### `extract()` - Synchronous Field Extraction

```rust
/// Extract Anesthesiologist fields from source entities
pub fn extract(
    facility: &Facility,
    patient_visit: &PatientVisit,
    procedure: &Procedure,
) -> Result<Anesthesiologist, AppError> {
    use crate::transforms::*;

    Ok(Anesthesiologist {
        source: facility.conformant_facility_vault_based_id.clone(),
        provcode: procedure.anesthesiologist_id.clone(),
        lname: procedure.anesthesiologist_last_name.clone(),
        fname: procedure.anesthesiologist_first_name.clone(),
        mname: procedure.anesthesiologist_middle_name.clone(),
        suffix: procedure.anesthesiologist_suffix.clone(),
        unitname: patient_visit.set_id.clone(),
        service: patient_visit.servicing_facility.clone(),
        prov_type: procedure.anesthesiologist_degree.clone(),
        provrole: Some("anesthesiologist".to_string()),
        source_type: None,
        vault_based_id: compute_provider_vault_based_id(
            &procedure.anesthesiologist_id,
            &procedure.anesthesiologist_last_name,
        ).unwrap_or(None),
    })
}
```

**Key characteristics**:
- Synchronous (no async)
- Takes source entities as parameters
- Returns `Result<EntityStruct, AppError>`
- Errors propagate (no silent failures)

#### `persist()` - Async Persistence (Persistent Entities Only)

```rust
/// Persist Anesthesiologist to database
pub async fn persist(
    anesthesiologist: &Anesthesiologist,
    conn: &mut DbConnection,
) -> Result<(), AppError> {
    use diesel::prelude::*;
    use diesel::sql_types::*;

    // Insert only if unicity fields are non-empty
    if !is_valid_for_insert(anesthesiologist) {
        return Ok(());
    }

    #[cfg(feature = "postgres")]
    {
        diesel::sql_query(
            r#"INSERT INTO provider_conformant
               (source, provcode, lname, fname, mname, suffix, unitname, service,
                prov_type, provrole, source_type, vault_based_id)
               VALUES ($1, $2, $3, $4, $5, $6, $7, $8, $9, $10, $11, $12)
               ON CONFLICT (source, provcode, lname, fname, mname, unitname, service)
               DO NOTHING"#
        )
    }
    #[cfg(feature = "mysql")]
    {
        diesel::sql_query(
            r#"INSERT IGNORE INTO provider_conformant
               (source, provcode, lname, fname, mname, suffix, unitname, service,
                prov_type, provrole, source_type, vault_based_id)
               VALUES (?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?)"#
        )
    }
    .bind::<Nullable<Text>, _>(&anesthesiologist.source)
    .bind::<Nullable<Text>, _>(&anesthesiologist.provcode)
    .bind::<Nullable<Text>, _>(&anesthesiologist.lname)
    .bind::<Nullable<Text>, _>(&anesthesiologist.fname)
    .bind::<Nullable<Text>, _>(&anesthesiologist.mname)
    .bind::<Nullable<Text>, _>(&anesthesiologist.suffix)
    .bind::<Nullable<Text>, _>(&anesthesiologist.unitname)
    .bind::<Nullable<Text>, _>(&anesthesiologist.service)
    .bind::<Nullable<Text>, _>(&anesthesiologist.prov_type)
    .bind::<Nullable<Text>, _>(&anesthesiologist.provrole)
    .bind::<Nullable<Text>, _>(&anesthesiologist.source_type)
    .bind::<Nullable<Text>, _>(&anesthesiologist.vault_based_id)
    .execute(conn)?;

    Ok(())
}
```

**Key characteristics**:
- Async (for database operations)
- Takes entity struct + database connection
- Only generated for persistent entities
- Errors propagate

#### `publish()` - Async NATS Publishing (Transient Entities Only)

```rust
/// Publish Procedure to NATS stream
pub async fn publish(
    procedure: &Procedure,
    jetstream: &jetstream::Context,
) -> Result<(), AppError> {
    let mut entity_json = serde_json::Map::new();

    if let Some(ref val) = procedure.set_id {
        entity_json.insert("set_id".to_string(), serde_json::json!(val));
    }
    if let Some(ref val) = procedure.procedure_code {
        entity_json.insert("procedure_code".to_string(), serde_json::json!(val));
    }
    // ... all fields

    if entity_json.is_empty() {
        tracing::debug!("Skipping empty Procedure entity");
        return Ok(());
    }

    let entity_json_str = serde_json::to_string(&entity_json)
        .map_err(|e| AppError::ValidationError(format!("Failed to serialize Procedure: {}", e)))?;

    let stream_subject = "entities.Procedure";
    jetstream.publish(stream_subject.clone(), entity_json_str.into()).await
        .map_err(|e| {
            tracing::error!("Failed to publish Procedure to {}: {:?}", stream_subject, e);
            AppError::ValidationError(format!("NATS publish failed: {}", e))
        })?;

    tracing::info!("Published Procedure to {}", stream_subject);
    Ok(())
}
```

**Key characteristics**:
- Async (for NATS operations)
- Only generated for transient entities
- Publishes to NATS stream
- Errors propagate

### 5. Coordinator Function (`mod.rs`)

The coordinator orchestrates extraction and persistence in dependency order:

```rust
pub async fn process_hl7v2messagefile_derived_entities(
    hl7v2messagefile: &Hl7v2MessageFileMessage,
    raw_json: &serde_json::Value,
    conn: &mut DbConnection,
    jetstream: &Context,
) -> Result<(), AppError> {
    //
    // LEVEL 1: Extract direct descendants of root
    //

    // Transient entities
    let filename = filename::extract(hl7v2messagefile)?;
    let hl7v2_message = hl7v2_message::extract(hl7v2messagefile)?;

    // Publish transient entities
    filename::publish(&filename, jetstream).await?;
    hl7v2_message::publish(&hl7v2_message, jetstream).await?;

    //
    // LEVEL 2: Extract descendants of Level 1
    //

    // From Filename
    let facility = facility::extract(&filename)?;

    // From Hl7v2Message (transient)
    let event_type = event_type::extract(&hl7v2_message)?;
    let message_header = message_header::extract(&hl7v2_message)?;
    let patient_identification = patient_identification::extract(&hl7v2_message)?;
    let patient_visit = patient_visit::extract(&hl7v2_message)?;

    // Publish transient entities
    event_type::publish(&event_type, jetstream).await?;
    message_header::publish(&message_header, jetstream).await?;
    patient_identification::publish(&patient_identification, jetstream).await?;
    patient_visit::publish(&patient_visit, jetstream).await?;

    // Extract repeated transient entities (Procedure, Diagnosis, etc.)
    let procedures: Vec<Procedure> = hl7v2_message.PR1.iter()
        .map(|segment| procedure::extract(&hl7v2_message, segment))
        .collect::<Result<Vec<_>, _>>()?;

    let diagnoses: Vec<Diagnosis> = hl7v2_message.DG1.iter()
        .map(|segment| diagnosis::extract(&hl7v2_message, segment))
        .collect::<Result<Vec<_>, _>>()?;

    // Publish repeated transient entities
    for procedure in &procedures {
        procedure::publish(procedure, jetstream).await?;
    }
    for diagnosis in &diagnoses {
        diagnosis::publish(diagnosis, jetstream).await?;
    }

    //
    // LEVEL 2: Persist entities (after extraction)
    //

    facility::persist(&facility, conn).await?;

    //
    // LEVEL 3: Process entities that depend on Level 2
    //

    // Non-repeated persistent entities
    let admitting = admitting::extract(&facility, &patient_visit)?;
    admitting::persist(&admitting, conn).await?;

    let attending = attending::extract(&facility, &patient_visit)?;
    attending::persist(&attending, conn).await?;

    // Repeated persistent entities (one per procedure)
    for procedure in &procedures {
        let anesthesiologist = anesthesiologist::extract(&facility, &patient_visit, procedure)?;
        anesthesiologist::persist(&anesthesiologist, conn).await?;

        let surgeon = surgeon::extract(&facility, &patient_visit, procedure)?;
        surgeon::persist(&surgeon, conn).await?;
    }

    tracing::info!("Processed all derived entities for Hl7v2MessageFile");
    Ok(())
}
```

### 6. Handling Repeated Entities

#### Source is Repeated

When a source entity is repeated (e.g., `Procedure`), the dependent entity processor is called in a loop:

```rust
// Extract all procedures once
let procedures: Vec<Procedure> = hl7v2_message.PR1.iter()
    .map(|segment| procedure::extract(&hl7v2_message, segment))
    .collect::<Result<Vec<_>, _>>()?;

// Process each procedure-dependent entity
for procedure in &procedures {
    let anesthesiologist = anesthesiologist::extract(&facility, &patient_visit, procedure)?;
    anesthesiologist::persist(&anesthesiologist, conn).await?;
}
```

**Key point**: Errors propagate - if any extraction fails, the entire batch fails.

#### Entity is Repeated

When the entity itself is repeated (has `repeated_for`), the extraction happens in a loop at the coordinator level:

```rust
// Procedure has repeated_for: { entity: Hl7v2Message, field: PR1 }
let procedures: Vec<Procedure> = hl7v2_message.PR1.iter()
    .map(|segment| procedure::extract(&hl7v2_message, segment))
    .collect::<Result<Vec<_>, _>>()?;
```

## Code Generation Changes

### 1. Generate Entity Structs (`src/entities.rs`)

```rust
// Auto-generated entity structs

#[derive(Debug, Clone)]
pub struct Facility {
    pub conformant_facility_vault_based_id: Option<String>,
    pub code: Option<String>,
    pub source: Option<String>,
}

#[derive(Debug, Clone)]
pub struct Anesthesiologist {
    pub source: Option<String>,
    pub provcode: Option<String>,
    pub lname: Option<String>,
    pub fname: Option<String>,
    // ... all fields
}
```

### 2. Update `entity_processor_rs.rs`

Generate separate `extract()`, `persist()`, and `publish()` functions:

```rust
pub fn generate_entity_processor_module(
    entity: &EntityDef,
    all_entities: &[EntityDef],
    output_dir: &Path,
) -> Result<(), Box<dyn Error>> {
    // Determine source entities from derivation.source_entities
    let source_entities = get_source_entities(entity, all_entities);

    // Generate extract() function
    generate_extract_function(&mut output, entity, &source_entities, all_entities)?;

    // Generate persist() for persistent entities
    if entity.is_persistent(all_entities) {
        generate_persist_function(&mut output, entity, all_entities)?;
    } else {
        // Generate publish() for transient entities
        generate_publish_function(&mut output, entity)?;
    }

    Ok(())
}
```

### 3. Update Coordinator Generation (`mod.rs`)

Generate processing in topological order:

```rust
pub fn generate_coordinator_function(
    root_entity: &EntityDef,
    all_entities: &[EntityDef],
    output: &mut File,
) -> Result<(), Box<dyn Error>> {
    // Build dependency graph
    let dep_graph = build_dependency_graph(root_entity, all_entities);

    // Topological sort
    let processing_order = topological_sort(&dep_graph)?;

    writeln!(output, "pub async fn process_{}_derived_entities(",
        root_entity.name.to_lowercase())?;
    writeln!(output, "    {}: &parsers::{}Message,",
        root_entity.name.to_lowercase(), root_entity.name)?;
    writeln!(output, "    raw_json: &serde_json::Value,")?;
    writeln!(output, "    conn: &mut DbConnection,")?;
    writeln!(output, "    jetstream: &Context,")?;
    writeln!(output, ") -> Result<(), AppError> {{")?;

    // Generate extraction and persistence in levels
    for level in processing_order {
        generate_level_processing(output, &level, all_entities)?;
    }

    writeln!(output, "    Ok(())")?;
    writeln!(output, "}}")?;

    Ok(())
}
```

## Implementation Strategy

### Phase 1: Struct Generation
1. ✅ Define struct schema from entity YAML
2. ✅ Generate struct definitions in `src/entities.rs`
3. ✅ Add `Clone` and `Debug` derives

### Phase 2: Dependency Analysis
1. ✅ Parse `derivation.source_entities` from YAML
2. ✅ Build dependency graph
3. ✅ Detect cycles (error if found)
4. ✅ Compute topological sort order

### Phase 3: Extract Function Generation
1. ✅ Generate function signature with source entity parameters
2. ✅ Generate field extraction using source entity fields
3. ✅ Return entity struct
4. ✅ Propagate errors

### Phase 4: Persist/Publish Function Generation
1. ✅ Generate `persist()` for persistent entities
2. ✅ Generate `publish()` for transient entities
3. ✅ Both propagate errors

### Phase 5: Coordinator Generation
1. ✅ Group entities by dependency level
2. ✅ Generate extraction calls in order
3. ✅ Store extracted entities in variables
4. ✅ Handle repeated entities with loops
5. ✅ Call persist/publish with correct entities

### Phase 6: Testing & Validation
1. ⬜ Unit tests for each entity processor
2. ⬜ Integration tests comparing old vs new output
3. ⬜ Performance benchmarks
4. ⬜ Validate database inserts match

## Benefits

### Performance
- **Extraction**: Each entity extracted exactly once (vs. N times currently)
- **Memory**: Predictable allocation pattern
- **Database**: Same number of inserts (no change)
- **NATS**: Same number of publishes (no change)

### Code Quality
- **Clarity**: Explicit dependency chain in function signatures
- **Testability**: Can test entities in isolation with mock sources
- **Maintainability**: Clear separation of extraction vs. persistence
- **Type Safety**: Structs prevent field name typos

### Developer Experience
- **Debugging**: Easier to trace data flow
- **Understanding**: Dependency graph visible in code structure
- **Modification**: Changes to extraction isolated from persistence

## Error Handling Strategy

All errors propagate up the call stack:

```rust
// Extraction error - propagates to coordinator
let facility = facility::extract(&filename)?;

// Persistence error - propagates to coordinator
facility::persist(&facility, conn).await?;

// Coordinator error - propagates to message handler
process_hl7v2messagefile_derived_entities(msg, raw_json, conn, jetstream).await?;

// Message handler logs and sends to DLQ
if let Err(e) = process_hl7v2messagefile_derived_entities(...).await {
    tracing::error!("Processing failed: {:?}", e);
    send_to_dlq(msg, jetstream).await?;
}
```

**No silent failures** - every error is either:
1. Logged and message sent to DLQ (top level)
2. Propagated to caller (all other levels)

## Migration Path

### Option A: Big Bang (Recommended)

1. Implement new architecture
2. Run both side-by-side in test environment
3. Validate equivalent output
4. Switch production to new architecture
5. Remove old code

### Option B: Gradual Migration

1. Implement new architecture with feature flag
2. Run both in production (shadow mode)
3. Compare outputs, log differences
4. Gradually increase traffic to new architecture
5. Full cutover when confidence is high

**Recommendation**: Option A - code is generated, so no manual migration risk.

## Example: Complete Flow

### Entity Definitions (YAML)

```yaml
# facility.yaml
entity:
  name: Facility
  type: derived
  derivation:
    source_entities:
      filename: Filename
  persistence:
    table: facility_conformant
    # ...
  fields:
    - name: conformant_facility_vault_based_id
      type: String
      computed_from:
        transform: copy_field
        sources:
          - source: filename
            field: f_facilityId

# anesthesiologist.yaml
entity:
  name: Anesthesiologist
  type: derived
  extends: Provider
  derivation:
    source_entities:
      facility: Facility
      patient_visit: PatientVisit
      procedure: Procedure
  persistence:
    table: provider_conformant
    # ...
  fields:
    - name: provcode
      type: String
      computed_from:
        transform: copy_field
        sources:
          - source: procedure
            field: anesthesiologist_id
```

### Generated Code

```rust
// src/entities.rs
#[derive(Debug, Clone)]
pub struct Facility {
    pub conformant_facility_vault_based_id: Option<String>,
    pub code: Option<String>,
    pub source: Option<String>,
}

#[derive(Debug, Clone)]
pub struct Anesthesiologist {
    pub source: Option<String>,
    pub provcode: Option<String>,
    pub lname: Option<String>,
    // ...
}

// src/entity_processors/facility.rs
pub fn extract(filename: &Filename) -> Result<Facility, AppError> {
    Ok(Facility {
        conformant_facility_vault_based_id: filename.f_facilityId.clone(),
        code: filename.f_HL7Ptr.clone(),
        source: filename.f_facilityId.clone(),
    })
}

pub async fn persist(
    facility: &Facility,
    conn: &mut DbConnection,
) -> Result<(), AppError> {
    diesel::sql_query(/* INSERT */)
        .bind(&facility.conformant_facility_vault_based_id)
        .execute(conn)?;
    Ok(())
}

// src/entity_processors/anesthesiologist.rs
pub fn extract(
    facility: &Facility,
    patient_visit: &PatientVisit,
    procedure: &Procedure,
) -> Result<Anesthesiologist, AppError> {
    Ok(Anesthesiologist {
        source: facility.conformant_facility_vault_based_id.clone(),
        provcode: procedure.anesthesiologist_id.clone(),
        lname: procedure.anesthesiologist_last_name.clone(),
        // ...
    })
}

pub async fn persist(
    anesthesiologist: &Anesthesiologist,
    conn: &mut DbConnection,
) -> Result<(), AppError> {
    diesel::sql_query(/* INSERT */)
        .bind(&anesthesiologist.source)
        .bind(&anesthesiologist.provcode)
        .execute(conn)?;
    Ok(())
}

// src/entity_processors/mod.rs
pub async fn process_hl7v2messagefile_derived_entities(
    hl7v2messagefile: &Hl7v2MessageFileMessage,
    raw_json: &serde_json::Value,
    conn: &mut DbConnection,
    jetstream: &Context,
) -> Result<(), AppError> {
    // Level 1: Extract from root
    let filename = filename::extract(hl7v2messagefile)?;
    let hl7v2_message = hl7v2_message::extract(hl7v2messagefile)?;

    filename::publish(&filename, jetstream).await?;
    hl7v2_message::publish(&hl7v2_message, jetstream).await?;

    // Level 2: Extract from Level 1
    let facility = facility::extract(&filename)?;
    let patient_visit = patient_visit::extract(&hl7v2_message)?;

    let procedures: Vec<Procedure> = hl7v2_message.PR1.iter()
        .map(|seg| procedure::extract(&hl7v2_message, seg))
        .collect::<Result<Vec<_>, _>>()?;

    // Persist Level 2
    facility::persist(&facility, conn).await?;

    for procedure in &procedures {
        procedure::publish(procedure, jetstream).await?;
    }

    // Level 3: Extract and persist
    for procedure in &procedures {
        let anesthesiologist = anesthesiologist::extract(
            &facility,
            &patient_visit,
            procedure
        )?;
        anesthesiologist::persist(&anesthesiologist, conn).await?;
    }

    Ok(())
}
```

## Open Questions

### 1. Memory Usage for Large Messages
**Question**: For a message with 1000 PR1 segments, we'll have a `Vec<Procedure>` with 1000 items in memory. Is this acceptable?

**Answer**: Yes - this is expected and necessary. The current architecture does the same but discards entities immediately after use. The new architecture keeps them in memory for the duration of message processing, which is acceptable for typical message sizes.

### 2. Parallel Processing
**Question**: Can we process independent entities in parallel?

**Future Enhancement**: Yes - entities at the same dependency level with no shared mutable state could be processed in parallel using `tokio::spawn`. This is a future optimization.

### 3. Conditional Dependencies
**Question**: What if a dependency is optional (e.g., segment might not exist)?

**Answer**: Use `Option<Entity>` parameters and skip processing if None:

```rust
pub fn extract(
    facility: &Facility,
    patient_visit: Option<&PatientVisit>,  // Optional
) -> Result<SomeEntity, AppError> {
    let unitname = patient_visit
        .and_then(|pv| pv.set_id.clone())
        .unwrap_or_else(|| Some("default".to_string()));
    // ...
}
```

---

**Status**: Design approved, ready for implementation.
