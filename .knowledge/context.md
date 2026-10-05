---
type: Project Context
title: Nadir Project Context
description: "Single-file project summary for LLM quick-loading"
purpose: Single-file project summary for LLM quick-loading
tokens: ~2000
last_updated: 2025-02-23
status: stable
---

# Nadir — Project Context

## What is Nadir?

Nadir is a Rust-native, artifact-driven photogrammetry pipeline engine. It turns
overlapping drone photographs into georeferenced orthomosaics, DSMs, DTM, point
clouds, and 3D meshes by orchestrating external engines (COLMAP, OpenMVS, GDAL,
PROJ, PDAL) through a declarative DAG with content-addressed caching, quality
control gates, and full provenance tracking.

The name comes from the **nadir point** — the point on the ground directly
beneath the camera. It is the fundamental ground-truth reference in every
aerial image.

## Core Architectural Principles

1. **Build system metaphor, not job queue.** Tasks produce artifacts from
   artifacts. Every node is an artifact, every arrow is a task, every artifact
   has provenance. This gives caching, incremental invalidation, resume, and
   reproducibility as architectural properties, not bolted-on features.

2. **Step-scoped tools, not monolithic wrapper.** Each pipeline stage calls a
   focused external engine (COLMAP for SfM, OpenMVS for MVS, GDAL for raster)
   rather than wrapping ODM as a single subprocess. This enables stage-level
   caching, parallel DAG branches, engine swapping, and granular resume.

3. **Content-addressed caching.** Every task result is identified by
   `blake3(task_kind ‖ input_hashes ‖ normalized_params ‖ engine_version)`.
   Changing one parameter invalidates only downstream artifacts.

4. **16 stable traits as the API boundary.** The pipeline engine knows only
   trait methods like `FeatureExtractor::extract()` and `DsmGenerator::generate()`.
   Engine-specific CLI flags live inside adapters. The pipeline never sees them.

5. **Three dependency tiers.** Native Rust (orchestration, math, rasterization)
   → C/C++ bindings via crates (GDAL, PROJ, OpenCV) → External subprocesses
   (COLMAP, OpenMVS, PDAL). Rust owns the build system; external engines own
   the hard math.

## Five Computational Domains

| Domain | Responsibility | Primary Engines |
|---|---|---|
| **Dataset** | Image ingest, EXIF, GPS, camera models | Rust native (`kamadak-exif`, `image`) |
| **Reconstruction** | Features, matching, SfM, dense MVS, mesh | COLMAP, OpenMVS (subprocess) |
| **Geometry** | CRS transforms, georeferencing, GCP | `nalgebra` + `proj` crate |
| **Surface** | Point cloud filtering, DSM, DTM | PDAL (subprocess) + Rust (`rayon`) |
| **Cartography** | Orthorectification, mosaicking, tiling | GDAL crate + Rust native |

## Tech Stack

| Layer | Technology |
|---|---|
| Language | Rust (core), TypeScript and Python (native SDK faces) |
| Build | Cargo workspace + Pixi (system deps via conda-forge) |
| Async | Tokio (subprocesses, I/O) + Rayon (CPU parallelism) |
| CLI | clap |
| DAG | petgraph |
| Config | serde + figment + TOML |
| Hashing | blake3 |
| Math | nalgebra, sprs, argmin |
| Geospatial | geo, geo-types, gdal crate, proj crate |
| Image | image crate, kamadak-exif |
| Point cloud | las crate |
| Serialization | serde (local), protobuf/prost (future distributed) |
| Logging | tracing + tracing-subscriber |
| Progress | indicatif |
| Engines | COLMAP ≥3.9, OpenMVS ≥2.2, GDAL ≥3.9, PROJ ≥9.4, PDAL ≥2.7 |
| Deployment | Single Docker container (inverted model), Railway or VPS |
| Package mgmt | Pixi (replaces apt/brew/rustup/nvm) |

## Key Decisions (Summary)

| # | Decision | Rationale |
|---|---|---|
| 001 | Step-scoped tools over ODM monolith | Caching, resume, parallelism, engine swapping |
| 002 | Three-language split (TS/Rust/Python) | TS for SDK/UI, Rust for worker, Python for future plugins |
| 003 | No Python engine adapter for MVP (superseded by 011 for SDK bindings) | Rust still calls external engines directly |
| 004 | Inverted container (no Docker-in-Docker) | PaaS hosts don't allow DinD; container IS the ODM environment |
| 005 | Per-step worker topology | Fat worker (V0) → domain workers (V1) → step workers (V2) |
| 006 | Pixi for dependency management | Single `pixi.toml` replaces apt/brew/rustup/nvm/Dockerfile RUN |
| 007 | 16-trait engine API surface | Small traits, small configs, big adapters |
| 008 | WASM/edge/browser strategy | Rust → wasm32 for browser preview, edge tiling, serverless post-processing |
| 011 | Shared transport and thin native bindings | PyO3 and N-API expose one versioned JSON-in/JSON-out dispatcher |

## Current Status

- **Phase:** Early scaffold. CLI shell and native SDK transport compile; pipeline execution is pending.
- **Next step:** V0.1 strict sparse profile — an immutable input snapshot, a qualified CPU-only COLMAP path, and canonical local `SparseScene v1` output.
- **V0 boundary:** no georeferencing, OpenMVS, PDAL, GDAL, DSM, orthomosaic, GPU profile, or worker service. The accepted contract is [V0 strict sparse reconstruction](architecture/v0-strict-sparse-profile.md).
- **Exit evidence:** two byte-identical fresh-store fixture runs and a same-store rerun that launches no COLMAP process for cache hits.
- **Later target:** mapping products, hosting, and language-facing workflow APIs build on explicit contracts after the sparse/cache base is qualified.

## Naming Conventions

| Surface | Name |
|---|---|
| Project | Nadir |
| CLI binary | `nadir` |
| CLI commands | Current scaffold: `nadir process`, `nadir inspect`, `nadir plan`; V0 target: `nadir reconstruct`, `nadir plan`, `nadir status`, `nadir explain`, `nadir cache` |
| Rust crates | `nadir-cli`, `nadir-core`, `nadir-artifacts`, `nadir-process`, `nadir-pipeline`, `nadir-executor`, `nadir-dataset`, `nadir-reconstruction`, `nadir-geometry`, `nadir-surface`, `nadir-cartography`, `nadir-math` |
| npm packages | `@nadir/sdk`, `@nadir/protocol` (future) |
| Python package | `nadir-python` (future) |
| SaaS | Nadir Cloud (future) |
| Pipeline files | `pipelines/standard.toml`, `pipelines/fast-ortho.toml`, etc. |

## File Navigation

- **Start here:** This file (`/context.md`)
- **Full map:** `/index.md`
- **Why we decided X:** `decisions/*.md`
- **System design:** `architecture/*.md`
- **Photogrammetry domain:** `photogrammetry/*.md`
- **How to build it:** `implementation/*.md`
- **How to run it:** `deployment/*.md`
- **What to build next:** `roadmap/*.md`
- **External references:** `references/*.md`
