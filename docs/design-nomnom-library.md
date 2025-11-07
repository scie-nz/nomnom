# Design Doc: Nomnom - Generic Data Transformation Library

**Author**: Claude (with user guidance)
**Date**: 2025-01-24
**Status**: Proposed
**Related**: [generalize-entity-framework-proposal.md](./generalize-entity-framework-proposal.md)

---

## Abstract

This document proposes extracting the general-purpose data transformation and entity framework logic from the `data_processor` Rust crate into a new library called `nomnom`. The goal is to create a reusable, format-agnostic library for building type-safe, declarative data transformation pipelines from YAML configurations, removing all data processing and structured data-specific dependencies from the core framework.

---

## Background

### Current State

The `data_processor` Rust crate currently contains two distinct categories of functionality:

1. **General-purpose data transformation infrastructure**:
   - Entity framework (traits, base types)
   - Transform registry system
   - YAML-to-Rust code generation
   - Field extraction abstractions
   - Derivation patterns (parent, repeated_for, multi-parent)
   - Serialization framework

2. **structured data-specific implementation**:
   - Segment parsing (pipe/caret delimiters)
   - HL7-specific transforms
   - Healthcare domain entities
   - HL7 message validation

This mixing of concerns creates several problems:
- The generic framework cannot be reused for other data formats (CSV, JSON, XML, EDI)
- Healthcare-specific concepts leak into the core abstractions
- Testing the framework requires HL7 knowledge
- Documentation and examples are data processing-focused

### Motivation

After completing the migration to a format-agnostic entity framework (removing `segment_type` coupling), it became clear that the core transformation logic is now truly generic. The only remaining HL7-specific parts are:
- The `Segment` struct and its parsing logic
- HL7-specific transform implementations
- Healthcare domain entity definitions

This presents an opportunity to extract a high-quality, reusable library that could benefit projects outside data processing.

---

## Goals

### Primary Goals

1. **Extract generic transformation logic** into a standalone `nomnom` crate
2. **Remove all HL7/data processing references** from the core framework
3. **Maintain full backward compatibility** for `data_processor` users
4. **Demonstrate generality** with non-data processing examples (CSV, JSON)
5. **Enable reuse** of the framework for other structured data formats

### Non-Goals

1. **Not redesigning the architecture** - extract what works, don't rebuild
2. **Not breaking existing HL7 functionality** - this is a refactoring, not a rewrite
3. **Not publishing to crates.io initially** - internal use first, publish later if desired
4. **Not adding new features** - focus on extraction and cleanup only
5. **Not changing the YAML configuration format** - keep it compatible

---

## Detailed Design

### Architecture

```
Workspace Structure:
├── nomnom/                          # New crate: generic framework
│   ├── Cargo.toml
│   ├── src/
│   │   ├── lib.rs                   # Public API
│   │   ├── entity.rs                # Entity trait
│   │   ├── transform_registry.rs   # Transform plugin system
│   │   ├── extraction.rs            # Field extraction
│   │   ├── serialization.rs         # to_dict, to_json, ndjson
│   │   └── python_bridge.rs         # PyO3 integration (optional)
│   ├── codegen/
│   │   ├── mod.rs
│   │   ├── types.rs                 # EntityDef, FieldDef, etc.
│   │   ├── rust_codegen.rs          # Rust code generation
│   │   ├── python_codegen.rs        # Python bindings generation
│   │   ├── yaml_loader.rs           # YAML parsing
│   │   └── utils.rs                 # Shared utilities
│   └── build.rs                     # Build script support
│
└── data_processor/                   # Existing crate: HL7-specific
    ├── Cargo.toml                   # Now depends on nomnom
    ├── src/
    │   ├── lib.rs
    │   ├── segment.rs               # HL7 segment parsing (stays)
    │   ├── transforms.rs            # HL7 transforms (stays)
    │   └── generated.rs             # Generated entities (stays)
    └── build.rs                     # Uses nomnom::codegen
```

### Component Breakdown

#### 1. Core Entity Framework (`nomnom/src/entity.rs`)

**Purpose**: Define the base trait and types for all entities, independent of data format.

**API Design**:
```rust
/// Core trait for all entities generated from YAML configurations.
///
/// This trait is format-agnostic and can be used for any structured data
/// (CSV, JSON, XML, HL7, EDI, etc.).
pub trait Entity: Serialize + Sized {
    /// Entity name (e.g., "User", "Transaction", "Category")
    const NAME: &'static str;

    /// Convert entity to a dictionary representation
    fn to_dict(&self) -> HashMap<String, FieldValue>;

    /// Serialize to JSON
    fn to_json(&self) -> Result<String, serde_json::Error> {
        serde_json::to_string(self)
    }

    /// Serialize to pretty JSON
    fn to_json_pretty(&self) -> Result<String, serde_json::Error> {
        serde_json::to_string_pretty(self)
    }
}

/// Represents a field value in an entity
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(untagged)]
pub enum FieldValue {
    String(String),
    Int(i64),
    Float(f64),
    Bool(bool),
    List(Vec<String>),
    Null,
}

/// Errors that can occur during entity creation
#[derive(Debug, thiserror::Error)]
pub enum EntityError {
    #[error("Missing required field: {field}")]
    MissingField { field: String },

    #[error("Invalid field value: {field} = {value}")]
    InvalidValue { field: String, value: String },

    #[error("Transform error: {0}")]
    TransformError(String),
}

/// Context for providing additional data during entity creation
#[derive(Debug, Clone, Default)]
pub struct ParsingContext {
    values: HashMap<String, String>,
}

impl ParsingContext {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn with_value(mut self, key: impl Into<String>, value: impl Into<String>) -> Self {
        self.values.insert(key.into(), value.into());
        self
    }

    pub fn get(&self, key: &str) -> Option<&str> {
        self.values.get(key).map(|s| s.as_str())
    }
}
```

