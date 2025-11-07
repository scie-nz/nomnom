# Nomnom: General-Purpose Data Transformation Library

## Executive Summary

Extract the general-purpose data transformation and entity framework logic from `data_processor` into a new crate called `nomnom`. This library will provide format-agnostic data parsing, transformation, and entity derivation capabilities.

**Goal**: Create a reusable library for building type-safe, declarative data transformation pipelines from YAML configurations, with no data processing or structured data-specific dependencies.

---

## Architecture Overview

```
nomnom/
├── Core entity framework (traits, base types)
├── Transform registry system
├── Code generation (YAML → Rust)
├── Derivation patterns (parent, repeated_for, etc.)
├── Field extraction abstractions
└── Serialization framework

data_processor/
├── depends on: nomnom
├── structured data-specific transforms
├── Healthcare domain entities
└── structured data message parsing
```

---

## What Goes Into `nomnom`

### 1. **Core Entity Framework**
**Purpose**: Generic entity trait and base types for any structured data format

**Components**:
- `Entity` trait (rename from `Hl7Entity`)
  ```rust
  pub trait Entity: Serialize + Sized {
      const NAME: &'static str;

      fn to_dict(&self) -> HashMap<String, FieldValue>;
      fn to_json(&self) -> Result<String, serde_json::Error>;
      fn to_json_pretty(&self) -> Result<String, serde_json::Error>;
  }
  ```
- `FieldValue` enum (String, Int, Float, Bool, List, Null)
- `EntityError` type for parsing errors
- `ParsingContext` for providing additional data during extraction

**Cleanup**:
- ❌ Remove: `const SEGMENT_TYPE` (HL7-specific)
- ❌ Remove: `from_segment()` method (HL7-specific)
- ✅ Keep: Serialization methods (to_dict, to_json)
- ✅ Add: Generic `from_parent()` and `from_parent_repeated()` patterns

**File**: `nomnom/src/entity.rs`

---

### 2. **Transform Registry System**
**Purpose**: Plugin architecture for registering and calling transformation functions

**Components**:
- `TransformRegistry` trait
  ```rust
  pub trait TransformRegistry {
      fn register(&mut self, name: &str, func: Box<dyn TransformFn>);
      fn call(&self, name: &str, args: &[Value]) -> Result<Value, Error>;
  }
  ```
- `TransformFn` trait for typed transforms
- Python bridge via PyO3 (optional feature: `python-bridge`)
- Rust-native transform implementations

**Cleanup**:
- ✅ Keep: Core registry pattern
- ✅ Keep: Python bridge (as optional feature)
- ❌ Move out: Specific transforms like `extract_from_hl7_segment` → goes to `data_processor`
- ✅ Keep: Generic transforms like `copy_from_context`, `coalesce`

**Files**:
- `nomnom/src/transform_registry.rs`
- `nomnom/src/python_bridge.rs` (feature-gated)

---

### 3. **Code Generation Framework**
**Purpose**: YAML-to-Rust codegen for entities, independent of data format

**Components**:
- `EntityDef` struct (entity configuration)
- `FieldDef` struct (field configuration)
- `ComputedFrom` struct (field derivation)
- Rust codegen (`generate_entity()`, `generate_derived_entity()`)
- Python bindings codegen (PyO3 wrapper generation)
- YAML loader and validator

**Cleanup**:
- ✅ Keep: All entity patterns (singleton, repeated, derived, root)
- ✅ Keep: `repeated_for`, `parent`, `parents` configuration
- ✅ Keep: `computed_from` with transform references
- ❌ Remove: Any HL7-specific defaults or assumptions
- ✅ Generalize: Field path syntax (currently "DG1.3.1" → generic "field.subfield.index")

**Files**:
- `nomnom/codegen/types.rs`
- `nomnom/codegen/rust_codegen.rs`
- `nomnom/codegen/python_codegen.rs`
- `nomnom/codegen/yaml_loader.rs`
- `nomnom/codegen/utils.rs`

---

