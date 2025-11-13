# Ingestion Reliability & Backpressure Strategy

## Problem Statement

**Key Questions:**
1. How does a client know if a message has been successfully ingested?
2. How do we handle backpressure when the database or ingestion service is overwhelmed?
3. What are the K8s-native patterns for reliable ingestion?

## Current Architecture Analysis

### Existing Flow (Synchronous)
```
Client → HTTP POST → Ingestion API → Parse → PostgreSQL → HTTP 200 OK → Client
```

**Characteristics:**
- Synchronous: Client waits for database write
- Strong consistency: Client knows immediately if write succeeded
- Simple: No intermediate queuing layer
- Limited scalability: Blocked on database performance
- No retry mechanism: Client must implement retries
- Backpressure: Implicit via HTTP timeouts and 5xx errors

**Problems:**
1. **Latency**: Client waits for full DB transaction (100-500ms+)
2. **No durability guarantee**: If pod dies mid-request, message lost
3. **Limited throughput**: Bounded by DB write speed
4. **Poor client experience**: Timeout errors when DB is slow
5. **No visibility**: Client can't check status later

## Solution Options

### Option 1: Enhanced Synchronous (Simple)

**Keep current architecture, add reliability features**

#### Implementation
```rust
// Return detailed response
#[derive(Serialize)]
struct IngestionResponse {
    message_id: String,      // UUID for tracking
    status: String,          // "accepted", "persisted", "failed"
    timestamp: DateTime<Utc>,
    errors: Vec<String>,
}

// Endpoint returns 200 only after DB commit
POST /api/ingest/message
→ 200 OK + message_id (persisted to DB)
→ 400 Bad Request (validation failed)
→ 503 Service Unavailable (DB unavailable, retry)
→ 429 Too Many Requests (backpressure applied)
```

#### Backpressure Mechanisms
1. **K8s-native HPA**: Scale pods based on CPU/memory
2. **Resource limits**: Prevent pod OOM
3. **Connection pooling**: Limit concurrent DB connections (r2d2)
4. **Rate limiting**: Nginx Ingress annotations
   ```yaml
   nginx.ingress.kubernetes.io/limit-rps: "100"
   nginx.ingress.kubernetes.io/limit-connections: "50"
   ```
5. **Circuit breaker**: Return 503 when DB latency > threshold
6. **Request timeout**: Nginx Ingress timeout settings

#### Status Checking
```rust
// New endpoint for status lookup
GET /api/ingest/status/{message_id}
→ 200 OK + status (lookup in DB by message_id)
```

#### Pros
- ✅ Simple to implement
- ✅ Strong consistency (client knows immediately)
- ✅ No additional infrastructure
- ✅ Works with existing Alpine containers

#### Cons
- ❌ Limited throughput (DB write speed)
- ❌ High latency for clients
- ❌ No durability if pod crashes
- ❌ Scaling limited by DB write capacity

#### When to Use
- **Low-medium throughput**: < 1000 msg/sec
- **Strong consistency required**: Client needs immediate confirmation
- **Simple deployment**: No ops team to manage queues
- **Cost-sensitive**: Minimal infrastructure

---

### Option 2: Asynchronous with Message Queue (Recommended for Scale)

**Decouple ingestion from persistence with durable queue**

#### Architecture
```
Client → HTTP POST → Ingestion API → Message Queue → Worker → PostgreSQL
                          ↓
                     202 Accepted + message_id

Client → HTTP GET → Status API → Check queue/DB → Return status
```

#### K8s-Native Implementation

##### Option 2A: NATS JetStream (Lightweight)
```yaml
# NATS is cloud-native, written in Go, K8s-friendly
# NATS Operator handles deployment

apiVersion: nats.io/v1alpha2
kind: NatsCluster
metadata:
  name: nomnom-nats
spec:
  size: 3
  version: "2.10.0"
  jetstream:
    enabled: true
    storage: 10Gi
```

**Flow:**
1. Client → POST message
2. API → Publish to NATS JetStream (durable)
3. API → Return 202 Accepted + message_id
4. Worker pods → Subscribe to stream
5. Worker → Parse + Write to DB
6. Worker → ACK to NATS
7. Client → GET /status/{message_id} → Check DB