**Changes from Current Code**:
- ❌ Remove: `const SEGMENT_TYPE` (HL7-specific)
- ❌ Remove: `from_segment()` method (HL7-specific)
- ✅ Rename: `Hl7Entity` → `Entity`
- ✅ Keep: All serialization methods unchanged
- ✅ Enhance: Better error types with `thiserror`

---

#### 2. Transform Registry (`nomnom/src/transform_registry.rs`)

**Purpose**: Provide a plugin system for registering and calling transformation functions.

**API Design**:
```rust
use std::collections::HashMap;
use serde_json::Value;

/// A transformation function that can be registered and called
pub trait TransformFn: Send + Sync {
    /// Call the transform with the given arguments
    fn call(&self, args: &HashMap<String, Value>) -> Result<Option<String>, TransformError>;
}

/// Registry for transformation functions
pub struct TransformRegistry {
    transforms: HashMap<String, Box<dyn TransformFn>>,
}

impl TransformRegistry {
    pub fn new() -> Self {
        Self {
            transforms: HashMap::new(),
        }
    }

    /// Register a transform function
    pub fn register(&mut self, name: impl Into<String>, func: Box<dyn TransformFn>) {
        self.transforms.insert(name.into(), func);
    }

    /// Check if a transform is registered
    pub fn has(&self, name: &str) -> bool {
        self.transforms.contains_key(name)
    }

    /// Call a transform by name
    pub fn call(
        &self,
        name: &str,
        args: &HashMap<String, Value>,
    ) -> Result<Option<String>, TransformError> {
        self.transforms
            .get(name)
            .ok_or_else(|| TransformError::NotFound(name.to_string()))?
            .call(args)
    }

    /// List all registered transforms
    pub fn list_transforms(&self) -> Vec<&str> {
        self.transforms.keys().map(|s| s.as_str()).collect()
    }
}

#[derive(Debug, thiserror::Error)]
pub enum TransformError {
    #[error("Transform not found: {0}")]
    NotFound(String),

    #[error("Invalid arguments: {0}")]
    InvalidArgs(String),

    #[error("Transform failed: {0}")]
    Failed(String),
}

/// Built-in generic transforms
pub mod builtin {
    use super::*;

    /// Copy value from context
    pub struct CopyFromContext;

    impl TransformFn for CopyFromContext {
        fn call(&self, args: &HashMap<String, Value>) -> Result<Option<String>, TransformError> {
            // Implementation
        }
    }

    /// Coalesce: return first non-null value
    pub struct Coalesce;

    impl TransformFn for Coalesce {
        fn call(&self, args: &HashMap<String, Value>) -> Result<Option<String>, TransformError> {
            // Implementation
        }
    }
}
```

**Features**:
- Generic transform registration
- Type-safe transform arguments via `serde_json::Value`
- Built-in transforms: `copy_from_context`, `coalesce`
- Python bridge via optional feature flag

---

#### 3. Python Bridge (`nomnom/src/python_bridge.rs`)

**Purpose**: Allow calling Python transform functions from Rust (optional feature).

**API Design**:
```rust
#[cfg(feature = "python-bridge")]
use pyo3::prelude::*;

#[cfg(feature = "python-bridge")]
pub struct PyTransformRegistry {
    module_name: String,
}

#[cfg(feature = "python-bridge")]
impl PyTransformRegistry {
    /// Create a new Python transform registry
    ///
    /// # Arguments
    /// * `module_name` - Python module containing TRANSFORM_REGISTRY
    pub fn new(module_name: impl Into<String>) -> Self {
        Self {
            module_name: module_name.into(),
        }
    }

    /// Call a Python transform function
    pub fn call_transform(
        &self,
        py: Python,
        name: &str,
        kwargs: HashMap<String, PyObject>,
    ) -> PyResult<Option<String>> {
        // Implementation using PyO3
    }
}
```

**Feature Flag**:
```toml
[features]
default = []
python-bridge = ["pyo3"]
```

---

#### 4. Code Generation Framework (`nomnom/codegen/`)

**Purpose**: Generate Rust structs and Python bindings from YAML entity configurations.

