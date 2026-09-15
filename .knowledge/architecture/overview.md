---
title: Architecture Overview
purpose: Big-picture system design — 5 domains, 3 dependency tiers, build system metaphor
last_updated: 2025-02-23
status: current
related:
  - ../CONTEXT.md
  - five-domains.md
  - engine-registry.md
  - pipeline-dag.md
  - artifact-model.md
  - ../decisions/001-step-scoped-vs-odm-monolith.md
---

# Architecture Overview

## TL;DR

Nadir is an **artifact-driven build system for geospatial computation**. Not a
job queue, not an ODM wrapper. Tasks produce artifacts from artifacts. Every
node in the DAG is an artifact, every edge is a task, every artifact has
provenance and a content hash. The system is organized into 5 computational
domains, uses 3 dependency tiers, and evolves from a CLI orchestrator around
external engines (V0) to a mostly Rust-native photogrammetry platform (V3).

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
             ┌─────────┼─────────┐
             ▼         ▼         ▼
          COLMAP    OpenMVS     GDAL
             │         │         │
             └─────────┼─────────┘
                       ▼
                    ARTIFACT
                       │
              ┌────────┴────────┐
              ▼                 ▼
            CACHE          PROVENANCE
              │
              ▼
            RESUME
```

This gives you caching, incremental invalidation, resume, partial execution,
reproducibility, alternative engines, and distributed execution as
**architectural properties**, not bolted-on features.

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

Above all five sits the **Workflow Engine**: DAG planner, task executor,
artifact cache, provenance store, and resource scheduler.

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

Plus all orchestration code you write: pipeline, executor, artifacts, cache,
provenance, config, georeferencing, DSM/DTM, orthomosaic.

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
processing result with identity, provenance, and a content hash.

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
    task type
    + input artifact hashes
    + normalized task parameters
    + engine identity
    + engine version
)
```

Same inputs + same config + same engine = cache hit. See
[artifact-model.md](./artifact-model.md).

## The Evolution Path

### V0: Orchestrator (current target)

```
Rust CLI/Worker
    ├── COLMAP (subprocess)
    ├── OpenMVS (subprocess)
    ├── GDAL (FFI)
    ├── PROJ (FFI)
    └── PDAL (subprocess)
```

Goal: produce correct mapping products. Time: months.

### V1: Processing Platform

Add: content-addressed cache, artifact model, provenance, QC gates, adaptive
planning, product recipes, resource-aware scheduler, dataset preflight.

Goal: make processing reliable, reproducible, and pleasant.

### V2: Scale & Native Engines

Add: dataset splitting, native Rust surface engines (DSM/DTM/filtering),
worker protocol (protobuf), multi-language SDK, distributed execution.

Goal: handle 10,000+ images, begin replacing external engines.

### V3: Full Platform

Add: native Rust reconstruction (features, matching, SfM), GPU compute
(wgpu), browser WASM, edge processing, SaaS control plane.

Goal: become something more interesting than an ODM reimplementation.

### The engine replacement timeline

```
V0                          V1                    V2                    V3
Features   → COLMAP         → COLMAP              → Rust                → Rust/GPU
Matching   → COLMAP         → COLMAP              → Rust                → Rust/GPU
SfM        → COLMAP         → COLMAP              → COLMAP              → Rust
Dense MVS  → OpenMVS        → OpenMVS             → OpenMVS             → Rust/GPU
PC Filter  → PDAL           → PDAL                → Rust                → Rust
Ground     → PDAL           → PDAL                → Rust                → Rust/ML
DSM        → Rust+GDAL      → Rust                → Rust                → Rust
DTM        → Rust+GDAL      → Rust                → Rust                → Rust
Mesh       → OpenMVS        → OpenMVS             → OpenMVS             → Rust
Ortho      → Rust+GDAL      → Rust                → Rust                → Rust
CRS        → PROJ           → PROJ                → PROJ                → PROJ (permanent)
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
- [Full roadmap](../roadmap/full-roadmap.md) — V0→V3 checklist