### 4. **Field Extraction Abstraction**
**Purpose**: Generic pattern for extracting fields from structured data

**Components**:
- `FieldPath` struct (generic path like `message.field.subfield[0]`)
- `Extractor` trait
  ```rust
  pub trait Extractor {
      fn extract(&self, path: &FieldPath) -> Option<String>;
  }
  ```
- Generic extraction helpers

**Cleanup**:
- ✅ Generalize: `FieldPath` to work with any delimiter (not just `.`)
- ❌ Remove: HL7-specific assumptions (pipe-delimited, caret-separated)
- ✅ Keep: The pattern of "extract value at path from structured data"

**File**: `nomnom/src/extraction.rs`

---

### 5. **Derivation Patterns**
**Purpose**: Patterns for deriving entities from parent entities

**Components**:
- Root entity pattern (file → message)
- Parent derivation (single parent via `parent: EntityName`)
- Repeated derivation (`repeated_for` pattern)
- Multi-parent derivation (`parents: [...]`)
- Transform-based field computation

**Cleanup**:
- ✅ Keep: All patterns as generic abstractions
- ✅ Document: Each pattern with non-HL7 examples
- ✅ Examples: CSV → Rows → Cells, JSON → Objects → Fields

**Documentation**: `nomnom/docs/derivation-patterns.md`

---

### 6. **Serialization Framework**
**Purpose**: Generic serialization to multiple formats

**Components**:
- Serialization trait
  ```rust
  pub trait Serializable {
      fn to_dict(&self) -> HashMap<String, Value>;
      fn to_json(&self) -> Result<String, Error>;
      fn to_ndjson_line(&self) -> Result<String, Error>;
  }
  ```
- NDJSON writer
- JSON writer
- Dict converter

**Cleanup**:
- ✅ Keep: All serialization methods
- ✅ Add: More formats (CSV, Parquet as optional features)

**File**: `nomnom/src/serialization.rs`

---

## What Stays in `data_processor`

### 1. **structured data-Specific Transforms**
- `extract_from_hl7_segment(segment, path)` - Parse HL7 field paths like "DG1.3.1"
- `extract_msh_field(message, field_index)` - Special MSH handling
- `build_segment_index(raw_message)` - Parse HL7 message into segments

**File**: `data_processor/src/transforms.rs`

---

### 2. **structured data Segment Parsing**
- `Segment` struct
  ```rust
  pub struct Segment {
      segment_type: String,
      fields: Vec<String>,
      // HL7-specific parsing logic
  }
  ```
- Field/component/subcomponent parsing
- HL7 escape sequence handling

**File**: `data_processor/src/segment.rs`

---

### 3. **Healthcare Domain Entities**
- All entity YAML configs (`config/entities/*.yaml`)
- Generated entity structs (Category, Action, User, etc.)
- structured dataMessageFile, Hl7v2Message root entities

**Files**:
- `config/entities/` (YAML configs)
- `data_processor/src/generated.rs` (generated code)

---

### 4. **structured data-Specific Integration**
- PyO3 module registration for HL7 entities
- Python bindings for HL7 entity classes
- HL7-specific transform registration

**File**: `data_processor/src/lib.rs`

---

## Migration Strategy

### Phase 1: Create `nomnom` Crate Structure
```
nomnom/
├── Cargo.toml
├── src/
│   ├── lib.rs
│   ├── entity.rs           # Core Entity trait
│   ├── transform_registry.rs
│   ├── extraction.rs
│   ├── serialization.rs
│   └── python_bridge.rs    # Optional PyO3 feature
├── codegen/
│   ├── mod.rs
│   ├── types.rs           # EntityDef, FieldDef, etc.
│   ├── rust_codegen.rs
│   ├── python_codegen.rs
│   ├── yaml_loader.rs
│   └── utils.rs
└── build.rs               # Codegen entry point
```