**Types** (`nomnom/codegen/types.rs`):
```rust
/// Entity definition loaded from YAML
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct EntityDef {
    pub name: String,
    pub source_type: String,  // "derived", "root", etc.
    pub repetition: Option<String>,  // "singleton", "repeated"
    pub doc: Option<String>,

    // Derivation config
    pub parent: Option<String>,
    pub parents: Vec<ParentDef>,
    pub repeated_for: Option<RepeatedForDef>,

    // Fields
    pub fields: Vec<FieldDef>,

    // Serialization
    pub serialization: Vec<String>,

    // Database (for permanent entities)
    pub database: Option<DatabaseConfig>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct FieldDef {
    pub name: String,
    pub r#type: String,
    pub nullable: bool,
    pub doc: Option<String>,
    pub computed_from: Option<ComputedFrom>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ComputedFrom {
    pub transform: String,
    pub sources: Vec<Source>,
    pub args: HashMap<String, serde_yaml::Value>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RepeatedForDef {
    pub entity: String,
    pub field: String,
    pub each_known_as: String,
}
```

**Rust Code Generation** (`nomnom/codegen/rust_codegen.rs`):
```rust
/// Generate Rust struct code for an entity
pub fn generate_entity_struct(entity: &EntityDef) -> String {
    // Generates:
    // - #[derive(Debug, Clone, Serialize, Deserialize)]
    // - pub struct EntityNameCore { fields... }
    // - impl Entity for EntityNameCore
    // - from_parent() or from_parent_repeated() constructors
}

/// Generate all entities from YAML configs
pub fn generate_all_entities(entities: &[EntityDef], output_path: &Path) -> Result<(), Error> {
    // Write generated code to output_path
}
```

**Python Code Generation** (`nomnom/codegen/python_codegen.rs`):
```rust
/// Generate PyO3 bindings for an entity
pub fn generate_python_bindings(entity: &EntityDef) -> String {
    // Generates:
    // - #[pyclass] wrapper
    // - #[pymethods] for to_dict, to_json, etc.
    // - Constructor methods
}
```

**YAML Loader** (`nomnom/codegen/yaml_loader.rs`):
```rust
/// Load entity definitions from YAML files
pub fn load_entities(dir_path: &Path) -> Result<Vec<EntityDef>, Error> {
    // Scan directory for *.yaml files
    // Parse each file
    // Validate entity definitions
    // Return all entities
}

/// Load a single entity from YAML
pub fn load_entity(yaml_path: &Path) -> Result<EntityDef, Error> {
    // Parse YAML file
    // Validate structure
    // Return EntityDef
}
```

---

#### 5. Field Extraction (`nomnom/src/extraction.rs`)

**Purpose**: Provide generic abstractions for extracting field values from structured data.

**API Design**:
```rust
/// A path to a field in structured data
///
/// Examples:
/// - "user.name" (JSON/object notation)
/// - "row.3" (CSV column 3)
/// - "DG1.3.1" (HL7 field notation)
#[derive(Debug, Clone, PartialEq)]
pub struct FieldPath {
    segments: Vec<String>,
}

impl FieldPath {
    /// Parse a field path from a string
    ///
    /// # Examples
    /// ```
    /// let path = FieldPath::parse("user.name");
    /// let path = FieldPath::parse("row[3]");
    /// ```
    pub fn parse(s: &str) -> Self {
        // Parse path string into segments
    }

    /// Get the segments of this path
    pub fn segments(&self) -> &[String] {
        &self.segments
    }
}

/// Trait for types that can extract field values by path
pub trait Extractor {
    /// Extract a field value at the given path
    ///
    /// Returns None if the path doesn't exist or the field is empty
    fn extract(&self, path: &FieldPath) -> Option<String>;

    /// Extract a field value or return a default
    fn extract_or(&self, path: &FieldPath, default: &str) -> String {
        self.extract(path).unwrap_or_else(|| default.to_string())
    }
}
```

**Design Rationale**:
- `FieldPath` is generic - not tied to any format
- Delimiter is part of the path syntax, not hardcoded
- `Extractor` trait allows any data structure to implement extraction
- HL7-specific parsing (pipe/caret delimiters) goes in `data_processor`

---

#### 6. Serialization (`nomnom/src/serialization.rs`)

**Purpose**: Provide serialization to multiple output formats.

**API Design**:
```rust
use serde::Serialize;
use std::collections::HashMap;

/// Trait for entities that can be serialized
pub trait Serializable: Serialize {
    /// Convert to a dictionary (HashMap)
    fn to_dict(&self) -> HashMap<String, FieldValue>;

    /// Serialize to JSON
    fn to_json(&self) -> Result<String, serde_json::Error> {
        serde_json::to_string(self)
    }

    /// Serialize to pretty JSON
    fn to_json_pretty(&self) -> Result<String, serde_json::Error> {
        serde_json::to_string_pretty(self)
    }

    /// Serialize to NDJSON line (no newline at end)
    fn to_ndjson_line(&self) -> Result<String, serde_json::Error> {
        serde_json::to_string(self)
    }
}

/// Write multiple entities to NDJSON file
pub fn write_ndjson<T: Serializable>(
    entities: &[T],
    path: &Path,
) -> Result<(), std::io::Error> {
    // Write each entity as a JSON line
}

