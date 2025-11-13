# Helm Testing Guide

## Fast Development Workflow (Recommended)

For rapid development and testing, use the **dev mode** which builds debug binaries locally and creates lightweight Docker images:

```bash
# 1. Build debug binaries and dev images (fast - ~30 seconds)
./build-dev.sh

# 2. Deploy to kind cluster with dev images
./test-helm-kind.sh --dev
```

### Why Dev Mode is Faster

- **Local builds**: Compiles debug binaries on your host (uses your local Rust cache)
- **Debug mode**: No optimizations, compiles 10x faster than release builds
- **Lightweight images**: Uses Debian Slim with pre-built binaries
- **No Alpine issues**: Avoids musl/static linking complexity

## Production-Like Testing

For testing optimized release builds (slower but closer to production):

```bash
./test-helm-kind.sh
```

This builds optimized release binaries inside Alpine containers. Takes 15-30 minutes depending on your machine.

## Quick Reference

```bash
# Fast iteration cycle (recommended for development)
./build-dev.sh && ./test-helm-kind.sh --dev

# Access services
curl http://localhost:8080/health                    # Ingestion API
open http://localhost:8081                           # Dashboard Frontend
curl http://localhost:3000/api/health                # Dashboard Backend

# View logs
kubectl logs -l app.kubernetes.io/instance=nomnom -n nomnom-dev

# Clean up
./test-helm-kind.sh cleanup
```

## Development Files

- `Dockerfile.dev` - Fast dev image for ingestion API
- `Dockerfile.worker.dev` - Fast dev image for worker
- `Dockerfile.dashboard.dev` - Fast dev image for dashboard backend
- `build-dev.sh` - Builds debug binaries and creates dev images
- `test-helm-kind.sh` - Deploys to kind cluster
  - `--dev` flag: Use dev images (fast)
  - No flag: Build production images (slow)

## Troubleshooting

### Build fails with missing library

If you see linking errors like `cannot find -lpq`:

```bash
# macOS
brew install postgresql libpq sqlite

# Ubuntu/Debian
sudo apt-get install libpq-dev libsqlite3-dev
```

### Binaries not found

If `build-dev.sh` complains about missing binaries:

```bash
# Build required binaries
cargo build --bin nats-api --bin nomnom
```

**Note**: The `worker` binary is not yet implemented. The build script will automatically use the `nomnom` binary as a placeholder for the worker container.

### Images not deploying

If kind can't pull images, check the registry:

```bash
# Verify images are in the registry
curl http://localhost:5001/v2/_catalog

# Should show: nomnom-ingestion-api, nomnom-worker, etc.
```

## Workflow Tips

1. **First time setup**: Run `./test-helm-kind.sh --dev` to create cluster and deploy
2. **Code changes**: Run `./build-dev.sh` to rebuild images
3. **Redeploy**: Run `helm upgrade nomnom nomnom-helm -n nomnom-dev` or restart pods
4. **Quick rebuild**: `./build-dev.sh && kubectl rollout restart deployment -n nomnom-dev`

## Performance Comparison

| Method | First Build | Rebuild | Image Size |
|--------|-------------|---------|------------|
| Dev Mode | ~30s | ~15s | ~120MB |
| Production | ~20min | ~15min | ~20MB |

Use dev mode for development, production mode for CI/CD and release testing.
