---
type: Roadmap
title: Full Roadmap
description: "ADR-012-aligned V0.1 → V3.0 checklist and dependency boundaries"
purpose: ADR-012-aligned V0.1 → V3.0 checklist and dependency boundaries
last_updated: 2026-10-05
status: stable
related:
  - v0-mvp.md
  - v1-platform.md
  - v2-scale.md
  - wasm-vision.md
  - ../../../.knowledge/architecture/overview.md
  - ../decisions/012-v0-strict-sparse-reconstruction-profile.md
---

# Full Roadmap

## TL;DR

Nadir evolves from a qualified local sparse-reconstruction profile (V0.1), to
local mapping products that consume explicit artifact contracts (V1.0), to
worker-scale processing (V2.0), and finally to native/WASM/browser/edge
capabilities (V3.0). ADR-012 deliberately narrows the first release: V0.1
proves input immutability, qualified CPU-only COLMAP execution, verified
stage-level caching and canonical `SparseScene v1` output before any CRS,
dense or mapping-product claim.

## Versioning Philosophy

| Phase | Goal | Timeline |
|---|---|---|
| **V0.1** | One bounded calibrated ImageSet → canonical local `SparseScene v1` on one machine | Evidence-driven |
| **V0.2** | Sparse-profile inspection, policy tuning, cache diagnostics and fixture expansion | After V0.1 evidence |
| **V1.0** | Georeferenced dense and mapping products on one machine | After sparse/cache contracts are stable |
| **V2.0** | Worker service, large datasets, object storage and remote execution | After local product contracts are stable |
| **V3.0** | Native engines, GPU/WASM/browser/edge platform work | Long-term |

## V0.1 — Strict Sparse Reconstruction Profile

> `nadir process ./images` writes a validated local-coordinate `SparseScene v1`
> and run/cache evidence. It does **not** write a CRS, dense cloud, DSM,
> orthomosaic, mesh, worker job or GPU result.

- [x] Cargo workspace with core crate scaffolding and native binding probes
- [x] `clap` CLI probes: `process`, `inspect`, `plan`, `engines`, `crates`
- [x] Shared protocol/dispatcher scaffold for CLI, Python and TypeScript probes
- [ ] `nadir-artifacts`: invocation keys distinct from verified output-tree manifests
- [ ] `nadir-process`: qualified subprocess runner with typed engine failures
- [ ] `nadir-dataset`: one immutable bounded calibrated ImageSet snapshot
- [ ] Runtime qualification: locked Pixi developer profile and digest-pinned OCI user profile
- [ ] Untrusted-input containment and host-side candidate import
- [ ] COLMAP adapter: CPU-only feature extraction, exhaustive matching and mapper stages
- [ ] COLMAP binary parser and canonical `SparseScene v1` exporter
- [ ] Fixed-profile executor with per-stage lifecycle and verified cache hits
- [ ] CLI: `nadir process` emits sparse scene output copies/reflinks, not cache paths
- [ ] Qualification evidence: two fresh stores with equal stage-tree digests and a same-store run with zero COLMAP launches

**Deliverable:** Working local CLI for the strict sparse contract, plus documented
fixture evidence and cache proof.

## V0.2 — Sparse Reliability & Inspection

- [ ] Expand public/owned fixture sets and fault cases
- [ ] ImageSet preflight report for V0 limits and calibration failures
- [ ] Versioned sparse-scene quality policy distinct from structural validity
- [ ] Cache explain/verify commands for invocation keys and output-tree manifests
- [ ] Resume/retry diagnostics for abandoned staging directories
- [ ] Provenance/reporting for sparse reconstruction facts and policy verdicts

**Deliverable:** Users and developers can understand why a strict sparse run was
accepted, rejected or reused from cache.

## V1.0 — Mapping Products on Explicit Contracts

- [ ] Local-to-CRS georeferencing from `SparseScene v1` using GPS/RTK/GCP evidence
- [ ] PROJ-backed CRS conversion and explicit accuracy reporting
- [ ] OpenMVS densification adapter and shipping/qualification ADR
- [ ] Dense cloud validation and LAS/LAZ export
- [ ] DSM/DTM generation and surface quality gates
- [ ] GDAL/GeoTIFF/COG writers for DSMs and orthomosaics
- [ ] Declarative DAG recipes and adaptive product pruning
- [ ] Resource-aware single-machine scheduling

