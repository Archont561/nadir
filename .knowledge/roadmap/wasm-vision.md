---
title: WASM / Edge / Browser Vision
purpose: V2+++ pure Rust math, WASM compilation, browser/edge/serverless topologies
last_updated: 2025-02-23
status: current
related:
  - full-roadmap.md
  - v2-scale.md
  - ../decisions/008-wasm-edge-browser-strategy.md
---

# WASM / Edge / Browser Vision

## TL;DR

Nadir's pure Rust math core (`nadir-math`) compiles to both native and
`wasm32` targets. This enables browser-native photogrammetry for small
datasets, edge tiling on Cloudflare Workers, and serverless WASM processing.
Heavy reconstruction stays on native GPU servers. Same codebase, three targets.

## WASM Compatibility Status

| Crate | WASM | Used For |
|---|---|---|
| `nalgebra` | ✅ | Linear algebra, transforms |
| `image` | ✅ | Image decoding |
| `geo` / `geo-types` | ✅ | Geospatial types |
| `blake3` | ✅ | Hashing |
| `sprs` | ✅ | Sparse matrices (bundle adjustment) |
| `argmin` | ✅ | Optimization |
| `tiff` | ✅ | GeoTIFF parsing |
| `las` | ✅ | Point clouds |
| `wgpu` | ⚠️ WebGPU | GPU compute (Chrome) |
| `gdal` (FFI) | ❌ | Needs pure Rust replacement |
| `proj` (FFI) | ⚠️ | Needs WASM build |

## The Three Topologies

### Browser (V3.0)

Process 50–100 images entirely in the browser. No upload. No server.
Privacy-first for sensitive data (military, infrastructure).

- Web Workers + SharedArrayBuffer for threading
- OPFS for intermediate storage
- `wasm-bindgen-rayon` for parallel computation
- Limit: ~2–4 GB RAM, minutes not hours

### Edge (V3.0)

Lightweight post-processing on Cloudflare Workers.

- Reprojection, tiling, COG creation, format conversion
- ~50ms cold start (vs ~2s Docker)
- 128 MB RAM limit
- Pay-per-invocation

### Serverless WASM (V3.5)

Full pipeline (except dense MVS) on Fly Machines / Wasmtime.

- Scale to zero when idle
- ~50ms cold start
- Cost-efficient for intermittent workloads

## Implementation Path

### V2.0: Pure Rust Math Core

```
crates/nadir-math/
├── linalg.rs       # nalgebra wrappers, SVD
├── bundle.rs       # Bundle adjustment (argmin + sprs)
├── triangulate.rs  # 3D point triangulation
├── epipolar.rs     # Essential/fundamental matrix
├── idw.rs          # DSM/DTM rasterization
├── smrf.rs         # Ground classification
├── reproject.rs    # Pixel → ground projection
└── transform.rs    # Similarity/Helmert transforms
```

`crate-type = ["cdylib", "rlib"]` — cdylib for WASM, rlib for native.

### V2.5: WASM Bindings

```rust
#[wasm_bindgen]
impl WasmPhotogrammetry {
    pub fn extract_features(&self, image: &[u8], max: u32) -> JsValue;
    pub fn triangulate(&self, matches: JsValue, poses: JsValue) -> JsValue;
    pub fn rasterize_dsm(&self, points: JsValue, res: f64) -> Float32Array;
    pub fn bundle_adjust(&self, obs: JsValue, max_iter: u32) -> JsValue;
}
```

### V3.0: Browser Pipeline

```typescript
import init, { WasmPhotogrammetry } from "@nadir/math";
await init();
const pg = new WasmPhotogrammetry();
const features = pg.extract_features(imageBuffer, 8192);
const reconstruction = pg.sparse_reconstruction(matches);
const dsm = pg.rasterize_dsm(dense.points, 0.05);
renderGeoTiffOnMap(dsm);
```

### V3.5: Edge Pipeline

```typescript
// Cloudflare Worker
export default {
  async fetch(request, env) {
    const pg = new WasmPhotogrammetry();
    const tile = pg.extract_tile(ortho, z, x, y);
    return new Response(tile, { headers: { "Content-Type": "image/png" } });
  }
};
```

### V4.0: Full Hybrid

```
Browser: preview, interact, measure (< 100 images)
Edge:    tile, reproject, convert, compress
Server:  reconstruct, dense, mesh (GPU)
```

Same Rust codebase. Three compilation targets.

## The Competitive Moat

No existing photogrammetry tool runs in the browser. ODM requires a server.
WebODM requires Docker. Pix4D and Agisoft are desktop apps. Browser-native
photogrammetry is a genuine differentiator.

## See Also

- [ADR-008: WASM strategy](../decisions/008-wasm-edge-browser-strategy.md)
- [Full roadmap](./full-roadmap.md)
- [V2 Scale](./v2-scale.md)
