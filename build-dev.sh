#!/bin/bash
set -e

# Fast development build script
# Builds debug binaries locally, then creates lightweight Docker images

echo "=========================================="
echo "Building nomnom (development mode)"
echo "=========================================="

# Build debug binaries locally (much faster than release)
echo "[1/4] Building debug binaries..."
echo "  - Building nats-api..."
cargo build --bin nats-api 2>&1 | tail -20

echo "  - Building nomnom..."
cargo build --bin nomnom 2>&1 | tail -20

echo "[2/4] Checking binaries..."
if [ ! -f target/debug/nats-api ]; then
    echo "ERROR: nats-api binary not found!"
    exit 1
fi

if [ ! -f target/debug/nomnom ]; then
    echo "ERROR: nomnom binary not found!"
    exit 1
fi

# Check if worker binary exists, if not use nomnom as placeholder
if [ ! -f target/debug/worker ]; then
    echo "  - Worker binary not found, creating placeholder from nomnom..."
    cp target/debug/nomnom target/debug/worker
fi

ls -lh target/debug/nats-api target/debug/worker target/debug/nomnom

echo "[3/4] Building Docker images..."

# Build ingestion API image
echo "  - Building nats-api image..."
docker build -f Dockerfile.dev -t nomnom-ingestion-api:dev .

# Build worker image (uses placeholder if worker doesn't exist)
echo "  - Building worker image..."
docker build -f Dockerfile.worker.dev -t nomnom-worker:dev .

# Build dashboard backend image
echo "  - Building dashboard backend image..."
docker build -f Dockerfile.dashboard.dev -t nomnom-dashboard-backend:dev .

# Build dashboard frontend (simple placeholder)
echo "  - Building dashboard frontend image..."
docker build -f Dockerfile.frontend -t nomnom-dashboard-frontend:dev .

echo "[4/4] Done!"
echo ""
echo "Images built:"
echo "  - nomnom-ingestion-api:dev"
echo "  - nomnom-worker:dev"
echo "  - nomnom-dashboard-backend:dev"
echo "  - nomnom-dashboard-frontend:dev"
echo ""
echo "Run './test-helm-kind.sh --dev' to deploy to kind cluster"
