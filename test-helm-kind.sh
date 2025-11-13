#!/bin/bash
set -e

# Colors for output
RED='\033[0;31m'
GREEN='\033[0;32m'
YELLOW='\033[1;33m'
BLUE='\033[0;34m'
NC='\033[0m' # No Color

# Configuration
CLUSTER_NAME="nomnom-test"
NAMESPACE="nomnom-dev"
REGISTRY_NAME="kind-registry"
REGISTRY_PORT="5001"
DEV_MODE=false

# Check for --dev flag
if [ "$1" = "--dev" ] || [ "$2" = "--dev" ]; then
    DEV_MODE=true
fi

echo -e "${BLUE}========================================${NC}"
if [ "$DEV_MODE" = true ]; then
    echo -e "${BLUE}Testing nomnom Helm Chart on kind (DEV MODE)${NC}"
else
    echo -e "${BLUE}Testing nomnom Helm Chart on kind${NC}"
fi
echo -e "${BLUE}========================================${NC}"

# Function to print colored messages
log_info() {
    echo -e "${BLUE}[INFO]${NC} $1"
}

log_success() {
    echo -e "${GREEN}[SUCCESS]${NC} $1"
}

log_warning() {
    echo -e "${YELLOW}[WARNING]${NC} $1"
}

log_error() {
    echo -e "${RED}[ERROR]${NC} $1"
}

# Function to check if command exists
command_exists() {
    command -v "$1" >/dev/null 2>&1
}

# Check prerequisites
log_info "Checking prerequisites..."
for cmd in kind kubectl helm docker; do
    if ! command_exists "$cmd"; then
        log_error "$cmd is not installed. Please install it first."
        exit 1
    fi
done
log_success "All prerequisites are installed"

# Create kind cluster with ingress support
log_info "Creating kind cluster '$CLUSTER_NAME'..."
if kind get clusters | grep -q "^${CLUSTER_NAME}$"; then
    log_warning "Cluster '$CLUSTER_NAME' already exists. Deleting it..."
    kind delete cluster --name "$CLUSTER_NAME"
fi

# Create kind cluster configuration
cat <<EOF | kind create cluster --name "$CLUSTER_NAME" --config=-
kind: Cluster
apiVersion: kind.x-k8s.io/v1alpha4
nodes:
- role: control-plane
  kubeadmConfigPatches:
  - |
    kind: InitConfiguration
    nodeRegistration:
      kubeletExtraArgs:
        node-labels: "ingress-ready=true"
  extraPortMappings:
  - containerPort: 30080
    hostPort: 8080
    protocol: TCP
  - containerPort: 30081
    hostPort: 8081
    protocol: TCP
  - containerPort: 30082
    hostPort: 3000
    protocol: TCP
containerdConfigPatches:
- |-
  [plugins."io.containerd.grpc.v1.cri".registry.mirrors."localhost:${REGISTRY_PORT}"]
    endpoint = ["http://${REGISTRY_NAME}:5000"]
EOF

log_success "Kind cluster created"

# Create local registry if it doesn't exist
log_info "Setting up local Docker registry..."
if ! docker ps | grep -q "$REGISTRY_NAME"; then
    if docker ps -a | grep -q "$REGISTRY_NAME"; then
        log_info "Starting existing registry..."
        docker start "$REGISTRY_NAME"
    else
        log_info "Creating local registry..."
        docker run -d --restart=always -p "${REGISTRY_PORT}:5000" --name "$REGISTRY_NAME" registry:2
    fi
fi

# Connect registry to kind network
log_info "Connecting registry to kind network..."
if ! docker network inspect kind | grep -q "$REGISTRY_NAME"; then
    docker network connect "kind" "$REGISTRY_NAME" 2>/dev/null || true
fi

log_success "Local registry is ready at localhost:${REGISTRY_PORT}"

