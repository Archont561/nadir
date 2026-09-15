---
title: "ADR-004: Inverted Container Model (No Docker-in-Docker)"
status: accepted
date: 2025-02-23
deciders: project lead
related:
  - ../deployment/docker-strategy.md
  - ../deployment/hosting-comparison.md
  - 001-step-scoped-vs-odm-monolith.md
  - 005-worker-topology-per-step.md
---

# ADR-004: Inverted Container Model (No Docker-in-Docker)

## TL;DR

PaaS hosts (Railway, Render, Fly.io) do not allow Docker-in-Docker because
they don't expose a Docker daemon socket or privileged container access.
Instead of the Rust worker launching engine Docker containers, the deployed
container **is** the engine environment. The Nadir Rust binary runs as PID 1
and spawns engines as local subprocesses within the same container.

## Context

The initial V0.1 design had the Rust worker spawning ODM via Docker:

```rust
// Original design — DOES NOT WORK on PaaS
Command::new("docker")
    .args(["run", "--rm",
           "-v", "/tmp/images:/project/images",
           "opendronemap/odm",
           "--project-path", "/project"])
    .spawn()?;
```

This requires:
- A Docker daemon running inside the container (Docker-in-Docker)
- Privileged container access (`--privileged` flag)
- A Docker socket mount (`/var/run/docker.sock`)

None of these are available on Railway, Render, or Fly.io. Attempting to run
Docker-in-Docker on these platforms results in `permission denied` or
`cannot connect to Docker daemon` errors.

## Decision

**Inverted container model.** The Docker image is built on top of the engine
environment (e.g., `opendronemap/odm:latest` or a custom image with COLMAP +
OpenMVS + GDAL). The Nadir Rust binary is copied into this image and runs as
the entrypoint. Engines are spawned as local subprocesses, not nested
containers.

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
```

### The Dockerfile

```dockerfile
# Stage 1: Build Rust binary
FROM rust:1.83-bookworm AS builder
WORKDIR /app
COPY . .
RUN cargo build --release -p nadir-cli

# Stage 2: Runtime (Pixi environment with all engines)
FROM ghcr.io/prefix-dev/pixi:latest
WORKDIR /app
COPY pixi.toml pixi.lock ./
RUN pixi install --locked --frozen
COPY --from=builder /app/target/release/nadir /usr/local/bin/nadir
ENV PATH="/app/.pixi/envs/default/bin:${PATH}"
EXPOSE 8080
ENTRYPOINT ["nadir", "serve", "--port", "8080"]
```

### The Rust code change

```rust
// Before (Docker-in-Docker — broken on PaaS):
Command::new("docker")
    .args(["run", "--rm", "opendronemap/odm", ...])

// After (direct subprocess — works everywhere):
Command::new("colmap")
    .args(["feature_extractor", "--database_path", &db, ...])

Command::new("DensifyPointCloud")
    .args(["-i", &scene_mvs, "--resolution-level", "1"])
```

## Rationale

| Factor | Docker-in-Docker | Inverted Container |
|---|---|---|
| Railway / Render / Fly.io | ❌ Permission denied | ✅ Works out of the box |
| Process control (SIGINT/kill) | ⚠️ Hard to reach nested container | ✅ Direct OS PID signals |
| Disk I/O overhead | ⚠️ Nested filesystem layers | ✅ Direct container disk |
| Progress streaming | ⚠️ Docker socket multiplexing | ✅ Standard piped stdout |
| Memory overhead | ⚠️ Docker daemon + nested container | ✅ Single process tree |
| Image size | ⚠️ Docker-in-Docker base image | ✅ Engine base + Rust binary |
| Local development | Requires Docker daemon | Same image: `docker run nadir` |
| CI testing | Complex (Docker-in-Docker in CI) | Simple (run the image) |

### Process control is significantly better

With Docker-in-Docker, cancelling a task requires:
1. Send SIGTERM to the Docker process
2. Docker forwards signal to the container
3. Container forwards signal to ODM
4. Hope the signal chain works

With the inverted model:
1. Send SIGTERM to the COLMAP PID
2. COLMAP terminates
3. Done

This matters for long-running photogrammetry tasks (hours) where users need
reliable cancellation.

## Consequences

### Positive
- Works on all major PaaS platforms without privileged access
- Simpler process management (direct PID signals)
- Better disk I/O performance (no nested filesystem)
- Cleaner progress streaming (standard stdout pipes)
- Smaller memory footprint (no Docker daemon)
- Same image works locally and in production

### Negative
- Docker image is larger than a pure Rust image (includes COLMAP, OpenMVS,
  GDAL, PROJ — typically 4–6GB)
- All engines share the same container resources (CPU, RAM) — no isolation
- Updating one engine requires rebuilding the entire image
- Cannot run different engine versions for different tasks simultaneously

### Mitigated
- Image size is acceptable for photogrammetry workloads (the data is large
  anyway)
- Resource isolation is handled by the `ResourceScheduler` (Tokio semaphores)
  rather than container boundaries
- Per-step worker topology (ADR-005) provides isolation when needed by
  running separate containers per domain
- Engine updates are infrequent (COLMAP releases ~yearly)

## See Also

- [ADR-005: Per-step worker topology](./005-worker-topology-per-step.md) — splitting into multiple containers
- [Docker strategy](../deployment/docker-strategy.md) — full Dockerfile details
- [Hosting comparison](../deployment/hosting-comparison.md) — platform constraints
- [Pixi setup](../deployment/pixi-setup.md) — how Pixi provides the engine binaries
