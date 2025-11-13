# FastAPI Ingestion Endpoint Design

## Overview

Add a FastAPI-based HTTP ingestion endpoint as an alternative to stdin-based message parsing. This allows messages to be sent via HTTP POST requests instead of piping data through the parser binary.

## Current Architecture

```
┌─────────────┐
│ Data Source │
└──────┬──────┘
       │ stdin pipe
       ▼
┌─────────────────┐
│ Parser Binary   │
│ (reads stdin)   │
└──────┬──────────┘
       │
       ▼
┌─────────────────┐
│ Database Insert │
└─────────────────┘
```

**Current Usage:**
```bash
cat messages.txt | ./target/debug/parser --execute-db
```

## Proposed Architecture

```
┌─────────────┐
│ Data Source │
└──────┬──────┘
       │ HTTP POST
       ▼
┌──────────────────────┐
│ FastAPI Server       │
│ POST /ingest/message │
│ POST /ingest/batch   │
└──────┬───────────────┘
       │
       ▼
┌─────────────────┐
│ Parser Logic    │
│ (reused core)   │
└──────┬──────────┘
       │
       ▼
┌─────────────────┐
│ Database Insert │
└─────────────────┘
```

**New Usage:**
```bash
# Single message
curl -X POST http://localhost:8080/ingest/message \
  -H "Content-Type: text/plain" \
  -d "O|1|100|O|123.45|1996-01-02|5-LOW|Clerk#000000951|0|nstructions sleep furiously among"

# Batch of messages
curl -X POST http://localhost:8080/ingest/batch \
  -H "Content-Type: text/plain" \
  -d @messages.txt

# Stream from file (keeps stdin compatibility)
cat messages.txt | curl -X POST http://localhost:8080/ingest/stream \
  -H "Content-Type: text/plain" \
  --data-binary @-
```

## Design Decisions

### 1. Code Generation Approach

**Option A: Generate Standalone FastAPI Server** ✅ RECOMMENDED
- Generate a separate `ingestion_server/` directory
- Contains `main.py`, `models.py`, `parsers.py`
- Uses generated Pydantic models for validation
- Calls database insertion logic directly

**Option B: Add HTTP Endpoints to Existing Parser Binary**
- Embed FastAPI server in Rust binary
- Use `actix-web` or `axum` framework
- More complex Rust/Python interop

**Decision: Option A** - Cleaner separation, easier to maintain, leverages Python ecosystem

### 2. API Design

#### Endpoint 1: Single Message Ingestion
```
POST /ingest/message
Content-Type: text/plain

Body: O|1|100|O|123.45|1996-01-02|5-LOW|Clerk#000000951|0|...

Response 200 OK:
{
  "status": "success",
  "entity": "Order",
  "id": 1,
  "timestamp": "2025-11-09T12:34:56Z"
}

Response 400 Bad Request:
{
  "status": "error",
  "message": "Invalid message format: expected '|' delimiter at position 5",
  "entity": "Order",
  "line": "O|1|100..."
}
```

#### Endpoint 2: Batch Ingestion
```
POST /ingest/batch
Content-Type: text/plain

Body: (newline-separated messages)
O|1|100|O|123.45|...
L|1|1|155190|7706|17|...
O|2|101|F|234.56|...

Response 200 OK:
{
  "status": "success",
  "processed": 3,
  "inserted": 3,
  "failed": 0,
  "errors": [],
  "duration_ms": 145
}

Response 207 Multi-Status (partial success):
{
  "status": "partial",
  "processed": 3,
  "inserted": 2,
  "failed": 1,
  "errors": [
    {
      "line_number": 2,
      "entity": "OrderLineItem",
      "message": "Foreign key violation: order_key=999 not found",
      "line": "L|999|1|..."
    }
  ],
  "duration_ms": 158
}
```