# Build and push Docker images
if [ "$DEV_MODE" = true ]; then
    log_info "Using DEV MODE - building debug images in Docker (Debian-based)..."

    # Build ingestion API
    log_info "Building ingestion API (debug mode)..."
    docker build -f Dockerfile.dev-complete -t localhost:${REGISTRY_PORT}/nomnom-ingestion-api:latest .
    docker push localhost:${REGISTRY_PORT}/nomnom-ingestion-api:latest

    # Build worker
    log_info "Building worker (debug mode)..."
    docker build -f Dockerfile.worker-dev-complete -t localhost:${REGISTRY_PORT}/nomnom-worker:latest .
    docker push localhost:${REGISTRY_PORT}/nomnom-worker:latest

    # Build dashboard backend with fixed code
    log_info "Building dashboard backend (debug mode with primary key fix)..."

    # First, build nomnom binary with the fix
    log_info "Building nomnom binary..."
    export RUSTFLAGS="-L /opt/homebrew/opt/libpq/lib"
    cargo build --bin nomnom

    # Generate dashboard with fixed code
    log_info "Generating dashboard with fixed code..."
    DASHBOARD_OUTPUT="/tmp/kind-test-dashboard"
    rm -rf "$DASHBOARD_OUTPUT"
    ./target/debug/nomnom generate-dashboard \
      --entities config/examples/tpch/entities \
      --output "$DASHBOARD_OUTPUT" \
      --database postgresql \
      --backend axum

    # Copy dev Dockerfile to dashboard output
    log_info "Copying Dockerfile.backend.dev to dashboard output..."
    cat > "$DASHBOARD_OUTPUT/Dockerfile.backend.dev" <<'DOCKERFILE'
