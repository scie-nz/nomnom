# Worker Module Splitting Plan

**Goal:** Split the monolithic 17K-line `main.rs` into separate modules to enable parallel compilation and dramatically improve build times.

**Expected Improvement:** 5-10x faster incremental builds, 2-3x faster clean builds

---

## Current Problem

### File Size Analysis
- **Current:** `nomnom-worker/src/main.rs` = 17,460 lines
- **Bottleneck:** Single compilation unit prevents parallelization
- **Issue:** Every code change recompiles the entire file

### What's in main.rs?
1. **Boilerplate** (~270 lines): Imports, NATS setup, main loop
2. **Message processing dispatcher** (~100 lines): Routes messages to entity processors
3. **Entity processors** (~17,000 lines): One massive function per entity with:
   - Field extraction from parsed message
   - JSON serialization (100s of `if let Some(ref val)` statements)
   - NATS publishing logic
   - Error handling

### Why This is Slow
```rust
// All in one file = ONE compilation unit
async fn process_patient(...) { /* 500 lines */ }
async fn process_diagnosis(...) { /* 800 lines */ }
async fn process_procedure(...) { /* 600 lines */ }
// ... 39 more entities ...
```

Rust compiles this **sequentially** because it's a single file. With 39 entities, this takes ~6 minutes.

---

## Proposed Solution

### New Structure (REVISED - One Module Per Derived Entity)

```
nomnom-worker/
├── src/
│   ├── main.rs                    # ~500 lines (orchestration only)
│   ├── database.rs                # (existing, unchanged)
│   ├── error.rs                   # (existing, unchanged)
│   ├── models.rs                  # (existing, unchanged)
│   ├── parsers.rs                 # (existing, unchanged)
│   ├── transforms.rs              # (existing, unchanged)
│   │
│   ├── entity_processors/         # NEW: One module per derived entity
│   │   ├── mod.rs                 # Coordinates processing order
│   │   │
│   │   ├── mpi.rs                 # ~400 lines (persistent)
│   │   ├── facility.rs            # ~350 lines (persistent)
│   │   ├── practitioner.rs        # ~500 lines (persistent)
│   │   ├── surgeon.rs             # ~450 lines (persistent)
│   │   ├── diagnosis.rs           # ~300 lines (transient)
│   │   ├── procedure.rs           # ~350 lines (transient)
│   │   ├── patient_visit.rs       # ~400 lines (transient)
│   │   ├── patient_account.rs     # ~380 lines (transient)
│   │   └── ...                    # (37 total entity modules)
│   │
│   └── message_processor.rs       # NEW: Orchestrates entity processing
│
├── Cargo.toml
└── ...
```

**Key Design Change:** Instead of grouping by root entity (which creates a 17K-line module), we create **one module per derived entity** (37 modules × ~400 lines each). This enables **maximum parallel compilation**.

### File Size Breakdown (After Split - REVISED)
- `main.rs`: 370 lines (NATS loop, setup, root entity processing)
- `message_processor.rs`: NOT NEEDED (processing done in entity_processors/mod.rs)
- `entity_processors/mod.rs`: ~200 lines (orchestrates processing of all 37 entities)
- `entity_processors/*.rs`: 37 files × ~450 lines average = 16,650 lines
  - 12 persistent entities (MPI, Facility, Practitioner, etc.)
  - 25 transient entities (Diagnosis, Procedure, PatientVisit, etc.)

**Total:** 37+ small files instead of 1 massive 17K-line file

**Compilation:** Rust can compile all 37 entity processor modules **in parallel across CPU cores**

### Why One Module Per Derived Entity? (Design Rationale)

**Initial Approach (WRONG):**
- Grouped by root entity: `entity_processors/hl7v2_message_file.rs` (17,101 lines)
- Problem: Still a monolithic file, defeats parallelization
- Only 1 compilation unit instead of 37

**Revised Approach (CORRECT):**
- One module per derived entity: `entity_processors/mpi.rs`, `entity_processors/diagnosis.rs`, etc.
- Each entity processor is independent and self-contained
- Each module ~300-500 lines (easy to understand and compile)
- 37 compilation units = maximum parallelization

**Compilation Graph:**
```
Before splitting:                After splitting (per-entity):
[===============================] [mpi.rs]    [facility.rs] [surgeon.rs]    ...
main.rs (17K lines)              [diag.rs]   [proc.rs]     [visit.rs]      ...
Sequential, ~6 min               [account.rs] [insurance.rs] [vaccine.rs]   ...
                                 Parallel across 8-16 cores, ~1-2 min
```

