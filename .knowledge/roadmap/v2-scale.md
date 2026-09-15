---
title: V2 Scale Details
purpose: V2.0 native engines, splitting, worker protocol, multi-language SDK
last_updated: 2025-02-23
status: current
related:
  - full-roadmap.md
  - v1-platform.md
  - ../architecture/worker-protocol.md
---

# V2 Scale Details

## V2.0 — Scale & Native Engines

**Goal:** Handle 10,000+ image datasets, begin replacing external engines
with native Rust, introduce the Worker Protocol for remote execution.

**Timeline:** 1–2 years after V1.0.

### Large Dataset Support

- [ ] `DatasetPartitioner` trait
- [ ] Spatial clustering partition (k-means on GPS)
- [ ] Overlap-aware splitting (tile boundary overlap)
- [ ] Sub-dataset artifact generation
- [ ] COLMAP model_merger adapter
- [ ] Point cloud merging with deduplication
- [ ] Raster mosaicking across tiles
- [ ] `partition → [tile_A, tile_B, tile_C] → merge` DAG pattern
- [ ] Parallel tile processing

### Native Rust Engine Replacements

- [ ] SMRF ground classification in pure Rust
- [ ] CSF ground classification in pure Rust
- [ ] Rayon-parallel point cloud iteration
- [ ] GeoTIFF reader/writer via `tiff` crate (simple cases)
- [ ] COG layout writer (tiled, overviewed, DEFLATE)
- [ ] Native Rust orthorectification (ray-surface intersection)
- [ ] Seam-line computation for mosaic blending
- [ ] Config switch: `engine = "builtin"` vs `"pdal"` vs `"gdal"`

### Worker Protocol (Phase 1)

- [ ] Protobuf schemas: task.proto, artifact.proto, worker.proto, event.proto, config.proto
- [ ] Rust bindings (`prost`)
- [ ] TypeScript bindings (`@connectrpc`)
- [ ] `nadir-worker`: gRPC/Connect server
- [ ] Task execution via existing executor
- [ ] Heartbeat and capability advertisement
- [ ] Workspace isolation per task

### Multi-Language SDK

- [ ] TypeScript SDK (`@nadir/sdk`)
  - [ ] `nadir.mapping({ quality: "survey", outputs: { orthomosaic: true } })`
  - [ ] Zod validation on protobuf types
  - [ ] Local + remote transports
  - [ ] Event streaming
- [ ] Python SDK (`nadir-python`)
  - [ ] `nadir.mapping(input="./images", quality=Quality.SURVEY)`
  - [ ] Pydantic validation
  - [ ] Local + remote transports

### Expected CLI Output

```bash
$ nadir process ./mega_flight --product orthomosaic
Preflight: 12,847 images, RTK, GSD 1.8cm
Strategy:  vocab tree matching, hierarchical SfM, 4-tile split

Partitioning into 4 tiles...
Tile A: 3,412 images  Tile B: 3,287 images
Tile C: 3,198 images  Tile D: 2,950 images

Processing tiles in parallel (2 concurrent, 16 cores each)...
✓ tile_A  2h 14m
✓ tile_B  2h 08m
✓ tile_C  1h 57m
✓ tile_D  1h 42m

Merging tiles...
✓ merge   12m 33s
✓ ortho   18m 04s  (COG, 2.1 GB)
```

## See Also

- [Full roadmap](./full-roadmap.md)
- [Worker protocol](../architecture/worker-protocol.md)
- [WASM vision](./wasm-vision.md)
