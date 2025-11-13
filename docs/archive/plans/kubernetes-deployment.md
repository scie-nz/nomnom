# Kubernetes Deployment Plan for nomnom

## Overview

Deploy nomnom ingestion server and dashboard to Kubernetes using Helm charts, with local testing via kind (Kubernetes in Docker).

## Architecture Decision: NATS JetStream + KEDA

### Primary Architecture: Async with NATS JetStream

**Flow:**
```
Client → Ingestion API → NATS JetStream → Workers → PostgreSQL
         (202 Accepted)    (Main Stream)    (KEDA)    (Data + Status)
                                ↓
                          DLQ Stream ← Failed after MAX_DELIVER attempts
                          (7-day retention)
```

**Components:**

1. **Ingestion API (Deployment)**
   - Stateless HTTP server
   - Publishes to NATS JetStream
   - Returns 202 Accepted immediately
   - Horizontally scalable
   - Low latency (< 50ms response)

2. **NATS JetStream (StatefulSet)**
   - Durable message queue
   - 3-replica cluster for HA
   - File-based storage on PVCs
   - At-least-once delivery guarantee
   - Message ordering per subject

3. **Worker Pods (Deployment with KEDA)**
   - Consume from NATS JetStream
   - Parse and validate messages
   - Write to PostgreSQL
   - Track message status in database (accepted/processing/completed/failed/dlq)
   - Route failed messages to DLQ after MAX_DELIVER attempts
   - Auto-scale based on queue depth (KEDA)
   - Can scale to zero when idle

4. **Dashboard (Deployment)**
   - Stateless query API
   - Reads from PostgreSQL
   - Horizontally scalable

### Why NATS Instead of Direct DB Writes

**Rationale:**
- **Decoupling**: API and DB write performance are independent
- **Throughput**: API can handle 10,000+ req/sec, DB writes batched
- **Durability**: Messages persisted in JetStream before ACK
- **Backpressure**: Natural queue-based flow control
- **Scalability**: Workers scale independently via KEDA
- **Reliability**: Automatic retries, dead-letter queues
- **Observability**: Full message tracking and status
- **Cost**: Workers can scale to zero during idle periods
- **Rust-native**: async-nats is idiomatic and performant

### Resource Types

- **Ingestion API**: Deployment (stateless, HPA)
- **NATS JetStream**: StatefulSet (requires persistent storage)
- **Workers**: Deployment (stateless, KEDA autoscaling)
- **PostgreSQL**: StatefulSet (requires persistent storage)
- **Dashboard**: Deployment (stateless, HPA)

## Helm Chart Structure

```
nomnom-helm/
├── Chart.yaml                      # Helm chart metadata
├── values.yaml                     # Default configuration values
├── values-dev.yaml                 # Development overrides (for kind)
├── values-prod.yaml                # Production overrides
├── templates/
│   ├── _helpers.tpl                # Template helpers
│   ├── NOTES.txt                   # Post-install notes
│   │
│   ├── namespace.yaml              # Namespace definition
│   ├── configmap.yaml              # Common configuration
│   ├── secrets.yaml                # Database credentials
│   │
│   ├── nats/
│   │   ├── statefulset.yaml        # NATS JetStream StatefulSet
│   │   ├── service.yaml            # NATS Service (client, cluster, monitor)
│   │   ├── pvc.yaml                # Persistent Volume Claim
│   │   └── configmap.yaml          # NATS configuration
│   │
│   ├── postgres/
│   │   ├── statefulset.yaml        # PostgreSQL StatefulSet
│   │   ├── service.yaml            # PostgreSQL Service
│   │   ├── pvc.yaml                # Persistent Volume Claim
│   │   └── configmap.yaml          # PostgreSQL configuration
│   │
│   ├── ingestion/
│   │   ├── api-deployment.yaml     # Ingestion API Deployment
│   │   ├── api-service.yaml        # Ingestion API Service
│   │   ├── api-hpa.yaml            # API Horizontal Pod Autoscaler
│   │   ├── worker-deployment.yaml  # Worker Deployment
│   │   ├── worker-scaledobject.yaml # KEDA ScaledObject for workers
│   │   └── servicemonitor.yaml     # Prometheus monitoring (optional)
│   │
│   ├── dashboard/
│   │   ├── deployment.yaml         # Dashboard backend Deployment
│   │   ├── service.yaml            # Dashboard backend Service
│   │   ├── hpa.yaml                # Horizontal Pod Autoscaler
│   │   └── frontend-deployment.yaml # Dashboard frontend Deployment
│   │
│   └── ingress.yaml                # Ingress for external access
│
├── charts/
│   └── keda/                       # KEDA as subchart dependency
│
└── tests/
    └── test-connection.yaml        # Helm test pod
```