#### Endpoint 3: Streaming Ingestion
```
POST /ingest/stream
Content-Type: text/plain
Transfer-Encoding: chunked

Body: (streaming data, processed line by line)

Response 200 OK (Server-Sent Events):
data: {"processed": 1, "entity": "Order", "status": "success"}
data: {"processed": 2, "entity": "OrderLineItem", "status": "success"}
data: {"processed": 3, "entity": "Order", "status": "error", "message": "..."}
...
data: {"total": 1000, "inserted": 998, "failed": 2, "duration_ms": 5432}
```

#### Endpoint 4: Health Check
```
GET /health

Response 200 OK:
{
  "status": "healthy",
  "database": "connected",
  "entities": ["Order", "OrderLineItem", "Customer", "Product"],
  "version": "0.1.0"
}
```

#### Endpoint 5: Stats
```
GET /stats

Response 200 OK:
{
  "total_messages_processed": 15234,
  "messages_by_entity": {
    "Order": 5000,
    "OrderLineItem": 10000,
    "Customer": 234
  },
  "errors": 12,
  "uptime_seconds": 86400,
  "requests_per_second": 150.5
}
```

### 3. Generated Code Structure

```
ingestion_server/
├── main.py              # FastAPI app, endpoints
├── config.py            # Database config, entity registry
├── models.py            # Pydantic models (generated from YAML)
├── parsers.py           # Message parsing logic (generated)
├── database.py          # Database insertion (uses SQLAlchemy/Diesel models)
├── requirements.txt     # Dependencies
├── Dockerfile           # Container for deployment
└── .env.example         # Database connection template
```

### 4. Code Generation Files

New files in `src/codegen/ingestion/`:
```
src/codegen/ingestion/
├── mod.rs               # Main orchestrator
├── fastapi_server.rs    # Generate main.py, endpoints
├── pydantic_models.rs   # Generate models.py from entities
├── message_parsers.rs   # Generate parsers.py (parsing logic)
└── docker.rs            # Generate Dockerfile
```

### 5. Parser Logic Reuse

**Shared Parsing Logic:**
- Current parser binary has field parsing, type conversion, validation
- Generate Python equivalent from entity YAML configs
- Use Pydantic for validation (mirrors entity field types)

**Example Generated Parser:**

```python
# parsers.py (auto-generated)
from typing import Optional
from datetime import date, datetime
from decimal import Decimal
from models import Order, OrderLineItem

class MessageParser:
    """Auto-generated message parser from entity configs"""

    @staticmethod
    def parse_order(line: str) -> Order:
        """Parse Order message: O|order_key|customer_key|..."""
        parts = line.split('|')

        if parts[0] != 'O':
            raise ValueError(f"Expected 'O' prefix, got '{parts[0]}'")

        if len(parts) < 10:
            raise ValueError(f"Expected 10 fields, got {len(parts)}")

        return Order(
            order_key=int(parts[1]),
            customer_key=int(parts[2]),
            order_status=parts[3],
            total_price=Decimal(parts[4]),
            order_date=datetime.strptime(parts[5], '%Y-%m-%d').date(),
            order_priority=parts[6],
            clerk=parts[7],
            ship_priority=int(parts[8]),
            comment=parts[9] if len(parts) > 9 else None
        )

    @staticmethod
    def parse_line(line: str) -> tuple[str, any]:
        """Route message to appropriate parser based on prefix"""
        if not line or line.startswith('#'):
            return None, None

        prefix = line.split('|')[0]

        parsers = {
            'O': MessageParser.parse_order,
            'L': MessageParser.parse_order_line_item,
            'C': MessageParser.parse_customer,
            'P': MessageParser.parse_product,
        }

        parser = parsers.get(prefix)
        if not parser:
            raise ValueError(f"Unknown message prefix: '{prefix}'")

        entity_name = prefix_to_entity[prefix]
        entity = parser(line)

        return entity_name, entity
```

### 6. CLI Integration

Add new command to `nomnom` CLI:

```bash
# Generate ingestion server
nomnom generate-ingestion \
  --entities config/examples/tpch/entities \
  --output ingestion_server \
  --database postgresql \
  --port 8080

# Generated output structure:
ingestion_server/
├── main.py
├── models.py
├── parsers.py
├── database.py
├── config.py
├── requirements.txt
└── Dockerfile

# Run the server
cd ingestion_server
pip install -r requirements.txt
uvicorn main:app --host 0.0.0.0 --port 8080

# Use the API
curl -X POST http://localhost:8080/ingest/message \
  -H "Content-Type: text/plain" \
  -d "O|1|100|O|123.45|1996-01-02|5-LOW|Clerk#000000951|0|test"
```

### 7. Configuration Options

**Generated config.py:**
```python
import os
from dotenv import load_dotenv

load_dotenv()

# Database configuration
DATABASE_URL = os.getenv("DATABASE_URL")
if not DATABASE_URL:
    raise ValueError("DATABASE_URL not set")

# Server configuration
PORT = int(os.getenv("PORT", "8080"))
HOST = os.getenv("HOST", "0.0.0.0")
WORKERS = int(os.getenv("WORKERS", "4"))

# Ingestion configuration
BATCH_SIZE = int(os.getenv("BATCH_SIZE", "1000"))
MAX_BATCH_ITEMS = int(os.getenv("MAX_BATCH_ITEMS", "10000"))
STREAMING_BUFFER_SIZE = int(os.getenv("STREAMING_BUFFER_SIZE", "100"))

# Entity registry (auto-generated from YAML)
ENTITIES = {
    'O': 'Order',
    'L': 'OrderLineItem',
    'C': 'Customer',
    'P': 'Product',
}
```

### 8. Error Handling

**Validation Errors:**
- Use Pydantic for field type validation
- Return 400 Bad Request with detailed error messages
- Log invalid messages for debugging

**Database Errors:**
- Catch foreign key violations
- Handle unique constraint violations
- Return 500 Internal Server Error for DB connection issues

**Partial Success in Batch:**
- Process as many valid messages as possible
- Return 207 Multi-Status with error details
- Don't rollback entire batch on single failure (optional: configurable)

### 9. Performance Considerations

**Batch Processing:**
- Use SQLAlchemy bulk insert for batches
- Configurable batch size (default: 1000)
- Connection pooling for concurrent requests

**Streaming:**
- Process messages as they arrive
- Yield progress updates via Server-Sent Events
- Buffer small batches for efficiency

**Async Processing:**
- Use FastAPI's async endpoints
- Database operations with `databases` library (async)
- Background tasks for large batches

### 10. Docker Deployment

**Generated Dockerfile:**
```dockerfile
FROM python:3.11-slim

WORKDIR /app

COPY requirements.txt .
RUN pip install --no-cache-dir -r requirements.txt

COPY . .

EXPOSE 8080

CMD ["uvicorn", "main:app", "--host", "0.0.0.0", "--port", "8080", "--workers", "4"]
```

**Docker Compose Integration:**
```yaml
services:
  ingestion-server:
    build: ./ingestion_server
    ports:
      - "8080:8080"
    environment:
      - DATABASE_URL=postgresql://user:pass@db:5432/dbname
    depends_on:
      - db
```

### 11. Testing Strategy

**Generate test script:**
```bash
# test_ingestion.sh (auto-generated)
#!/bin/bash

echo "Testing single message ingestion..."
curl -X POST http://localhost:8080/ingest/message \
  -H "Content-Type: text/plain" \
  -d "O|1|100|O|123.45|1996-01-02|5-LOW|Clerk#000000951|0|test"

echo "Testing batch ingestion..."
curl -X POST http://localhost:8080/ingest/batch \
  -H "Content-Type: text/plain" \
  -d @test_messages.txt

echo "Testing invalid message..."
curl -X POST http://localhost:8080/ingest/message \
  -H "Content-Type: text/plain" \
  -d "INVALID|MESSAGE"
```

