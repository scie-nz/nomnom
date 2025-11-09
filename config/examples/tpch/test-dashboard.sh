#!/bin/bash
# Test script for TPC-H Axum Dashboard
# Generates the dashboard, starts the backend server, and provides instructions for frontend
#
# Usage:
#   ./test-dashboard.sh          - Generate and start dashboard
#   ./test-dashboard.sh --debug  - Show server log in real-time (helpful for troubleshooting)
#   ./test-dashboard.sh --clean  - Clean and regenerate dashboard from scratch

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
    DASHBOARD_DIR="$(realpath /tmp/tpch-dashboard 2>/dev/null || echo /tmp/tpch-dashboard)"
else
    DASHBOARD_DIR="/tmp/tpch-dashboard"
fi
BACKEND_URL="http://localhost:3000"
FRONTEND_URL="http://localhost:5173"
DEBUG_MODE=false
CLEAN_MODE=false

# Parse arguments
for arg in "$@"; do
    case $arg in
        --debug)
            DEBUG_MODE=true
            echo -e "${YELLOW}Debug mode enabled - server log will be shown in real-time${NC}"
            echo
            ;;
        --clean)
            CLEAN_MODE=true
            echo -e "${YELLOW}Clean mode enabled - will regenerate dashboard${NC}"
            echo
            ;;
    esac
done

echo -e "${BLUE}=== TPC-H Axum Dashboard Test ===${NC}"
echo

# Clean if requested
if [ "$CLEAN_MODE" = true ] && [ -d "$DASHBOARD_DIR" ]; then
    echo -e "${YELLOW}Cleaning existing dashboard...${NC}"
    rm -rf "$DASHBOARD_DIR"
    echo -e "${GREEN}✓ Dashboard cleaned${NC}"
    echo
fi

# Check if dashboard exists
if [ ! -d "$DASHBOARD_DIR" ]; then
    echo -e "${YELLOW}Dashboard not found. Generating...${NC}"
    cd ../../..
    export RUSTFLAGS="-L /opt/homebrew/opt/libpq/lib"
    ./target/debug/nomnom generate-dashboard \
        --entities config/examples/tpch/entities \
        --output /tmp/tpch-dashboard \
        --database postgresql \
        --backend axum
    cd "$SCRIPT_DIR"

    # Re-resolve the path after generation
    if command -v realpath > /dev/null; then
        DASHBOARD_DIR="$(realpath /tmp/tpch-dashboard)"
    fi
    echo -e "${BLUE}Using dashboard directory: $DASHBOARD_DIR${NC}"
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

# Update dashboard .env with database credentials
echo -e "${BLUE}Configuring dashboard backend...${NC}"
cat > "$DASHBOARD_DIR/.env" <<EOF
# Database connection
DATABASE_URL=${DATABASE_URL}

# Server configuration
PORT=3000
HOST=0.0.0.0

# Logging
RUST_LOG=info
EOF
echo -e "${GREEN}✓ Configuration updated${NC}"
echo

# Build dashboard backend if needed
if [ ! -f "$DASHBOARD_DIR/target/release/dashboard" ]; then
    echo -e "${BLUE}Building dashboard backend (this may take a few minutes)...${NC}"
    cd "$DASHBOARD_DIR"
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

    echo -e "${GREEN}✓ Dashboard backend built${NC}"
    echo
fi

# Verify backend binary exists
BACKEND_BINARY="$DASHBOARD_DIR/target/release/dashboard"
echo -e "${BLUE}Checking for backend binary at: $BACKEND_BINARY${NC}"
if [ ! -f "$BACKEND_BINARY" ]; then
    echo -e "${RED}✗ Backend binary not found${NC}"
    echo
    echo -e "${YELLOW}Searching for binary in possible locations:${NC}"
    find /tmp -name "dashboard" -type f 2>/dev/null | head -5 || echo "  No binaries found"
    find /private/tmp -name "dashboard" -type f 2>/dev/null | head -5 || echo "  No binaries found"
    exit 1
fi
echo -e "${GREEN}✓ Backend binary found${NC}"
echo

# Show environment configuration
echo -e "${BLUE}Backend configuration:${NC}"
echo "  Binary: $BACKEND_BINARY"
echo "  Working directory: $DASHBOARD_DIR"
echo "  .env file:"
cat "$DASHBOARD_DIR/.env" | sed 's/^/    /'
echo

# Start dashboard backend in background
echo -e "${BLUE}Starting dashboard backend...${NC}"
export RUSTFLAGS="-L /opt/homebrew/opt/libpq/lib"

if [ "$DEBUG_MODE" = true ]; then
    # In debug mode, show log in real-time in background
    cd "$DASHBOARD_DIR"
    "$BACKEND_BINARY" 2>&1 | tee /tmp/dashboard-backend.log &
    BACKEND_PID=$!
    cd "$SCRIPT_DIR"
else
    # Normal mode - log to file only
    cd "$DASHBOARD_DIR"
    "$BACKEND_BINARY" > /tmp/dashboard-backend.log 2>&1 &
    BACKEND_PID=$!
    cd "$SCRIPT_DIR"
fi
echo -e "${GREEN}✓ Backend started with PID $BACKEND_PID${NC}"
echo -e "${BLUE}Log file: /tmp/dashboard-backend.log${NC}"
echo

# Wait for backend to be ready
echo -e "${YELLOW}Waiting for backend to start...${NC}"
echo -e "${BLUE}Backend PID: $BACKEND_PID${NC}"
echo -e "${BLUE}Checking health endpoint: ${BACKEND_URL}/api/health${NC}"
echo