FROM rust:1-slim-bookworm AS builder
RUN apt-get update && apt-get install -y \
    libpq-dev \
    libsqlite3-dev \
    pkg-config \
    libssl-dev \
    && rm -rf /var/lib/apt/lists/*
WORKDIR /app
COPY Cargo.toml ./
COPY src ./src
RUN rm -f Cargo.lock
RUN cargo build --bin dashboard

FROM debian:bookworm-slim
RUN apt-get update && apt-get install -y \
    libpq5 \
    libssl3 \
    ca-certificates \
    && rm -rf /var/lib/apt/lists/*
RUN groupadd -g 1000 appuser && \
    useradd -r -u 1000 -g appuser appuser
WORKDIR /app
COPY --from=builder --chown=appuser:appuser /app/target/debug/dashboard /app/
USER appuser:appuser
EXPOSE 3000
CMD ["/app/dashboard"]
DOCKERFILE

    # Build dashboard backend image
    log_info "Building dashboard backend Docker image..."
    docker build -f "$DASHBOARD_OUTPUT/Dockerfile.backend.dev" \
      -t localhost:${REGISTRY_PORT}/nomnom-dashboard-backend:latest \
      "$DASHBOARD_OUTPUT"
    docker push localhost:${REGISTRY_PORT}/nomnom-dashboard-backend:latest

    # Build dashboard frontend
    log_info "Building dashboard frontend..."
    docker build -f Dockerfile.frontend -t localhost:${REGISTRY_PORT}/nomnom-dashboard-frontend:latest .
    docker push localhost:${REGISTRY_PORT}/nomnom-dashboard-frontend:latest

    log_success "Dev images built and pushed to registry"
else
    log_info "Building Docker images (production mode - this will take a while)..."

    # Build ingestion API
    log_info "Building ingestion API image..."
    docker build -t localhost:${REGISTRY_PORT}/nomnom-ingestion-api:latest -f- . <<'DOCKERFILE'
FROM rustlang/rust:nightly-alpine AS builder
RUN apk add --no-cache musl-dev openssl-dev openssl-libs-static pkgconfig
WORKDIR /app
COPY Cargo.toml ./
COPY src ./src
RUN cargo build --release --bin nats-api

FROM alpine:latest
RUN apk add --no-cache ca-certificates libgcc
COPY --from=builder /app/target/release/nats-api /usr/local/bin/
EXPOSE 8080
CMD ["nats-api"]
DOCKERFILE

docker push localhost:${REGISTRY_PORT}/nomnom-ingestion-api:latest

# Build worker
log_info "Building worker image..."
docker build -t localhost:${REGISTRY_PORT}/nomnom-worker:latest -f- . <<'DOCKERFILE'
FROM rustlang/rust:nightly-alpine AS builder
RUN apk add --no-cache musl-dev openssl-dev openssl-libs-static pkgconfig
WORKDIR /app
COPY Cargo.toml ./
COPY src ./src
RUN cargo build --release --bin worker

FROM alpine:latest
RUN apk add --no-cache ca-certificates libgcc
COPY --from=builder /app/target/release/worker /usr/local/bin/
CMD ["worker"]
DOCKERFILE

docker push localhost:${REGISTRY_PORT}/nomnom-worker:latest

# Build dashboard backend (placeholder - adjust based on your actual implementation)
log_info "Building dashboard backend image..."
docker build -t localhost:${REGISTRY_PORT}/nomnom-dashboard-backend:latest -f- . <<'DOCKERFILE'
FROM rustlang/rust:nightly-alpine AS builder
RUN apk add --no-cache musl-dev openssl-dev openssl-libs-static pkgconfig
WORKDIR /app
COPY Cargo.toml ./
COPY src ./src
RUN cargo build --release

FROM alpine:latest
RUN apk add --no-cache ca-certificates libgcc
COPY --from=builder /app/target/release/nomnom /usr/local/bin/
EXPOSE 3000
CMD ["nomnom", "serve-dashboard"]
DOCKERFILE

docker push localhost:${REGISTRY_PORT}/nomnom-dashboard-backend:latest

# Build dashboard frontend (placeholder)
    log_info "Building dashboard frontend image..."
    docker build -f Dockerfile.frontend -t localhost:${REGISTRY_PORT}/nomnom-dashboard-frontend:latest .
    docker push localhost:${REGISTRY_PORT}/nomnom-dashboard-frontend:latest

    log_success "All images built and pushed to registry"
fi

# Create namespace
log_info "Creating namespace '$NAMESPACE'..."
kubectl create namespace "$NAMESPACE" --dry-run=client -o yaml | kubectl apply -f -

# Update values-dev.yaml with correct registry
log_info "Updating values-dev.yaml with local registry..."
cat > /tmp/values-dev-kind.yaml <<EOF
global:
  namespace: $NAMESPACE
  domain: localhost

nats:
  replicas: 1
  cluster:
    enabled: false
  jetstream:
    storage:
      size: 1Gi
      storageClassName: standard
  resources:
    requests:
      memory: 128Mi
      cpu: 50m
    limits:
      memory: 256Mi
      cpu: 200m

postgresql:
  storage:
    size: 1Gi
    storageClassName: standard
  resources:
    requests:
      memory: 128Mi
      cpu: 50m
    limits:
      memory: 256Mi
      cpu: 200m

database:
  host: ""
  port: 5432
  name: nomnom
  username: postgres
  password: devpassword123

ingestion:
  api:
    enabled: true
    replicas: 1
    image:
      repository: localhost:${REGISTRY_PORT}/nomnom-ingestion-api
      tag: latest
      pullPolicy: Always
    service:
      type: NodePort
      port: 8080
      nodePort: 30080
    autoscaling:
      enabled: false
    resources:
      requests:
        memory: 64Mi
        cpu: 50m
      limits:
        memory: 128Mi
        cpu: 200m

  worker:
    enabled: true
    image:
      repository: localhost:${REGISTRY_PORT}/nomnom-worker
      tag: latest
      pullPolicy: Always
    env:
      NATS_STREAM: "MESSAGES"
      NATS_CONSUMER: "workers"
      MAX_DELIVER: "3"
      BATCH_SIZE: "5"
      POLL_INTERVAL_MS: "500"
    resources:
      requests:
        memory: 64Mi
        cpu: 50m
      limits:
        memory: 128Mi
        cpu: 200m
    keda:
      enabled: false

dashboard:
  backend:
    enabled: true
    replicas: 1
    image:
      repository: localhost:${REGISTRY_PORT}/nomnom-dashboard-backend
      tag: latest
      pullPolicy: Always
    service:
      type: NodePort
      port: 3000
      nodePort: 30082
    resources:
      requests:
        memory: 64Mi
        cpu: 25m
      limits:
        memory: 128Mi
        cpu: 100m

  frontend:
    enabled: true
    replicas: 1
    image:
      repository: localhost:${REGISTRY_PORT}/nomnom-dashboard-frontend
      tag: latest
      pullPolicy: Always
    service:
      type: NodePort
      port: 5173
      nodePort: 30081
    resources:
      requests:
        memory: 32Mi
        cpu: 25m
      limits:
        memory: 64Mi
        cpu: 50m
    healthCheck: null

ingress:
  enabled: false

keda:
  enabled: false
EOF

# Install Helm chart
log_info "Installing Helm chart..."
helm upgrade --install nomnom nomnom-helm \
    -f /tmp/values-dev-kind.yaml \
    -n "$NAMESPACE" \
    --wait \
    --timeout 5m

log_success "Helm chart installed"

# Wait for pods to be ready
log_info "Waiting for pods to be ready..."
kubectl wait --for=condition=ready pod -l app.kubernetes.io/instance=nomnom -n "$NAMESPACE" --timeout=300s || {
    log_error "Pods did not become ready in time"
    kubectl get pods -n "$NAMESPACE"
    kubectl describe pods -n "$NAMESPACE"
    exit 1
}

log_success "All pods are ready"

# Display deployment status
echo ""
log_info "Deployment Status:"
kubectl get all -n "$NAMESPACE"

echo ""
log_info "Pod Logs Preview:"
for pod in $(kubectl get pods -n "$NAMESPACE" -o jsonpath='{.items[*].metadata.name}'); do
    echo ""
    echo -e "${YELLOW}=== Logs from $pod ===${NC}"
    kubectl logs "$pod" -n "$NAMESPACE" --tail=10 || true
done

# Test ingestion API
echo ""
log_info "Testing Ingestion API..."
sleep 5
API_URL="http://localhost:8080"

# Health check
log_info "Checking API health..."
if curl -s -f "${API_URL}/health" > /dev/null; then
    log_success "API health check passed"
else
    log_warning "API health check failed (this might be expected if /health endpoint is not implemented yet)"
fi

# Test ingestion
log_info "Testing data ingestion..."
RESPONSE=$(curl -s -w "\n%{http_code}" -X POST "${API_URL}/ingest/test_customer" \
    -H "Content-Type: application/json" \
    -d '{"id": 1, "name": "Test Customer", "email": "test@example.com"}' || echo "000")

HTTP_CODE=$(echo "$RESPONSE" | tail -n1)
if [ "$HTTP_CODE" = "200" ] || [ "$HTTP_CODE" = "201" ]; then
    log_success "Data ingestion test passed (HTTP $HTTP_CODE)"
else
    log_warning "Data ingestion test returned HTTP $HTTP_CODE (endpoint may not be fully implemented)"
fi

echo ""
echo -e "${BLUE}========================================${NC}"
echo -e "${GREEN}Deployment Complete!${NC}"
echo -e "${BLUE}========================================${NC}"
echo ""
echo "Access the services:"
echo "  Ingestion API:      http://localhost:8080"
echo "  Dashboard Frontend: http://localhost:8081"
echo "  Dashboard Backend:  http://localhost:3000"
echo ""
echo "Useful commands:"
echo "  View pods:          kubectl get pods -n $NAMESPACE"
echo "  View logs:          kubectl logs -l app.kubernetes.io/instance=nomnom -n $NAMESPACE"
echo "  Port-forward NATS:  kubectl port-forward svc/nomnom-nats -n $NAMESPACE 4222:4222"
echo "  Port-forward DB:    kubectl port-forward svc/nomnom-postgres -n $NAMESPACE 5432:5432"
echo ""
echo "To clean up:"
echo "  ./test-helm-kind.sh cleanup"
echo ""
echo "To rebuild and redeploy:"
echo "  ./test-helm-kind.sh --dev    (fast - uses local debug builds)"
echo "  ./test-helm-kind.sh          (slow - builds optimized releases)"
echo ""

# Cleanup function
if [ "$1" = "cleanup" ] || [ "$2" = "cleanup" ]; then
    log_info "Cleaning up..."
    helm uninstall nomnom -n "$NAMESPACE" 2>/dev/null || true
    kubectl delete namespace "$NAMESPACE" 2>/dev/null || true
    kind delete cluster --name "$CLUSTER_NAME"
    docker stop "$REGISTRY_NAME" 2>/dev/null || true
    docker rm "$REGISTRY_NAME" 2>/dev/null || true
    log_success "Cleanup complete"
    exit 0
fi
