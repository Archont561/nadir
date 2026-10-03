---
type: Reference
title: Dependency Matrix
description: "All Rust crates and external tools with versions, tiers, and WASM status"
purpose: All Rust crates and external tools with versions, tiers, and WASM status
last_updated: 2026-09-30
status: stable
related:
  - sources.md
  - ../../backlog/docs/decisions/006-pixi-for-dependency-management.md
---

# Dependency Matrix

## Rust Crates (Tier 1: Native)

| Crate | Version | Purpose | WASM |
|---|---|---|---|
| `tokio` | 1.x | Async runtime, subprocesses, I/O | ⚠️ partial |
| `rayon` | 1.10 | CPU parallelism | ✅ (wasm-bindgen-rayon) |
| `clap` | 4.x | CLI argument parsing | N/A |
| `serde` | 1.x | Serialization/deserialization | ✅ |
| `serde_json` | 1.x | JSON serialization | ✅ |
| `toml` | 0.8 | TOML config parsing | ✅ |
| `figment` | 0.10 | Layered configuration | ✅ |
| `blake3` | 1.x | Content hashing | ✅ |
| `petgraph` | 0.6 | DAG / graph data structure | ✅ |
| `nalgebra` | 0.33 | Linear algebra, transforms | ✅ |
| `sprs` | 0.11 | Sparse matrices | ✅ |
| `argmin` | 0.10 | Optimization (LM, CG) | ✅ |
| `geo` | 0.29 | Geospatial algorithms | ✅ |
| `geo-types` | 0.7 | Geospatial types | ✅ |
| `image` | 0.25 | Image decoding (JPEG/TIFF/PNG) | ✅ |
| `kamadak-exif` | 0.5 | EXIF metadata extraction | ✅ |
| `las` | 0.9 | LAS/LAZ point cloud I/O | ✅ |
| `tiff` | 0.9 | TIFF parsing | ✅ |
| `tracing` | 0.1 | Structured logging | ✅ |
| `tracing-subscriber` | 0.3 | Log output formatting | ✅ |
| `anyhow` | 1.x | Error handling (application) | ✅ |
| `thiserror` | 2.x | Error handling (library) | ✅ |
| `uuid` | 1.x | Unique identifiers | ✅ |
| `chrono` | 0.4 | Date/time | ✅ |
| `indicatif` | 0.17 | CLI progress bars | N/A |
| `regex` | 1.x | Progress parsing | ✅ |
| `which` | 6.x | Binary discovery | ⚠️ |
| `object_store` | 0.11 | S3/GCS/Azure/local storage | ⚠️ |
| `wgpu` | 23.x | GPU compute (WebGPU) | ✅ (WebGPU) |
| `wasm-bindgen` | 0.2 | Rust ↔ JS interop | ✅ (target) |
| `wasm-bindgen-rayon` | 1.2 | Threading in browser | ✅ |

## C/C++ Bindings (Tier 2: FFI)

| Crate | Version | Wraps | WASM |
|---|---|---|---|
| `gdal` | 0.17 | GDAL C library | ❌ |
| `proj` | 0.28 | PROJ C library | ⚠️ needs WASM build |
| `opencv` | 0.93 | OpenCV C++ library | ❌ |

## External Engines (Tier 3: Subprocess)

| Engine | Version | Pixi Package | Purpose |
|---|---|---|---|
| COLMAP | ≥ 3.9 | `colmap` | Features, matching, SfM, BA |
| OpenMVS | ≥ 2.2 | `openmvs` (if available) | Dense MVS, mesh, texture |
| PDAL | ≥ 2.7 | `pdal` | Point cloud filtering, classification |
| GDAL CLI | ≥ 3.9 | `gdal` | Raster reprojection, warping |
| PROJ CLI | ≥ 9.4 | `proj` | CRS transformations |
| FFmpeg | any | `ffmpeg` | Video frame extraction (optional) |

## System Libraries (via Pixi)

| Library | Version | Pixi Package | Used By |
|---|---|---|---|
| libgdal | ≥ 3.9 | `gdal` | `gdal` crate |
| libproj | ≥ 9.4 | `proj` | `proj` crate |
| libgeos | ≥ 3.12 | `geos` | `geo` crate (optional) |
| libopencv | ≥ 4.9 | `libopencv` | `opencv` crate |
| laszip | any | `laszip` | `las` crate (LAZ support) |
| libtiff | any | `libtiff` | `tiff` crate |
| cmake | ≥ 3.28 | `cmake` | Building C++ deps |
| ninja | any | `ninja` | Build system |
| pkg-config | any | `pkg-config` | Finding C libraries |

## Dev Tools (via Pixi)

| Tool | Version | Pixi Package | Environment |
|---|---|---|---|
| Rust | ≥ 1.98.1, < 1.99 | `rust` | default (`rust` feature) |
| Bun | ≥ 1.3.11, < 2 | `bun` | default (`js` feature) |
| Python | ≥ 3.13, < 3.14 | `python` | default (`python` feature) |
| cargo-nextest | ≥ 0.9.144 | `cargo-nextest` | default (`rust` feature) |
| cargo-llvm-cov | ≥ 0.9.1 | `cargo-llvm-cov` | default (`rust` feature) |
| cargo-deny | ≥ 0.20.2 | `cargo-deny` | default (`rust` feature) |
| ruff | ≥ 0.14, < 0.15 | `ruff` | default (`python` feature) |
| convco | ≥ 0.7.2 | `convco` | default (`utils` feature) |
| lefthook | ≥ 2.1.15 | `lefthook` | default (`utils` feature) |
| actionlint | ≥ 1.7.12 | `actionlint` | default (`utils` feature) |
| turbo | npm | *not conda* — Bun workspace dependency | root `package.json` |
| biome | npm | *not conda* — Bun workspace dependency | root `package.json` |
| wasm-pack | any | `wasm-pack` | *not declared* — no `wasm` environment yet |
| CUDA | ≥ 12.0 | `cuda-toolkit` | *not declared* — no `gpu` environment yet |

> **Node.js and pnpm are deliberately absent** (2026-09-30). The rows that listed
> `nodejs = "22.*"` and `pnpm = ">=9"` under a `web` environment described the design-era
> plan; Bun is the runtime, the package manager and the test runner, and there is one
> environment (`default`) rather than a per-feature set of them. See
> [ADR-010](../../backlog/docs/decisions/010-bun-not-node.md). Versions above are the pins in
> [`/pixi.toml`](../../pixi.toml), which is authoritative.

## Infrastructure (V2.0+)

| Tool | Version | Purpose | When |
|---|---|---|---|
| Protobuf | 3.x | Wire format | V2.0 |
| ConnectRPC | latest | Type-safe RPC | V2.0 |
| NATS JetStream | 2.x | Event streaming | V3.0 |
| Temporal | latest | Durable workflows | V3.0 |
| PostgreSQL | 16.x | Metadata storage | V3.0 |
| S3/R2 | — | Object storage | V2.0 |

## See Also

- [Sources](./sources.md) — documentation links
- [Pixi setup](../deployment/pixi-setup.md) — how these are installed
- [ADR-006: Pixi](../../backlog/docs/decisions/006-pixi-for-dependency-management.md)
