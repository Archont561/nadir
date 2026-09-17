---
type: Roadmap
title: Full Roadmap
description: "V0.1 → V3.0 complete checklist, dependency matrix, three rules"
purpose: V0.1 → V3.0 complete checklist, dependency matrix, three rules
last_updated: 2025-02-23
status: stable
related:
  - v0-mvp.md
  - v1-platform.md
  - v2-scale.md
  - wasm-vision.md
  - ../architecture/overview.md
---

# Full Roadmap

## TL;DR

Nadir evolves from a CLI orchestrator around external engines (V0) to a
production processing platform (V1) to a scalable distributed system with
native Rust engines (V2) to a full platform with WASM/browser/edge support
(V3). The architecture stays the same throughout — only the engine
implementations and deployment topology change.

## Versioning Philosophy

| Phase | Goal | Timeline |
|---|---|---|
| **V0.1** | Process drone images → orthomosaic on a single machine | Months |
| **V0.2** | Make processing reliable, resumable, inspectable | Months |
| **V1.0** | Production-grade local processing platform | ~1 year |
| **V2.0** | Scale to large datasets, begin native Rust engines | 1–2 years |
| **V3.0** | Distributed execution, full platform, WASM | 2+ years |

## V0.1 — Minimum Viable Pipeline

> `nadir process ./images` produces orthomosaic, DSM, point cloud.

- [ ] Cargo workspace with all crate scaffolding
- [ ] `clap` CLI: `process`, `inspect`, `plan`
- [ ] Layered config (`serde` + `figment` + `toml`)
- [ ] `nadir-process`: `tokio::process` subprocess runner
- [ ] `nadir-artifacts`: blake3 content hashing + cache
- [ ] `nadir-dataset`: EXIF/GPS via `kamadak-exif`
- [ ] `nadir-reconstruction`: COLMAP adapter (features, matching, SfM)
- [ ] COLMAP binary model parser (cameras.bin, images.bin, points3D.bin)
- [ ] `nadir-reconstruction`: OpenMVS adapter (InterfaceCOLMAP, DensifyPointCloud)
- [ ] `nadir-geometry`: Umeyama 7-param transform (`nalgebra`)
- [ ] `nadir-geometry`: PROJ CRS conversion
- [ ] `nadir-surface`: IDW DSM rasterizer (`rayon`)
- [ ] `nadir-cartography`: GeoTIFF/COG writer (`gdal` crate)
- [ ] `nadir-pipeline`: TOML DAG parser (`petgraph`)
- [ ] `nadir-executor`: topological task runner with cache check
- [ ] `pipelines/standard.toml`
- [ ] `nadir serve --port 8080` worker mode
- [ ] Docker deployment (inverted container model)

**Deliverable:** Working CLI + worker that processes drone imagery end-to-end.

## V0.2 — Reliability & Inspection

- [ ] Preflight inspector (GSD, overlap, GPS coverage)
- [ ] `nadir inspect ./flight` command
- [ ] QC gates (SfM registration %, reprojection RMSE)
- [ ] Resume interrupted runs (`nadir resume`)
- [ ] Retry policy (max_retries, retryable vs fatal)
- [ ] Provenance recording (lineage JSON per artifact)
- [ ] `nadir explain <artifact>` command
- [ ] Project report generator (`nadir report`)

**Deliverable:** Users can inspect, resume, and understand their results.

## V1.0 — Production Platform

- [ ] Adaptive matching strategy (exhaustive/spatial/vocab by image count)
- [ ] Adaptive SfM (pose priors for RTK, hierarchical for large datasets)
- [ ] Product recipes (`--product orthomosaic|terrain|3d-model|full`)
- [ ] DAG pruning (remove steps not needed for target product)
- [ ] Pipeline variants (fast-ortho, dem-only, model-only)
- [ ] DTM generation (PDAL SMRF ground classification)
- [ ] Mesh + texture pipeline (OpenMVS ReconstructMesh + TextureMesh)
- [ ] Point cloud export (LAS/LAZ via `las` crate)
- [ ] COLMAP dense MVS as alternative to OpenMVS
- [ ] GCP support (ground control points)
- [ ] Resource-aware scheduler (Tokio semaphores for CPU/GPU)
- [ ] Concurrent DAG branch execution

**Deliverable:** Flexible, adaptive, resource-aware processing platform.

