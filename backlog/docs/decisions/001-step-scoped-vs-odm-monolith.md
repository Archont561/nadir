---
type: Architecture Decision Record
title: "ADR-001: Step-Scoped Tools vs ODM Monolith"
description: "Records the accepted architecture decision on Step-Scoped Tools vs ODM Monolith."
status: accepted
date: 2025-02-23
deciders: project lead
related:
  - ../../../.knowledge/architecture/overview.md
  - ../../../.knowledge/architecture/engine-registry.md
  - ../../../.knowledge/architecture/artifact-model.md
  - ../../../.knowledge/photogrammetry/engine-assignment.md
---

# ADR-001: Step-Scoped Tools vs ODM Monolith

## TL;DR

We decompose the photogrammetry pipeline into individual stages, each calling a
focused external engine (COLMAP for SfM, OpenMVS for MVS, GDAL for raster),
rather than wrapping OpenDroneMap as a single monolithic subprocess. The extra
~1 week of implementation pays for itself immediately via stage-level caching,
resume from arbitrary stages, parallel DAG branches, and engine swapping.

## Context

OpenDroneMap (ODM) provides a complete photogrammetry pipeline as a single
Docker image (~8GB). It internally runs a linked-list of stages: dataset →
OpenSfM → OpenMVS → filtering → meshing → texturing → georeferencing → DEM →
orthophoto → report. Each stage calls `process()` then invokes the next stage.

We considered two approaches for Nadir's V0.1:

**Option A: ODM Monolith.** Write a thin Rust adapter that maps our semantic
config (`quality: "survey"`) to ODM's 100+ CLI flags and spawns a single
`python3 /code/run.py` subprocess. One call, ODM handles everything internally.

**Option B: Step-Scoped Tools.** Call individual engines as separate
subprocesses. COLMAP for feature extraction, matching, and SfM. OpenMVS for
dense reconstruction and meshing. GDAL for raster output. Each behind a Rust
trait. The pipeline DAG orchestrates the sequence.

## Decision

**Step-scoped tools (Option B).** Each pipeline stage calls a focused external
engine with a small, well-documented CLI. The Rust pipeline engine owns the
DAG, caching, resume, and artifact management. External engines own the
photogrammetry math.

## Rationale

| Factor | ODM Monolith | Step-Scoped |
|---|---|---|
| Stage-level caching | ❌ All or nothing | ✅ Per-artifact, content-hashed |
| Resume from arbitrary stage | ⚠️ `--rerun-from` is fragile | ✅ Check artifact hash, skip |
| Engine swapping | ❌ Locked to OpenSfM + OpenMVS | ✅ Per-step engine selection |
| Parallel DAG branches | ❌ Linear linked list | ✅ DSM + Mesh run concurrently |
| Intermediate artifact inspection | ❌ Opaque project directory | ✅ Typed artifacts with metadata |
| Granular progress | ⚠️ Coarse stage % | ✅ Per-step with clear boundaries |
| Debugging | ODM internals (Python, opaque) | Individual tool logs (clear) |
| Config complexity | 100+ flags, many interactions | 5–10 flags per step |
| Initial adapter code | ~200 lines (flag mapping) | ~600 lines (4 small adapters) |
| Docker image | ~8GB (ODM) | ~6GB (COLMAP + OpenMVS + GDAL) |
| Long-term architecture | Dead end (wrapper) | Platform foundation |
| Time to demo | ~2 weeks | ~3–4 weeks |

### The caching argument is decisive

With ODM monolith, changing DEM resolution from 0.05 to 0.10 forces a full
rerun: features, matching, SfM, dense reconstruction — potentially 3+ hours of
wasted compute.

With step-scoped tools:

```
ingest       ✓ cached (unchanged)
features     ✓ cached (unchanged)
matching     ✓ cached (unchanged)
sfm          ✓ cached (unchanged)
georef       ✓ cached (unchanged)
dense        ✓ cached (unchanged)
dsm          ✗ STALE  (resolution changed: 0.05 → 0.10)
dtm          ✗ STALE  (depends on dsm)
ortho        ✗ STALE  (depends on dsm)
mesh         ✓ cached (independent branch)
```

Only 3 of 10 stages recompute. The other 7 are instant cache hits. This is the
single strongest argument for step-scoped architecture.

### The parallelism argument

ODM's internal pipeline is a linked list: stage A calls stage B calls stage C.
DSM generation and mesh generation cannot run concurrently even though they
are independent (both consume the dense point cloud).

With step-scoped DAG:

```
         Dense Point Cloud
          │             │
          ▼             ▼
        DSM           Mesh      ← concurrent via Tokio
        │             │
        ▼             ▼
    Orthomosaic    TexturedMesh
```

On a 32-core machine, this can cut wall-clock time by 30–40% for full-survey
pipelines.

## Consequences

### Positive
- Stage-level content-addressed caching from day one
- True resume: check which artifacts exist, skip the rest
- Engine swapping per step (`sfm.engine = "colmap"` → `"opensfm"`)
- Parallel execution of independent DAG branches
- Full intermediate artifact inspection and provenance
- Each engine's CLI is small and well-documented
- Clean path to replacing individual engines with Rust implementations

### Negative
- ~4 engine adapters to write instead of 1 (~600 lines vs ~200 lines)
- Data format conversion between tools (COLMAP binary model → OpenMVS `.mvs`
  scene via `InterfaceCOLMAP`)
- Slightly more complex deployment (multiple binaries instead of one Docker image)
- Need to manage shared workspace directories between steps

### Neutral
- The total external dependency footprint is similar (COLMAP + OpenMVS + GDAL
  are all inside the ODM Docker image anyway)
- ODM remains a useful reference for pipeline ordering and expected outputs

## See Also

- [Engine assignment](../../../.knowledge/photogrammetry/engine-assignment.md) — which tool handles which stage
- [Engine registry](../../../.knowledge/architecture/engine-registry.md) — the 16-trait API surface
- [Artifact model](../../../.knowledge/architecture/artifact-model.md) — content-based hashing and caching
- [Pipeline & DAG](../../../.knowledge/architecture/pipeline-dag.md) — how the DAG executor works
