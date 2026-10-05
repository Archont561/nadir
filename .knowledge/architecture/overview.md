---
type: Architecture
title: Architecture Overview
description: "Big-picture system design — 5 domains, 3 dependency tiers, build system metaphor"
purpose: Big-picture system design — 5 domains, 3 dependency tiers, build system metaphor
last_updated: 2026-10-05
status: stable
related:
  - ../context.md
  - five-domains.md
  - engine-registry.md
  - pipeline-dag.md
  - artifact-model.md
  - ../../backlog/docs/decisions/001-step-scoped-vs-odm-monolith.md
  - ../../backlog/docs/decisions/012-v0-strict-sparse-reconstruction-profile.md
---

# Architecture Overview

## TL;DR

Nadir is an **artifact-driven build system for photogrammetry artifacts**. Not
a job queue, not an ODM wrapper. Tasks produce artifacts from artifacts; every
artifact has provenance and verified bytes. ADR-012 narrows V0 to one bounded
calibrated ImageSet, immutable input snapshots, a qualified CPU-only COLMAP
sparse path, verified stage caching, and canonical local `SparseScene v1`.
Georeferencing, dense reconstruction, workers and mapping products come later.

## The Mental Model

Do not think:

> "Run command A, then command B, then command C."

Think:

> "Produce artifact B from artifact A using these parameters and this engine."

```
                    INPUT DATA
                        │
                        ▼
                  ┌───────────┐
                  │   PLAN    │
                  └─────┬─────┘
                        │
                        ▼
                     DAG
                        │
                        ▼
              ┌──────────────────┐
              │   TASK EXECUTOR  │
              └────────┬─────────┘
                       │
                       ▼
              CPU-only COLMAP (V0)
                       │
                       ▼
              SparseScene v1 (local_sfm)
                       │
          post-V0: CRS / dense / products
                       │
              ┌────────┴────────┐
              ▼                 ▼
            CACHE          PROVENANCE
              │
              ▼
            RESUME
```

This gives you caching, incremental invalidation, resume, reproducibility,
alternative engines, and eventually distributed execution as **architectural
properties**, not bolted-on features. V0 proves those properties on the smallest
useful boundary before adding configurable recipes or product branches.

## The Five Computational Domains

The photogrammetry pipeline decomposes into five domains, each with its own
crate group, trait hierarchy, and engine assignments:

```
┌────────────────────────────────────────────┐
│ 1. DATASET                                 │
│    images / EXIF / GPS / camera metadata   │
│    Engines: Rust native                    │
└────────────────────┬───────────────────────┘
                     │
┌────────────────────▼───────────────────────┐
│ 2. RECONSTRUCTION                          │
│    features → matches → SfM → MVS          │
│    Engines: COLMAP / OpenMVS / future Rust │
└────────────────────┬───────────────────────┘
                     │
┌────────────────────▼───────────────────────┐
│ 3. GEOMETRY                                │
│    coordinate frames / GPS / RTK / GCP/CRS │
│    Engines: nalgebra / PROJ / Rust         │
└────────────────────┬───────────────────────┘
                     │
┌────────────────────▼───────────────────────┐
│ 4. SURFACE                                 │
│    point cloud → DSM / DTM / mesh          │
│    Engines: PDAL / OpenMVS / Rust / GDAL   │
└────────────────────┬───────────────────────┘
                     │
┌────────────────────▼───────────────────────┐
│ 5. CARTOGRAPHY                             │
│    orthorectification / mosaicking / tiles  │
│    Engines: Rust / GDAL                    │
└────────────────────────────────────────────┘
```

Above all five sits the **Workflow Engine**: profile/recipe planner, task executor,
artifact cache, provenance store, and resource scheduler. V0 touches Dataset and
Reconstruction only; Geometry, Surface and Cartography are post-V0 consumers of
explicit artifact contracts.

See [five-domains.md](./five-domains.md) for the detailed breakdown.

## The Three Dependency Tiers

Not everything is pure Rust, and that's fine. The architecture uses three
tiers of dependencies:

### Tier 1: Native Rust (you own completely)

Pure Rust crates. No FFI. No external binaries. Full control.

```
nalgebra        3D math, transforms, quaternions
geo/geo-types   geospatial types, algorithms
image           JPEG/TIFF/PNG decoding
kamadak-exif    EXIF metadata
las/laz         point cloud I/O
petgraph        DAG for pipeline
tokio           async runtime, subprocesses
rayon           CPU parallelism
serde           serialization
blake3          content hashing
clap            CLI
tracing         structured logging
```

Plus all orchestration code you write: profile/recipe planning, executor,
artifacts, cache, provenance, config and, after V0, georeferencing, DSM/DTM and
orthomosaic generation.

### Tier 2: C/C++ Bindings (stable FFI)

Mature Rust crates that wrap C/C++ libraries. You link against system
libraries installed via Pixi.

```
gdal crate  →  GDAL C library   (raster/vector/CRS)
proj crate  →  PROJ C library   (coordinate math)
opencv crate → OpenCV C++ lib   (calibration, vision)
```

These are "Rust enough" for production. Don't rewrite them.

### Tier 3: External Engines (subprocess)

Standalone binaries invoked via `tokio::process`. No FFI. No linking. Clean
process boundary. Replaceable without recompiling Nadir.

```
COLMAP     SfM, features, matching, bundle adjustment
OpenMVS    dense reconstruction, mesh, texturing
PDAL       point cloud filtering, classification
```