**Backpressure:**
- Queue depth limits in NATS
- Consumer pace matching
- Worker pods scale with KEDA based on queue depth

##### Option 2B: Apache Kafka (Enterprise-grade)
```yaml
# Strimzi Operator for Kafka on K8s
apiVersion: kafka.strimzi.io/v1beta2
kind: Kafka
metadata:
  name: nomnom-kafka
spec:
  kafka:
    replicas: 3
    storage:
      type: persistent-claim
      size: 20Gi
  zookeeper:
    replicas: 3
    storage:
      type: persistent-claim
      size: 10Gi
```

**Flow:** Same as NATS but with Kafka semantics

**Backpressure:**
- Partition backpressure
- Consumer lag monitoring
- Worker scaling via KEDA

##### Option 2C: Redis Streams (Minimal)
```yaml
# Redis with persistence enabled
apiVersion: v1
kind: ConfigMap
metadata:
  name: redis-config
data:
  redis.conf: |
    appendonly yes
    appendfsync everysec
```

**Flow:** Similar but simpler, less durable than NATS/Kafka

#### KEDA Autoscaling (K8s-native)
```yaml
apiVersion: keda.sh/v1alpha1
kind: ScaledObject
metadata:
  name: ingestion-worker-scaler
spec:
  scaleTargetRef:
    name: ingestion-worker
  minReplicaCount: 1
  maxReplicaCount: 20
  triggers:
    - type: nats-jetstream
      metadata:
        stream: messages
        consumer: workers
        lagThreshold: "100"  # Scale up if >100 messages pending
```

#### Implementation Changes

**Ingestion API:**
```rust
// New endpoint returns immediately
#[post("/api/ingest/message")]
async fn ingest_message(
    body: String,
    nats: web::Data<NatsClient>,
) -> Result<HttpResponse, AppError> {
    let message_id = Uuid::new_v4();

    // Publish to NATS (durable)
    nats.publish("messages.ingest", &MessageEnvelope {
        id: message_id,
        body,
        timestamp: Utc::now(),
    }).await?;

    Ok(HttpResponse::Accepted().json(IngestionResponse {
        message_id: message_id.to_string(),
        status: "accepted".to_string(),
        message: "Message queued for processing",
    }))
}

// Status endpoint
#[get("/api/ingest/status/{message_id}")]
async fn check_status(
    message_id: web::Path<Uuid>,
    pool: web::Data<DbPool>,
) -> Result<HttpResponse, AppError> {
    let status = check_db_for_message(message_id, pool).await?;
    Ok(HttpResponse::Ok().json(status))
}
```

**Worker Pod (new component):**
```rust
// Separate worker that consumes from queue
#[tokio::main]
async fn main() -> Result<()> {
    let nats = connect_nats().await?;
    let db_pool = create_db_pool()?;

    // Subscribe to stream
    let mut subscriber = nats
        .subscribe("messages.ingest")
        .await?;

    while let Some(msg) = subscriber.next().await {
        match process_message(&msg, &db_pool).await {
            Ok(_) => msg.ack().await?,  // ACK on success
            Err(e) => {
                log::error!("Failed to process: {}", e);
                msg.nak().await?;  // NAK for retry
            }
        }
    }
}

async fn process_message(msg: &Message, pool: &DbPool) -> Result<()> {
    let envelope: MessageEnvelope = serde_json::from_slice(&msg.data)?;

    // Parse message
    let parsed = parse_message(&envelope.body)?;

    // Write to DB
    insert_to_db(parsed, pool).await?;

    Ok(())
}
```

#### Helm Chart Changes
```yaml
# New worker deployment
worker:
  enabled: true
  replicas: 3
  image:
    repository: nomnom-worker
    tag: latest
  env:
    - name: NATS_URL
      value: "nats://nomnom-nats:4222"
    - name: DATABASE_URL
      valueFrom:
        secretKeyRef:
          name: db-credentials
          key: url

# NATS subchart
nats:
  enabled: true
  cluster:
    enabled: true
    replicas: 3
  jetstream:
    enabled: true
    storage: 10Gi

# KEDA autoscaling
keda:
  enabled: true
  scaledObjects:
    - name: worker-scaler
      target: worker
      minReplicas: 1
      maxReplicas: 20
```