for i in {1..30}; do
    # Check if process is still running
    if ! ps -p $BACKEND_PID > /dev/null 2>&1; then
        echo -e "${RED}✗ Backend process died${NC}"
        echo
        echo -e "${YELLOW}Backend log (last 50 lines):${NC}"
        tail -50 /tmp/dashboard-backend.log
        exit 1
    fi

    # Try to connect to health endpoint
    echo -n "Attempt $i/30: "
    if curl -s -f "${BACKEND_URL}/api/health" > /dev/null 2>&1; then
        echo -e "${GREEN}✓ Backend is ready${NC}"
        break
    else
        echo -e "${YELLOW}not ready yet...${NC}"
    fi

    # After 5 seconds, show recent log output to help with debugging
    if [ $i -eq 5 ] && [ "$DEBUG_MODE" != true ]; then
        echo
        echo -e "${YELLOW}Backend still not ready after 5 seconds. Recent log output:${NC}"
        tail -10 /tmp/dashboard-backend.log | sed 's/^/  /'
        echo
    fi

    if [ $i -eq 30 ]; then
        echo -e "${RED}✗ Backend failed to start after 30 seconds${NC}"
        echo
        echo -e "${YELLOW}Backend log (last 50 lines):${NC}"
        tail -50 /tmp/dashboard-backend.log
        echo
        echo -e "${YELLOW}Backend process status:${NC}"
        ps aux | grep dashboard | grep -v grep || echo "Process not found"
        echo
        echo -e "${BLUE}Tip: Run with --debug flag to see real-time backend output${NC}"
        exit 1
    fi
    sleep 1
done
echo

# Test backend endpoints
echo -e "${BLUE}1. Health Check:${NC}"
curl -s "${BACKEND_URL}/api/health" | jq .
echo

echo -e "${BLUE}2. Entity Configuration:${NC}"
curl -s "${BACKEND_URL}/api/entities" | jq .
echo

echo -e "${BLUE}3. Database Statistics:${NC}"
curl -s "${BACKEND_URL}/api/stats" | jq .
echo

# Check if frontend dependencies are installed
echo -e "${BLUE}Checking frontend setup...${NC}"
if [ ! -d "$DASHBOARD_DIR/frontend/node_modules" ]; then
    echo -e "${YELLOW}Frontend dependencies not installed. Installing...${NC}"
    cd "$DASHBOARD_DIR/frontend"
    npm install > /dev/null 2>&1
    if [ $? -eq 0 ]; then
        echo -e "${GREEN}✓ Frontend dependencies installed${NC}"
    else
        echo -e "${RED}✗ Failed to install frontend dependencies${NC}"
        echo -e "${YELLOW}You can install manually with:${NC}"
        echo -e "  cd $DASHBOARD_DIR/frontend && npm install"
    fi
    cd "$SCRIPT_DIR"
    echo
else
    echo -e "${GREEN}✓ Frontend dependencies already installed${NC}"
    echo
fi

# Start frontend in background
echo -e "${BLUE}Starting frontend development server...${NC}"
cd "$DASHBOARD_DIR/frontend"
npm run dev > /tmp/dashboard-frontend.log 2>&1 &
FRONTEND_PID=$!
cd "$SCRIPT_DIR"

# Cleanup function to stop both frontend and backend
cleanup() {
    if [ -n "$FRONTEND_PID" ]; then
        echo
        echo -e "${YELLOW}Stopping frontend server...${NC}"
        kill $FRONTEND_PID 2>/dev/null || true
        sleep 1
        echo -e "${GREEN}✓ Frontend stopped${NC}"
    fi

    if [ -n "$BACKEND_PID" ]; then
        echo -e "${YELLOW}Stopping dashboard backend...${NC}"
        kill $BACKEND_PID 2>/dev/null || true

        # Wait up to 5 seconds for graceful shutdown
        for i in {1..5}; do
            if ! ps -p $BACKEND_PID > /dev/null 2>&1; then
                echo -e "${GREEN}✓ Backend stopped${NC}"
                return
            fi
            sleep 1
        done

        # If still running, force kill
        echo -e "${YELLOW}Force stopping backend...${NC}"
        kill -9 $BACKEND_PID 2>/dev/null || true
        echo -e "${GREEN}✓ Backend stopped${NC}"
    fi
}

# Register cleanup to run on exit
trap cleanup EXIT

# Wait for frontend to start
sleep 3
if ps -p $FRONTEND_PID > /dev/null 2>&1; then
    echo -e "${GREEN}✓ Frontend started with PID $FRONTEND_PID${NC}"
    echo -e "${BLUE}Frontend log: /tmp/dashboard-frontend.log${NC}"
else
    echo -e "${YELLOW}⚠ Frontend may have failed to start. Check log: /tmp/dashboard-frontend.log${NC}"
fi
echo

echo -e "${GREEN}✓ Dashboard backend is running!${NC}"
echo
echo -e "${BLUE}=== Dashboard URLs ===${NC}"
echo -e "  Backend API:  ${BACKEND_URL}/api/health"
echo -e "  WebSocket:    ws://${BACKEND_URL#http://}/ws"
echo -e "  Frontend:     ${FRONTEND_URL}"
echo
echo -e "${BLUE}=== API Endpoints ===${NC}"
echo -e "  Health:       ${BACKEND_URL}/api/health"
echo -e "  Entities:     ${BACKEND_URL}/api/entities"
echo -e "  Stats:        ${BACKEND_URL}/api/stats"
echo -e "  WebSocket:    ${BACKEND_URL}/ws"
echo
echo -e "${BLUE}Backend log: /tmp/dashboard-backend.log${NC}"
echo
echo -e "${YELLOW}Press Ctrl+C to stop the backend server${NC}"

# Keep the script running so the backend stays alive
wait $BACKEND_PID