## Kubernetes Resources

### 1. Namespace
```yaml
apiVersion: v1
kind: Namespace
metadata:
  name: nomnom
  labels:
    app.kubernetes.io/name: nomnom
    app.kubernetes.io/instance: nomnom-tpch
```

### 2. PostgreSQL (StatefulSet)
- **Resource**: StatefulSet (requires persistence)
- **Replicas**: 1 (primary only, can extend to HA later)
- **Storage**: PersistentVolumeClaim (10Gi for dev, configurable)
- **Service**: ClusterIP headless service for stable DNS
- **Image**: postgres:17-alpine
- **Init**: Schema creation via initdb scripts
  - Entity tables (orders, line_items, etc.)
  - `message_status` table for tracking message lifecycle
    - Fields: message_id, entity_type, status, received_at, processed_at, retry_count, error_message
    - Status values: accepted, processing, completed, failed, dlq

### 3. NATS JetStream (StatefulSet)
- **Resource**: StatefulSet
- **Replicas**: 3 (cluster mode for HA)
- **Storage**: PersistentVolumeClaim (10Gi per replica)
- **Service**:
  - Headless service for cluster
  - ClusterIP for clients (4222)
  - Monitoring endpoint (8222)
- **Image**: nats:2.10-alpine
- **JetStream**: Enabled with file storage
- **Streams**:
  - Main stream: `MESSAGES` (subject: `messages.ingest.>`, 24h retention, 1GB max)
  - DLQ stream: `MESSAGESDLQ` (subject: `messages.dlq.>`, 7-day retention, 1GB max)
- **Health checks**:
  - Liveness: HTTP /healthz
  - Readiness: HTTP /healthz
- **Resources**:
  - Requests: 100m CPU, 256Mi memory
  - Limits: 500m CPU, 1Gi memory

### 4. Ingestion API (Deployment)
- **Resource**: Deployment
- **Replicas**: 3 (configurable, HPA-enabled)
- **Image**: Generated from nomnom (Alpine hardened)
- **Port**: 8080
- **Service**: ClusterIP with load balancing
- **Environment**:
  - NATS_URL: nats://nats-service:4222
  - DATABASE_URL: (for status queries)
- **Health checks**:
  - Liveness: GET /health
  - Readiness: GET /ready
- **Resources**:
  - Requests: 100m CPU, 128Mi memory
  - Limits: 500m CPU, 512Mi memory
- **HPA**: Scale 2-10 based on CPU (70% target)

### 5. Worker Pods (Deployment with KEDA)
- **Resource**: Deployment
- **Replicas**: Managed by KEDA (min 1, max 20)
- **Image**: Generated worker binary (Alpine hardened)
- **Environment**:
  - NATS_URL: nats://nats-service:4222
  - NATS_STREAM: MESSAGES
  - NATS_CONSUMER: workers
  - DATABASE_URL: postgresql://...
  - MAX_DELIVER: 3 (max delivery attempts before DLQ)
  - BATCH_SIZE: 10 (messages per batch)
  - POLL_INTERVAL_MS: 100 (polling interval)
- **KEDA Scaling**:
  - Trigger: NATS JetStream consumer lag
  - Threshold: 100 pending messages per pod
  - Scale to zero: Yes (when queue empty)
- **Resources**:
  - Requests: 100m CPU, 256Mi memory
  - Limits: 500m CPU, 1Gi memory

### 6. Dashboard Backend (Deployment)
- **Resource**: Deployment
- **Replicas**: 2 (configurable)
- **Image**: Generated from nomnom (our Alpine hardened image)
- **Port**: 3000
- **Service**: ClusterIP
- **Health checks**:
  - Liveness: GET /health
  - Readiness: GET /api/health
- **Resources**:
  - Requests: 100m CPU, 128Mi memory
  - Limits: 500m CPU, 512Mi memory

### 7. Dashboard Frontend (Deployment)
- **Resource**: Deployment
- **Replicas**: 2
- **Image**: Generated from nomnom (Vite React app)
- **Port**: 5173
- **Service**: ClusterIP

