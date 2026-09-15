---
title: "ADR-005: Per-Step Worker Topology"
status: accepted
date: 2025-02-23
deciders: project lead
related:
  - 004-inverted-container-no-dind.md
  - ../architecture/worker-protocol.md
  - ../deployment/docker-strategy.md
---

# ADR-005: Per-Step Worker Topology

## TL;DR

The same Nadir binary supports three deployment topologies via the `--engines`
flag: a single fat worker (V0.1), domain-scoped workers on shared volumes
(V1.0), and per-engine workers on specialized hardware (V2.0). The code does
not change between topologies — only the deployment configuration.

## Context

Photogrammetry pipelines use engines with very different resource profiles:

| Engine | CPU | RAM | GPU | Disk | Image Size |
|---|---|---|---|---|---|
| COLMAP | 16+ cores | 32–64 GB | Optional | 50+ GB | ~2 GB |
| OpenMVS | 12+ cores | 32 GB | Optional | 20+ GB | ~1.5 GB |
| GDAL | 4 cores | 8 GB | No | 10 GB | ~500 MB |
| PDAL | 4 cores | 8 GB | No | 5 GB | ~300 MB |
| Rust native | 4–8 cores | 4–16 GB | No | 5 GB | ~50 MB |

A single fat worker bundles all engines into one ~6GB container. This is
simple but wastes resources when a lightweight GDAL task is queued behind a
heavy COLMAP reconstruction.

The question: should we split into per-step or per-domain workers, and if so,
how do we handle the massive data transfer between them (dense point clouds
can be 20–50 GB)?

## Decision

**Evolutionary topology.** Start with one fat worker (V0.1). Split into
domain workers on shared volumes (V1.0). Split into per-engine workers on
specialized hardware with object storage (V2.0). The binary is the same;
only `--engines` and deployment config change.

### Topology A: Fat Worker (V0.1)

```
┌─────────────────────────────────────┐
│         Single Worker               │
│  COLMAP + OpenMVS + GDAL + PDAL     │
│  ~6GB image, all steps local        │
│  nadir serve --port 8080            │
└─────────────────────────────────────┘
```

### Topology B: Domain Workers (V1.0)

```
┌──────────────┐  ┌──────────────┐  ┌──────────────┐
│  Recon       │  │  Surface     │  │  Cartography │
│  COLMAP      │  │  OpenMVS     │  │  GDAL + PROJ │
│  OpenMVS     │  │  PDAL        │  │  Rust native │
│  ~4GB        │  │  ~2GB        │  │  ~500MB      │
│  --engines   │  │  --engines   │  │  --engines   │
│  colmap,     │  │  openmvs,    │  │  gdal,proj,  │
│  openmvs     │  │  pdal        │  │  builtin     │
└──────┬───────┘  └──────┬───────┘  └──────┬───────┘
       │                 │                 │
       └────────┬────────┴─────────────────┘
                │
          Shared Volume (zero network transfer)
```

### Topology C: Per-Engine Workers (V2.0+)

```
┌────────┐ ┌────────┐ ┌────────┐ ┌────────┐
│ COLMAP │ │ OpenMVS│ │  PDAL  │ │  GDAL  │
│  GPU   │ │  GPU   │ │  CPU   │ │  CPU   │
│ Fly.io │ │ Fly.io │ │Railway │ │Railway │
└───┬────┘ └───┬────┘ └───┬────┘ └───┬────┘
    │          │          │          │
    └──────────┴──────────┴──────────┘
                     │
            S3/R2 Object Storage
```

## Rationale

### Why shared volumes solve the data transfer problem (V1.0)

Photogrammetry artifacts are enormous (dense point clouds: 20–50 GB).
Transferring them over the network between workers is prohibitively slow.

On a single machine with Docker Compose, a shared volume gives zero-cost
data transfer:

```yaml
services:
  recon:
    image: nadir-worker
    command: ["nadir", "serve", "--engines", "colmap,openmvs"]
    volumes:
      - artifacts:/var/nadir/artifacts
  carto:
    image: nadir-worker
    command: ["nadir", "serve", "--engines", "gdal,builtin"]
    volumes:
      - artifacts:/var/nadir/artifacts
volumes:
  artifacts:
```

Both workers read/write the same filesystem. The 20GB dense point cloud
never crosses a network boundary.

### Why the same binary works for all topologies

The `--engines` flag controls which engine adapters are loaded at startup:

```rust
let engines: Vec<&str> = args.engines.split(',').collect();
let registry = EngineRegistry::load(&engines);
```

The scheduler routes tasks based on advertised capabilities:

```rust
WorkerCapabilities {
    worker_id: "recon-1".into(),
    engines: vec!["colmap".into(), "openmvs".into()],
    tasks: vec![ExtractFeatures, MatchFeatures, SparseReconstruction,
                DenseReconstruction, MeshGeneration],
}
```

No code changes. Just different flags and deployment configs.

### Data locality scoring in the scheduler

When workers are on different machines (V2.0), the scheduler prefers
workers that already have the input artifacts locally:

```rust
fn score_worker(&self, worker: &WorkerHandle, task: &Task) -> i64 {
    let mut score = 0;
    for input in &task.inputs {
        if worker.has_local_artifact(input) {
            score += 1000; // huge bonus for local data
        }
    }
    score += worker.available_ram_gb() as i64 * 10;
    score -= worker.pending_tasks() as i64 * 50;
    score
}
```

## Consequences

### Positive
- V0.1 is dead simple (one container, one command)
- V1.0 gets resource isolation without network transfer
- V2.0 gets GPU-specialized workers with data locality
- Same binary, same protocol, same artifact model across all topologies
- Gradual migration: add workers one at a time

### Negative
- V2.0 requires object storage (S3/R2) and a proper scheduler
- V2.0 has real data transfer costs for large artifacts
- Shared volumes don't work across machines (V1.0 is single-machine only)

## See Also

- [ADR-004: Inverted container](./004-inverted-container-no-dind.md) — single container model
- [Worker protocol](../architecture/worker-protocol.md) — capability advertisement
- [Docker strategy](../deployment/docker-strategy.md) — deployment details