---

## Implementation Plan (REVISED)

### Phase 1: Update Nomnom Codegen (Core Changes)

#### 1.1: Create New Codegen Module Structure

**File:** `nomnom/src/codegen/worker/entity_processor_rs.rs` (NEW)

```rust
/// Generate entity processor modules (one per entity)
///
/// This replaces the inline entity processing in main.rs
/// Each entity gets its own file for parallel compilation

pub fn generate_entity_processor(
    entity: &EntityDef,
    output_dir: &Path,
    config: &WorkerConfig,
) -> Result<(), Box<dyn Error>> {
    let entity_name_snake = to_snake_case(&entity.name);
    let category = if entity.persistent { "persistent" } else { "transient" };

    let processor_dir = output_dir.join("src/entity_processors").join(category);
    std::fs::create_dir_all(&processor_dir)?;

    let processor_file = processor_dir.join(format!("{}.rs", entity_name_snake));
    let mut output = std::fs::File::create(&processor_file)?;

    // Generate imports
    writeln!(output, "use crate::parsers::ParsedMessage;")?;
    writeln!(output, "use crate::error::AppError;")?;
    writeln!(output, "use crate::database::DbConnection;")?;
    writeln!(output, "use async_nats::jetstream::Context;")?;
    writeln!(output, "use diesel::prelude::*;")?;
    writeln!(output, "use serde_json::{{json, Map, Value}};\n")?;

    // Generate processor function
    writeln!(output, "/// Process {} entity from parsed message", entity.name)?;
    writeln!(output, "pub async fn process_{}(", entity_name_snake)?;
    writeln!(output, "    parsed: &ParsedMessage,")?;
    writeln!(output, "    conn: &mut DbConnection,")?;
    writeln!(output, "    jetstream: &Context,")?;
    writeln!(output, ") -> Result<(), AppError> {{")?;

    // Generate field extraction logic (moved from main.rs)
    generate_field_extraction(entity, &mut output)?;

    // Generate JSON serialization (moved from main.rs)
    generate_json_serialization(entity, &mut output)?;

    // Generate NATS publishing (moved from main.rs)
    generate_nats_publishing(entity, &mut output)?;

    writeln!(output, "}}")?;

    Ok(())
}
```

#### 1.2: Create Module Index Generators

**File:** `nomnom/src/codegen/worker/entity_processor_rs.rs` (continued)

```rust
/// Generate entity_processors/mod.rs
pub fn generate_entity_processors_mod(
    entities: &[EntityDef],
    output_dir: &Path,
) -> Result<(), Box<dyn Error>> {
    let mod_file = output_dir.join("src/entity_processors/mod.rs");
    let mut output = std::fs::File::create(&mod_file)?;

    writeln!(output, "// Auto-generated entity processor modules\n")?;

    writeln!(output, "pub mod persistent;")?;
    writeln!(output, "pub mod transient;\n")?;

    // Re-export all processors for easy access
    writeln!(output, "pub use persistent::*;")?;
    writeln!(output, "pub use transient::*;")?;

    Ok(())
}

/// Generate entity_processors/persistent/mod.rs
pub fn generate_persistent_mod(
    entities: &[EntityDef],
    output_dir: &Path,
) -> Result<(), Box<dyn Error>> {
    let mod_file = output_dir.join("src/entity_processors/persistent/mod.rs");
    let mut output = std::fs::File::create(&mod_file)?;

    writeln!(output, "// Persistent entity processors\n")?;

    for entity in entities.iter().filter(|e| e.persistent) {
        let module_name = to_snake_case(&entity.name);
        writeln!(output, "pub mod {};", module_name)?;
    }

    writeln!(output)?;
    for entity in entities.iter().filter(|e| e.persistent) {
        let module_name = to_snake_case(&entity.name);
        writeln!(output, "pub use {}::process_{};", module_name, module_name)?;
    }

    Ok(())
}

/// Generate entity_processors/transient/mod.rs
pub fn generate_transient_mod(
    entities: &[EntityDef],
    output_dir: &Path,
) -> Result<(), Box<dyn Error>> {
    // Similar to persistent but for transient entities
    // ... implementation ...
    Ok(())
}
```

#### 1.3: Create Message Processor Module

**File:** `nomnom/src/codegen/worker/message_processor_rs.rs` (NEW)

