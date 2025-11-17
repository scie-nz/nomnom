# Nomnom - YAML-Based Code Generator for Data Transformation

Nomnom is a code generation framework that creates Rust and Python data transformation libraries from YAML entity definitions.

## Architecture Overview

Nomnom generates three types of binaries/libraries, each serving a different purpose in a data processing pipeline:

```
┌─────────────────────────────────────────────────────────────┐
│                    Nomnom Code Generator                     │
│             (YAML entity specs → Rust/Python code)           │
└───────────────┬─────────────────┬───────────────┬───────────┘
                │                 │               │
    ┌───────────▼──────┐ ┌───────▼────────┐ ┌───▼─────────────┐
    │ Parser Binary    │ │ NATS Worker    │ │ Ingestion       │
    │ (Python .so)     │ │ (Rust binary)  │ │ Server          │
    │                  │ │                │ │ (Rust binary)   │
    │ Used for: Tests  │ │ Used for:      │ │ Used for:       │
    │ Python scripts   │ │ Background     │ │ HTTP API        │
    │                  │ │ processing     │ │ endpoints       │
    └──────────────────┘ └────────────────┘ └─────────────────┘
```

## Commands

### `build-parser-binary`

**Purpose**: Build a Python extension module (`.so` file) with PyO3 bindings for parsing and transforming data in Python.

**Use Case**:
- Python test suites
- Python scripts that need to parse/transform data
- Development and debugging
- Creating derived entities from raw input

**Configuration**: Comprehensive `nomnom.yaml` with:
- Project metadata (name, version, authors)
- Dependencies (nomnom, hl7utils, etc.)
- Transform functions (Rust code snippets)
- Paths for code generation
- Database configuration

**Example**:
```bash
# Generate and build Python extension
nomnom build-parser-binary --config config/nomnom.yaml

# Use in Python
from hl7_ingestion import Hl7v2MessageFile, Hl7v2MessageCore

msg_file = Hl7v2MessageFile.from_string("path/to/message.hl7")
msg = Hl7v2MessageCore.from_sources(msg_file)
print(msg.patient_name)
```

**Outputs**:
- Rust code: `rust_build/src/generated.rs`, `lib.rs`, etc.
- Python module: `.venv/lib/python3.x/site-packages/_rust.so`
- Cargo.toml, pyproject.toml

---

### `generate-worker`

**Purpose**: Generate a standalone Rust binary that consumes JSON messages from NATS JetStream, parses them, creates derived entities, and persists to database.

**Use Case**:
- Production message processing
- Kubernetes deployments
- Scalable async processing
- NATS-based architectures

**Configuration**: Minimal CLI args only:
- Entity directory path
- Output directory
- Database type (postgresql/mysql/mariadb)

**Example**:
```bash
# Generate worker
nomnom generate-worker \
  --entities config/entities \
  --output worker \
  --database mysql

# Build and run
cd worker
cargo build --release
./target/release/worker
```

**Input**: JSON via NATS stream `entities.Hl7v2MessageFile`
```json
{
  "file_name": "facility_20250117.hl7",
  "message": "MSH|^~\\&|APP|FAC|...\rPID|..."
}
```

**Outputs**:
- Standalone Rust binary
- Consumes from NATS → Parses → Persists to DB
- No output (side effect: database records)

---

### `generate-ingestion-server`

**Purpose**: Generate a standalone Axum HTTP server that receives JSON messages via POST requests, parses them, and publishes to NATS.

**Use Case**:
- HTTP API endpoint for external systems
- Synchronous request/response interface
- Entry point for data pipeline

**Configuration**: Minimal CLI args:
- Entity directory path
- Output directory
- Database type
- Port number

**Example**:
```bash
# Generate server
nomnom generate-ingestion-server \
  --entities config/entities \
  --output ingestion-server \
  --database mysql \
  --port 8080

# Build and run
cd ingestion-server
cargo build --release
./target/release/ingestion-server
```

**API Usage**:
```bash
curl -X POST http://localhost:8080/ingest \
  -H "Content-Type: application/json" \
  -d '{"file_name": "test.hl7", "message": "MSH|^~\\&|..."}'
```

**Outputs**:
- Standalone Axum server
- HTTP 200/error responses
- Publishes to NATS for async processing

---

## Complete Data Pipeline

```
External System
    │
    ▼ HTTP POST /ingest
┌─────────────────────┐
│ Ingestion Server    │  (generate-ingestion-server)
│ (Axum HTTP)         │
└──────────┬──────────┘
           │ Publish to NATS stream
           ▼
┌─────────────────────┐
│ NATS JetStream      │
│ Stream: entities.*  │
└──────────┬──────────┘
           │ Pull messages
           ▼
┌─────────────────────┐
│ Worker 1, 2, 3...   │  (generate-worker)
│ (Rust binaries)     │
└──────────┬──────────┘
           │ Parse & Persist
           ▼
┌─────────────────────┐
│ MySQL Database      │
│                     │
└─────────────────────┘

Meanwhile, separately:

Python Test Suite
    │
    ▼ Import module
┌─────────────────────┐
│ Parser Binary       │  (build-parser-binary)
│ (_rust.so)          │
│                     │
│ from hl7_ingestion  │
│ import Entity       │
└─────────────────────┘
```

## Database Support

All commands support PostgreSQL, MySQL, and MariaDB. Specify with `--database` flag:

```bash
# PostgreSQL (default)
nomnom build-parser-binary --config nomnom.yaml --database postgresql

# MySQL
nomnom build-parser-binary --config nomnom.yaml --database mysql

# MariaDB
nomnom build-parser-binary --config nomnom.yaml --database mariadb
```

## Key Differences Summary

| Feature | Parser Binary | Worker | Ingestion Server |
|---------|---------------|--------|------------------|
| **Output** | Python `.so` module | Rust binary | Rust binary |
| **Input** | Files/strings in Python | JSON from NATS | JSON via HTTP POST |
| **Output** | Python objects | Database records | HTTP response + NATS |
| **Runtime** | Python process (PyO3) | Standalone Rust | Standalone Rust (Axum) |
| **Use Case** | Tests, dev, scripting | Background processing | API endpoint |
| **Config** | Comprehensive `nomnom.yaml` | Minimal CLI args | Minimal CLI args |
| **Deployment** | Python package | Docker/K8s pod | Docker/K8s pod |

## Installation

```bash
cargo install --path .
```

## License

MIT
