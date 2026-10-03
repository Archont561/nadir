---
type: Domain Guide
title: Engine Assignment
description: "Which tool handles which stage, why, and the V0→V3 replacement path"
purpose: Which tool handles which stage, why, and the V0→V3 replacement path
last_updated: 2025-02-23
status: stable
related:
  - pipeline-stages.md
  - geospatial-stack.md
  - ../architecture/engine-registry.md
  - ../../backlog/docs/decisions/001-step-scoped-vs-odm-monolith.md
---

# Engine Assignment

## TL;DR

Each pipeline stage is assigned to the best available tool. COLMAP handles
the hard SfM math. OpenMVS handles dense 3D. PROJ handles coordinate math.
GDAL handles raster I/O. Rust handles orchestration, caching, geospatial
glue, and progressively replaces surface/cartography operations.

## Stage → Engine Map

| Stage | What it does | V0 Engine | V2 Engine | V3 Engine |
|---|---|---|---|---|
| Image discovery | Find/validate images | **Rust** | Rust | Rust |
| EXIF/GPS | Camera metadata | **Rust** (kamadak-exif) | Rust | Rust |
| Image decode | JPEG/TIFF | **Rust** (image crate) | Rust | Rust |
| Preflight | Overlap/GSD check | **Rust** | Rust | Rust |
| Camera calibration | Intrinsics/distortion | **COLMAP** | COLMAP | Rust |
| Feature extraction | SIFT/ORB keypoints | **COLMAP** | COLMAP | Rust/GPU |
| Feature matching | Image correspondences | **COLMAP** | COLMAP | Rust/GPU |
| Track construction | Merge pairwise matches | **COLMAP** | COLMAP | Rust |
| SfM | Camera poses + sparse 3D | **COLMAP** | COLMAP | Rust |
| Bundle adjustment | Optimize cameras+points | **COLMAP** (Ceres) | COLMAP | Rust |
| GPS/RTK integration | Georeference | **Rust** + PROJ | Rust + PROJ | Rust + PROJ |
| GCP alignment | Survey control | **Rust** + COLMAP | Rust | Rust |
| Dense reconstruction | Depth maps / dense cloud | **OpenMVS** | OpenMVS | Rust/GPU |
| Point cloud filter | Remove noise | **PDAL** | **Rust** | Rust |
| Ground classification | Ground vs non-ground | **PDAL** (SMRF) | **Rust** | Rust/ML |
| DSM | Surface elevation | **Rust** + GDAL | **Rust** | Rust |
| DTM | Terrain elevation | **Rust** + GDAL | **Rust** | Rust |
| Mesh | 3D surface | **OpenMVS** | OpenMVS | Rust |
| Texturing | Project images on mesh | **OpenMVS** | OpenMVS | Rust |
| Orthorectification | Image → ground | **Rust** + GDAL | **Rust** | Rust |
| Mosaicking | Merge orthophotos | **Rust** + GDAL | **Rust** | Rust |
| CRS transforms | WGS84/UTM/etc. | **PROJ** | PROJ | PROJ |
| Raster I/O | GeoTIFF/COG | **GDAL** | GDAL/Rust | Rust |
| Point cloud I/O | LAS/LAZ | **Rust** (las crate) | Rust | Rust |
| Tiling | Spatial partition | **Rust** | Rust | Rust |
| Pipeline/DAG | Task orchestration | **Rust** | Rust | Rust |
| Cache | Artifact reuse | **Rust** | Rust | Rust |
| Provenance | Reproducibility | **Rust** | Rust | Rust |

## Why These Assignments

### COLMAP = "figure out the camera geometry"

This is the hardest mathematical part of photogrammetry. COLMAP provides:
- State-of-the-art incremental SfM
- Ceres-based bundle adjustment
- Multiple matching strategies (exhaustive, spatial, vocab tree)
- Multiple reconstruction strategies (incremental, hierarchical, global)
- GPU-accelerated feature extraction (SIFT)
- PatchMatch stereo for dense reconstruction

