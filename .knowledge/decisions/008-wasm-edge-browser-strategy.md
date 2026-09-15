---
title: "ADR-008: WASM / Edge / Browser Strategy"
status: accepted
date: 2025-02-23
deciders: project lead
related:
  - ../roadmap/wasm-vision.md
  - ../architecture/overview.md
  - 002-three-language-split.md
---

# ADR-008: WASM / Edge / Browser Strategy

## TL;DR

Nadir's pure Rust math core (`nadir-math`) compiles to both native and
`wasm32` targets from the same codebase. This enables three deployment
topologies beyond the native server: **browser-native photogrammetry** for
small datasets (<100 images, privacy-first), **edge processing** for
lightweight post-processing (tiling, reprojection on Cloudflare Workers), and
**serverless WASM** for cost-efficient intermittent workloads. The heavy
reconstruction (SfM, dense MVS) stays on native GPU servers.

## Context

The Rust ecosystem has mature WASM support. Key crates Nadir depends on
already compile to `wasm32`:

| Crate | WASM Status | Used For |
|---|---|---|
| `nalgebra` | ✅ Full | Linear algebra, transforms, quaternions |
| `image` | ✅ Full | JPEG/TIFF/PNG decoding |
| `geo` / `geo-types` | ✅ Full | Geospatial types and algorithms |
| `blake3` | ✅ Full | Content hashing |
| `sprs` | ✅ Full | Sparse matrices (bundle adjustment) |
| `argmin` | ✅ Full | Optimization (Levenberg-Marquardt) |
| `tiff` | ✅ Full | GeoTIFF parsing |
| `las` | ✅ Full | LAS/LAZ point clouds |
| `wgpu` | ⚠️ WebGPU | GPU compute (Chrome only for now) |
| `gdal` (FFI) | ❌ No | Requires C library |
| `proj` (FFI) | ⚠️ Partial | Needs `proj-sys` WASM build |

This means most of the photogrammetry math can run in WASM without
modification. The FFI-dependent crates (GDAL, PROJ) are replaced with pure
Rust implementations for the WASM target.

## Decision

**Single codebase, three compilation targets.** The `nadir-math` crate uses
`#[cfg(target_arch)]` to provide native and WASM implementations. The trait
API (ADR-007) remains the same.

### Target 1: Browser (V3.0)

```
┌─────────────────────────────────────┐
│              Browser                 │
│                                      │
│  Web Workers + SharedArrayBuffer     │
│  WASM: features, matching, SfM,     │
│        DSM rasterization, tiling     │
│  OPFS: intermediate storage          │
│  Limit: ~50–100 images, ~2–4GB RAM  │
│                                      │
│  Use case: privacy-first field       │
│  survey, no upload required          │
└─────────────────────────────────────┘
```

### Target 2: Edge (V3.0)

```
┌─────────────────────────────────────┐
│        Edge Runtime                  │
│        (Cloudflare Workers)          │
│                                      │
│  WASM: reprojection, tiling, COG,   │
│        format conversion, stats      │
│  Cold start: ~50ms (vs ~2s Docker)  │
│  Limit: CPU-only, 128MB RAM         │
│                                      │
│  Use case: on-the-fly tile serving,  │
│  user-requested reprojection         │
└─────────────────────────────────────┘
```

### Target 3: Serverless WASM (V3.5)

```
┌─────────────────────────────────────┐
│     WASM Serverless                  │
│     (Fly Machines / Wasmtime)        │
│                                      │
│  WASM: full pipeline except dense   │
│  Cold start: ~50ms                   │
│  Scale to zero when idle             │
│                                      │
│  Use case: cost-efficient            │
│  intermittent processing             │
└─────────────────────────────────────┘
```

### What stays native

Dense MVS, large-scale SfM (>1000 images), and GPU-accelerated operations
remain on native servers. WASM is complementary, not a replacement.

### The crate structure

```toml
# crates/nadir-math/Cargo.toml
[lib]
crate-type = ["cdylib", "rlib"]  # cdylib for WASM, rlib for native

[dependencies]
nalgebra = "0.33"
sprs = "0.11"
argmin = "0.10"
geo = "0.29"
image = "0.25"

[target.'cfg(target_arch = "wasm32")'.dependencies]
wasm-bindgen = "0.2"
wasm-bindgen-rayon = "1.2"
getrandom = { version = "0.2", features = ["js"] }
```

### WASM bindings via wasm-bindgen

```rust
#[wasm_bindgen]
impl WasmPhotogrammetry {
    pub fn extract_features(&self, image: &[u8], max: u32) -> JsValue { ... }
    pub fn triangulate(&self, matches: JsValue, poses: JsValue) -> JsValue { ... }
    pub fn rasterize_dsm(&self, points: JsValue, res: f64) -> Float32Array { ... }
    pub fn bundle_adjust(&self, obs: JsValue, max_iter: u32) -> JsValue { ... }
}
```

## Rationale

| Factor | Native only | Native + WASM |
|---|---|---|
| Privacy | ❌ Must upload to server | ✅ Process locally in browser |
| Offline use | ❌ Requires server | ✅ Works without internet |
| Edge tiling | ❌ Server bottleneck | ✅ ~50ms cold start at edge |
| Cost | Fixed server cost | Pay-per-invocation for light tasks |
| Developer reach | Rust/TS/Python devs | Also web devs |
| Competitive moat | ⚠️ ODM clone | ✅ Nobody else does browser photogrammetry |
| Code maintenance | One target | Two targets (but same codebase) |

### The killer use case: privacy-first local processing

A field surveyor with sensitive infrastructure photos (military, energy,
telecom) can process 50 images entirely in their browser. No upload. No
server. No third-party access. This is impossible with ODM, WebODM, or any
existing cloud photogrammetry service.

### The competitive moat

No existing photogrammetry tool runs in the browser. ODM requires a server.
WebODM requires a Docker deployment. Pix4D and Agisoft are desktop apps.
A browser-native photogrammetry engine is a genuine differentiator.

## Consequences

### Positive
- Privacy-first local processing for small datasets
- Edge tiling with ~50ms cold start (vs ~2s Docker)
- Cost-efficient serverless for intermittent workloads
- Single codebase compiles to native + WASM
- Genuine competitive differentiator
- Web developers can integrate photogrammetry into web apps

### Negative
- Browser memory limits (~2–4GB practical)
- Browser CPU is slower than native (~5–10x for compute-heavy tasks)
- WebGPU not yet universal (Chrome yes, Firefox/Safari partial)
- FFI crates (GDAL, PROJ) need pure Rust replacements for WASM
- SharedArrayBuffer requires COOP/COEP headers (cross-origin isolation)

### Deferred
- V2.0: Pure Rust math core (`nadir-math`) with zero FFI
- V2.5: `wasm-bindgen` bindings, `wasm-pack` build
- V3.0: Browser pipeline (<100 images), edge tiling
- V3.5: Serverless WASM, WebGPU compute
- V4.0: Full hybrid (browser preview + edge tiles + server reconstruction)

## See Also

- [WASM vision](../roadmap/wasm-vision.md) — detailed V2+++ roadmap
- [ADR-002: Three-language split](./002-three-language-split.md) — language responsibilities
- [Architecture overview](../architecture/overview.md) — evolution path
