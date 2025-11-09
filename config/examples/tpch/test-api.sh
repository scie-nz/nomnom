#!/bin/bash
# Test script for TPC-H ingestion server API
# Generates random test data and sends it via HTTP POST to the ingestion server
#
# Usage:
#   ./test-api.sh          - Run tests with server in background
#   ./test-api.sh --debug  - Show server log in real-time (helpful for troubleshooting)

set -e

SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
cd "$SCRIPT_DIR"

# Colors for output
GREEN='\033[0;32m'
BLUE='\033[0;34m'
YELLOW='\033[1;33m'
RED='\033[0;31m'
NC='\033[0m' # No Color

# Configuration
# Use realpath to resolve /tmp symlink on macOS
if command -v realpath > /dev/null; then
    INGESTION_SERVER_DIR="$(realpath /tmp/tpch-ingestion-server 2>/dev/null || echo /tmp/tpch-ingestion-server)"
else
    INGESTION_SERVER_DIR="/tmp/tpch-ingestion-server"
fi
API_URL="http://localhost:8080"
DEFAULT_COUNT=5
DEBUG_MODE=false

# Parse arguments
if [[ "$1" == "--debug" ]]; then
    DEBUG_MODE=true
    echo -e "${YELLOW}Debug mode enabled - server log will be shown in real-time${NC}"
    echo
fi

echo -e "${BLUE}=== TPC-H Ingestion Server API Test ===${NC}"
echo

# Check if ingestion server exists
if [ ! -d "$INGESTION_SERVER_DIR" ]; then
    echo -e "${YELLOW}Ingestion server not found. Generating...${NC}"
    cd ../../..
    export RUSTFLAGS="-L /opt/homebrew/opt/libpq/lib"
    ./target/debug/nomnom generate-ingestion-server \
        --entities config/examples/tpch/entities \
        --output /tmp/tpch-ingestion-server \
        --database postgresql
    cd "$SCRIPT_DIR"

    # Re-resolve the path after generation
    if command -v realpath > /dev/null; then
        INGESTION_SERVER_DIR="$(realpath /tmp/tpch-ingestion-server)"
    fi
    echo -e "${BLUE}Using server directory: $INGESTION_SERVER_DIR${NC}"
    echo
fi

# Check if database is running
echo -e "${BLUE}Checking database...${NC}"
if ! docker compose ps | grep -q "Up"; then
    echo -e "${YELLOW}Database not running. Starting database...${NC}"
    ./db.sh start
    echo
else
    echo -e "${GREEN}✓ Database is running${NC}"
    echo
fi

# Load environment variables
export $(cat .env.test | grep -v '^#' | grep -v '^$' | xargs)

# Update ingestion server .env with database credentials
echo -e "${BLUE}Configuring ingestion server...${NC}"
cat > "$INGESTION_SERVER_DIR/.env" <<EOF
# Database connection
DATABASE_URL=${DATABASE_URL}

# Server configuration
PORT=8080
HOST=0.0.0.0

# Logging
RUST_LOG=info
EOF
echo -e "${GREEN}✓ Configuration updated${NC}"
echo

# Build ingestion server if needed
if [ ! -f "$INGESTION_SERVER_DIR/target/release/ingestion-server" ]; then
    echo -e "${BLUE}Building ingestion server (this may take a few minutes)...${NC}"
    cd "$INGESTION_SERVER_DIR"
    export RUSTFLAGS="-L /opt/homebrew/opt/libpq/lib"

    # Capture full output to check for errors
    BUILD_OUTPUT=$(cargo build --release 2>&1)
    BUILD_EXIT=$?

    # Show compilation progress
    echo "$BUILD_OUTPUT" | grep -E "(Compiling|Finished|error|warning:)" || true

    cd "$SCRIPT_DIR"

    if [ $BUILD_EXIT -ne 0 ]; then
        echo -e "${RED}✗ Build failed${NC}"
        echo
        echo -e "${YELLOW}Full build output:${NC}"
        echo "$BUILD_OUTPUT"
        exit 1
    fi

    echo -e "${GREEN}✓ Ingestion server built${NC}"
    echo