/// Write entities to JSON array file
pub fn write_json_array<T: Serializable>(
    entities: &[T],
    path: &Path,
) -> Result<(), std::io::Error> {
    // Write as JSON array
}
```

---

### Derivation Patterns

These patterns are already implemented in the current codebase and will move to `nomnom` unchanged:

#### Pattern 1: Root Entity
```yaml
entity:
  name: File
  source_type: root
  fields:
    - name: filename
      type: String
```

**Use Cases**:
- File metadata (CSV, JSON, HL7 files)
- HTTP request/response
- Database connection

---

#### Pattern 2: Parent Derivation (Single Parent)
```yaml
entity:
  name: Message
  source_type: derived
  parent: File
  fields:
    - name: content
      computed_from:
        transform: read_file_content
        sources:
          - source: parent
            field: filename
```

**Use Cases**:
- Parse file content
- Extract structured data from parent
- Transform one entity to another

---

#### Pattern 3: Repeated Derivation
```yaml
entity:
  name: Row
  source_type: derived
  repetition: repeated
  repeated_for:
    entity: File
    field: lines
    each_known_as: line
  fields:
    - name: data
      computed_from:
        transform: parse_line
        sources:
          - line
```

**Use Cases**:
- CSV rows from file
- JSON objects from array
- HL7 segments from message

---

#### Pattern 4: Multi-Parent Derivation
```yaml
entity:
  name: EnrichedRow
  source_type: derived
  parents:
    - name: row
      type: Row
    - name: metadata
      type: Metadata
  fields:
    - name: combined_data
      computed_from:
        transform: merge_data
        sources:
          - source: row
            field: data
          - source: metadata
            field: info
```

**Use Cases**:
- Join data from multiple sources
- Enrich entities with metadata
- Combine related entities

---

### Migration from HL7 Ingestion

**Files That Move to `nomnom`**:

| Current File | New Location | Changes |
|--------------|--------------|---------|
| `src/entity.rs` | `nomnom/src/entity.rs` | Remove `SEGMENT_TYPE`, `from_segment()` |
| `src/python_transform.rs` | `nomnom/src/python_bridge.rs` | Make module name configurable |
| `codegen/types.rs` | `nomnom/codegen/types.rs` | Remove HL7-specific comments |
| `codegen/rust_codegen.rs` | `nomnom/codegen/rust_codegen.rs` | Remove HL7-specific defaults |
| `codegen/python_codegen.rs` | `nomnom/codegen/python_codegen.rs` | No changes needed |
| `codegen/yaml_loader.rs` | `nomnom/codegen/yaml_loader.rs` | No changes needed |
| `codegen/utils.rs` | `nomnom/codegen/utils.rs` | No changes needed |

**Files That Stay in `data_processor`**:

| File | Reason |
|------|--------|
| `src/segment.rs` | HL7-specific parsing (pipe/caret delimiters) |
| `src/transforms.rs` | HL7-specific transforms (`extract_from_hl7_segment`, etc.) |
| `src/lib.rs` | HL7-specific module registration |
| `src/generated.rs` | HL7 entities (generated from YAML) |
| `build.rs` | Will import from `nomnom::codegen` |

**New `data_processor/build.rs`**:
```rust
use nomnom::codegen::{load_entities, generate_all_entities};

