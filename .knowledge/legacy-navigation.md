---
type: Navigation Guide
title: Nadir Knowledge Base Index
description: "Master map for LLM context loading — load this after /context.md"
purpose: Master map for LLM context loading — load this after /context.md
last_updated: 2025-02-23
status: stable
---

# Nadir Knowledge Base Index

## Quick Start for LLMs

1. **Always load `/context.md` first.** It contains the full project summary
   in ~2,000 tokens: architecture, stack, decisions, status, naming.
2. **Load additional files based on the task** using the table below.
3. **Cross-references are explicit.** Every file links to related files via
   relative paths. Follow them when you need deeper context.

## Task-Based Loading Guide

| User asks about... | Load these files (after /context.md) |
|---|---|
| General project questions | Nothing more needed |
| System architecture | `architecture/overview.md`, `architecture/five-domains.md` |
| Pipeline / DAG / executor | `architecture/pipeline-dag.md` |
| Artifacts / caching / hashing | `architecture/artifact-model.md` |
| Engine API / traits / adapters | `architecture/engine-registry.md` |
| Worker protocol / distributed | `architecture/worker-protocol.md` |
| Photogrammetry concepts | `photogrammetry/pipeline-stages.md` |
| Which engine does what | `photogrammetry/engine-assignment.md` |
| COLMAP integration | `photogrammetry/colmap-integration.md` |
| OpenMVS integration | `photogrammetry/openmvs-integration.md` |
| GDAL / PROJ / PDAL / geo stack | `photogrammetry/geospatial-stack.md` |
| Crate layout / Cargo.toml | `implementation/crate-structure.md` |
| Rust types / structs | `implementation/core-types.md` |
| Subprocess management | `implementation/process-runner.md` |
| Configuration system | `implementation/config-model.md` |
| QC gates / preflight / adaptive | `implementation/qc-and-adaptive.md` |
| CLI design / commands | `implementation/cli-design.md` |
| Docker / containerization | `deployment/docker-strategy.md` |
| Hosting / PaaS comparison | `deployment/hosting-comparison.md` |
| Pixi / dev environment / CI | `deployment/pixi-setup.md` |
| Demo plan / 4-week sprint | `deployment/demo-plan.md` |
| Full roadmap | `roadmap/full-roadmap.md` |
| V0 MVP details | `roadmap/v0-mvp.md` |
| V1 platform details | `roadmap/v1-platform.md` |
| V2 scale / native engines | `roadmap/v2-scale.md` |
| WASM / browser / edge | `roadmap/wasm-vision.md` |
| "Why did we decide X?" | `decisions/001-*.md` through `decisions/010-*.md` |
| External docs / links | `references/sources.md` |
| Dependency versions | `references/dependency-matrix.md` |
| Everything (max context) | All files in all directories |

---

## Complete File Index

### Foundation (2 files)