fi

# Verify server binary exists
SERVER_BINARY="$INGESTION_SERVER_DIR/target/release/ingestion-server"
echo -e "${BLUE}Checking for server binary at: $SERVER_BINARY${NC}"
if [ ! -f "$SERVER_BINARY" ]; then
    echo -e "${RED}✗ Server binary not found${NC}"
    echo
    echo -e "${YELLOW}Searching for binary in possible locations:${NC}"
    find /tmp -name "ingestion-server" -type f 2>/dev/null | head -5 || echo "  No binaries found"
    find /private/tmp -name "ingestion-server" -type f 2>/dev/null | head -5 || echo "  No binaries found"
    exit 1
fi
echo -e "${GREEN}✓ Server binary found${NC}"
echo

# Show environment configuration
echo -e "${BLUE}Server configuration:${NC}"
echo "  Binary: $SERVER_BINARY"
echo "  Working directory: $INGESTION_SERVER_DIR"
echo "  .env file:"
cat "$INGESTION_SERVER_DIR/.env" | sed 's/^/    /'
echo

# Start ingestion server in background
echo -e "${BLUE}Starting ingestion server...${NC}"
export RUSTFLAGS="-L /opt/homebrew/opt/libpq/lib"

if [ "$DEBUG_MODE" = true ]; then
    # In debug mode, show log in real-time in background
    cd "$INGESTION_SERVER_DIR"
    "$SERVER_BINARY" 2>&1 | tee /tmp/ingestion-server.log &
    SERVER_PID=$!
    cd "$SCRIPT_DIR"
else
    # Normal mode - log to file only
    cd "$INGESTION_SERVER_DIR"
    "$SERVER_BINARY" > /tmp/ingestion-server.log 2>&1 &
    SERVER_PID=$!
    cd "$SCRIPT_DIR"
fi
echo -e "${GREEN}✓ Server started with PID $SERVER_PID${NC}"
echo -e "${BLUE}Log file: /tmp/ingestion-server.log${NC}"
echo

# Function to cleanup on exit
cleanup() {
    if [ -n "$SERVER_PID" ]; then
        echo
        echo -e "${YELLOW}Stopping ingestion server...${NC}"
        kill $SERVER_PID 2>/dev/null || true

        # Wait up to 5 seconds for graceful shutdown
        for i in {1..5}; do
            if ! ps -p $SERVER_PID > /dev/null 2>&1; then
                echo -e "${GREEN}✓ Server stopped${NC}"
                return
            fi
            sleep 1
        done

        # If still running, force kill
        echo -e "${YELLOW}Force stopping server...${NC}"
        kill -9 $SERVER_PID 2>/dev/null || true
        echo -e "${GREEN}✓ Server stopped${NC}"
    fi
}
trap cleanup EXIT

# Wait for server to be ready
echo -e "${YELLOW}Waiting for server to start...${NC}"
echo -e "${BLUE}Server PID: $SERVER_PID${NC}"
echo -e "${BLUE}Checking health endpoint: ${API_URL}/health${NC}"
echo

for i in {1..30}; do
    # Check if process is still running
    if ! ps -p $SERVER_PID > /dev/null 2>&1; then
        echo -e "${RED}✗ Server process died${NC}"
        echo
        echo -e "${YELLOW}Server log (last 50 lines):${NC}"
        tail -50 /tmp/ingestion-server.log
        exit 1
    fi

    # Try to connect to health endpoint
    echo -n "Attempt $i/30: "
    if curl -s -f "${API_URL}/health" > /dev/null 2>&1; then
        echo -e "${GREEN}✓ Server is ready${NC}"
        break
    else
        echo -e "${YELLOW}not ready yet...${NC}"
    fi

    # After 5 seconds, show recent log output to help with debugging
    if [ $i -eq 5 ] && [ "$DEBUG_MODE" != true ]; then
        echo
        echo -e "${YELLOW}Server still not ready after 5 seconds. Recent log output:${NC}"
        tail -10 /tmp/ingestion-server.log | sed 's/^/  /'
        echo
    fi

    if [ $i -eq 30 ]; then
        echo -e "${RED}✗ Server failed to start after 30 seconds${NC}"
        echo
        echo -e "${YELLOW}Server log (last 50 lines):${NC}"
        tail -50 /tmp/ingestion-server.log
        echo
        echo -e "${YELLOW}Server process status:${NC}"
        ps aux | grep ingestion-server | grep -v grep || echo "Process not found"
        echo
        echo -e "${BLUE}Tip: Run with --debug flag to see real-time server output${NC}"
        exit 1
    fi
    sleep 1