fn main() {
    println!("cargo:rerun-if-changed=../../../config/entities/");

    // Load HL7 entity configurations
    let entities = load_entities("../../../config/entities")
        .expect("Failed to load entity configs");

    // Generate Rust code using nomnom's codegen
    generate_all_entities(&entities, "src/generated.rs")
        .expect("Failed to generate entity code");
}
```

---

## Implementation Plan

### Phase 1: Create `nomnom` Crate (1-2 hours)

**Tasks**:
1. Create `nomnom/` directory at workspace root
2. Create `Cargo.toml` with dependencies and features
3. Create module structure (`src/lib.rs`, `src/entity.rs`, etc.)
4. Create `codegen/` subdirectory
5. Set up basic tests and CI

**Deliverables**:
- `nomnom/` compiles successfully
- Basic `Entity` trait defined
- Module exports configured

**Acceptance Criteria**:
- `cargo build` succeeds
- `cargo test` runs (even if no tests yet)

---

### Phase 2: Extract Core Components (4-6 hours)

**Tasks**:
1. **Extract `entity.rs`**:
   - Copy from `data_processor/src/entity.rs`
   - Remove `const SEGMENT_TYPE`
   - Remove `from_segment()` method
   - Rename `Hl7Entity` → `Entity`
   - Add comprehensive tests

2. **Extract transform registry**:
   - Copy `python_transform.rs` → `python_bridge.rs`
   - Make module name configurable (not hardcoded to "data_processor.transforms")
   - Create `transform_registry.rs` with core registry logic
   - Add built-in transforms (`copy_from_context`, `coalesce`)
   - Feature-gate PyO3 code with `python-bridge` feature

3. **Extract codegen**:
   - Copy entire `codegen/` directory
   - Remove HL7-specific comments and documentation
   - Update imports to use `nomnom::` instead of `crate::`
   - Add tests for code generation

4. **Extract field extraction**:
   - Create `extraction.rs` with `FieldPath` and `Extractor` trait
   - Move generic extraction logic from `data_processor`
   - Document with non-HL7 examples

5. **Create serialization module**:
   - Extract serialization traits and helpers
   - Add NDJSON writer
   - Add JSON array writer

**Deliverables**:
- All generic code in `nomnom`
- Comprehensive unit tests for each module
- Documentation with non-HL7 examples

**Acceptance Criteria**:
- All `nomnom` tests pass
- No references to "HL7", "segment", "data processing" in `nomnom` code
- Can generate an entity from a simple YAML config

---

### Phase 3: Update `data_processor` (2-3 hours)

**Tasks**:
1. **Add `nomnom` dependency**:
   ```toml
   [dependencies]
   nomnom = { path = "../nomnom", features = ["python-bridge"] }
   ```

2. **Update imports**:
   - Replace `use crate::entity::*` with `use nomnom::entity::*`
   - Replace `use crate::python_transform::*` with `use nomnom::python_bridge::*`
   - Update all references throughout codebase

3. **Update `build.rs`**:
   - Import codegen from `nomnom::codegen`
   - Keep YAML configs in `data_processor/config/entities/`
   - Generate code to `data_processor/src/generated.rs`

4. **Keep HL7-specific code**:
   - `segment.rs` stays unchanged
   - `transforms.rs` stays unchanged
   - Entity configs stay in place

5. **Update tests**:
   - Ensure all HL7 tests still pass
   - No changes to test logic needed (internal refactoring only)

**Deliverables**:
- `data_processor` compiles and passes all tests
- No duplicate code between crates
- Clean separation of concerns

**Acceptance Criteria**:
- All 422 tests in `data_processor` still pass
- Build time is same or better
- No functionality changes visible to users

---

### Phase 4: Documentation & Examples (3-4 hours)

**Tasks**:
1. **Write `nomnom` README**:
   - Overview of the library
   - CSV parsing example
   - JSON parsing example
   - API documentation links

2. **Write examples**:
   - `examples/csv_parser/` - Parse CSV files
   - `examples/json_parser/` - Parse JSON documents
   - Each example includes:
     - YAML entity configs
     - Custom transforms
     - Build script
     - Tests

3. **Write API documentation**:
   - Document all public types and traits
   - Add code examples to each module
   - Create high-level architecture diagram

4. **Migration guide**:
   - Document for potential external users
   - Show how to use `nomnom` for new data formats
   - Include troubleshooting section

**Deliverables**:
- Complete `nomnom/README.md`
- Two working examples (CSV, JSON)
- API documentation with examples
- Migration guide

**Acceptance Criteria**:
- Examples compile and run successfully
- Documentation is clear and complete
- No data processing/HL7 references in examples

---

### Phase 5: Testing & Cleanup (2-3 hours)

**Tasks**:
1. **Add comprehensive tests**:
   - Unit tests for each `nomnom` module
   - Integration tests showing CSV and JSON parsing
   - Property-based tests for field extraction
   - Benchmark tests for performance

2. **Code review and cleanup**:
   - Remove any remaining HL7 references from `nomnom`
   - Ensure consistent naming conventions
   - Add missing documentation
   - Fix clippy warnings

3. **CI/CD setup**:
   - Add `nomnom` to workspace CI
   - Run tests on multiple Rust versions
   - Check documentation builds correctly
   - Ensure no warnings in release builds

4. **Performance validation**:
   - Ensure no regression in `data_processor` performance
   - Benchmark code generation speed
   - Profile memory usage

**Deliverables**:
- Comprehensive test suite
- Clean, well-documented code
- CI passing for both crates
- Performance benchmarks

**Acceptance Criteria**:
- Test coverage >80%
- No clippy warnings
- CI passes on main branch
- No performance regressions

---

## Testing Strategy

### Unit Tests (nomnom)

```rust
// nomnom/src/entity.rs
#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_parsing_context() {
        let ctx = ParsingContext::new()
            .with_value("key1", "value1")
            .with_value("key2", "value2");

        assert_eq!(ctx.get("key1"), Some("value1"));
        assert_eq!(ctx.get("key2"), Some("value2"));
        assert_eq!(ctx.get("missing"), None);
    }

    #[test]
    fn test_field_value_serialization() {
        let val = FieldValue::String("test".to_string());
        let json = serde_json::to_string(&val).unwrap();
        assert_eq!(json, r#""test""#);
    }
}
```

### Integration Tests (nomnom)

```rust
// nomnom/tests/csv_example.rs
use nomnom::codegen::{load_entity, generate_entity_struct};