| File | Description |
|---|---|
| [`/context.md`](.//context.md) | Project summary: architecture, stack, decisions, status, naming (~2k tokens) |
| [`/index.md`](.//index.md) | This file. Master map with task-based loading guide |

### Decisions — Architectural Decision Records (8 files)

| File | Title | Key Point |
|---|---|---|
| [`decisions/001-step-scoped-vs-odm-monolith.md`](./decisions/001-step-scoped-vs-odm-monolith.md) | Step-Scoped Tools vs ODM Monolith | Decompose pipeline into per-stage engine calls for caching, resume, parallelism |
| [`decisions/002-three-language-split.md`](./decisions/002-three-language-split.md) | Three-Language Split | TS for SDK/UI, Rust for worker/infrastructure, Python for future engine plugins |
| [`decisions/003-drop-python-adapter-for-mvp.md`](./decisions/003-drop-python-adapter-for-mvp.md) | Drop Python Adapter for MVP | Rust calls engines directly via `tokio::process`; config compilation is ~50 lines of `match` |
| [`decisions/004-inverted-container-no-dind.md`](./decisions/004-inverted-container-no-dind.md) | Inverted Container Model | No Docker-in-Docker on PaaS; the deployed container IS the engine environment |
| [`decisions/005-worker-topology-per-step.md`](./decisions/005-worker-topology-per-step.md) | Per-Step Worker Topology | Fat worker (V0) → domain workers (V1) → per-engine workers (V2); same binary, `--engines` flag |
| [`decisions/006-pixi-for-dependency-management.md`](./decisions/006-pixi-for-dependency-management.md) | Pixi for Dependency Management | Single `pixi.toml` + `pixi.lock` replaces apt/brew/rustup/nvm/Dockerfile RUN lines |
| [`decisions/007-engine-trait-api-surface.md`](./decisions/007-engine-trait-api-surface.md) | Engine Trait API Surface | 16 stable traits, small config structs, big adapters that hide tool-specific flags |
| [`decisions/008-wasm-edge-browser-strategy.md`](./decisions/008-wasm-edge-browser-strategy.md) | WASM / Edge / Browser Strategy | Rust → wasm32 for browser preview (<100 imgs), edge tiling, serverless post-processing |
| [`decisions/009-one-manifest-per-root.md`](./decisions/009-one-manifest-per-root.md) | One Manifest per Root | `pixi.toml` is workspace *and* `nadir-cli` package; `cargo install` cannot install from a virtual manifest |
| [`decisions/010-bun-not-node.md`](./decisions/010-bun-not-node.md) | Bun, Not Node | One JS runtime, one `bun.lock`; no `nodejs`/`pnpm` in any feature; tools via `bun x`, never `node_modules/.bin` |

### Architecture — System Design (6 files)

| File | Description |
|---|---|
| [`architecture/overview.md`](./architecture/overview.md) | Big picture: build system metaphor, 5 domains, 3 dependency tiers, V1→V3 evolution path |
| [`architecture/five-domains.md`](./architecture/five-domains.md) | Dataset, Reconstruction, Geometry, Surface, Cartography — crate groups, trait hierarchies, engine assignments |
| [`architecture/engine-registry.md`](./architecture/engine-registry.md) | 16 stable traits with full Rust signatures, `EngineRegistry` resolution, config structs, adapter pattern |
| [`architecture/pipeline-dag.md`](./architecture/pipeline-dag.md) | TOML pipeline format, `petgraph` DAG construction, topological executor, concurrent branches, `standard.toml` |
| [`architecture/artifact-model.md`](./architecture/artifact-model.md) | `ArtifactKind` enum, blake3 content hashing, hash chain invalidation, provenance, persistence policies |
| [`architecture/worker-protocol.md`](./architecture/worker-protocol.md) | Worker interface, protobuf schema, transport independence, `LocalRuntime`/`TemporalRuntime`/`CloudRuntime`, SaaS separation |

### Photogrammetry — Domain Knowledge (5 files)

| File | Description |
|---|---|
| [`photogrammetry/pipeline-stages.md`](./photogrammetry/pipeline-stages.md) | Full 16-stage walkthrough: images → EXIF → features → matching → SfM → bundle adjustment → georeferencing → dense MVS → filtering → DSM → DTM → mesh → texture → orthorectification → mosaic. Includes math (GSD, pinhole, reprojection error, triangulation) and ASCII diagrams |
| [`photogrammetry/engine-assignment.md`](./photogrammetry/engine-assignment.md) | Stage → tool mapping table, 3 computational sub-domains (vision/geometry/GIS), why each tool was chosen, V1→V3 replacement path |
| [`photogrammetry/colmap-integration.md`](./photogrammetry/colmap-integration.md) | 22 COLMAP CLI commands mapped to Nadir tasks, binary format parser (`cameras.bin`, `images.bin`, `points3D.bin`), adapter code, progress parsing, resume behavior |
| [`photogrammetry/openmvs-integration.md`](./photogrammetry/openmvs-integration.md) | OpenMVS tool chain (`InterfaceCOLMAP`, `DensifyPointCloud`, `ReconstructMesh`, `TextureMesh`), MVS scene format, COLMAP→OpenMVS data conversion, adapter code |
| [`photogrammetry/geospatial-stack.md`](./photogrammetry/geospatial-stack.md) | GDAL, PROJ, PDAL, nalgebra, geo — what each does, Rust crate bindings, library vs CLI usage, dependency tier rationale |

### Implementation — How to Build It (6 files)

| File | Description |
|---|---|
| [`implementation/crate-structure.md`](./implementation/crate-structure.md) | Cargo workspace layout, per-crate responsibilities, inter-crate dependency graph, `[workspace.dependencies]` |
| [`implementation/core-types.md`](./implementation/core-types.md) | Key Rust structs with full code: `ImageData`, `CameraModel`, `CameraPose`, `SparsePointCloud`, `DensePointCloud`, `RasterGrid`, `SimilarityTransform`, `Artifact`, `Task` |
| [`implementation/process-runner.md`](./implementation/process-runner.md) | Generic `tokio::process` supervisor: `ProcessSpec`, `ProgressParser` trait, cancellation via `CancellationToken`, timeout, stdout/stderr streaming |
| [`implementation/config-model.md`](./implementation/config-model.md) | Layered config precedence (defaults → system → project → mission → env → CLI), `serde`/`figment`, `NadirConfig` struct, per-domain config structs |
| [`implementation/qc-and-adaptive.md`](./implementation/qc-and-adaptive.md) | Preflight inspector (GSD, overlap, GPS), QC gates (reprojection RMSE, coverage), adaptive planner (matching strategy by image count), product recipes, DAG pruning |
| [`implementation/cli-design.md`](./implementation/cli-design.md) | CLI commands (`process`, `inspect`, `plan`, `serve`, `resume`, `explain`, `report`), `--serve` mode for worker, JSON output, UX patterns |

### Deployment — How to Run It (4 files)

| File | Description |
|---|---|
| [`deployment/docker-strategy.md`](./deployment/docker-strategy.md) | Inverted container model, multi-stage Dockerfile (Pixi builder → engine runtime), lean production variant, local vs PaaS usage |
| [`deployment/hosting-comparison.md`](./deployment/hosting-comparison.md) | Railway vs Render vs Vercel vs Fly.io vs Hetzner comparison table, photogrammetry resource requirements, demo hosting cost (~$25/mo) |
| [`deployment/pixi-setup.md`](./deployment/pixi-setup.md) | Full `pixi.toml` with features/environments (default, full, web, python, wasm), developer onboarding, Docker integration, GitHub Actions CI |
| [`deployment/demo-plan.md`](./deployment/demo-plan.md) | 4-week demo plan, what to cut, tech stack (Node.js BFF + Next.js UI + Rust CLI worker), demo script, BFF code, UI code |

### Roadmap — Version Planning (5 files)

| File | Description |
|---|---|
| [`roadmap/full-roadmap.md`](./roadmap/full-roadmap.md) | V0.1 → V3.0 complete checklist with checkboxes, dependency matrix by version, 3 rules |
| [`roadmap/v0-mvp.md`](./roadmap/v0-mvp.md) | V0.1 (CLI + engines + pipeline) and V0.2 (preflight, QC, resume, provenance, reporting) detailed milestones and deliverables |
| [`roadmap/v1-platform.md`](./roadmap/v1-platform.md) | V1.0: adaptive planning, product recipes, pipeline variants, mesh/texture, GCP, resource scheduling |
| [`roadmap/v2-scale.md`](./roadmap/v2-scale.md) | V2.0: dataset splitting, native Rust surface engines, worker protocol (protobuf), multi-language SDK |
| [`roadmap/wasm-vision.md`](./roadmap/wasm-vision.md) | V2+++: pure Rust math core → WASM bindings → browser photogrammetry → edge tiling → hybrid architecture |

### References — External Sources (2 files)

| File | Description |
|---|---|
| [`references/sources.md`](./references/sources.md) | All external documentation links: COLMAP, OpenMVS, GDAL, PROJ, PDAL, ODM, GeoRust, nalgebra, Ceres, OpenCV |
| [`references/dependency-matrix.md`](./references/dependency-matrix.md) | Rust crates + external tools organized by version and tier, WASM compatibility status |

---

## Maintenance Rules

1. **Update after every significant conversation.** Merge new decisions and
   specs into the relevant files. Do not let knowledge live only in chat history.
2. **One decision per ADR.** Do not bundle unrelated decisions into one file.
3. **Keep `/context.md` under 2,500 tokens.** It is the hot path for every session.
4. **Cross-reference explicitly.** Use relative markdown paths, not "as we
   discussed earlier." LLMs have no memory between sessions.
5. **Include code examples in implementation files.** LLMs generate better code
   when they can see the target types and signatures.
6. **Mark status in YAML frontmatter.** Use `current`, `draft`, `superseded`
   for specs. Use `accepted`, `proposed`, `deprecated` for decisions.
7. **Re-run Batch 1 after major changes.** `/context.md` and `/index.md` should
   always reflect the current state of all other files.