**Dependencies**:
```toml
[dependencies]
serde = { version = "1.0", features = ["derive"] }
serde_json = "1.0"
pyo3 = { version = "0.20", optional = true, features = ["abi3-py38"] }

[features]
default = []
python-bridge = ["pyo3"]
```

---

### Phase 2: Extract Generic Code
1. **Copy and cleanup `entity.rs`**:
   - Remove `const SEGMENT_TYPE`
   - Remove `from_segment()` method
   - Rename `Hl7Entity` → `Entity`
   - Keep serialization methods

2. **Copy and cleanup `python_transform.rs` → `python_bridge.rs`**:
   - Make module name configurable
   - Remove hard-coded "data_processor.transforms" import
   - Add generic transform call interface

3. **Copy entire `codegen/` directory**:
   - Remove HL7-specific comments
   - Remove HL7-specific default values
   - Generalize field path handling

4. **Extract `extraction.rs`**:
   - Remove HL7 segment parsing
   - Keep generic `FieldPath` logic
   - Add `Extractor` trait

---

### Phase 3: Update `data_processor` to Use `nomnom`
1. **Add dependency**:
   ```toml
   [dependencies]
   nomnom = { path = "../nomnom", features = ["python-bridge"] }
   ```

2. **Implement `nomnom::Entity` for HL7 entities**:
   ```rust
   impl nomnom::Entity for CategoryCore {
       const NAME: &'static str = "Category";
       // ... implement required methods
   }
   ```

3. **Register HL7-specific transforms**:
   ```rust
   registry.register("extract_from_hl7_segment", extract_from_hl7_segment);
   registry.register("extract_msh_field", extract_msh_field);
   ```

4. **Keep HL7-specific code**:
   - `segment.rs` stays
   - HL7 transforms stay
   - Entity configs stay

---

### Phase 4: Update Build Process
1. **Move codegen to `nomnom`**:
   - `data_processor/build.rs` imports from `nomnom::codegen`
   - YAML files stay in `data_processor`
   - Generated code stays in `data_processor`

2. **Example `data_processor/build.rs`**:
   ```rust
   use nomnom::codegen::{load_entities, generate_rust_code};

   fn main() {
       // Load HL7-specific entity configs
       let entities = load_entities("config/entities");

       // Generate code using nomnom's codegen
       generate_rust_code(&entities, "src/generated.rs");
   }
   ```

---

## Benefits

### 1. **Reusability**
- `nomnom` can parse CSV, JSON, XML, EDI, etc.
- Same entity framework for any structured data
- Transform registry works for any domain

### 2. **Separation of Concerns**
- Healthcare logic isolated in `data_processor`
- Generic data transformation in `nomnom`
- Clear boundaries

### 3. **Testability**
- Test `nomnom` with simple data formats
- No HL7 complexity in generic tests
- HL7-specific tests stay in `data_processor`

### 4. **Documentation**
- `nomnom` docs show non-data processing examples
- Easier to understand patterns
- Broader applicability

---

## Example: Using `nomnom` for CSV

```yaml
# config/csv_entities/row.yaml
entity:
  name: CsvRow
  source_type: derived
  parent: CsvFile
  fields:
    - name: first_name
      type: String
      computed_from:
        transform: extract_csv_field
        sources:
          - source: parent
            field: raw_line
        args:
          column_index: 0
```

```rust
use nomnom::Entity;

// Register CSV-specific transform
registry.register("extract_csv_field", |line, index| {
    line.split(',').nth(index).map(|s| s.to_string())
});

// Generate entities from YAML
let entities = nomnom::codegen::load_entities("config/csv_entities");
nomnom::codegen::generate_rust_code(&entities, "src/generated.rs");
```

---

## Example: Using `nomnom` for JSON

```yaml
# config/json_entities/user.yaml
entity:
  name: User
  source_type: derived
  parent: JsonDocument
  fields:
    - name: username
      type: String
      computed_from:
        transform: extract_json_field
        sources:
          - source: parent
            field: raw_json
        args:
          json_path: "$.user.name"
```

---

## Migration Checklist