Don't initially implement: SIFT, matching, RANSAC, track building,
incremental SfM, global SfM, bundle adjustment. That's decades of research.

### OpenMVS = "turn cameras into dense 3D"

After SfM produces camera poses and sparse points, OpenMVS:
- Imports COLMAP models via `InterfaceCOLMAP`
- Performs multi-view stereo densification
- Generates meshes (Poisson/Delaunay)
- Textures meshes from source images

Keeps dense reconstruction replaceable.

### PROJ = "coordinate system math"

Delegate completely. PROJ handles:
- 7,000+ CRS definitions
- Helmert/Molodensky/NTv2 datum transformations
- Compound CRS (horizontal + vertical)
- Grid-shift corrections

You will never rewrite this. It's a 40-year-old international standard.

### GDAL = "geospatial plumbing"

Use heavily at first for:
- GeoTIFF/COG creation and reading
- Raster reprojection and warping
- Geotransform management
- Overview/pyramid generation
- Multi-band raster handling

Progressively replace with pure Rust for simple operations (single-band
GeoTIFF write, basic resampling).

### PDAL = "specialized point cloud processing"

Use selectively for:
- SMRF ground classification
- Statistical outlier filtering
- Point cloud reprojection
- LAS/LAZ format conversion

Replace with Rust for V2 (SMRF and statistical filters are straightforward
to implement with rayon).

### Rust = "everything else"

Rust owns:
- Pipeline orchestration (DAG, executor, scheduler)
- Artifact management (hashing, caching, provenance)
- Georeferencing math (Umeyama transform via nalgebra)
- DSM/DTM rasterization (IDW via rayon)
- Orthorectification (ray-surface intersection)
- Mosaicking (seam blending)
- CLI and configuration
- All I/O between stages

This is where the differentiation lives.

## The Three Sub-Domains

```
                 Rust pipeline
                       │
        ┌──────────────┼──────────────┐
        ▼              ▼              ▼
   COMPUTER VISION   GEOMETRY       GIS
        │              │              │
     COLMAP         nalgebra       GDAL/PROJ
     OpenCV         SfM/MVS        geo/raster
     OpenMVS                       PDAL/pointcloud
        │              │              │
        └──────────────┼──────────────┘
                       ▼
                   ARTIFACTS
```

### Computer Vision (Domain 2A)
Features, matching. Primarily COLMAP. Eventually Rust/GPU.

### Geometry (Domain 2B + 3)
SfM, MVS, bundle adjustment, georeferencing. COLMAP + OpenMVS + nalgebra.
The hardest math. Last to replace.

### GIS (Domain 4 + 5)
DSM, DTM, orthomosaic, tiling, CRS. GDAL + PROJ + Rust. First to replace
with native Rust because the algorithms are well-specified and have good
Rust crate support.

## The Replacement Priority

Replace in this order (easiest/most valuable first):

1. **Point cloud filtering** — statistical outlier is ~50 lines of Rust
2. **Ground classification** — SMRF is ~200 lines of Rust
3. **DSM/DTM rasterization** — IDW is ~100 lines of Rust + rayon
4. **Orthorectification** — ray-surface intersection, parallelizable
5. **Feature extraction** — ORB/AKAZE in pure Rust (no GPU needed)
6. **Feature matching** — approximate nearest neighbor
7. **Mesh generation** — Poisson/Delaunay (complex but well-documented)
8. **SfM** — incremental SfM (very complex, years of work)
9. **Dense MVS** — GPU plane-sweep stereo (requires wgpu/CUDA)
10. **Bundle adjustment** — sparse nonlinear optimization (Ceres-level)
11. **PROJ** — never. Use the standard.

## See Also

- [Pipeline stages](./pipeline-stages.md) — what each stage does
- [Geospatial stack](./geospatial-stack.md) — GDAL/PROJ/PDAL detail
- [Engine registry](../architecture/engine-registry.md) — trait implementations
- [COLMAP integration](./colmap-integration.md) — COLMAP adapter detail
- [OpenMVS integration](./openmvs-integration.md) — OpenMVS adapter detail