```rust
/// Generate message_processor.rs
///
/// This handles routing parsed messages to the correct entity processor

pub fn generate_message_processor_rs(
    entities: &[EntityDef],
    output_dir: &Path,
) -> Result<(), Box<dyn Error>> {
    let file = output_dir.join("src/message_processor.rs");
    let mut output = std::fs::File::create(&file)?;

    writeln!(output, "// Auto-generated message processor\n")?;

    writeln!(output, "use crate::parsers::ParsedMessage;")?;
    writeln!(output, "use crate::database::DbConnection;")?;
    writeln!(output, "use crate::error::AppError;")?;
    writeln!(output, "use crate::entity_processors;")?;
    writeln!(output, "use async_nats::jetstream::Context;\n")?;

    writeln!(output, "/// Process derived entities from root entity")?;
    writeln!(output, "pub async fn process_derived_entities(")?;
    writeln!(output, "    parsed: &ParsedMessage,")?;
    writeln!(output, "    conn: &mut DbConnection,")?;
    writeln!(output, "    jetstream: &Context,")?;
    writeln!(output, ") -> Result<(), AppError> {{")?;

    // Generate calls to each entity processor
    for entity in entities {
        let processor_name = format!("process_{}", to_snake_case(&entity.name));
        writeln!(output, "    // Process {}", entity.name)?;
        writeln!(output, "    if let Err(e) = entity_processors::{}(parsed, conn, jetstream).await {{", processor_name)?;
        writeln!(output, "        tracing::warn!(\"Failed to process {}: {{:?}}\", e);", entity.name)?;
        writeln!(output, "    }}\n")?;
    }

    writeln!(output, "    Ok(())")?;
    writeln!(output, "}}")?;

    Ok(())
}
```

#### 1.4: Simplify main.rs Generation

**File:** `nomnom/src/codegen/worker/main_rs.rs` (MODIFY EXISTING)

```rust
pub fn generate_main_rs(
    entities: &[EntityDef],
    output_dir: &Path,
    config: &WorkerConfig,
) -> Result<(), Box<dyn Error>> {
    let main_file = output_dir.join("src/main.rs");
    let mut output = std::fs::File::create(&main_file)?;

    // Header
    writeln!(output, "// Auto-generated NATS worker\n")?;

    // Imports
    writeln!(output, "use async_nats::jetstream;")?;
    writeln!(output, "use futures::StreamExt;")?;
    writeln!(output, "use std::time::Duration;\n")?;

    // Module declarations
    writeln!(output, "mod parsers;")?;
    writeln!(output, "mod models;")?;
    writeln!(output, "mod database;")?;
    writeln!(output, "mod error;")?;
    writeln!(output, "mod transforms;")?;
    writeln!(output, "mod entity_processors;  // NEW: Entity processing modules")?;
    writeln!(output, "mod message_processor;  // NEW: Message routing\n")?;

    writeln!(output, "use database::create_pool;")?;
    writeln!(output, "use parsers::MessageParser;")?;
    writeln!(output, "use message_processor::process_derived_entities;\n")?;

    // Message envelope struct
    generate_message_envelope(&mut output)?;

    // Main function
    writeln!(output, "#[tokio::main]")?;
    writeln!(output, "async fn main() {{")?;

    // NATS setup (existing code)
    generate_nats_setup(&mut output)?;

    // Message processing loop
    writeln!(output, "    // Message processing loop")?;
    writeln!(output, "    while let Some(message) = messages.next().await {{")?;
    writeln!(output, "        match message {{")?;
    writeln!(output, "            Ok(msg) => {{")?;
    writeln!(output, "                // Parse envelope")?;
    writeln!(output, "                let envelope: MessageEnvelope = serde_json::from_slice(&msg.payload)")?;
    writeln!(output, "                    .expect(\"Failed to parse message envelope\");\n")?;

    writeln!(output, "                // Parse HL7v2 message")?;
    writeln!(output, "                let parsed = MessageParser::parse(&envelope.body)")?;
    writeln!(output, "                    .expect(\"Failed to parse HL7v2 message\");\n")?;

    writeln!(output, "                // Process all derived entities")?;
    writeln!(output, "                let mut conn = db_pool.get().expect(\"Failed to get DB connection\");")?;
    writeln!(output, "                if let Err(e) = process_derived_entities(&parsed, &mut conn, &jetstream).await {{")?;
    writeln!(output, "                    tracing::error!(\"Failed to process message: {{:?}}\", e);")?;
    writeln!(output, "                    msg.ack_with(async_nats::jetstream::AckKind::Nak(None)).await.ok();")?;
    writeln!(output, "                }} else {{")?;
    writeln!(output, "                    msg.ack().await.ok();")?;
    writeln!(output, "                }}")?;
    writeln!(output, "            }}")?;
    writeln!(output, "            Err(e) => {{")?;
    writeln!(output, "                tracing::error!(\"NATS error: {{:?}}\", e);")?;
    writeln!(output, "            }}")?;
    writeln!(output, "        }}")?;
    writeln!(output, "    }}")?;
    writeln!(output, "}}")?;

    Ok(())
}
```

