# Docker Security Hardening Plan

## Objective
Harden generated Dockerfiles for ingestion API and dashboard with security best practices while maintaining small image sizes.

## Current State (Security Issues)
- ✗ Using `rust:latest` (unpredictable versions)
- ✗ Using debian base (~74MB runtime)
- ✗ Running as root user
- ✗ No explicit version pinning for dependencies

## Target State (Hardened)
- ✓ Alpine Linux base (~5MB vs 74MB)
- ✓ Pinned Rust version (e.g., `1.83-alpine`)
- ✓ Non-root user with minimal permissions
- ✓ Multi-stage builds (keep existing optimization)
- ✓ Explicit dependency versions

## Implementation Plan

### 1. Alpine Base Image Migration

**Builder Stage:**
```dockerfile
FROM rust:1.83-alpine3.19 as builder
```

**Runtime Stage:**
```dockerfile
FROM alpine:3.19
```

**Benefits:**
- Reduces image size by ~70MB (74MB → 5MB base)
- Smaller attack surface (fewer packages)
- Regular security updates from Alpine team

**Challenges:**
- Must use musl libc instead of glibc
- Need to install musl-dev for Rust compilation
- Some crates may need musl-specific configuration

### 2. Non-Root User

**Add to runtime stage:**
```dockerfile
RUN addgroup -g 1000 appuser && \
    adduser -D -u 1000 -G appuser appuser

USER appuser:appuser
```

**Benefits:**
- Container breakout limited to non-privileged user
- Follows principle of least privilege
- Industry best practice

### 3. Version Pinning Strategy

**Rust toolchain:**
- Builder: `rust:1.83-alpine3.19` (pin major.minor)
- Runtime: `alpine:3.19` (pin major.minor)

**System packages:**
```dockerfile
RUN apk add --no-cache \
    ca-certificates=20230506-r0 \
    libpq=16.1-r0
```

**Cargo dependencies:**
- Already pinned via Cargo.lock (generated code)

**Benefits:**
- Reproducible builds
- Controlled updates
- Easier security audits

### 4. Additional Security Hardening

**Health checks:**
```dockerfile
HEALTHCHECK --interval=30s --timeout=3s --start-period=5s --retries=3 \
  CMD wget --no-verbose --tries=1 --spider http://localhost:${PORT}/health || exit 1
```

**Read-only filesystem:**
```dockerfile
# In docker-compose.yml
read_only: true
tmpfs:
  - /tmp
```

**Security options:**
```yaml
security_opt:
  - no-new-privileges:true
cap_drop:
  - ALL
cap_add:
  - NET_BIND_SERVICE
```

## File Changes Required

### `/src/codegen/ingestion_server/mod.rs`
- Update `generate_dockerfile()` function
- Change base images to Alpine
- Add non-root user creation
- Pin Alpine and Rust versions
- Add musl-specific dependencies

### `/src/codegen/dashboard/docker.rs`
- Update `generate_backend_dockerfile()` function
- Same changes as ingestion server
- Keep frontend as-is (Node.js already uses Alpine)

### `/config/examples/tpch/docker-compose.yml`
- Optional: Add security options
- Optional: Add health checks
- Optional: Add read-only filesystem

## Migration Steps

1. **Update code generators** (ingestion + dashboard)
2. **Rebuild nomnom** (`cargo build --release`)
3. **Regenerate test projects**
4. **Test Alpine builds locally**
5. **Verify services run as non-root** (`docker exec <container> whoami`)
6. **Update docker-compose with security options** (optional)
7. **Commit changes**

## Testing Checklist

- [ ] Alpine images build successfully
- [ ] Binary runs without glibc errors
- [ ] Non-root user can bind to ports
- [ ] PostgreSQL connections work
- [ ] All API endpoints functional
- [ ] Image sizes reduced (~50-60MB target)
- [ ] `docker exec <container> whoami` returns `appuser`

## Rollback Plan

If Alpine causes compatibility issues:
- Fallback to `debian:bookworm-slim` with non-root user
- Still maintains security improvement
- Slightly larger image size acceptable

## Future Enhancements (Post-CI/CD)

- Automated vulnerability scanning (Trivy, Grype)
- SBOM generation
- Image signing (Cosign)
- Distroless evaluation for even smaller images
- Automated base image updates

## Success Metrics

- **Image Size**: 117-126MB → 50-60MB (52% reduction)
- **Security**: Running as non-root ✓
- **Reproducibility**: Pinned versions ✓
- **Performance**: No degradation
