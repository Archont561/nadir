---
type: Deployment Guide
title: Docker Strategy
description: "Inverted container model, multi-stage Dockerfile, lean production variant"
purpose: Inverted container model, multi-stage Dockerfile, lean production variant
last_updated: 2025-02-23
status: stable
related:
  - hosting-comparison.md
  - pixi-setup.md
  - ../../backlog/docs/decisions/004-inverted-container-no-dind.md
  - ../../backlog/docs/decisions/005-worker-topology-per-step.md
---

# Docker Strategy

## TL;DR

Nadir uses the **inverted container model**: the deployed container IS the
engine environment (COLMAP + OpenMVS + GDAL + PROJ installed via Pixi), and
the Nadir Rust binary runs as PID 1 spawning engines as local subprocesses.
No Docker-in-Docker. Works on Railway, Render, Fly.io, and any VPS.

## Why Not Docker-in-Docker

PaaS hosts (Railway, Render, Fly.io) do not allow Docker-in-Docker because
they don't expose a Docker daemon socket or privileged container access.

The initial design had the Rust worker spawning `docker run opendronemap/odm`.
This fails on all major PaaS platforms with `permission denied`.

See [ADR-004](../../backlog/docs/decisions/004-inverted-container-no-dind.md) for the full
decision rationale.

## The Inverted Container Model

```
PaaS (Railway / Render / Fly.io)
 └── Single Docker Container
      │
      ├── Nadir Rust Worker (`nadir serve`)   ← PID 1 (HTTP server)
      │    │
      │    │ tokio::process::Command::new("colmap")
      │    ▼
      └── COLMAP binary (same container)      ← Subprocess
      └── OpenMVS binaries (same container)   ← Subprocess
      └── GDAL / PROJ libraries               ← Linked via FFI
      └── PDAL binary (same container)        ← Subprocess
```

The Rust code calls engines directly:

```rust
// NOT this (Docker-in-Docker — broken on PaaS):
Command::new("docker").args(["run", "opendronemap/odm", ...])

// THIS (direct subprocess — works everywhere):
Command::new("colmap").args(["feature_extractor", ...])
Command::new("DensifyPointCloud").args(["-i", &scene, ...])
```

## Multi-Stage Dockerfile (Pixi-based)

### Development / Full Build

```dockerfile
# ── Stage 1: Build Rust binary ──
FROM rust:1.83-bookworm AS builder

# Install Pixi
COPY --from=ghcr.io/prefix-dev/pixi:latest /pixi /usr/local/bin/pixi

WORKDIR /app

# Install system dependencies via Pixi (cached layer)
COPY pixi.toml pixi.lock ./
RUN pixi install --locked --frozen

# Build Rust binary
COPY . .
RUN pixi run build

# ── Stage 2: Runtime ──
FROM ghcr.io/prefix-dev/pixi:latest

WORKDIR /app

# Copy the Pixi environment (includes COLMAP, OpenMVS, GDAL, PROJ, PDAL)
COPY --from=builder /app/.pixi /app/.pixi

# Copy the built binary
COPY --from=builder /app/target/release/nadir /usr/local/bin/nadir

# Set up environment
ENV PATH="/app/.pixi/envs/default/bin:${PATH}"
ENV LD_LIBRARY_PATH="/app/.pixi/envs/default/lib:${LD_LIBRARY_PATH}"
ENV PROJ_DATA="/app/.pixi/envs/default/share/proj"
ENV GDAL_DATA="/app/.pixi/envs/default/share/gdal"

# Create workspace
RUN mkdir -p /var/nadir/workspace && chmod 777 /var/nadir/workspace

EXPOSE 8080

HEALTHCHECK --interval=30s --timeout=5s --retries=3 \
    CMD curl -f http://localhost:8080/health || exit 1

ENTRYPOINT ["nadir", "serve", "--port", "8080", "--work-dir", "/var/nadir/workspace"]
```

### Lean Production Variant

For smaller image size, copy only the runtime libraries:

```dockerfile
# ── Stage 1: Build (same as above) ──
FROM rust:1.83-bookworm AS builder
COPY --from=ghcr.io/prefix-dev/pixi:latest /pixi /usr/local/bin/pixi
WORKDIR /app
COPY pixi.toml pixi.lock ./
RUN pixi install --locked --frozen
COPY . .
RUN pixi run build

# ── Stage 2: Lean runtime ──
FROM debian:bookworm-slim

RUN apt-get update && apt-get install -y --no-install-recommends \
    ca-certificates \
    libgomp1 \
    libstdc++6 \
    && rm -rf /var/lib/apt/lists/*

WORKDIR /app

# Copy only the runtime libraries from Pixi environment
COPY --from=builder /app/.pixi/envs/default/lib /usr/local/lib
COPY --from=builder /app/.pixi/envs/default/bin/colmap /usr/local/bin/
COPY --from=builder /app/.pixi/envs/default/bin/pdal /usr/local/bin/
COPY --from=builder /app/.pixi/envs/default/bin/InterfaceCOLMAP /usr/local/bin/
COPY --from=builder /app/.pixi/envs/default/bin/DensifyPointCloud /usr/local/bin/
COPY --from=builder /app/.pixi/envs/default/bin/ReconstructMesh /usr/local/bin/
COPY --from=builder /app/.pixi/envs/default/bin/TextureMesh /usr/local/bin/
COPY --from=builder /app/.pixi/envs/default/share/proj /usr/share/proj
COPY --from=builder /app/.pixi/envs/default/share/gdal /usr/share/gdal

# Copy the Nadir binary
COPY --from=builder /app/target/release/nadir /usr/local/bin/nadir

RUN ldconfig

RUN mkdir -p /var/nadir/workspace
EXPOSE 8080

ENTRYPOINT ["nadir", "serve", "--port", "8080"]
```

### Image Size Comparison

| Variant | Approximate Size | Notes |
|---|---|---|
| Full Pixi runtime | ~4–6 GB | All deps, easy to maintain |
| Lean production | ~2–3 GB | Only runtime libs, harder to update |
| ODM Docker (reference) | ~8 GB | Includes Python + all ODM deps |

## Local Development

```bash
# Build the image
docker build -t nadir-worker .

# Run the worker
docker run -p 8080:8080 \
    -v $(pwd)/data:/var/nadir/workspace \
    nadir-worker

# Or use Pixi directly (no Docker needed for local dev)
pixi install
pixi run serve
```

## Deployment to Railway

1. Push repo to GitHub
2. Link repository to Railway project
3. Railway detects `Dockerfile` and builds automatically
4. Set environment variables:
   - `PORT=8080`
   - `WORKSPACE_DIR=/var/nadir/workspace`
5. Attach a persistent volume to `/var/nadir/workspace`
6. Set instance to 8GB RAM minimum

### Railway-specific considerations

| Concern | Solution |
|---|---|
| Build timeout | Use Railway's build cache; Pixi lockfile speeds this up |
| Disk space | Attach persistent volume (Railway: up to 100GB) |
| RAM | Use 8GB instance minimum for photogrammetry |
| CPU | Use 4+ vCPU instance |
| Long-running tasks | Railway allows long-running processes (no timeout like Vercel) |
| Health checks | `/health` endpoint returns 200 when worker is ready |

## Deployment to Fly.io

```toml
# fly.toml
app = "nadir-worker"
primary_region = "ord"

[build]
  dockerfile = "Dockerfile"

[http_service]
  internal_port = 8080
  force_https = true

[mounts]
  source = "nadir_workspace"
  destination = "/var/nadir/workspace"

[vm]
  cpu_kind = "performance"
  cpus = 4
  memory_mb = 8192
```

Fly.io advantage: GPU machines available (paid tier) for COLMAP/OpenMVS.

## Multi-Worker Deployment (V1.0)

When splitting into domain workers:

```yaml
# docker-compose.yml
services:
  scheduler:
    image: nadir-worker
    command: ["nadir", "scheduler", "--port", "8080"]
    ports: ["8080:8080"]

  recon:
    image: nadir-worker
    command: ["nadir", "serve", "--engines", "colmap,openmvs"]
    deploy:
      resources:
        reservations:
          cpus: "12"
          memory: 24G
    volumes:
      - artifacts:/var/nadir/artifacts

  carto:
    image: nadir-worker
    command: ["nadir", "serve", "--engines", "gdal,proj,builtin"]
    deploy:
      resources:
        reservations:
          cpus: "4"
          memory: 8G
    volumes:
      - artifacts:/var/nadir/artifacts

volumes:
  artifacts:
```

Same image, different `--engines` flags. Shared volume for zero-cost data
transfer between workers.

## See Also

- [ADR-004: Inverted container](../../backlog/docs/decisions/004-inverted-container-no-dind.md)
- [ADR-005: Per-step workers](../../backlog/docs/decisions/005-worker-topology-per-step.md)
- [Hosting comparison](./hosting-comparison.md)
- [Pixi setup](./pixi-setup.md)
