# nomnom Helm Chart

A production-ready Helm chart for deploying the nomnom data ingestion pipeline with NATS JetStream, PostgreSQL, and real-time dashboards on Kubernetes.

## Architecture

```
┌─────────────┐
│   Ingress   │
└──────┬──────┘
       │
       ├─────────────┐──────────────┐
       │             │              │
┌──────▼─────┐ ┌────▼─────┐ ┌──────▼──────┐
│ Ingestion  │ │Dashboard │ │  Dashboard  │
│    API     │ │ Backend  │ │  Frontend   │
└──────┬─────┘ └────┬─────┘ └─────────────┘
       │            │
       │            │
   ┌───▼────────────▼───┐
   │                    │
┌──▼──────┐      ┌──────▼────┐
│  NATS   │      │PostgreSQL │
│JetStream│      │           │
└──┬──────┘      └───────────┘
   │
   │ Messages
   │
┌──▼─────┐
│Workers │ (KEDA autoscaling)
└────────┘
```

## Features

- **NATS JetStream**: Reliable message streaming with Dead Letter Queue (DLQ)
- **PostgreSQL**: Message status tracking and data persistence
- **Ingestion API**: REST API for data ingestion with HPA autoscaling
- **Workers**: Message processing with KEDA-based queue depth autoscaling
- **Dashboard**: Real-time monitoring with backend API and frontend
- **Ingress**: Unified routing for all services
- **Production-ready**: Health checks, resource limits, persistent storage

## Prerequisites

- Kubernetes 1.19+
- Helm 3.0+
- KEDA (optional, for worker autoscaling)
- Ingress controller (optional, for ingress)

## Installation

### Quick Start

```bash
# Install with default values
helm install nomnom . -n nomnom --create-namespace

# View deployment status
helm status nomnom -n nomnom
```

### Development Installation (kind/minikube)

```bash
# Install with development values
helm install nomnom . -f values-dev.yaml -n nomnom-dev --create-namespace
```

### Production Installation

```bash
# Create production values file
cat > values-prod.yaml <<EOF
global:
  namespace: nomnom-prod
  domain: nomnom.example.com

ingestion:
  api:
    replicas: 5
    autoscaling:
      enabled: true
      minReplicas: 3
      maxReplicas: 20

  worker:
    keda:
      enabled: true
      minReplicas: 2
      maxReplicas: 50

database:
  password: <CHANGE-ME>  # Use sealed secrets in production!

ingress:
  enabled: true
  tls:
    - secretName: nomnom-tls
      hosts:
        - nomnom.example.com
EOF

# Install with production values
helm install nomnom . -f values-prod.yaml -n nomnom-prod --create-namespace
```

## Configuration

### Key Configuration Values

| Parameter | Description | Default |
|-----------|-------------|---------|
| `global.namespace` | Kubernetes namespace | `nomnom` |
| `global.domain` | Domain for ingress | `nomnom.local` |
| `nats.replicas` | NATS cluster size | `3` |
| `postgresql.storage.size` | PostgreSQL storage size | `10Gi` |
| `ingestion.api.replicas` | API replica count | `3` |
| `ingestion.worker.keda.enabled` | Enable KEDA autoscaling | `true` |
| `dashboard.backend.enabled` | Enable dashboard backend | `true` |
| `dashboard.frontend.enabled` | Enable dashboard frontend | `true` |
| `ingress.enabled` | Enable ingress | `true` |

### Complete Configuration

See [values.yaml](values.yaml) for all available configuration options.

## Testing on kind

A comprehensive test script is provided to deploy and test the chart on a local kind cluster:

```bash
# Run the test script from the repository root
cd ..
./test-helm-kind.sh

# The script will:
# - Create a kind cluster
# - Set up a local Docker registry
# - Build and push all Docker images
# - Install the Helm chart
# - Run basic health checks

# Clean up when done
./test-helm-kind.sh cleanup
```

## Accessing Services

### Port Forwarding (Development)

```bash
# Ingestion API
kubectl port-forward -n nomnom svc/nomnom-ingestion-api 8080:8080

# Dashboard Frontend
kubectl port-forward -n nomnom svc/nomnom-dashboard-frontend 3001:5173

# Dashboard Backend
kubectl port-forward -n nomnom svc/nomnom-dashboard-backend 3000:3000

# NATS Monitoring
kubectl port-forward -n nomnom svc/nomnom-nats 8222:8222

# PostgreSQL
kubectl port-forward -n nomnom svc/nomnom-postgres 5432:5432
```