These are the hardest algorithms in photogrammetry. They represent decades
of research. Don't rewrite them until you have a compelling reason.

### The tier diagram

```
┌─────────────────────────────────────────────────┐
│              TIER 1: NATIVE RUST                 │
│  nalgebra · geo · image · tokio · rayon · blake3 │
│  + all orchestration code you write              │
└────────────────────┬────────────────────────────┘
                     │ FFI (safe Rust bindings)
┌────────────────────▼────────────────────────────┐
│            TIER 2: C/C++ BINDINGS                │
│  gdal crate · proj crate · opencv crate          │
└────────────────────┬────────────────────────────┘
                     │ subprocess / IPC
┌────────────────────▼────────────────────────────┐
│           TIER 3: EXTERNAL ENGINES               │
│  COLMAP · OpenMVS · PDAL                         │
└─────────────────────────────────────────────────┘
```

## The Engine Abstraction

The pipeline asks for **concepts**, not engines:

```
"perform SparseReconstruction"
```

Not:

```
"run colmap mapper"
```

The engine registry resolves concepts to implementations:

```
SparseReconstruction
        ↓
ColmapSparseReconstruction
```

Or later:

```
SparseReconstruction
        ↓
RustSparseReconstruction
```

No pipeline changes required. See [engine-registry.md](./engine-registry.md).

## The Artifact-First Principle

Artifacts are more important than files. An artifact represents a meaningful
processing result with identity, provenance, and verified bytes. V0 distinguishes
the invocation key that identifies requested work from the output-tree digest
that verifies produced bytes.

```rust
pub struct Artifact {
    pub id: ArtifactId,
    pub kind: ArtifactKind,
    pub location: ArtifactLocation,
    pub hash: ArtifactHash,       // blake3(task + inputs + params + engine)
    pub metadata: ArtifactMetadata,
    pub provenance: Provenance,
}
```

Content-based identity enables:

```
hash(
    task and contract versions
    + named input artifact digests
    + normalized resolved configuration
    + qualified toolchain identity
    + every byte-affecting setting
)
```

The key maps to a verified manifest of output-tree digests. Same inputs + same
config + same qualified runtime = cache lookup; the hit is usable only after the
manifest bytes revalidate. See [artifact-model.md](./artifact-model.md).

## The Evolution Path

### V0: Strict sparse reconstruction (current target)

```
Rust CLI
    └── qualified CPU-only COLMAP sparse path
            ├── feature_extractor
            ├── exhaustive_matcher
            └── mapper
                ↓
          SparseScene v1 (local_sfm)
```

Goal: prove immutable ImageSet snapshots, verified stage caching, runtime
qualification, containment and canonical local sparse output. No CRS, dense
reconstruction, GPU execution, worker service or mapping products.

### V1: Local mapping products

Add: georeferencing, CRS transforms, dense reconstruction, DSM/DTM,
orthomosaics, GeoTIFF/COG output, adaptive recipes and resource-aware local
scheduling.

Goal: make mapping products reliable by consuming explicit V0 artifact
contracts.

### V2: Worker service and scale

Add: worker protocol, HTTP serving, object-store policies, dataset splitting,
partition/merge execution, remote transports and workflow SDKs.

Goal: handle large datasets and remote execution without changing local
artifact semantics.

### V3: Native/WASM/edge platform

Add: selected native Rust engines, GPU qualification profiles, browser WASM,
edge processing and optional SaaS control plane.

Goal: become something more interesting than an ODM reimplementation.

### The engine replacement timeline

```
V0                          V1                    V2                    V3
Features   → COLMAP CPU     → COLMAP              → Rust candidate      → Rust/GPU
Matching   → COLMAP CPU     → COLMAP              → Rust candidate      → Rust/GPU
SfM        → COLMAP CPU     → COLMAP              → COLMAP/Rust         → Rust
Sparse out → SparseScene v1 → SparseScene v1      → SparseScene v1      → SparseScene v1
Georef     → deferred       → PROJ + Rust         → PROJ + Rust         → PROJ (permanent)
Dense MVS  → deferred       → OpenMVS/COLMAP      → OpenMVS/Rust        → Rust/GPU
PC Filter  → deferred       → PDAL                → Rust candidate      → Rust
Ground     → deferred       → PDAL                → Rust candidate      → Rust/ML
DSM        → deferred       → Rust+GDAL           → Rust                → Rust
DTM        → deferred       → Rust+GDAL           → Rust                → Rust
Mesh       → deferred       → OpenMVS             → OpenMVS/Rust        → Rust
Ortho      → deferred       → Rust+GDAL           → Rust                → Rust
Workers    → none           → local only          → Worker Protocol     → distributed
```

PROJ is the one dependency you never rewrite. Coordinate reference systems
are a 40-year-old international standard.

## The Architectural Rule

> **Photogrammetry algorithms are plugins. The workflow/artifact system is
> the product.**

This allows the project to start as an orchestrator around established
engines and gradually become a mostly Rust-native photogrammetry platform,
without ever redesigning the core architecture.

## See Also

- [Five domains](./five-domains.md) — detailed domain breakdown
- [Engine registry](./engine-registry.md) — 16 traits and resolution
- [Pipeline & DAG](./pipeline-dag.md) — how the DAG executor works
- [Artifact model](./artifact-model.md) — hashing, caching, provenance
- [Worker protocol](./worker-protocol.md) — distributed execution
- [Full roadmap](../../backlog/docs/roadmaps/full-roadmap.md) — ADR-012-aligned V0→V3 checklist