#[test]
fn test_csv_entity_generation() {
    // Load a simple CSV entity config
    let entity = load_entity("tests/fixtures/csv_row.yaml").unwrap();

    // Generate Rust code
    let code = generate_entity_struct(&entity);

    // Verify code contains expected elements
    assert!(code.contains("struct CsvRowCore"));
    assert!(code.contains("impl Entity for CsvRowCore"));
}
```

### Compatibility Tests (data_processor)

```rust
// data_processor/tests/nomnom_compatibility.rs
#[test]
fn test_all_hl7_tests_still_pass() {
    // Run existing test suite
    // This ensures the refactoring didn't break anything
}
```

---

## Examples

### Example 1: CSV Parser

**Entity Config** (`examples/csv_parser/config/row.yaml`):
```yaml
entity:
  name: CsvRow
  source_type: derived
  parent: CsvFile
  repetition: repeated
  repeated_for:
    entity: CsvFile
    field: lines
    each_known_as: line
  fields:
    - name: first_name
      type: String
      computed_from:
        transform: extract_csv_field
        sources:
          - line
        args:
          column_index: 0
    - name: last_name
      type: String
      computed_from:
        transform: extract_csv_field
        sources:
          - line
        args:
          column_index: 1
    - name: email
      type: String
      computed_from:
        transform: extract_csv_field
        sources:
          - line
        args:
          column_index: 2
  serialization:
    - to_dict
    - to_json
```

**Custom Transform**:
```rust
// examples/csv_parser/src/transforms.rs
use nomnom::transform_registry::{TransformFn, TransformError};

pub struct ExtractCsvField;

impl TransformFn for ExtractCsvField {
    fn call(&self, args: &HashMap<String, Value>) -> Result<Option<String>, TransformError> {
        let line = args.get("line")
            .and_then(|v| v.as_str())
            .ok_or_else(|| TransformError::InvalidArgs("missing 'line'".to_string()))?;

        let index = args.get("column_index")
            .and_then(|v| v.as_u64())
            .ok_or_else(|| TransformError::InvalidArgs("missing 'column_index'".to_string()))? as usize;

        let value = line.split(',').nth(index).map(|s| s.trim().to_string());
        Ok(value)
    }
}
```

**Usage**:
```rust
use csv_parser::{CsvFileCore, CsvRowCore};
use nomnom::Entity;

fn main() {
    let file = CsvFileCore::from_filename("data.csv");
    let rows = CsvRowCore::from_parent_repeated(&file);

    for row in rows {
        println!("{}: {} <{}>",
            row.first_name,
            row.last_name,
            row.email
        );
    }
}
```

---

### Example 2: JSON Parser

**Entity Config** (`examples/json_parser/config/user.yaml`):
```yaml
entity:
  name: User
  source_type: derived
  parent: JsonDocument
  fields:
    - name: id
      type: String
      computed_from:
        transform: extract_json_field
        sources:
          - source: parent
            field: raw_json
        args:
          json_path: "$.user.id"
    - name: username
      type: String
      computed_from:
        transform: extract_json_field
        sources:
          - source: parent
            field: raw_json
        args:
          json_path: "$.user.name"
    - name: email
      type: String
      computed_from:
        transform: extract_json_field
        sources:
          - source: parent
            field: raw_json
        args:
          json_path: "$.user.email"
  serialization:
    - to_dict
    - to_json
```

**Custom Transform**:
```rust
use serde_json::Value;
use nomnom::transform_registry::{TransformFn, TransformError};

pub struct ExtractJsonField;