### 8. Ingress
- **Class**: nginx (for kind)
- **Routes**:
  - `/api/ingest/*` → ingestion-service:8080
  - `/api/*` → dashboard-backend-service:3000
  - `/*` → dashboard-frontend-service:5173
- **TLS**: Optional (cert-manager for prod)

### 9. ConfigMap
- Database connection parameters (non-sensitive)
- Application settings
- Logging levels
- Port configurations

### 10. Secret
- PostgreSQL credentials
- Database URL
- Any API keys (if needed)

## kind Cluster Setup

### 1. kind Configuration

```yaml
# kind-config.yaml
kind: Cluster
apiVersion: kind.x-k8s.io/v1alpha4
name: nomnom-dev
nodes:
  - role: control-plane
    kubeadmConfigPatches:
      - |
        kind: InitConfiguration
        nodeRegistration:
          kubeletExtraArgs:
            node-labels: "ingress-ready=true"
    extraPortMappings:
      - containerPort: 80
        hostPort: 8080
        protocol: TCP
      - containerPort: 443
        hostPort: 8443
        protocol: TCP
  - role: worker
  - role: worker
```

### 2. Required kind Setup Steps

```bash
# 1. Create kind cluster
kind create cluster --config kind-config.yaml

# 2. Install NGINX Ingress Controller
kubectl apply -f https://raw.githubusercontent.com/kubernetes/ingress-nginx/main/deploy/static/provider/kind/deploy.yaml

# 3. Wait for ingress controller
kubectl wait --namespace ingress-nginx \
  --for=condition=ready pod \
  --selector=app.kubernetes.io/component=controller \
  --timeout=90s

# 4. Install KEDA (K8s Event-Driven Autoscaler)
kubectl apply --server-side -f https://github.com/kedacore/keda/releases/download/v2.12.0/keda-2.12.0.yaml

# Wait for KEDA
kubectl wait --for=condition=ready pod -l app=keda-operator -n keda --timeout=90s
kubectl wait --for=condition=ready pod -l app=keda-metrics-apiserver -n keda --timeout=90s

# 5. Load local Docker images into kind
kind load docker-image tpch-ingestion-api-test:latest --name nomnom-dev
kind load docker-image tpch-ingestion-worker-test:latest --name nomnom-dev
kind load docker-image tpch-dashboard-backend-test:latest --name nomnom-dev

# 6. Install Helm chart
helm install nomnom ./nomnom-helm -f ./nomnom-helm/values-dev.yaml

# 7. Watch pods come up
kubectl get pods -n nomnom -w

# 8. Port forward for testing (if not using Ingress)
kubectl port-forward -n nomnom svc/ingestion-api-service 8080:8080
kubectl port-forward -n nomnom svc/dashboard-service 3000:3000
kubectl port-forward -n nomnom svc/nats-service 4222:4222  # NATS client port
kubectl port-forward -n nomnom svc/nats-service 8222:8222  # NATS monitoring
```

### 3. Testing with kind

```bash
# Test ingestion endpoint (returns 202 Accepted + message_id)
curl -X POST http://localhost:8080/api/ingest/message \
  -H "Content-Type: text/plain" \
  -d "O|123|456|F|123.45|2024-01-01|urgent|clerk1|1|comment"

# Response:
# {
#   "message_id": "550e8400-e29b-41d4-a716-446655440000",
#   "status": "accepted",
#   "timestamp": "2024-01-01T12:00:00Z"
# }

# Check message status
curl http://localhost:8080/api/ingest/status/550e8400-e29b-41d4-a716-446655440000

# Monitor NATS main stream
kubectl exec -it -n nomnom nats-0 -- nats stream info MESSAGES

# Monitor NATS DLQ stream
kubectl exec -it -n nomnom nats-0 -- nats stream info MESSAGESDLQ

# Monitor NATS consumer (workers)
kubectl exec -it -n nomnom nats-0 -- nats consumer info MESSAGES workers

# View messages in DLQ
kubectl exec -it -n nomnom nats-0 -- nats stream view MESSAGESDLQ

# Check KEDA scaling
kubectl get scaledobject -n nomnom
kubectl get hpa -n nomnom  # KEDA creates HPA

# Watch workers scale
kubectl get pods -n nomnom -l app=ingestion-worker -w

# Test dashboard
curl http://localhost:3000/api/stats

# Access dashboard UI
open http://localhost:5173

# View NATS monitoring UI (if port-forwarded)
open http://localhost:8222
```