## V2.0 — Scale & Native Engines

- [ ] Dataset splitting (spatial clustering for 10,000+ images)
- [ ] Sub-model alignment and merging
- [ ] Tiling in the pipeline DAG
- [ ] Native Rust SMRF/CSF ground classification
- [ ] Native Rust GeoTIFF reader/writer (simple cases)
- [ ] Native Rust orthorectification
- [ ] Protobuf schemas (task, artifact, worker, event, config)
- [ ] `nadir-worker`: gRPC/Connect server
- [ ] Worker capability advertisement
- [ ] TypeScript SDK (`@nadir/sdk`)
- [ ] Python SDK (`nadir-python`)
- [ ] `object_store` integration (S3, R2, GCS)
- [ ] Artifact persistence policies

**Deliverable:** Handles large datasets, multi-language SDKs, remote workers.

## V3.0 — Distributed Platform & WASM

- [ ] Multi-worker scheduler (capability matching + data locality)
- [ ] Durable artifact storage (S3/R2)
- [ ] NATS JetStream event transport
- [ ] Temporal workflow runtime (optional)
- [ ] Pure Rust math core (`nadir-math`, zero FFI)
- [ ] WASM bindings (`wasm-bindgen`)
- [ ] Browser photogrammetry (< 100 images)
- [ ] Edge tiling (Cloudflare Workers)
- [ ] WebGPU compute (`wgpu`)
- [ ] Dynamic engine plugin system
- [ ] SaaS control plane (optional)

**Deliverable:** Full distributed platform with browser/edge capabilities.

## Dependency Matrix by Version

| Dependency | V0.1 | V0.2 | V1.0 | V2.0 | V3.0 |
|---|---|---|---|---|---|
| **Native Rust** | | | | | |
| tokio | ✅ | ✅ | ✅ | ✅ | ✅ |
| clap | ✅ | ✅ | ✅ | ✅ | ✅ |
| serde/toml | ✅ | ✅ | ✅ | ✅ | ✅ |
| blake3 | ✅ | ✅ | ✅ | ✅ | ✅ |
| petgraph | ✅ | ✅ | ✅ | ✅ | ✅ |
| nalgebra | ✅ | ✅ | ✅ | ✅ | ✅ |
| rayon | ✅ | ✅ | ✅ | ✅ | ✅ |
| image | ✅ | ✅ | ✅ | ✅ | ✅ |
| kamadak-exif | ✅ | ✅ | ✅ | ✅ | ✅ |
| tracing | ✅ | ✅ | ✅ | ✅ | ✅ |
| las | | | ✅ | ✅ | ✅ |
| geo/geo-types | | | ✅ | ✅ | ✅ |
| object_store | | | | ✅ | ✅ |
| wgpu | | | | | ✅ |
| **C/C++ Bindings** | | | | | |
| gdal crate | ✅ | ✅ | ✅ | ✅ | ✅ |
| proj crate | ✅ | ✅ | ✅ | ✅ | ✅ |
| opencv crate | | | | ✅ | ✅ |
| **External Engines** | | | | | |
| COLMAP | ✅ | ✅ | ✅ | ✅ | ⚠️ opt |
| OpenMVS | ✅ | ✅ | ✅ | ✅ | ⚠️ opt |
| PDAL | | | ✅ | ⚠️ opt | ❌ |
| **Infrastructure** | | | | | |
| Protobuf/Connect | | | | ✅ | ✅ |
| NATS JetStream | | | | | ✅ |
| Temporal | | | | | ✅ |
| PostgreSQL | | | | | ✅ |
| S3/R2 | | | | ✅ | ✅ |

## The Three Rules

1. **V0 ships fast.** Use every external engine available. Don't write
   photogrammetry algorithms. Prove the platform concept.

2. **V1 ships smart.** Add the intelligence layer (QC, adaptive planning,
   caching) that makes Nadir better than a shell script.

3. **V2+ ships native.** Replace external engines only when you have a
   compelling reason (performance, deployment simplicity, GPU control).
   Never rewrite for ideology.

## See Also

- [V0 MVP details](./v0-mvp.md)
- [V1 Platform details](./v1-platform.md)
- [V2 Scale details](./v2-scale.md)
- [WASM vision](./wasm-vision.md)
- [Architecture overview](../architecture/overview.md)
