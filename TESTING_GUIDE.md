# NATS Architecture Testing Guide

## Quick Test (Automated)

I've created an automated test script that sets up the entire stack and runs tests:

```bash
# Make sure nomnom is built first
export RUSTFLAGS="-L /opt/homebrew/opt/libpq/lib"
cargo build --bin nomnom

# Run the automated test
./test-nats-flow.sh
```

This will:
1. Generate ingestion server with NATS support
2. Generate worker binary
3. Start NATS, PostgreSQL, Ingestion API, and Worker in Docker
4. Send test messages
5. Verify they're processed and written to the database

---

## Manual Testing (Step-by-Step)

If you want to understand each step or test manually:

### 1. Generate Components

```bash
# Build nomnom CLI
export RUSTFLAGS="-L /opt/homebrew/opt/libpq/lib"
cargo build --bin nomnom

# Generate ingestion server
./target/debug/nomnom generate-ingestion-server \
  --entities config/examples/tpch/entities \
  --output /tmp/test-ingestion \
  --database postgresql

# Generate worker
./target/debug/nomnom generate-worker \
  --entities config/examples/tpch/entities \
  --output /tmp/test-worker \
  --database postgresql
```

### 2. Start NATS and PostgreSQL

```bash
cd /tmp/test-ingestion

# Start just NATS and PostgreSQL first
docker compose -f docker-compose.nats.yml up -d nats postgres

# Wait for services to be healthy
docker compose -f docker-compose.nats.yml ps
```

### 3. Build and Run Ingestion API Locally

```bash
cd /tmp/test-ingestion

# Configure environment
cp .env.example .env
# Edit .env if needed (defaults should work with Docker Compose)

# Build and run
export RUSTFLAGS="-L /opt/homebrew/opt/libpq/lib"
cargo build
cargo run
```

The API will start on `http://localhost:8080`

### 4. Build and Run Worker Locally

**In another terminal:**

```bash
cd /tmp/test-worker

# Configure environment
cp .env.example .env
# Make sure NATS_URL and DATABASE_URL match your setup

# Build and run
export RUSTFLAGS="-L /opt/homebrew/opt/libpq/lib"
cargo build
cargo run
```

The worker will connect to NATS and start consuming messages.

### 5. Send Test Messages

**In another terminal:**

```bash
# Test 1: Send a single order message
curl -X POST http://localhost:8080/ingest/message \
  -H "Content-Type: application/json" \
  -d '{
    "order_key": "ORD-001",
    "customer_key": "CUST-123",
    "order_status": "O",
    "total_price": 999.99,
    "order_date": "2025-01-15"
  }'

# You should get a response like:
# {
#   "message_id": "550e8400-e29b-41d4-a716-446655440000",
#   "status": "accepted",
#   "timestamp": "2025-11-09T22:00:00Z"
# }

# Test 2: Send a batch
curl -X POST http://localhost:8080/ingest/batch \
  -H "Content-Type: text/plain" \
  -d '{"order_key": "ORD-002", "customer_key": "CUST-456", "order_status": "F", "total_price": 1500.00, "order_date": "2025-01-16"}
{"order_key": "ORD-003", "customer_key": "CUST-789", "order_status": "P", "total_price": 750.00, "order_date": "2025-01-17"}'
```

### 6. Verify Processing

**Check worker logs:**
```bash
# You should see logs like:
# INFO worker: Message processed successfully: 550e8400-e29b-41d4-a716-446655440000
# INFO worker: Inserted Order: ORD-001
```

**Check NATS stats:**
```bash
# See JetStream stats
curl http://localhost:8222/jsz | jq

# Should show messages being processed
```

**Check database:**
```bash
# Connect to PostgreSQL
docker compose -f docker-compose.nats.yml exec postgres psql -U nomnom -d nomnom

# Run query
SELECT order_key, customer_key, order_status, total_price, order_date
FROM orders
ORDER BY order_key;

# You should see your orders!
```

---

## Docker-Only Testing

The simplest way if you don't want to build locally:

### 1. Generate with Docker Support

```bash
./target/debug/nomnom generate-ingestion-server \
  --entities config/examples/tpch/entities \
  --output /tmp/test-docker

./target/debug/nomnom generate-worker \
  --entities config/examples/tpch/entities \
  --output /tmp/test-docker-worker
```

### 2. Update docker-compose.nats.yml

Edit `/tmp/test-docker/docker-compose.nats.yml` and add the worker service:

```yaml
  worker:
    build:
      context: /tmp/test-docker-worker
      dockerfile: Dockerfile
    environment:
      - DATABASE_URL=postgresql://nomnom:nomnom@postgres:5432/nomnom
      - NATS_URL=nats://nats:4222
      - NATS_STREAM=MESSAGES
      - RUST_LOG=info
    depends_on:
      nats:
        condition: service_healthy
      postgres:
        condition: service_healthy
```

### 3. Start Everything

```bash
cd /tmp/test-docker
docker compose -f docker-compose.nats.yml up --build
```

This starts:
- NATS JetStream (port 4222, monitoring on 8222)
- PostgreSQL (port 5432)
- Ingestion API (port 8080)
- Worker (processing in background)

### 4. Send Messages

```bash
curl -X POST http://localhost:8080/ingest/message \
  -H "Content-Type: application/json" \
  -d '{"order_key": "ORD-001", "customer_key": "CUST-123", "order_status": "O", "total_price": 999.99, "order_date": "2025-01-15"}'
```

### 5. Watch Logs

```bash
docker compose -f docker-compose.nats.yml logs -f worker
```

---

## Monitoring

### NATS Monitoring Endpoints

```bash
# General server info
curl http://localhost:8222/varz

# JetStream stats
curl http://localhost:8222/jsz | jq

# Connections
curl http://localhost:8222/connz | jq

# Subscriptions
curl http://localhost:8222/subsz | jq
```

### Database Monitoring

```bash
# Connect to PostgreSQL
docker compose -f docker-compose.nats.yml exec postgres psql -U nomnom -d nomnom

# Check table stats
SELECT
  schemaname,
  tablename,
  n_tup_ins AS inserts,
  n_tup_upd AS updates,
  n_tup_del AS deletes
FROM pg_stat_user_tables;

# Check recent orders
SELECT * FROM orders ORDER BY order_date DESC LIMIT 10;
```

### Application Logs

```bash
# View all logs
docker compose -f docker-compose.nats.yml logs -f

# View specific service
docker compose -f docker-compose.nats.yml logs -f worker
docker compose -f docker-compose.nats.yml logs -f ingestion-api
docker compose -f docker-compose.nats.yml logs -f nats
```

---

## Troubleshooting

### Worker not processing messages

1. Check worker logs:
   ```bash
   docker compose logs worker
   ```

2. Check NATS connection:
   ```bash
   curl http://localhost:8222/connz | jq '.connections'
   ```

3. Check if messages are in the queue:
   ```bash
   curl http://localhost:8222/jsz | jq '.streams[].state.messages'
   ```

### Ingestion API returning errors

1. Check API logs:
   ```bash
   docker compose logs ingestion-api
   ```

2. Test health endpoint:
   ```bash
   curl http://localhost:8080/health
   ```

3. Check NATS connectivity:
   ```bash
   docker compose exec ingestion-api nc -zv nats 4222
   ```

### Messages not appearing in database

1. Check worker is running:
   ```bash
   docker compose ps worker
   ```

2. Check worker can connect to database:
   ```bash
   docker compose exec worker nc -zv postgres 5432
   ```

3. Check database schema exists:
   ```bash
   docker compose exec postgres psql -U nomnom -d nomnom -c "\dt"
   ```

---

## Performance Testing

### Load Testing with wrk

```bash
# Install wrk if needed
brew install wrk

# Create test payload
cat > /tmp/order.json <<EOF
{"order_key": "ORD-PERF-{{id}}", "customer_key": "CUST-123", "order_status": "O", "total_price": 999.99, "order_date": "2025-01-15"}
EOF

# Run load test
wrk -t4 -c100 -d30s \
  -s /tmp/order.json \
  http://localhost:8080/ingest/message
```

### Watch Processing Rate

```bash
# Monitor NATS message rate
watch -n 1 'curl -s http://localhost:8222/jsz | jq ".streams[].state.messages"'

# Monitor database insert rate
watch -n 1 'docker compose exec postgres psql -U nomnom -d nomnom -t -c "SELECT COUNT(*) FROM orders"'
```

---

## Cleanup

```bash
# Stop services
cd /tmp/test-ingestion
docker compose -f docker-compose.nats.yml down

# Remove volumes (⚠️ deletes data)
docker compose -f docker-compose.nats.yml down -v

# Remove test directories
rm -rf /tmp/test-ingestion /tmp/test-worker
```

---

## Next Steps

Once you've verified the basic flow works:

1. **Deploy to Kubernetes** - See `PLAN_KUBERNETES_DEPLOYMENT.md`
2. **Add message status table** - Track processing lifecycle
3. **Implement monitoring** - Prometheus, Grafana
4. **Add dead letter queue** - Handle persistent failures
5. **Scale workers** - Use KEDA for autoscaling
