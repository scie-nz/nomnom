# HL7 Nomnom Parser Helm Chart

**AUTO-GENERATED** by nomnom `generate-helm-chart` command. Do not edit manually.

## Overview

This Helm chart deploys the HL7 Nomnom Parser system, which includes:

- **Ingestion Server**: HTTP endpoint for receiving HL7 files
- **Worker**: Processes HL7 messages and persists to database
- **Benthos Pipelines**: ETL pipelines for streaming transient entities to data warehouse
- **NATS JetStream**: Message broker for reliable message delivery
- **MySQL/PostgreSQL**: Database for permanent entity storage

## Generated From

- **Entities**: {{ENTITY_COUNT}} entities ({{PERMANENT_COUNT}} permanent, {{TRANSIENT_COUNT}} transient)
- **Chart Version**: {{VERSION}}
- **Generated**: {{TIMESTAMP}}

## Installation

```bash
helm dependency update
helm install hl7-parser . -f values.yaml
```

## Configuration

See `values.yaml` for all configuration options.

### Key Configuration Sections

- `ingestionServer`: Ingestion server deployment settings
- `worker`: Worker deployment settings
- `benthos`: Benthos pipeline settings ({{TRANSIENT_COUNT}} pipelines)
- `mysql`: MySQL database settings
- `nats`: NATS JetStream settings

## Smart Defaults

This chart uses smart defaults derived from your entity definitions:

- Worker replicas: Based on permanent entity count
- Worker memory: Based on entity field complexity
- Benthos replicas: 3 for repeated entities, 2 for others
- NATS storage: Based on transient entity count

## Components

### Ingestion Server
- Deployment: `ingestion-server-deployment.yaml`
- Service: `ingestion-server-service.yaml`

### Worker
- Deployment: `worker-deployment.yaml`

### Benthos Pipelines
- Deployments: `benthos-deployment.yaml` ({{TRANSIENT_COUNT}} pipelines)
- ConfigMaps: `benthos-configmap.yaml`
- Service: `benthos-service.yaml` (metrics)
- Schema Init: `schema-init-job.yaml` (pre-install hook)

## Regenerating This Chart

To regenerate this chart after entity changes:

```bash
nomnom generate-helm-chart \
  --entities config/entities \
  --output helm-chart \
  --chart-version {{VERSION}}
```
