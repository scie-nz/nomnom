#!/bin/bash
set -e

echo "🧪 Testing NATS Architecture End-to-End"
echo "======================================="
echo ""

# Configuration
TEST_DIR="/tmp/nomnom-nats-test"
ENTITIES_DIR="config/examples/tpch/entities"

echo "📁 Creating test directory: $TEST_DIR"
rm -rf "$TEST_DIR"
mkdir -p "$TEST_DIR"

# Step 1: Generate the ingestion server with NATS
echo ""
echo "📦 Step 1: Generating ingestion server with NATS support..."
./target/debug/nomnom generate-ingestion-server \
  --entities "$ENTITIES_DIR" \
  --output "$TEST_DIR/ingestion-server" \
  --database postgresql \
  --port 8080

# Step 2: Generate the worker
echo ""
echo "📦 Step 2: Generating worker binary..."
./target/debug/nomnom generate-worker \
  --entities "$ENTITIES_DIR" \
  --output "$TEST_DIR/worker" \
  --database postgresql

# Step 3: Check the docker-compose file
echo ""
echo "📋 Step 3: Reviewing docker-compose.nats.yml..."
if [ -f "$TEST_DIR/ingestion-server/docker-compose.nats.yml" ]; then
    echo "✓ docker-compose.nats.yml exists"
else
    echo "✗ docker-compose.nats.yml not found!"
    exit 1
fi

# Step 4: Start NATS and PostgreSQL
echo ""
echo "🚀 Step 4: Starting NATS and PostgreSQL..."
cd "$TEST_DIR/ingestion-server"

# Create a complete docker-compose file with worker included
echo ""
echo "📝 Creating docker-compose.test.yml with all services..."
cat > docker-compose.test.yml <<'EOF'
# Docker Compose for NATS Testing - Full Stack
services:
  # NATS JetStream server
  nats:
    image: nats:latest
    ports:
      - "4222:4222"
      - "8222:8222"
    command:
      - "-js"
      - "-m"
      - "8222"
    healthcheck:
      test: ["CMD-SHELL", "timeout 1 sh -c 'cat < /dev/null > /dev/tcp/127.0.0.1/4222' || exit 1"]
      interval: 5s
      timeout: 3s
      retries: 3
      start_period: 5s

  # PostgreSQL database
  postgres:
    image: postgres:16-alpine
    environment:
      POSTGRES_USER: nomnom
      POSTGRES_PASSWORD: nomnom
      POSTGRES_DB: nomnom
    ports:
      - "5432:5432"
    volumes:
      - postgres_data:/var/lib/postgresql/data
    healthcheck:
      test: ["CMD-SHELL", "pg_isready -U nomnom"]
      interval: 5s
      timeout: 3s
      retries: 3

  # Ingestion API (HTTP -> NATS publisher)
  ingestion-api:
    build: .
    ports:
      - "8080:8080"
    environment:
      DATABASE_URL: postgresql://nomnom:nomnom@postgres:5432/nomnom
      NATS_URL: nats://nats:4222
      NATS_STREAM: MESSAGES
      RUST_LOG: info
    depends_on:
      nats:
        condition: service_healthy
      postgres:
        condition: service_healthy

  # Worker (NATS consumer -> Database writer)
  worker:
    build:
      context: ../worker
      dockerfile: Dockerfile
    environment:
      DATABASE_URL: postgresql://nomnom:nomnom@postgres:5432/nomnom
      NATS_URL: nats://nats:4222
      NATS_STREAM: MESSAGES
      RUST_LOG: info
    depends_on:
      nats:
        condition: service_healthy
      postgres:
        condition: service_healthy
    restart: unless-stopped

volumes:
  postgres_data:
EOF

echo ""
echo "🐳 Starting all services (NATS, PostgreSQL, Ingestion API, Worker)..."
docker compose -f docker-compose.test.yml up -d --build

echo ""
echo "⏳ Waiting for services to be ready (30 seconds)..."
sleep 30

# Step 5: Check service health
echo ""
echo "🏥 Step 5: Checking service health..."