## values.yaml Structure

```yaml
# Global settings
global:
  namespace: nomnom
  domain: nomnom.local

# NATS JetStream configuration
nats:
  enabled: true
  image: nats:2.10-alpine
  replicas: 3
  cluster:
    enabled: true
  jetstream:
    enabled: true
    storage:
      size: 10Gi
      storageClass: standard
    # Main stream configuration
    mainStream:
      maxAge: 24h  # 24 hours
      maxBytes: 1073741824  # 1GB
      subject: "messages.ingest.>"
    # DLQ stream configuration
    dlqStream:
      enabled: true
      maxAge: 168h  # 7 days (longer retention for DLQ)
      maxBytes: 1073741824  # 1GB
      subject: "messages.dlq.>"
  service:
    client:
      port: 4222
    cluster:
      port: 6222
    monitor:
      port: 8222
  resources:
    requests:
      cpu: 100m
      memory: 256Mi
    limits:
      cpu: 500m
      memory: 1Gi

# KEDA configuration
keda:
  enabled: true
  # Install KEDA operator
  operator:
    image: ghcr.io/kedacore/keda:2.12

# PostgreSQL configuration
postgresql:
  enabled: true
  image: postgres:17-alpine
  replicas: 1
  storage:
    size: 10Gi
    storageClass: standard
  resources:
    requests:
      cpu: 250m
      memory: 512Mi
    limits:
      cpu: 1000m
      memory: 2Gi
  config:
    maxConnections: 100
    sharedBuffers: 256MB
  initdb:
    scripts:
      # Message tracking table for status and DLQ routing
      01-message-status.sql: |
        CREATE TABLE IF NOT EXISTS message_status (
          message_id UUID PRIMARY KEY,
          entity_type VARCHAR(255) NOT NULL,
          status VARCHAR(50) NOT NULL,
          received_at TIMESTAMP NOT NULL,
          processed_at TIMESTAMP,
          retry_count INTEGER DEFAULT 0,
          error_message TEXT
        );
        CREATE INDEX idx_message_status_status ON message_status(status);
        CREATE INDEX idx_message_status_received_at ON message_status(received_at);

# Ingestion server configuration
ingestion:
  # API component (publishes to NATS)
  api:
    enabled: true
    image:
      repository: nomnom-ingestion-api
      tag: latest
      pullPolicy: IfNotPresent
    replicas: 3
    port: 8080
    service:
      type: ClusterIP
      port: 8080
    env:
      natsUrl: "nats://nats-service:4222"
      natsStream: "MESSAGES"
    resources:
      requests:
        cpu: 100m
        memory: 128Mi
      limits:
        cpu: 500m
        memory: 512Mi
    autoscaling:
      enabled: true
      minReplicas: 2
      maxReplicas: 10
      targetCPUUtilizationPercentage: 70
    healthCheck:
      liveness:
        path: /health
        initialDelaySeconds: 30
        periodSeconds: 10
      readiness:
        path: /ready
        initialDelaySeconds: 5
        periodSeconds: 5

  # Worker component (consumes from NATS)
  worker:
    enabled: true
    image:
      repository: nomnom-ingestion-worker
      tag: latest
      pullPolicy: IfNotPresent
    env:
      natsUrl: "nats://nats-service:4222"
      natsStream: "MESSAGES"
      natsConsumer: "workers"
      maxDeliver: "3"          # Max delivery attempts before DLQ
      batchSize: "10"          # Messages per batch
      pollIntervalMs: "100"    # Polling interval in milliseconds
    resources:
      requests:
        cpu: 100m
        memory: 256Mi
      limits:
        cpu: 500m
        memory: 1Gi
    # KEDA autoscaling (replaces HPA)
    keda:
      enabled: true
      minReplicas: 1
      maxReplicas: 20
      cooldownPeriod: 300
      pollingInterval: 15
      triggers:
        - type: nats-jetstream
          metadata:
            natsServerMonitoringEndpoint: "nats-service.nomnom.svc.cluster.local:8222"
            stream: "MESSAGES"      # Main stream only (not DLQ)
            consumer: "workers"      # Main consumer only (not DLQ)
            lagThreshold: "100"      # Scale up if > 100 msgs per pod

# Dashboard backend configuration
dashboard:
  backend:
    enabled: true
    image:
      repository: nomnom-dashboard-backend
      tag: latest
      pullPolicy: IfNotPresent
    replicas: 2
    port: 3000
    service:
      type: ClusterIP
      port: 3000
    resources:
      requests:
        cpu: 100m
        memory: 128Mi
      limits:
        cpu: 500m
        memory: 512Mi

  frontend:
    enabled: true
    image:
      repository: nomnom-dashboard-frontend
      tag: latest
      pullPolicy: IfNotPresent
    replicas: 2
    port: 5173
    service:
      type: ClusterIP
      port: 5173

# Ingress configuration
ingress:
  enabled: true
  className: nginx
  annotations:
    nginx.ingress.kubernetes.io/rewrite-target: /
  hosts:
    - host: nomnom.local
      paths:
        - path: /api/ingest
          pathType: Prefix
          service: ingestion-service
          port: 8080
        - path: /api
          pathType: Prefix
          service: dashboard-backend-service
          port: 3000
        - path: /
          pathType: Prefix
          service: dashboard-frontend-service
          port: 5173
  tls: []

# Database credentials (use Sealed Secrets in prod)
database:
  host: postgres-service
  port: 5432
  name: tpch
  username: postgres
  password: changeme  # Override in values-dev.yaml

# Monitoring (optional)
monitoring:
  enabled: false
  serviceMonitor:
    enabled: false
```