### NodePort (kind)

When using `values-dev.yaml` with kind:
- Ingestion API: http://localhost:8080
- Dashboard Frontend: http://localhost:8081
- Dashboard Backend: http://localhost:3000

### Ingress (Production)

When ingress is enabled, all services are available via the configured domain:
- Ingestion API: http://nomnom.example.com/api/ingest
- Dashboard Backend: http://nomnom.example.com/api
- Dashboard Frontend: http://nomnom.example.com/

## Usage Examples

### Ingest Data

```bash
# Send a message to the ingestion API
curl -X POST http://localhost:8080/ingest/customer \
  -H "Content-Type: application/json" \
  -d '{
    "id": 1,
    "name": "John Doe",
    "email": "john@example.com"
  }'
```

### View Logs

```bash
# View all pods
kubectl get pods -n nomnom

# View API logs
kubectl logs -l app.kubernetes.io/component=ingestion-api -n nomnom

# View worker logs
kubectl logs -l app.kubernetes.io/component=worker -n nomnom

# View NATS logs
kubectl logs -l app.kubernetes.io/component=nats -n nomnom
```

### Monitor Message Status

```bash
# Connect to PostgreSQL
kubectl exec -it nomnom-postgres-0 -n nomnom -- psql -U postgres -d nomnom

# Query message status
SELECT * FROM message_status ORDER BY received_at DESC LIMIT 10;
```

### Access NATS Monitoring

```bash
# Port-forward NATS monitoring
kubectl port-forward -n nomnom svc/nomnom-nats 8222:8222

# View in browser
open http://localhost:8222
```

## Troubleshooting

### Pods Not Starting

```bash
# Check pod status
kubectl get pods -n nomnom

# Describe pod for events
kubectl describe pod <pod-name> -n nomnom

# Check logs
kubectl logs <pod-name> -n nomnom
```

### Database Connection Issues

```bash
# Check database secret
kubectl get secret nomnom-db-credentials -n nomnom -o yaml

# Test database connection
kubectl exec -it nomnom-postgres-0 -n nomnom -- psql -U postgres -d nomnom -c "SELECT 1"
```

### NATS Connection Issues

```bash
# Check NATS pods
kubectl get pods -l app.kubernetes.io/component=nats -n nomnom

# Check NATS logs
kubectl logs -l app.kubernetes.io/component=nats -n nomnom

# Port-forward and test
kubectl port-forward svc/nomnom-nats -n nomnom 4222:4222
nats context add nomnom --server localhost:4222
nats stream ls
```

### Worker Not Scaling

```bash
# Check KEDA installation
kubectl get scaledobjects -n nomnom

# Check KEDA operator logs
kubectl logs -n keda -l app=keda-operator

# Check worker ScaledObject
kubectl describe scaledobject nomnom-worker -n nomnom
```

## Upgrading

```bash
# Update chart values
helm upgrade nomnom . -f values-prod.yaml -n nomnom-prod

# Rollback if needed
helm rollback nomnom -n nomnom-prod
```

## Uninstallation

```bash
# Uninstall release
helm uninstall nomnom -n nomnom

# Delete namespace (optional)
kubectl delete namespace nomnom
```

## Architecture Details

### NATS JetStream Streams

- **MESSAGES**: Main ingestion stream
  - Retention: 24 hours
  - Max size: 1GB
  - Subject: `messages.ingest.>`

- **MESSAGES_DLQ**: Dead letter queue
  - Retention: 7 days
  - Max size: 1GB
  - Subject: `messages.dlq.>`

### Message Processing Flow

1. Client sends message to Ingestion API
2. API publishes to NATS JetStream (MESSAGES stream)
3. Workers consume from MESSAGES stream
4. On success: Update message_status table to "completed"
5. On failure: Retry up to MAX_DELIVER times
6. After max retries: Move to DLQ stream

### Autoscaling

- **Ingestion API**: HPA based on CPU utilization (70% target)
- **Workers**: KEDA based on NATS queue depth (100 messages per pod)

## Contributing

See the main repository README for contribution guidelines.

## License

See the main repository for license information.