### 12. Monitoring & Observability

**Metrics Endpoint:**
```
GET /metrics (Prometheus format)

# HELP ingestion_messages_total Total messages processed
# TYPE ingestion_messages_total counter
ingestion_messages_total{entity="Order",status="success"} 5000
ingestion_messages_total{entity="Order",status="error"} 12

# HELP ingestion_duration_seconds Message processing duration
# TYPE ingestion_duration_seconds histogram
ingestion_duration_seconds_bucket{le="0.001"} 4500
ingestion_duration_seconds_bucket{le="0.01"} 5000
```

## Migration Path

### Phase 1: Basic Implementation
- Generate FastAPI server with single message endpoint
- Generate Pydantic models from entity YAML
- Generate message parsers
- CLI command: `nomnom generate-ingestion`

### Phase 2: Batch & Streaming
- Add batch ingestion endpoint
- Add streaming ingestion with progress
- Performance optimizations (bulk inserts)

### Phase 3: Advanced Features
- Metrics and monitoring
- Rate limiting
- Authentication/authorization
- Message validation rules

### Phase 4: Integration
- Docker deployment
- Integration with dashboard (unified docker-compose)
- End-to-end testing

## Benefits

1. **Easier Integration**: HTTP API is more accessible than stdin pipes
2. **Better Error Handling**: Structured error responses vs stderr parsing
3. **Scalability**: Multiple workers, load balancing, horizontal scaling
4. **Observability**: Metrics, health checks, request tracing
5. **Flexibility**: Can accept messages from any HTTP client
6. **Modern Stack**: FastAPI provides automatic OpenAPI docs
7. **Consistency**: Reuses same validation logic as parser binary

## Alternatives Considered

### Alternative 1: gRPC Service
- **Pros**: Better performance, streaming built-in
- **Cons**: More complex, requires protobuf definitions
- **Decision**: HTTP REST is simpler for this use case

### Alternative 2: Message Queue (Kafka/RabbitMQ)
- **Pros**: Built for high-throughput ingestion
- **Cons**: Additional infrastructure, more complex
- **Decision**: Can add later if needed, HTTP is good starting point

### Alternative 3: Extend Parser Binary with HTTP
- **Pros**: Single binary deployment
- **Cons**: Rust HTTP server + Python parser is complex
- **Decision**: Separate FastAPI server is cleaner

## Open Questions

1. **Authentication**: Should we generate API key authentication by default?
2. **Rate Limiting**: Should we add rate limiting to prevent abuse?
3. **Validation**: Should validation be strict (reject invalid) or lenient (log & skip)?
4. **Transactions**: Should batches be transactional (all-or-nothing) or best-effort?
5. **Schema Evolution**: How to handle entity schema changes with running server?

## Next Steps

1. Create `src/codegen/ingestion/` module structure
2. Implement Pydantic model generation from entity YAML
3. Implement message parser generation
4. Implement FastAPI server generation
5. Add CLI command `generate-ingestion`
6. Add integration tests
7. Update documentation

## Example End-to-End Flow

```bash
# 1. Generate ingestion server
./target/debug/nomnom generate-ingestion \
  --entities config/examples/tpch/entities \
  --output tpch_ingestion \
  --database postgresql

# 2. Configure database
cd tpch_ingestion
echo "DATABASE_URL=postgresql://user:pass@localhost:5432/tpch_db" > .env

# 3. Install dependencies
python3 -m venv venv
source venv/bin/activate
pip install -r requirements.txt

# 4. Run server
uvicorn main:app --host 0.0.0.0 --port 8080

# 5. Send messages
curl -X POST http://localhost:8080/ingest/batch \
  -H "Content-Type: text/plain" \
  -d @messages.txt

# 6. Check stats
curl http://localhost:8080/stats

# 7. View API docs
open http://localhost:8080/docs
```