#### 1.5: Update Worker Code Generator Entry Point

**File:** `nomnom/src/codegen/worker/mod.rs` (MODIFY EXISTING)

```rust
pub fn generate_worker(
    entities: &[EntityDef],
    output_dir: &Path,
    config: &WorkerConfig,
) -> Result<(), Box<dyn Error>> {
    // Create directories
    std::fs::create_dir_all(output_dir.join("src/entity_processors/persistent"))?;
    std::fs::create_dir_all(output_dir.join("src/entity_processors/transient"))?;

    println!("  ✓ Generating Cargo.toml...");
    cargo_toml::generate_cargo_toml(output_dir, config)?;

    println!("  ✓ Generating main.rs...");
    main_rs::generate_main_rs(entities, output_dir, config)?;

    println!("  ✓ Generating message_processor.rs...");
    message_processor_rs::generate_message_processor_rs(entities, output_dir)?;

    println!("  ✓ Generating entity processors...");
    // Generate individual entity processor files
    for entity in entities {
        entity_processor_rs::generate_entity_processor(entity, output_dir, config)?;
    }

    // Generate module indexes
    entity_processor_rs::generate_entity_processors_mod(entities, output_dir)?;
    entity_processor_rs::generate_persistent_mod(entities, output_dir)?;
    entity_processor_rs::generate_transient_mod(entities, output_dir)?;

    println!("  ✓ Generating parsers.rs...");
    parsers_rs::generate_parsers_rs(entities, output_dir)?;

    println!("  ✓ Generating database.rs...");
    database_rs::generate_database_rs(output_dir, config)?;

    println!("  ✓ Generating error.rs...");
    error_rs::generate_error_rs(output_dir)?;

    println!("  ✓ Generating models.rs...");
    models_rs::generate_models_rs(output_dir)?;

    println!("  ✓ Generating transforms.rs...");
    transforms_rs::generate_transforms_rs(output_dir)?;

    Ok(())
}
```

---

### Phase 2: Extract Refactored Code

The key is to move existing logic from `main_rs.rs` into the new modules:

#### What Gets Moved to `entity_processor_rs.rs`

**From:** `nomnom/src/codegen/worker/main_rs.rs` lines ~400-1600

**Functions to extract:**
1. `generate_entity_processing()` → Split into:
   - `generate_field_extraction()`
   - `generate_json_serialization()`
   - `generate_nats_publishing()`

2. Helper functions:
   - `to_snake_case()` → Keep in shared utils
   - `is_list_type()` → Move to entity_processor_rs
   - Field type mapping logic → Move to entity_processor_rs

**Example extraction:**

```rust
// OLD: In main_rs.rs (generates inline code in main.rs)
fn generate_entity_processing(entity: &EntityDef, output: &mut File) {
    writeln!(output, "async fn process_{}(...) {{", entity.name)?;
    // ... 500 lines of codegen ...
    writeln!(output, "}}")?;
}

// NEW: In entity_processor_rs.rs (generates separate file)
fn generate_field_extraction(entity: &EntityDef, output: &mut File) {
    writeln!(output, "    // Extract fields from parsed message")?;
    for field in &entity.fields {
        // Generate extraction logic
    }
}
```

---

### Phase 3: Testing Strategy

#### 3.1: Correctness Tests

**Test that generated worker is functionally identical:**

```bash
# Before changes
cd /home/bogdan/nomnom
git checkout main
cargo build
./target/debug/nomnom generate-worker \
  --entities /home/bogdan/ingestion/config/entities \
  --output /tmp/worker-before \
  --database mysql

# After changes
git checkout feature/module-splitting
cargo build
./target/debug/nomnom generate-worker \
  --entities /home/bogdan/ingestion/config/entities \
  --output /tmp/worker-after \
  --database mysql

# Compare behavior (not exact source, since structure changed)
cd /tmp/worker-before && cargo test
cd /tmp/worker-after && cargo test

# Deploy both and run integration test
# (Send same HL7 messages, verify identical database state)
```