#### Pros
- ✅ **High throughput**: Decopled ingestion from DB writes
- ✅ **Durability**: Messages persisted in queue
- ✅ **Scalability**: Workers scale independently
- ✅ **Backpressure**: Natural queue-based backpressure
- ✅ **Retries**: Automatic retry on failure
- ✅ **Visibility**: Client can check status anytime
- ✅ **K8s-native**: KEDA, operators, standard patterns

#### Cons
- ❌ **Complexity**: Additional components (queue, workers)
- ❌ **Eventual consistency**: Client doesn't know immediately
- ❌ **Operational overhead**: Queue management, monitoring
- ❌ **Cost**: Additional infrastructure

#### When to Use
- **High throughput**: > 1000 msg/sec
- **Bursty traffic**: Need to buffer spikes
- **Reliability critical**: Can't lose messages
- **Scale to zero**: KEDA can scale workers to 0 when idle
- **Enterprise deployment**: Ops team available

---

### Option 3: Hybrid Approach (Best of Both Worlds)

**Synchronous by default, queue for overflow**

#### Architecture
```
Normal load:  Client → API → PostgreSQL → 200 OK
High load:    Client → API → Queue → 202 Accepted
              Worker → Queue → PostgreSQL
```

#### Implementation
```rust
#[post("/api/ingest/message")]
async fn ingest_message(
    body: String,
    pool: web::Data<DbPool>,
    nats: web::Data<Option<NatsClient>>,
    metrics: web::Data<Metrics>,
) -> Result<HttpResponse, AppError> {
    let message_id = Uuid::new_v4();

    // Check current load
    let db_latency = metrics.db_write_latency.avg();
    let connection_util = pool.state().connections as f32 / pool.max_size() as f32;

    // Use queue if under pressure
    if db_latency > 500.0 || connection_util > 0.8 {
        if let Some(nats_client) = nats.as_ref() {
            // Queue mode
            nats_client.publish("messages.ingest", &envelope).await?;
            return Ok(HttpResponse::Accepted().json(IngestionResponse {
                message_id: message_id.to_string(),
                status: "queued".to_string(),
            }));
        }
    }

    // Synchronous mode
    match write_to_db_with_timeout(&body, &pool, Duration::from_secs(1)).await {
        Ok(_) => Ok(HttpResponse::Ok().json(IngestionResponse {
            message_id: message_id.to_string(),
            status: "persisted".to_string(),
        })),
        Err(_) if nats.is_some() => {
            // Fallback to queue
            nats.as_ref().unwrap().publish("messages.ingest", &envelope).await?;
            Ok(HttpResponse::Accepted().json(IngestionResponse {
                message_id: message_id.to_string(),
                status: "queued".to_string(),
            }))
        },
        Err(e) => Err(e),
    }
}
```

#### Pros
- ✅ Low latency when possible
- ✅ Graceful degradation under load
- ✅ Queue is optional (can disable for simple deployments)

#### Cons
- ❌ Complexity of both modes
- ❌ Inconsistent response times

---

## Kubernetes-Native Backpressure Mechanisms

### 1. Horizontal Pod Autoscaler (HPA)
```yaml
apiVersion: autoscaling/v2
kind: HorizontalPodAutoscaler
metadata:
  name: ingestion-hpa
spec:
  scaleTargetRef:
    apiVersion: apps/v1
    kind: Deployment
    name: ingestion-server
  minReplicas: 3
  maxReplicas: 20
  metrics:
    - type: Resource
      resource:
        name: cpu
        target:
          type: Utilization
          averageUtilization: 70
    - type: Resource
      resource:
        name: memory
        target:
          type: Utilization
          averageUtilization: 80
  behavior:
    scaleUp:
      stabilizationWindowSeconds: 30
      policies:
        - type: Percent
          value: 100
          periodSeconds: 30
    scaleDown:
      stabilizationWindowSeconds: 300
      policies:
        - type: Pods
          value: 1
          periodSeconds: 60
```