impl TransformFn for ExtractJsonField {
    fn call(&self, args: &HashMap<String, Value>) -> Result<Option<String>, TransformError> {
        let json_str = args.get("raw_json")
            .and_then(|v| v.as_str())
            .ok_or_else(|| TransformError::InvalidArgs("missing 'raw_json'".to_string()))?;

        let path = args.get("json_path")
            .and_then(|v| v.as_str())
            .ok_or_else(|| TransformError::InvalidArgs("missing 'json_path'".to_string()))?;

        let json: Value = serde_json::from_str(json_str)
            .map_err(|e| TransformError::Failed(e.to_string()))?;

        // Simple JSON path implementation
        let value = extract_by_path(&json, path);
        Ok(value.and_then(|v| v.as_str().map(|s| s.to_string())))
    }
}
```

---

## Alternatives Considered

### Alternative 1: Keep Everything in `data_processor`

**Pros**:
- No refactoring needed
- No risk of breaking changes
- Simpler maintenance

**Cons**:
- Cannot reuse framework for other formats
- Framework continues to have HL7-specific coupling
- Documentation and examples remain data processing-focused
- Missed opportunity for broader impact

**Decision**: Rejected - the framework is now truly generic, extraction makes sense

---

### Alternative 2: Complete Rewrite

**Pros**:
- Opportunity to redesign from scratch
- Could use latest Rust patterns
- Fresh start, no legacy code

**Cons**:
- High risk of bugs
- Months of development time
- Would break existing `data_processor` users
- Current architecture works well

**Decision**: Rejected - extract what works, don't rebuild

---

### Alternative 3: Extract Only Codegen

**Pros**:
- Smaller scope, faster to complete
- Codegen is the most reusable part
- Less risk of breaking changes

**Cons**:
- Partial solution - entity traits still HL7-coupled
- Transform registry stays in `data_processor`
- Cannot fully demonstrate generality

**Decision**: Rejected - go all the way, extract entire framework

---

### Alternative 4: Use Existing Library (e.g., `serde`)

**Pros**:
- Don't reinvent the wheel
- Well-tested, maintained library
- Community support

**Cons**:
- `serde` doesn't support our derivation patterns
- No YAML-to-Rust codegen for entity frameworks
- No transform registry system
- Would require significant architecture changes

**Decision**: Rejected - our framework has unique value

---

## Open Questions

### 1. Workspace vs Separate Repository?

**Question**: Should `nomnom` be:
- **Option A**: A workspace member in the same repo as `data_processor`
- **Option B**: A separate Git repository

**Recommendation**: Start with **Option A** (workspace member), move to separate repo later if needed.

**Rationale**:
- Easier to iterate during extraction
- Shared CI/CD initially
- Can split later if we decide to publish to crates.io

---

### 2. Publish to crates.io?

**Question**: Should we publish `nomnom` to crates.io?

**Recommendation**: **Not initially**, but keep it as a future option.

**Rationale**:
- Internal use first, validate the API
- Avoid premature API stability commitments
- Can publish later once mature

---

### 3. Python Bindings: Required or Optional?

**Question**: Should Python bindings (PyO3) be:
- **Option A**: Required dependency
- **Option B**: Optional feature flag
- **Option C**: Separate crate (`nomnom-python`)

**Recommendation**: **Option B** (optional feature flag)

**Rationale**:
- Pure Rust users don't need PyO3
- Keeps `nomnom` core minimal
- Easy to enable for Python integration

---

### 4. How Generic Should Field Paths Be?

**Question**: Should `FieldPath` support:
- **Option A**: Simple dot notation only (`field.subfield`)
- **Option B**: JSON Path syntax (`$.field.subfield[0]`)
- **Option C**: Pluggable path parsers

**Recommendation**: **Option A** initially, **Option C** eventually

**Rationale**:
- Start simple, add complexity as needed
- Pluggable parsers allow format-specific extensions
- Don't over-engineer initially

---

### 5. Additional Output Formats?

**Question**: Should `nomnom` support:
- Parquet (via Apache Arrow)
- Avro
- Protobuf
- CSV output

**Recommendation**: **Add as optional features later**, not in initial version

**Rationale**:
- Focus on extraction first
- Add features based on actual demand
- Keep initial scope manageable

---

## Success Criteria

### Must Have (MVP)

✅ `nomnom` crate compiles and has zero references to "HL7", "segment", or "data processing"
✅ `nomnom` documentation uses CSV and JSON examples, not HL7
✅ All 422 `data_processor` tests pass after migration
✅ `nomnom` has comprehensive unit tests (>80% coverage)
✅ Can generate a working entity from YAML for a non-HL7 format
✅ No code duplication between `nomnom` and `data_processor`
✅ Build time for `data_processor` is same or better

### Should Have (Post-MVP)

✅ CSV parsing example works end-to-end
✅ JSON parsing example works end-to-end
✅ API documentation is complete with examples
✅ Migration guide for using `nomnom` with other formats
✅ Benchmarks showing no performance regression

### Nice to Have (Future)

✅ Published to crates.io
✅ Used by at least one non-data processing project
✅ Community contributions
✅ Additional output formats (Parquet, Avro)
✅ IDE support (YAML schema, LSP)

---

## Timeline & Milestones

### Week 1: Foundation
- **Day 1-2**: Phase 1 (Create `nomnom` structure)
- **Day 3-5**: Phase 2 (Extract core components)
- **Milestone**: `nomnom` crate compiles, basic tests pass

### Week 2: Integration
- **Day 1-2**: Phase 3 (Update `data_processor`)
- **Day 3-4**: Phase 4 (Documentation & examples)
- **Day 5**: Phase 5 (Testing & cleanup)
- **Milestone**: All tests pass, examples work, docs complete

### Total Estimate: 11-17 hours of focused work

---

## Risks & Mitigations

### Risk 1: Breaking Changes in `data_processor`

**Likelihood**: Medium
**Impact**: High

**Mitigation**:
- Keep comprehensive test suite
- Run tests at each step
- Use feature flags for gradual migration
- Keep rollback option available

---

### Risk 2: Performance Regression

**Likelihood**: Low
**Impact**: Medium

**Mitigation**:
- Benchmark before and after
- Profile hot paths
- Keep same algorithms, just reorganize code
- Optimize only if needed

---

### Risk 3: API Design Mistakes

**Likelihood**: Medium
**Impact**: Low (internal use initially)

**Mitigation**:
- Start with working code (extraction, not redesign)
- Get feedback from users before publishing
- Use semantic versioning
- Mark APIs as unstable initially

---

### Risk 4: Incomplete Extraction

**Likelihood**: Low
**Impact**: Medium

**Mitigation**:
- Systematic review of all files
- Grep for HL7-specific references
- Code review by second person
- Comprehensive testing

---

## Future Enhancements

After the initial extraction, consider:

1. **Additional Transforms**:
   - Date parsing/formatting
   - String manipulation (trim, uppercase, lowercase)
   - Math operations (sum, average, min, max)
   - Conditional logic (if/then/else)

2. **Validation Framework**:
   - Required field validation
   - Type validation
   - Custom validators
   - Schema validation

3. **More Derivation Patterns**:
   - Flattening (nested → flat)
   - Aggregation (many → one with grouping)
   - Splitting (one → many based on delimiter)
   - Filtering (conditional entity creation)

4. **Performance Optimizations**:
   - Parallel entity creation
   - Lazy field evaluation
   - Memory pooling for allocations
   - SIMD for parsing

5. **IDE Support**:
   - JSON Schema for YAML entity configs
   - VS Code extension
   - Language Server Protocol (LSP)
   - Syntax highlighting

6. **Additional Output Formats**:
   - Parquet (via Apache Arrow)
   - Avro
   - Protobuf
   - CSV

7. **Query Language**:
   - Filter entities by criteria
   - Transform pipelines
   - Aggregations

---

## Conclusion

Extracting the generic transformation framework from `data_processor` into `nomnom` is a natural evolution of the codebase. The recent migration to a format-agnostic entity framework removed most HL7-specific coupling, leaving clean abstractions that can benefit projects outside data processing.

The extraction is low-risk (refactoring existing code, not rewriting), high-value (enables reuse across domains), and achievable in 11-17 hours of focused work.

The key to success is:
1. **Extract, don't redesign** - use what works
2. **Test thoroughly** - maintain 100% backward compatibility
3. **Document with non-HL7 examples** - prove generality
4. **Start internal** - publish externally only when mature

With `nomnom`, we'll have a powerful, general-purpose library for building type-safe data transformation pipelines from declarative YAML configurations, demonstrating that the architecture developed for data processing can benefit any domain dealing with structured data.

---

## Appendix A: File Inventory

### Files Moving to `nomnom`

| Source | Destination | LoC | Changes |
|--------|-------------|-----|---------|
| `src/entity.rs` | `nomnom/src/entity.rs` | ~150 | Remove SEGMENT_TYPE, from_segment |
| `src/python_transform.rs` | `nomnom/src/python_bridge.rs` | ~200 | Configurable module name |
| `codegen/types.rs` | `nomnom/codegen/types.rs` | ~400 | Remove HL7 comments |
| `codegen/rust_codegen.rs` | `nomnom/codegen/rust_codegen.rs` | ~1200 | No changes |
| `codegen/python_codegen.rs` | `nomnom/codegen/python_codegen.rs` | ~800 | No changes |
| `codegen/yaml_loader.rs` | `nomnom/codegen/yaml_loader.rs` | ~300 | No changes |
| `codegen/utils.rs` | `nomnom/codegen/utils.rs` | ~150 | No changes |

**Total**: ~3,200 lines of code

### New Files in `nomnom`

| File | Purpose | Est. LoC |
|------|---------|----------|
| `src/lib.rs` | Public API exports | ~50 |
| `src/transform_registry.rs` | Core transform system | ~300 |
| `src/extraction.rs` | Field extraction abstractions | ~150 |
| `src/serialization.rs` | Serialization framework | ~200 |
| `codegen/mod.rs` | Codegen module exports | ~20 |

**Total New**: ~720 lines

### Files Staying in `data_processor`

| File | LoC | Reason |
|------|-----|--------|
| `src/segment.rs` | ~400 | HL7-specific parsing |
| `src/transforms.rs` | ~300 | HL7-specific transforms |
| `src/lib.rs` | ~100 | HL7 module registration |
| `src/generated.rs` | ~15000 | Generated HL7 entities |

---

## Appendix B: Cargo.toml Files

### `nomnom/Cargo.toml`

```toml
[package]
name = "nomnom"
version = "0.1.0"
edition = "2021"
authors = ["Your Team"]
description = "Generic data transformation framework with YAML-based entity definitions"
license = "MIT OR Apache-2.0"
repository = "https://github.com/yourorg/nomnom"

[dependencies]
serde = { version = "1.0", features = ["derive"] }
serde_json = "1.0"
serde_yaml = "0.9"
thiserror = "1.0"

# Optional: Python bindings
pyo3 = { version = "0.20", optional = true, features = ["abi3-py38"] }

[dev-dependencies]
tempfile = "3.0"

[features]
default = []
python-bridge = ["pyo3"]

[build-dependencies]
# If needed for codegen
```

### `data_processor/Cargo.toml` (updated)

```toml
[package]
name = "data_processor_rust"
version = "0.1.0"
edition = "2021"

[dependencies]
nomnom = { path = "../nomnom", features = ["python-bridge"] }
serde = { version = "1.0", features = ["derive"] }
serde_json = "1.0"
pyo3 = { version = "0.20", features = ["extension-module", "abi3-py38"] }

[lib]
name = "data_processor_rust"
crate-type = ["cdylib"]
```

---

## Appendix C: References

- [Serde Documentation](https://serde.rs/)
- [PyO3 User Guide](https://pyo3.rs/)
- [Rust API Guidelines](https://rust-lang.github.io/api-guidelines/)
- [HL7 v2.7 Specification](http://www.hl7.org/implement/standards/product_brief.cfm?product_id=185)

---

**End of Design Document**
