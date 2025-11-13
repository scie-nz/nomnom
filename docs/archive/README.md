# Documentation Archive

This directory contains historical documentation from the development of nomnom. These documents are kept for reference but represent completed work or superseded designs.

## Structure

### `/implementations/`
Documentation of completed implementation work, bug fixes, and migrations:
- `axum-ingestion.md` - Axum-based HTTP ingestion server implementation
- `nats-transition.md` - NATS JetStream architecture migration
- `derived-entities.md` - Support for derived/nested entities
- `transforms-codegen.md` - Transform function code generation
- And other implementation notes...

### `/designs/`
Initial design documents that were explored or superseded:
- `fastapi-ingestion.md` - FastAPI design (not chosen, went with Rust)
- `rust-ingestion-api.md` - Early Rust ingestion design
- `axum-dashboard.md` - Dashboard design exploration

### `/plans/`
Planning documents for features that have since been implemented:
- `nats-jetstream.md` - NATS JetStream integration planning
- `kubernetes-deployment.md` - K8s deployment strategy
- `derived-entities.md` - Derived entity feature planning
- And other completed plans...

## Current Documentation

For up-to-date documentation, see:
- `/README.md` - Main project documentation
- `/docs/README.md` - Library documentation
- `/docs/architecture/` - Current architecture documentation
- `/docs/transform_yaml_schema.md` - YAML configuration reference
- `/TESTING_GUIDE.md` - Testing instructions