### Pre-Migration
- [ ] Review all files in `data_processor/src/` and `data_processor/codegen/`
- [ ] Identify HL7-specific code (grep for "hl7", "segment", "MSH", etc.)
- [ ] Document all transform functions and their purposes
- [ ] Create test suite for generic functionality

### Phase 1: Create `nomnom`
- [ ] Create `nomnom/` directory structure
- [ ] Set up `Cargo.toml` with features
- [ ] Create basic `lib.rs` with module exports
- [ ] Set up CI/testing for `nomnom`

### Phase 2: Extract Code
- [ ] Extract and cleanup `entity.rs`
- [ ] Extract and cleanup transform registry
- [ ] Extract entire `codegen/` directory
- [ ] Extract field extraction abstractions
- [ ] Add comprehensive tests for each module

### Phase 3: Update `data_processor`
- [ ] Add `nomnom` dependency
- [ ] Update imports to use `nomnom::`
- [ ] Implement `nomnom::Entity` for all entities
- [ ] Register HL7 transforms with nomnom registry
- [ ] Update `build.rs` to use nomnom codegen

### Phase 4: Testing & Documentation
- [ ] All `data_processor` tests pass
- [ ] Create `nomnom` documentation with examples
- [ ] Create migration guide for other users
- [ ] Add CSV/JSON examples to demonstrate generality

### Phase 5: Cleanup
- [ ] Remove duplicate code from `data_processor`
- [ ] Update README files
- [ ] Publish `nomnom` crate (if open-sourcing)

---

## Naming Rationale

**Why "nomnom"?**
- Short, memorable, playful
- Suggests "consuming" and "digesting" data
- Format-agnostic (not tied to any domain)
- Available on crates.io (check first!)

**Alternative names**:
- `datanom` - "data consumer"
- `structparse` - "structured data parser"
- `entigen` - "entity generator"
- `yamlents` - "YAML entities"

---

## Timeline Estimate

- **Phase 1** (Create structure): 1-2 hours
- **Phase 2** (Extract code): 4-6 hours
- **Phase 3** (Update data_processor): 2-3 hours
- **Phase 4** (Build process): 1-2 hours
- **Testing & Documentation**: 3-4 hours

**Total**: 11-17 hours

---

## Questions to Resolve

1. **Should `nomnom` be a workspace member or separate repo?**
   - Same repo: Easier to iterate, shared CI
   - Separate repo: Cleaner separation, independent versioning

2. **Should Python bindings be optional?**
   - Yes, use feature flag `python-bridge`
   - Allows pure Rust usage without PyO3

3. **How generic should field paths be?**
   - Support JSON path syntax? (`$.field.subfield[0]`)
   - Support XPath for XML?
   - Start simple, extend later

4. **Should we support other output formats?**
   - Parquet (via arrow)
   - Avro
   - Protobuf
   - Add as optional features

---

## Success Criteria

✅ `nomnom` has zero references to "HL7", "segment", "data processing", etc.
✅ `nomnom` documentation uses CSV/JSON examples, not HL7
✅ All `data_processor` tests pass after migration
✅ `nomnom` has its own comprehensive test suite
✅ Can demonstrate `nomnom` parsing a non-HL7 format (CSV or JSON)
✅ Build time for `data_processor` is same or better
✅ No duplicate code between crates

---

## Future Enhancements (Post-Migration)

1. **Add more transforms**:
   - Date parsing/formatting
   - String manipulation (trim, uppercase, etc.)
   - Math operations (sum, avg, etc.)
   - Conditional logic (if/then/else)

2. **Add validation framework**:
   - Required fields
   - Type validation
   - Custom validators

3. **Add more derivation patterns**:
   - Flattening (nested → flat)
   - Aggregation (many → one)
   - Splitting (one → many)

4. **Performance optimizations**:
   - Parallel entity creation
   - Lazy field evaluation
   - Memory pooling

5. **IDE support**:
   - YAML schema for entity configs
   - VS Code extension
   - LSP for YAML validation