## values-dev.yaml (kind overrides)

```yaml
# Development overrides for kind cluster
global:
  domain: localhost

nats:
  replicas: 1  # Single node for dev
  jetstream:
    storage:
      size: 1Gi
      storageClass: standard

postgresql:
  storage:
    storageClass: standard  # kind default

ingestion:
  api:
    image:
      repository: localhost/tpch-ingestion-api-test
      tag: latest
      pullPolicy: Never  # Use local images
    replicas: 1
    autoscaling:
      enabled: false

  worker:
    image:
      repository: localhost/tpch-ingestion-worker-test
      tag: latest
      pullPolicy: Never
    keda:
      minReplicas: 1
      maxReplicas: 5

dashboard:
  backend:
    image:
      repository: localhost/tpch-dashboard-backend-test
      tag: latest
      pullPolicy: Never
    replicas: 1
  frontend:
    replicas: 1

database:
  password: devpassword

ingress:
  enabled: true
  hosts:
    - host: localhost
```

## Implementation Steps

### Phase 1: Write Static Helm Chart
1. **Create canonical Helm chart structure**
   ```bash
   mkdir -p nomnom-helm/templates/{nats,postgres,ingestion,dashboard}
   ```

2. **Write standard templates** (one-time, reusable):
   - Chart.yaml with version and metadata
   - values.yaml with all configurable parameters
   - Template files for each component (NATS, PostgreSQL, ingestion, dashboard)
   - Helper templates for common labels
   - NOTES.txt with deployment instructions
   - values-dev.yaml for kind/local development
   - values-prod.yaml template for production

3. **What varies per deployment**: Only the `values.yaml` overrides
   - Image names/tags (e.g., `tpch-ingestion-api` vs `ecommerce-ingestion-api`)
   - Database name (e.g., `tpch` vs `ecommerce`)
   - Resource limits (dev vs prod)
   - Replica counts (dev vs prod)

### Phase 2: kind Cluster Setup
1. Install kind: `brew install kind` (macOS) or download binary
2. Create cluster: `kind create cluster --config kind-config.yaml`
3. Install NGINX Ingress Controller
4. Verify cluster: `kubectl cluster-info`

### Phase 3: Image Preparation
1. Build Docker images with new Alpine hardened Dockerfiles
2. Tag images appropriately
3. Load images into kind cluster
4. Verify images available: `docker exec -it nomnom-dev-control-plane crictl images`

### Phase 4: Database Setup
1. Deploy PostgreSQL StatefulSet
2. Wait for pod ready
3. Run schema initialization
4. Verify database connectivity

### Phase 5: Application Deployment
1. Install Helm chart: `helm install nomnom ./nomnom-helm`
2. Watch pods come up: `kubectl get pods -n nomnom -w`
3. Check logs: `kubectl logs -n nomnom -l app=ingestion-server`
4. Verify services: `kubectl get svc -n nomnom`

### Phase 6: Testing
1. Port-forward or use Ingress
2. Test ingestion API
3. Test dashboard API
4. Test dashboard UI
5. Run Helm tests: `helm test nomnom`