done
echo

# Test 1: Health check
echo -e "${BLUE}1. Health Check:${NC}"
curl -s "${API_URL}/health" | jq .
echo

# Test 2: Generate test data and send via API
echo -e "${BLUE}2. Sending test messages via API:${NC}"
python3 generate_test_data.py --count $DEFAULT_COUNT --seed 42 > /tmp/tpch_test_data.txt

# Send messages one by one
MESSAGE_COUNT=0
SUCCESS_COUNT=0
FAILED_COUNT=0

while IFS= read -r line; do
    MESSAGE_COUNT=$((MESSAGE_COUNT + 1))
    echo -e "${YELLOW}Sending message $MESSAGE_COUNT...${NC}"

    # Send to ingestion endpoint and capture response
    RESPONSE_FILE=$(mktemp)
    HTTP_CODE=$(curl -s -w "%{http_code}" -X POST "${API_URL}/ingest/message" \
        -H "Content-Type: text/plain" \
        -d "$line" \
        -o "$RESPONSE_FILE")

    BODY=$(cat "$RESPONSE_FILE")
    rm -f "$RESPONSE_FILE"

    if [ "$HTTP_CODE" = "200" ]; then
        SUCCESS_COUNT=$((SUCCESS_COUNT + 1))
        echo -e "${GREEN}✓ Success (HTTP $HTTP_CODE)${NC}"
        echo "$BODY" | jq -c '{status, entity, id}' 2>/dev/null || echo "$BODY"
    else
        FAILED_COUNT=$((FAILED_COUNT + 1))
        echo -e "${RED}✗ Failed (HTTP $HTTP_CODE)${NC}"
        echo "$BODY"
    fi
    echo
done < /tmp/tpch_test_data.txt

echo -e "${BLUE}Summary:${NC}"
echo -e "  Total messages: $MESSAGE_COUNT"
echo -e "  ${GREEN}Successful: $SUCCESS_COUNT${NC}"
echo -e "  ${RED}Failed: $FAILED_COUNT${NC}"
echo

# Test 3: Batch ingestion
echo -e "${BLUE}3. Batch Ingestion:${NC}"
BATCH_RESPONSE=$(curl -s -X POST "${API_URL}/ingest/batch" \
    -H "Content-Type: text/plain" \
    --data-binary @/tmp/tpch_test_data.txt)

echo "$BATCH_RESPONSE" | jq .
echo

# Test 4: Verify data in database
echo -e "${BLUE}4. Verifying data in database:${NC}"
docker compose exec -T postgres psql -U tpch_user -d tpch_db -c "SELECT COUNT(*) as order_count FROM orders;"
docker compose exec -T postgres psql -U tpch_user -d tpch_db -c "SELECT COUNT(*) as line_item_count FROM order_line_items;"
echo

# Test 5: Show recent orders
echo -e "${BLUE}5. Recent orders in database:${NC}"
docker compose exec -T postgres psql -U tpch_user -d tpch_db -c "SELECT order_key, customer_key, total_price, order_date FROM orders ORDER BY id DESC LIMIT 5;"
echo

echo -e "${GREEN}✓ All tests complete!${NC}"
echo
echo -e "${BLUE}API Endpoints:${NC}"
echo "  Health:  ${API_URL}/health"
echo "  Ingest:  ${API_URL}/ingest/message"
echo "  Batch:   ${API_URL}/ingest/batch"
echo "  Stats:   ${API_URL}/stats"
echo "  Swagger: ${API_URL}/swagger-ui"
echo
echo -e "${BLUE}Server log: /tmp/ingestion-server.log${NC}"
