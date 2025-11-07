# Nomnom: Generic Data Transformation Library

## Overview

Nomnom is a general-purpose, format-agnostic data transformation and entity framework library extracted from the HL7 ingestion system. It provides type-safe, declarative data transformation pipelines from YAML configurations.

**Status**: ✅ Extracted and implemented (see `nomnom/` crate in repository root)

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

data_processor/                   (structured data-specific implementation)
├── Depends on: nomnom
├── structured data-specific transforms
├── Healthcare domain entities
└── structured data message parsing
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
- structured data messages (current implementation)
- CSV files
- JSON documents
- XML documents
- EDI messages
- Any structured data format

---

## Documentation

### Design Documents

This directory contains the original design documents for nomnom extraction:

- **[design-nomnom-library.md](design-nomnom-library.md)** - Detailed design proposal (2025-01-24)
  - Motivation and background
  - Architecture design
  - Migration strategy
  - API examples

- **[nomnom-extraction-plan.md](nomnom-extraction-plan.md)** - Execution summary
  - High-level architecture
  - What goes into nomnom vs data_processor
  - Implementation phases

### Implementation

The nomnom library has been successfully extracted and is located in:

**`nomnom/`** directory at repository root

See `nomnom/README.md` for:
- API documentation
- Usage examples
- Build instructions
- Transform YAML schema

---

## Current Status

### ✅ Completed

- Core entity framework extracted
- Transform registry system implemented
- Code generation from YAML configs working
- CLI tool (`nomnom` binary) functional
- Build system unified under `nomnom.yaml`
- PyO3 bindings for Python integration
- Diesel ORM code generation
- Parser binary generation

### 🔄 In Progress

- Documentation improvements
- Additional transform helpers
- Performance optimizations

### 🔮 Future Plans

- Extract to separate GitHub repository
- Publish to crates.io
- Add support for more data formats
- Comprehensive test suite
- Tutorial and examples for non-HL7 use cases

---

## Usage Example

See the HL7 ingestion implementation as a reference example:

1. **Define entities** in `config/entities/*.yaml`
2. **Define transforms** in `config/nomnom.yaml`
3. **Build** with `nomnom build-from-config`
4. **Use generated code** in your application

---

## Related Documentation

- **Main Documentation**: [../README.md](../README.md)
- **Architecture Overview**: [../ARCHITECTURE.md](../ARCHITECTURE.md)
- **Archived Design Proposals**: [../archive/design-proposals/](../archive/design-proposals/)
  - generalize-entity-framework-proposal.md
  - rust-derived-entities-design.md

---

## Contributing

The nomnom library is actively developed as part of the HL7 ingestion system. Contributions that improve the generic framework (without adding HL7-specific features) are welcome.

---

**Note**: This directory consolidates historical design documents. For current nomnom implementation details, see `nomnom/README.md` at the repository root.