**Deliverable:** Georeferenced dense reconstruction and mapping products that
consume, rather than redefine, the V0 sparse/cache boundary.

## V2.0 — Worker Service & Scale

- [ ] Worker Protocol schemas and compatibility rules
- [ ] `nadir serve` HTTP/gRPC/Connect worker surface using the same executor
- [ ] Worker capability advertisement and lifecycle events
- [ ] Dataset splitting, tile processing and model/product merging
- [ ] Object-store integration and artifact persistence policies
- [ ] TypeScript and Python workflow SDKs over local/remote transports

**Deliverable:** Remote execution and large-dataset processing without changing
the local artifact contracts.

## V3.0 — Native/WASM/Edge Platform

- [ ] Native Rust replacements for selected surface and cartography operations
- [ ] Native feature/matching/SfM research path where justified
- [ ] WebGPU compute (`wgpu`) and GPU qualification profiles
- [ ] WASM/browser preview and edge tiling
- [ ] Dynamic engine plugin system
- [ ] Optional SaaS control plane

**Deliverable:** Full distributed platform with browser/edge capabilities and
selectively native engines.

## Dependency Matrix by Version

| Dependency | V0.1 | V0.2 | V1.0 | V2.0 | V3.0 |
|---|---|---|---|---|---|
| **Native Rust** | | | | | |
| tokio | ✅ | ✅ | ✅ | ✅ | ✅ |
| clap | ✅ | ✅ | ✅ | ✅ | ✅ |
| serde/toml | ✅ | ✅ | ✅ | ✅ | ✅ |
| blake3 | ✅ | ✅ | ✅ | ✅ | ✅ |
| petgraph / typed DAG model | fixed profile only | fixed profile only | ✅ recipes | ✅ | ✅ |
| nalgebra | local pose math | ✅ | ✅ georef | ✅ | ✅ |
| rayon | | | ✅ | ✅ | ✅ |
| image | ✅ snapshot validation | ✅ | ✅ | ✅ | ✅ |
| kamadak-exif | calibration metadata | ✅ | GPS/RTK evidence | ✅ | ✅ |
| tracing | ✅ | ✅ | ✅ | ✅ | ✅ |
| las | | | ✅ | ✅ | ✅ |
| geo/geo-types | | | ✅ | ✅ | ✅ |
| object_store | | | | ✅ | ✅ |
| wgpu | | | | | ✅ |
| **C/C++ Bindings** | | | | | |
| gdal crate / GDAL | | | ✅ products | ✅ | optional/native alternatives |
| proj crate / PROJ | | | ✅ CRS | ✅ | ✅ |
| opencv crate | | | optional | ✅ | ✅ |
| **External Engines** | | | | | |
| COLMAP | ✅ CPU sparse only | ✅ | ✅ | ✅ | optional/native alternatives |
| OpenMVS | | | ✅ dense/mesh | ✅ | optional/native alternatives |
| PDAL | | | ✅ filtering | optional | optional/native alternatives |
| **Infrastructure** | | | | | |
| OCI containment | ✅ local untrusted profile | ✅ | ✅ | ✅ | ✅ |
| Protobuf/Connect | | | | ✅ | ✅ |
| NATS JetStream | | | | | ✅ |
| Temporal | | | | | optional |
| PostgreSQL | | | | optional | optional |
| S3/R2 | | | | ✅ | ✅ |

## Roadmap Rules

1. **V0 proves the sparse/cache boundary.** Do not add georeferencing, dense
   reconstruction, GPU execution, HTTP serving or mapping products to V0.1.

2. **V1 consumes contracts.** Mapping products must start from explicit
   `SparseScene v1`, georeferencing, dense and raster contracts rather than
   reaching into engine-private workspaces.

3. **V2+ scales what is already qualified.** Workers, SDK workflows and native
   engines arrive after local execution semantics are stable.

## See Also

- [V0 MVP details](v0-mvp.md)
- [V1 Platform details](v1-platform.md)
- [V2 Scale details](v2-scale.md)
- [WASM vision](wasm-vision.md)
- [Architecture overview](../../../.knowledge/architecture/overview.md)
- [ADR-012](../decisions/012-v0-strict-sparse-reconstruction-profile.md)