### 2. KEDA (Event-driven autoscaling)
```yaml
apiVersion: keda.sh/v1alpha1
kind: ScaledObject
metadata:
  name: worker-scaler
spec:
  scaleTargetRef:
    name: ingestion-worker
  minReplicaCount: 1
  maxReplicaCount: 50
  cooldownPeriod: 300
  triggers:
    # Scale based on queue depth
    - type: nats-jetstream
      metadata:
        natsServerMonitoringEndpoint: "nomnom-nats:8222"
        stream: "messages"
        consumer: "workers"
        lagThreshold: "100"

    # Scale based on pending messages
    - type: prometheus
      metadata:
        serverAddress: http://prometheus:9090
        metricName: nats_consumer_num_pending
        threshold: "1000"
        query: sum(nats_consumer_num_pending{stream="messages"})
```

### 3. Ingress Rate Limiting
```yaml
apiVersion: networking.k8s.io/v1
kind: Ingress
metadata:
  name: ingestion-ingress
  annotations:
    # Limit requests per second per IP
    nginx.ingress.kubernetes.io/limit-rps: "100"

    # Limit concurrent connections per IP
    nginx.ingress.kubernetes.io/limit-connections: "50"

    # Return 429 when limit exceeded
    nginx.ingress.kubernetes.io/limit-req-status-code: "429"

    # Burst allowance
    nginx.ingress.kubernetes.io/limit-burst-multiplier: "5"
```

### 4. Pod Disruption Budget (Prevent over-disruption)
```yaml
apiVersion: policy/v1
kind: PodDisruptionBudget
metadata:
  name: ingestion-pdb
spec:
  minAvailable: 2
  selector:
    matchLabels:
      app: ingestion-server
```

### 5. Resource Quotas (Namespace-level limits)
```yaml
apiVersion: v1
kind: ResourceQuota
metadata:
  name: nomnom-quota
  namespace: nomnom
spec:
  hard:
    requests.cpu: "10"
    requests.memory: 20Gi
    limits.cpu: "20"
    limits.memory: 40Gi
    persistentvolumeclaims: "10"
```

### 6. Network Policies (Prevent DDoS)
```yaml
apiVersion: networking.k8s.io/v1
kind: NetworkPolicy
metadata:
  name: ingestion-network-policy
spec:
  podSelector:
    matchLabels:
      app: ingestion-server
  policyTypes:
    - Ingress
  ingress:
    - from:
        - namespaceSelector:
            matchLabels:
              name: ingress-nginx
      ports:
        - protocol: TCP
          port: 8080
```

### 7. Circuit Breaker (Istio Service Mesh)
```yaml
apiVersion: networking.istio.io/v1alpha3
kind: DestinationRule
metadata:
  name: ingestion-circuit-breaker
spec:
  host: ingestion-service
  trafficPolicy:
    connectionPool:
      tcp:
        maxConnections: 100
      http:
        http1MaxPendingRequests: 50
        http2MaxRequests: 100
        maxRequestsPerConnection: 2
    outlierDetection:
      consecutiveErrors: 5
      interval: 30s
      baseEjectionTime: 30s
      maxEjectionPercent: 50
```

## Recommendation Matrix

| Scenario | Solution | Rationale |
|----------|----------|-----------|
| < 100 msg/sec, simple deployment | **Option 1** (Enhanced Sync) | Minimal complexity, sufficient for load |
| 100-1000 msg/sec, some ops | **Option 3** (Hybrid) | Best of both worlds, graceful degradation |
| > 1000 msg/sec, enterprise | **Option 2A** (NATS) | Cloud-native, K8s-friendly, proven |
| > 10,000 msg/sec, large scale | **Option 2B** (Kafka) | Battle-tested at scale, rich ecosystem |
| Minimal resources, experimental | **Option 2C** (Redis) | Lightweight, easy to start |

## Implementation Roadmap