### Phase 7: Monitoring & Observability (Optional)
1. Add Prometheus ServiceMonitor
2. Add Grafana dashboards
3. Configure logging aggregation
4. Set up alerts

## Testing Strategy

### 1. Helm Chart Testing
```bash
# Lint chart
helm lint nomnom-helm

# Dry run
helm install nomnom ./nomnom-helm --dry-run --debug

# Template rendering
helm template nomnom ./nomnom-helm -f values-dev.yaml

# Install
helm install nomnom ./nomnom-helm -f values-dev.yaml

# Test
helm test nomnom
```

### 2. Kubernetes Resource Testing
```bash
# Check all resources created
kubectl get all -n nomnom

# Check pod logs
kubectl logs -n nomnom -l app=ingestion-server --tail=50
kubectl logs -n nomnom -l app=dashboard-backend --tail=50

# Check pod describe for errors
kubectl describe pod -n nomnom <pod-name>

# Exec into pod for debugging
kubectl exec -it -n nomnom <pod-name> -- /bin/sh
```

### 3. Functional Testing
```bash
# Test ingestion
curl -X POST http://localhost:8080/api/ingest/message \
  -H "Content-Type: text/plain" \
  -d "OLI|1|789|1|100|123.45|0.05|0.07|N|O|2024-01-01|2024-01-02|2024-01-03|DELIVER|SHIP|test"

# Test dashboard stats
curl http://localhost:3000/api/stats | jq

# Test dashboard orders
curl http://localhost:3000/api/orders?limit=10 | jq

# Load testing (optional)
kubectl run -it --rm load-test --image=williamyeh/wrk --restart=Never -- \
  wrk -t4 -c100 -d30s http://ingestion-service.nomnom.svc.cluster.local:8080/api/ingest/message
```

### 4. Scaling Testing
```bash
# Manual scaling
kubectl scale deployment -n nomnom ingestion-server --replicas=5

# Watch HPA
kubectl get hpa -n nomnom -w

# Generate load to trigger autoscaling
# (use load testing tool)

# Verify pod distribution
kubectl get pods -n nomnom -o wide
```

### 5. Rolling Update Testing
```bash
# Update image tag
helm upgrade nomnom ./nomnom-helm \
  --set ingestion.image.tag=v2 \
  --reuse-values

# Watch rollout
kubectl rollout status deployment/ingestion-server -n nomnom

# Check rollout history
kubectl rollout history deployment/ingestion-server -n nomnom

# Rollback if needed
kubectl rollout undo deployment/ingestion-server -n nomnom
```

## Production Considerations

### Security
1. **Secrets Management**:
   - Use Sealed Secrets or External Secrets Operator
   - Never commit secrets to values.yaml
   - Rotate credentials regularly

2. **RBAC**:
   - Create ServiceAccounts for each component
   - Apply least-privilege principle
   - Use PodSecurityPolicies/PodSecurityStandards

3. **Network Policies**:
   - Restrict ingestion → postgres traffic
   - Restrict dashboard → postgres traffic
   - Deny all by default, allow explicitly

### High Availability
1. **Multi-zone deployment**: Use pod anti-affinity
2. **Database HA**: PostgreSQL with replication
3. **Ingress HA**: Multiple ingress controllers
4. **Resource quotas**: Prevent resource exhaustion

### Observability
1. **Metrics**: Prometheus + Grafana
2. **Logs**: ELK/Loki stack
3. **Tracing**: Jaeger/Tempo (optional)
4. **Alerts**: AlertManager

### Performance
1. **Resource limits**: Tune based on load testing
2. **HPA tuning**: Adjust thresholds and behavior
3. **Database connection pooling**: Configure in application
4. **Ingress optimization**: Enable caching, compression

## Helm Chart Structure (Static, Reusable)

The Helm chart is **written once** and reused across all deployments. Only `values.yaml` changes between deployments.

