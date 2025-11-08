# Nomnom: Generic Data Transformation Library

## Overview

Nomnom is a general-purpose, format-agnostic data transformation and entity framework library. It provides type-safe, declarative data transformation pipelines from YAML configurations.

**Status**: ✅ Implemented and ready for use

---

## What is Nomnom?

Nomnom provides:

1. **Core Entity Framework** - Generic entity traits and base types for structured data
2. **Transform Registry** - Runtime registry for data transformation functions
3. **Code Generation** - YAML-to-Rust code generation for entities and transformations
4. **Derivation Patterns** - Parent, repeated_for, and multi-parent derivation support
5. **Field Extraction** - Declarative field extraction and transformation
6. **Serialization** - Built-in JSON, NDJSON serialization support

### Architecture

```
nomnom/                          (Generic framework library)
├── Core entity framework
├── Transform registry system
├── Code generation (YAML → Rust)
├── Derivation patterns
├── Field extraction abstractions
└── Serialization framework
```

---

## Key Features

### 1. Declarative Entity Configuration

Define entities in YAML with automatic Rust code generation:

```yaml
entity:
  name: User
  type: derived
  parent: RawMessage
  fields:
    - name: user_id
      type: String
      computed_from:
        transform: extract_field
        sources: [raw_message]
        args: {path: "PID.3"}
```

### 2. Transform System

Register and use transforms in entity definitions:

```rust
pub fn register_transform(name: &str, func: TransformFn) {
    TRANSFORM_REGISTRY.register(name, func);
}
```

### 3. Code Generation

Generate type-safe Rust code from YAML configurations:

```bash
nomnom build-from-config --config nomnom.yaml
```

### 4. Multi-Format Support

Nomnom is format-agnostic - use it for:
- CSV files
- JSON documents
- XML documents
- Binary message formats
- Any structured data format

---

## Documentation

### Getting Started

See the main `README.md` at repository root for:
- API documentation
- Usage examples
- Build instructions
- Transform YAML schema

### Additional Documentation

- **[transform_yaml_schema.md](transform_yaml_schema.md)** - Detailed transform YAML syntax and examples

---

## Current Status

### ✅ Completed

- Core entity framework
- Transform registry system
- Code generation from YAML configs
- CLI tool (`nomnom` binary)
- Build system unified under `nomnom.yaml`
- PyO3 bindings for Python integration
- Diesel ORM code generation
- Parser binary generation
- Comprehensive test suite (80 tests passing)

### 🔮 Future Plans

- Publish to crates.io
- Add support for more data formats
- Additional transform helpers
- Performance optimizations
- Tutorial and examples for various use cases

---

## Usage Example

Basic workflow:

1. **Define entities** in `config/entities/*.yaml`
2. **Define transforms** in `config/nomnom.yaml`
3. **Build** with `nomnom build-from-config`
4. **Use generated code** in your application

---

## Contributing

Contributions are welcome! Please see the main repository README for contribution guidelines.