echo "  Checking NATS..."
curl -s http://localhost:8222/healthz > /dev/null && echo "  ✓ NATS is healthy" || echo "  ✗ NATS is not responding"

echo "  Checking Ingestion API..."
curl -s http://localhost:8080/health > /dev/null && echo "  ✓ Ingestion API is healthy" || echo "  ✗ Ingestion API is not responding"

# Step 6: Send test messages
echo ""
echo "📨 Step 6: Sending test messages..."

echo ""
echo "  Test 1: Valid Order message"
RESPONSE=$(curl -s -X POST http://localhost:8080/ingest/message \
  -H "Content-Type: application/json" \
  -d '{
    "order_key": "ORD-001",
    "customer_key": "CUST-123",
    "order_status": "O",
    "total_price": 999.99,
    "order_date": "2025-01-15"
  }')

echo "  Response: $RESPONSE"
MESSAGE_ID=$(echo "$RESPONSE" | grep -o '"message_id":"[^"]*"' | cut -d'"' -f4)
echo "  Message ID: $MESSAGE_ID"

echo ""
echo "  Test 2: Another Order"
curl -s -X POST http://localhost:8080/ingest/message \
  -H "Content-Type: application/json" \
  -d '{
    "order_key": "ORD-002",
    "customer_key": "CUST-456",
    "order_status": "F",
    "total_price": 1500.00,
    "order_date": "2025-01-16",
    "order_priority": "1-URGENT",
    "clerk": "Clerk#000000123"
  }' | grep -o '"message_id":"[^"]*"' | cut -d'"' -f4

echo ""
echo "  Test 3: Batch of messages"
curl -s -X POST http://localhost:8080/ingest/batch \
  -H "Content-Type: text/plain" \
  -d '{"order_key": "ORD-003", "customer_key": "CUST-789", "order_status": "O", "total_price": 500.00, "order_date": "2025-01-17"}
{"order_key": "ORD-004", "customer_key": "CUST-101", "order_status": "P", "total_price": 750.00, "order_date": "2025-01-18"}
{"order_key": "ORD-005", "customer_key": "CUST-202", "order_status": "F", "total_price": 2000.00, "order_date": "2025-01-19"}'

# Step 7: Wait for worker to process
echo ""
echo "⏳ Step 7: Waiting for worker to process messages (10 seconds)..."
sleep 10

# Step 8: Check NATS stats
echo ""
echo "📊 Step 8: Checking NATS JetStream stats..."
echo ""
curl -s http://localhost:8222/jsz | jq '.streams[] | {name: .name, messages: .state.messages, consumers: .state.consumers}'

# Step 9: Check database
echo ""
echo "🗄️  Step 9: Checking database for processed orders..."
docker compose -f docker-compose.test.yml exec -T postgres psql -U nomnom -d nomnom -c "SELECT order_key, customer_key, order_status, total_price, order_date FROM orders ORDER BY order_key;" || echo "Note: Table might not exist yet"

# Step 10: Check worker logs
echo ""
echo "📋 Step 10: Worker logs (last 20 lines)..."
docker compose -f docker-compose.test.yml logs --tail=20 worker

# Summary
echo ""
echo "✨ Test Summary"
echo "==============="
echo ""
echo "Services running:"
echo "  - NATS JetStream:  http://localhost:4222 (monitoring: http://localhost:8222)"
echo "  - PostgreSQL:      localhost:5432"
echo "  - Ingestion API:   http://localhost:8080"
echo "  - Worker:          Processing messages from NATS"
echo ""
echo "Useful commands:"
echo "  View logs:         cd $TEST_DIR/ingestion-server && docker compose -f docker-compose.test.yml logs -f"
echo "  Check NATS stats:  curl http://localhost:8222/jsz | jq"
echo "  Query database:    docker compose -f docker-compose.test.yml exec postgres psql -U nomnom -d nomnom"
echo "  Stop services:     docker compose -f docker-compose.test.yml down"
echo ""
echo "🎉 Test complete!"