#### 3.2: Performance Benchmarks

**Measure compilation time improvements:**

```bash
# Clean build
cd /tmp/worker-before
cargo clean
time cargo build  # Baseline: ~6 minutes

cd /tmp/worker-after
cargo clean
time cargo build  # Expected: ~2-3 minutes (2-3x faster)

# Incremental build (change one entity processor)
cd /tmp/worker-before
touch src/main.rs
time cargo build  # Baseline: ~90 seconds (recompiles everything)

cd /tmp/worker-after
touch src/entity_processors/persistent/mpi.rs
time cargo build  # Expected: ~10-15 seconds (only recompiles one module)
```

#### 3.3: Integration Tests

**Test complete workflow:**

```bash
# Generate worker with new codegen
./nomnom generate-worker \
  --entities /home/bogdan/ingestion/config/entities \
  --output /home/bogdan/ingestion/nomnom-worker \
  --database mysql

# Build Docker image
cd /home/bogdan/ingestion
docker build -t worker:test -f nomnom-worker/Dockerfile.dev.sccache .

# Deploy to kind
cd /home/bogdan/ingestion
./scripts/test-kind.sh

# Send test messages
curl -X POST http://localhost:8080/ingest/message \
  -H "Content-Type: text/plain" \
  --data-binary @tests/data/messages_500/FACILITY_ABC_20250101_120000.hl7

# Verify data in database
kubectl exec -it deploy/mariadb -n hl7-test -- \
  mysql -u nomnom -pnomnom_test hl7_ingestion \
  -e "SELECT COUNT(*) FROM mpi_id_conformant;"
```

---

## Implementation Checklist

### Code Changes in `nomnom`

- [ ] **Create new files:**
  - [ ] `src/codegen/worker/entity_processor_rs.rs`
  - [ ] `src/codegen/worker/message_processor_rs.rs`

- [ ] **Modify existing files:**
  - [ ] `src/codegen/worker/mod.rs` - Add new modules, update generate_worker()
  - [ ] `src/codegen/worker/main_rs.rs` - Simplify, remove inline entity processing

- [ ] **Extract and refactor:**
  - [ ] Move `generate_entity_processing()` logic to `entity_processor_rs.rs`
  - [ ] Split into `generate_field_extraction()`, `generate_json_serialization()`, etc.
  - [ ] Create module index generators

### Testing

- [ ] **Unit tests:**
  - [ ] Test entity processor generation
  - [ ] Test module structure creation
  - [ ] Test message_processor.rs generation

- [ ] **Integration tests:**
  - [ ] Generate worker with test entities
  - [ ] Verify file structure
  - [ ] Verify compilation succeeds
  - [ ] Run worker against test data

- [ ] **Performance benchmarks:**
  - [ ] Measure clean build time (before vs after)
  - [ ] Measure incremental build time (before vs after)
  - [ ] Document speedup in commit message

### Deployment

- [ ] **Build nomnom:**
  ```bash
  cd /home/bogdan/nomnom
  cargo build --release
  ```

- [ ] **Regenerate worker:**
  ```bash
  ./target/release/nomnom generate-worker \
    --entities /home/bogdan/ingestion/config/entities \
    --output /home/bogdan/ingestion/nomnom-worker \
    --database mysql
  ```

- [ ] **Test build:**
  ```bash
  cd /home/bogdan/ingestion/nomnom-worker
  cargo clean
  time cargo build  # Should be 2-3x faster
  ```

- [ ] **Full integration test:**
  ```bash
  cd /home/bogdan/ingestion
  ./scripts/test-kind.sh
  # Verify pods start successfully
  # Send test messages
  # Verify data in database
  ```

---

## Expected Results

### Before Module Splitting

```
nomnom-worker/
└── src/
    └── main.rs (17,460 lines)

Compilation time:
- Clean build: ~6 minutes
- Incremental (code change): ~90 seconds
- Bottleneck: Single massive compilation unit
```

### After Module Splitting

```
nomnom-worker/
└── src/
    ├── main.rs (500 lines)
    ├── message_processor.rs (300 lines)
    └── entity_processors/
        ├── persistent/ (12 modules, ~400 lines each)
        └── transient/ (27 modules, ~350 lines each)

Compilation time:
- Clean build: ~2-3 minutes (2-3x faster)
- Incremental (code change): ~10-15 seconds (6-9x faster)
- Benefit: Parallel compilation across 39+ modules
```