```
nomnom-helm/
├── Chart.yaml                      # Static metadata
├── values.yaml                     # Default configuration
├── values-dev.yaml                 # kind/local overrides
├── values-prod.yaml                # Production template
├── values-tpch.yaml                # TPCH example overrides
├── README.md                       # Usage instructions
├── templates/
│   ├── _helpers.tpl                # Common labels, names
│   ├── NOTES.txt                   # Post-install instructions
│   ├── namespace.yaml              # Namespace resource
│   ├── configmap.yaml              # App configuration
│   ├── secrets.yaml                # Database secrets
│   │
│   ├── nats/
│   │   ├── statefulset.yaml        # NATS JetStream
│   │   ├── service.yaml            # NATS services
│   │   └── configmap.yaml          # NATS config
│   │
│   ├── postgres/
│   │   ├── statefulset.yaml        # PostgreSQL
│   │   ├── service.yaml            # PostgreSQL service
│   │   └── pvc.yaml                # Storage claim
│   │
│   ├── ingestion/
│   │   ├── api-deployment.yaml     # Ingestion API
│   │   ├── api-service.yaml        # API service
│   │   ├── api-hpa.yaml            # API autoscaling
│   │   ├── worker-deployment.yaml  # Worker pods
│   │   └── worker-scaledobject.yaml # KEDA scaling
│   │
│   ├── dashboard/
│   │   ├── backend-deployment.yaml # Dashboard backend
│   │   ├── backend-service.yaml    # Backend service
│   │   ├── frontend-deployment.yaml # Dashboard frontend
│   │   └── frontend-service.yaml   # Frontend service
│   │
│   └── ingress.yaml                # Ingress routes
└── tests/
    └── test-connection.yaml        # Helm test
```

**Key principle**: Templates use Go template syntax with values from `values.yaml`. Different deployments just provide different values files.

**Example deployment workflow**:
```bash
# TPCH deployment
helm install tpch-nomnom ./nomnom-helm -f values-tpch.yaml

# E-commerce deployment
helm install ecommerce-nomnom ./nomnom-helm -f values-ecommerce.yaml

# Same chart, different configurations!
```

Where `values-tpch.yaml` contains:
```yaml
ingestion:
  api:
    image:
      repository: tpch-ingestion-api
      tag: v1.0.0
database:
  name: tpch
```

And `values-ecommerce.yaml` contains:
```yaml
ingestion:
  api:
    image:
      repository: ecommerce-ingestion-api
      tag: v1.0.0
database:
  name: ecommerce
```

## Success Criteria

- [ ] Helm chart written and validates successfully (`helm lint`)
- [ ] Chart templates render correctly (`helm template`)
- [ ] kind cluster creates without errors
- [ ] NATS and KEDA install successfully
- [ ] Images load into kind successfully
- [ ] PostgreSQL StatefulSet reaches Ready state
- [ ] NATS StatefulSet reaches Ready state with JetStream enabled
- [ ] Ingestion API Deployment scales to desired replicas
- [ ] Worker Deployment managed by KEDA
- [ ] Dashboard Deployment scales to desired replicas
- [ ] All pods pass health checks (liveness and readiness)
- [ ] Ingress routes traffic correctly
- [ ] Ingestion API accepts messages (202 Accepted)
- [ ] Workers process messages from NATS
- [ ] Message status tracking works (accepted → processing → completed)
- [ ] DLQ routing works (failed messages after MAX_DELIVER attempts)
- [ ] Dashboard API returns data
- [ ] Dashboard UI loads and displays data
- [ ] KEDA scaling triggers based on NATS queue depth
- [ ] Rolling updates complete successfully
- [ ] Helm tests pass

## Next Steps

1. **Write static Helm chart**:
   - Create `nomnom-helm/` directory structure
   - Write Kubernetes resource templates (NATS, PostgreSQL, ingestion, dashboard)
   - Create `values.yaml` with all configurable parameters
   - Create `values-dev.yaml` for kind testing
   - Write `_helpers.tpl` for common labels and names
   - Add `NOTES.txt` with post-install instructions

2. **Test with kind**:
   - Set up kind cluster with ingress and KEDA
   - Build and load Docker images
   - Deploy using `helm install nomnom ./nomnom-helm -f values-dev.yaml`
   - Validate all components (NATS, PostgreSQL, workers, API, dashboard)
   - Test end-to-end: message ingestion → NATS → workers → PostgreSQL → dashboard
   - Test DLQ routing with intentionally failing messages
   - Test KEDA autoscaling under load

3. **Production hardening**:
   - Add network policies (restrict inter-service communication)
   - Create `values-prod.yaml` template
   - Document secrets management approach (Sealed Secrets/External Secrets)
   - Add resource quotas and limits

4. **Documentation**:
   - Write deployment guide (README in nomnom-helm/)
   - Create troubleshooting guide
   - Document common operations (scaling, updates, rollbacks)
   - Add architecture diagrams showing K8s components