### Phase 1: Enhanced Synchronous (Week 1)
1. Add message_id to responses
2. Implement status endpoint
3. Add Nginx rate limiting
4. Configure HPA properly
5. Add circuit breaker logic
6. Monitor and tune

### Phase 2: Queue Infrastructure (Week 2-3)
1. Deploy NATS operator to kind
2. Create JetStream streams
3. Build worker component
4. Implement KEDA scaling
5. Add monitoring/alerting
6. Load test and tune

### Phase 3: Hybrid Mode (Week 4)
1. Implement adaptive routing
2. Add metrics for decision making
3. Configure fallback logic
4. Integration testing
5. Performance benchmarking

### Phase 4: Production Hardening (Week 5-6)
1. Add Prometheus metrics
2. Create Grafana dashboards
3. Set up alerting rules
4. Document runbooks
5. Chaos testing
6. Disaster recovery planning

## Monitoring & Observability

### Key Metrics to Track

```yaml
# Prometheus metrics to expose
ingestion_messages_total          # Counter: total messages received
ingestion_messages_queued         # Counter: messages sent to queue
ingestion_messages_persisted      # Counter: messages written to DB
ingestion_messages_failed         # Counter: failures
ingestion_latency_seconds         # Histogram: end-to-end latency
ingestion_db_latency_seconds      # Histogram: DB write latency
ingestion_queue_depth             # Gauge: current queue depth
ingestion_db_connections_active   # Gauge: active DB connections
ingestion_db_connections_idle     # Gauge: idle DB connections
```

### Alerting Rules

```yaml
# Alert if queue depth is growing
- alert: IngestionQueueBacklog
  expr: nats_consumer_num_pending > 10000
  for: 5m
  annotations:
    summary: "Ingestion queue has {{ $value }} pending messages"

# Alert if error rate is high
- alert: IngestionHighErrorRate
  expr: rate(ingestion_messages_failed[5m]) / rate(ingestion_messages_total[5m]) > 0.05
  for: 5m
  annotations:
    summary: "Ingestion error rate is {{ $value }}%"

# Alert if DB latency is high
- alert: DatabaseSlowWrites
  expr: histogram_quantile(0.95, ingestion_db_latency_seconds) > 1.0
  for: 5m
  annotations:
    summary: "95th percentile DB write latency is {{ $value }}s"
```

## Testing Strategy

### Load Testing (k6)
```javascript
import http from 'k6/http';
import { check } from 'k6';

export const options = {
  stages: [
    { duration: '2m', target: 100 },   // Ramp up
    { duration: '5m', target: 100 },   // Steady state
    { duration: '2m', target: 500 },   // Spike
    { duration: '5m', target: 500 },   // High load
    { duration: '2m', target: 0 },     // Ramp down
  ],
  thresholds: {
    http_req_duration: ['p(95)<500'],  // 95% under 500ms
    http_req_failed: ['rate<0.01'],     // Less than 1% errors
  },
};

export default function() {
  const payload = 'O|123|456|F|123.45|2024-01-01|urgent|clerk1|1|comment';
  const res = http.post('http://ingestion-service:8080/api/ingest/message', payload);

  check(res, {
    'status is 200 or 202': (r) => r.status === 200 || r.status === 202,
    'has message_id': (r) => r.json('message_id') !== undefined,
  });
}
```

## Summary

**Recommended Approach:**

1. **Start with Option 1** (Enhanced Synchronous):
   - Simple, works for most cases
   - Add proper monitoring and HPA
   - Implement rate limiting at Ingress
   - Add circuit breaker logic

2. **Evolve to Option 2A** (NATS) when:
   - Throughput > 1000 msg/sec
   - Need better durability guarantees
   - Want to scale workers independently
   - Have ops capacity for queue management

3. **Use K8s-native tools:**
   - HPA for pod scaling
   - KEDA for event-driven scaling
   - Ingress rate limiting
   - Network policies
   - PodDisruptionBudgets

The beauty of this approach is you can **start simple and add complexity only when needed**, while keeping everything K8s-native and cloud-portable.