### Compilation Graph Visualization

**Before:**
```
[====== main.rs (17K lines) ======]  ← Sequential, 6 minutes
```

**After:**
```
[main.rs]
[msg_proc]
[mpi.rs] [facility.rs] [surgeon.rs] ...  ← Parallel across CPU cores
[diag.rs] [proc.rs] [visit.rs] ...       ← 2-3 minutes total
```

---

## Success Metrics

### Must Have
- ✅ Generated worker compiles without errors
- ✅ All integration tests pass
- ✅ Data correctness verified (same output as before)
- ✅ Clean build **< 3 minutes** (vs 6 minutes before)

### Should Have
- ✅ Incremental build **< 20 seconds** (vs 90 seconds before)
- ✅ Module structure is clean and organized
- ✅ Documentation updated

### Nice to Have
- ✅ CI/CD pipeline updated to cache per-module
- ✅ Build time metrics tracked over time
- ✅ Consider applying same pattern to ingestion server

---

## Risks and Mitigations

### Risk 1: Generated Code Differences
**Risk:** New modular structure might have subtle behavioral differences

**Mitigation:**
- Comprehensive integration tests
- Side-by-side comparison with test data
- Gradual rollout (test in dev, then staging, then prod)

### Risk 2: Increased Complexity
**Risk:** More files to manage in codegen

**Mitigation:**
- Clear separation of concerns
- Good documentation
- Helper functions to reduce code duplication

### Risk 3: Module Interdependencies
**Risk:** Circular dependencies between entity processors

**Mitigation:**
- Each processor is independent (only depends on shared types)
- No cross-entity references in generated code
- Clear module hierarchy (processors → core types)

---

## Future Enhancements

### After Module Splitting Works

1. **Apply to Ingestion Server:**
   - Similar 80K line issue in generated code
   - Same pattern can apply

2. **Incremental Diesel Schema:**
   - Split schema into per-table modules
   - Further parallelization

3. **Compile-Time Feature Flags:**
   - Allow disabling unused entities
   - Reduce binary size for specialized deployments

4. **Code Generation Caching:**
   - Cache entity processor generation
   - Only regenerate changed entities

---

## Timeline Estimate

### Quick Implementation (1-2 hours)
- Create basic module structure
- Move entity processing to separate files
- Get it compiling

### Thorough Implementation (4-6 hours)
- Clean refactoring
- Comprehensive tests
- Documentation
- Performance benchmarking

### With Full Testing (1 day)
- Integration tests
- Performance comparison
- Multiple test scenarios
- Documentation updates

---

## Getting Started

### Step 1: Create Feature Branch
```bash
cd /home/bogdan/nomnom
git checkout -b feature/worker-module-splitting
```

### Step 2: Create New Module Files
```bash
mkdir -p src/codegen/worker
touch src/codegen/worker/entity_processor_rs.rs
touch src/codegen/worker/message_processor_rs.rs
```

### Step 3: Start with Message Processor
This is the simplest piece - routing logic that calls entity processors.

### Step 4: Extract One Entity Processor
Start with a small entity (like `Filename`) to validate the pattern.

### Step 5: Expand to All Entities
Once one works, apply pattern to remaining 38 entities.

### Step 6: Test and Benchmark
Run full test suite and measure improvements.

---

## Questions to Answer During Implementation

1. **Should we split persistent/transient?**
   - Yes - different patterns, cleaner organization

2. **Should entity processors share helper functions?**
   - Yes - create `entity_processors/utils.rs` for common code

3. **Should we keep main.rs generation in main_rs.rs?**
   - Yes - just simplify it significantly

4. **How to handle cross-entity dependencies?**
   - Use message_processor.rs as coordinator
   - Processors only depend on core types, not each other

5. **Should we generate tests for each processor?**
   - Nice to have, not critical for first version
   - Can add in follow-up PR

---

## Conclusion

This refactoring will:
- ✅ **5-10x faster** incremental builds
- ✅ **2-3x faster** clean builds
- ✅ Better code organization
- ✅ Easier to debug (one entity per file)
- ✅ Enables future optimizations

The pattern can be applied to other large generated codebases (ingestion server, dashboard, etc.).

**Let's make Rust compilation actually fast! 🚀**
