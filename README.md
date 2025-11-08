# Nomnom

**General-purpose data transformation and entity framework with YAML-based code generation**

Nomnom provides a format-agnostic framework for parsing, transforming, and deriving entities from any structured data format (CSV, JSON, XML, EDI, etc.).

## Features

- **Format-agnostic entity framework**: Define entities using YAML configurations that work with any structured data format
- **Transform registry system**: Plugin architecture for registering domain-specific transformation functions
- **Code generation**: Auto-generate Rust structs and Python bindings from YAML entity definitions
- **Derivation patterns**: Support for parent, repeated, and multi-parent entity derivation
- **Python bridge** (optional): PyO3 integration for calling Python transformation functions from Rust

## Quick Start

### Building

Use the provided build script which handles MySQL client library linking:

```bash
# Build library (debug mode)
./build.sh

# Build in release mode
./build.sh --release

# Build and run tests
./build.sh --test

# Clean and rebuild everything
./build.sh --clean --all

# Get help
./build.sh --help
```

The build script automatically detects your OS and sets up the correct MySQL library paths for macOS (Homebrew) and Linux.

### Define an Entity (YAML)

```yaml
# config/entities/user.yaml
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

    - name: email
      type: String
      computed_from:
        transform: extract_json_field
        sources:
          - source: parent
            field: raw_json
        args:
          json_path: "$.user.email"
```

### Register Transforms (Rust)

```rust
use nomnom::TransformRegistry;
use serde_json::Value;

let mut registry = TransformRegistry::new();

// Register JSON field extraction transform
registry.register("extract_json_field", Box::new(|args| {
    let json_str = args.get("raw_json")
        .and_then(|v| v.as_str())
        .ok_or_else(|| TransformError::InvalidArgs("Missing raw_json".to_string()))?;

    let path = args.get("json_path")
        .and_then(|v| v.as_str())
        .ok_or_else(|| TransformError::InvalidArgs("Missing json_path".to_string()))?;

    // Parse JSON and extract field at path
    let json: Value = serde_json::from_str(json_str)?;
    // ... implement JSON path extraction

    Ok(Some(extracted_value))
}));
```

### Generate Code (build.rs)

```rust
// build.rs
use nomnom::codegen::{load_entities, generate_rust_code};

fn main() {
    // Load entity configurations
    let entities = load_entities("config/entities").unwrap();

    // Generate Rust code
    generate_rust_code(&entities, "src/generated.rs").unwrap();
}
```

## Entity Derivation Patterns

### Single Parent

```yaml
entity:
  name: Row
  source_type: derived
  parent: CsvFile
  fields:
    - name: first_name
      computed_from:
        transform: extract_csv_field
        sources:
          - source: parent
            field: raw_line
        args:
          column_index: 0
```

### Repeated (One-to-Many)

```yaml
entity:
  name: Tag
  source_type: derived
  repetition: repeated
  repeated_for:
    entity: Article
    field: tags
    each_known_as: tag_data
  fields:
    - name: name
      computed_from:
        transform: extract_field
        sources:
          - tag_data
        args:
          field_index: 0
```

### Multi-Parent

```yaml
entity:
  name: OrderLineItem
  source_type: derived
  parents:
    - Order
    - Product
    - Customer
  fields:
    - name: customer_id
      computed_from:
        transform: copy_from_parent
        sources:
          - source: parent
            field: customer_id
```

## Features

### Default Features

By default, nomnom includes only core functionality with no external dependencies beyond serde.

### Python Bridge

Enable Python integration for calling Python transformation functions from Rust:

```toml
[dependencies]
nomnom = { version = "0.1", features = ["python-bridge"] }
```

## Examples

### CSV Parser

See `examples/csv_parser.rs` for a complete example of parsing CSV files using nomnom.

### JSON Parser

See `examples/json_parser.rs` for a complete example of parsing JSON documents using nomnom.

## Core Concepts

### Entity

The `Entity` trait is the core abstraction representing a data entity:

```rust
pub trait Entity: Serialize + Sized {
    const NAME: &'static str;

    fn to_dict(&self) -> HashMap<String, FieldValue>;
    fn to_json(&self) -> Result<String, serde_json::Error>;
    fn to_json_pretty(&self) -> Result<String, serde_json::Error>;
    fn to_ndjson_line(&self) -> Result<String, serde_json::Error>;
}
```

### Transform Registry

The transform registry provides a plugin system for data transformations:

```rust
pub struct TransformRegistry {
    transforms: HashMap<String, Box<dyn TransformFn>>,
}

impl TransformRegistry {
    pub fn register(&mut self, name: impl Into<String>, func: Box<dyn TransformFn>);
    pub fn call(&self, name: &str, args: &HashMap<String, Value>)
        -> Result<Option<String>, TransformError>;
}
```

### Field Extraction

Generic field path-based extraction:

```rust
pub struct FieldPath {
    pub raw: String,
    pub segments: Vec<PathSegment>,
}

pub trait Extractor {
    fn extract(&self, path: &FieldPath) -> Option<String>;
}
```

## Architecture

Nomnom is designed to be the generic foundation for domain-specific data processing libraries. You can use it to build:

1. CSV/TSV parsers with custom entity extraction
2. JSON/XML document processors
3. EDI and proprietary format parsers
4. Custom data transformation pipelines

```
┌─────────────────────────────────┐
│     Your Domain Library         │
│   (CSV, JSON, EDI, custom)      │
├─────────────────────────────────┤
│  - Domain-specific transforms   │
│  - Entity YAML configurations   │
│  - Custom field extractors      │
└─────────────────────────────────┘
              │
              ▼
┌─────────────────────────────────┐
│           nomnom                │
├─────────────────────────────────┤
│  - Entity trait                 │
│  - Transform registry           │
│  - Code generation              │
│  - Derivation patterns          │
│  - Serialization                │
└─────────────────────────────────┘
```

## Status

**Current Version**: 0.1.0 (Phase 1 - Core Framework)

### Completed
- ✅ Core entity trait and types
- ✅ Transform registry system
- ✅ Field extraction abstractions
- ✅ Serialization framework (JSON, NDJSON)
- ✅ Python bridge (feature-gated)
- ✅ YAML type definitions
- ✅ 19 passing tests

### In Progress (Phase 2)
- ⏳ YAML loader enhancements
- ⏳ Additional transform implementations
- ⏳ Rust struct generation improvements
- ⏳ Python bindings enhancements

### Planned
- 📋 Complete examples (CSV, JSON)
- 📋 Comprehensive documentation
- 📋 Performance benchmarks
- 📋 Additional output formats (Parquet, Avro)

## License

MIT

## Contributing

This library is currently in active development. Contributions are welcome! Please see the [design document](docs/design-nomnom-library.md) for architectural details.
