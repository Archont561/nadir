---
title: External Sources
purpose: All documentation links for engines, libraries, and references
last_updated: 2025-02-23
status: current
---

# External Sources

## Photogrammetry Engines

| Source | URL | Description |
|---|---|---|
| OpenDroneMap | https://docs.opendronemap.org/ | ODM docs: orthophotos, DEMs, workflows, hardware |
| COLMAP | https://colmap.github.io/ | SfM, MVS, CLI, camera models, reconstruction |
| COLMAP Tutorial | https://colmap.github.io/tutorial.html | Feature extraction → matching → SfM → MVS |
| COLMAP Format | https://colmap.github.io/format.html | cameras.bin, images.bin, points3D.bin format |
| COLMAP GitHub | https://github.com/colmap/colmap | Source code and issue tracker |
| OpenMVS | https://github.com/cdcseacave/openMVS | MVS: dense reconstruction, mesh, texturing |
| OpenSfM | https://github.com/mapillary/OpenSfM | Alternative SfM engine (Python) |
| MicMac | https://github.com/micmacIGN/micmac | French IGN photogrammetry suite |

## Geospatial Libraries

| Source | URL | Description |
|---|---|---|
| GDAL | https://gdal.org/ | Raster/vector processing, GeoTIFF, reprojection |
| PROJ | https://proj.org/ | CRS transformations, datum shifts |
| PROJ Transforms | https://proj.org/en/stable/operations/transformations/ | Helmert, grid-shift, affine |
| PROJ Helmert | https://proj.org/en/stable/operations/transformations/helmert.html | 3/4/7-parameter transforms |
| PDAL | https://pdal.io/ | Point cloud processing, LAS/LAZ, filtering |
| GeoRust | https://georust.org/ | Rust geospatial ecosystem |

## Math & Optimization

| Source | URL | Description |
|---|---|---|
| Ceres Solver | https://ceres-solver.org/ | Nonlinear least-squares (bundle adjustment) |
| OpenCV | https://opencv.org/ | Computer vision, calibration, features |
| nalgebra | https://nalgebra.rs/ | Rust linear algebra |

## Rust Crates

| Crate | URL | Used For |
|---|---|---|
| `gdal` | https://docs.rs/gdal | GDAL Rust bindings |
| `proj` | https://docs.rs/proj | PROJ Rust bindings |
| `image` | https://docs.rs/image | Image decoding |
| `kamadak-exif` | https://docs.rs/kamadak-exif | EXIF metadata |
| `las` | https://docs.rs/las | LAS/LAZ point clouds |
| `geo` | https://docs.rs/geo | Geospatial algorithms |
| `petgraph` | https://docs.rs/petgraph | Graph/DAG data structure |
| `object_store` | https://docs.rs/object_store | S3/GCS/Azure/local storage |
| `nalgebra` | https://docs.rs/nalgebra | Linear algebra |
| `sprs` | https://docs.rs/sprs | Sparse matrices |
| `argmin` | https://docs.rs/argmin | Optimization |
| `wgpu` | https://docs.rs/wgpu | GPU compute (WebGPU) |
| `wasm-bindgen` | https://docs.rs/wasm-bindgen | Rust ↔ JS interop |
| `tiff` | https://docs.rs/tiff | TIFF/GeoTIFF parsing |
| `blake3` | https://docs.rs/blake3 | Content hashing |
| `tracing` | https://docs.rs/tracing | Structured logging |

## Specifications

| Source | URL | Description |
|---|---|---|
| COG | https://www.cogeo.org/ | Cloud Optimized GeoTIFF specification |
| LAS 1.4 | https://www.asprs.org/divisions-committees/lidar-division | LAS point cloud format |
| GeoTIFF | https://gdal.org/drivers/raster/gtiff.html | GeoTIFF format reference |
| EPSG | https://epsg.org/ | CRS registry |

## Tools & Infrastructure

| Source | URL | Description |
|---|---|---|
| Pixi | https://pixi.sh/ | Package/environment manager (Conda) |
| Temporal | https://temporal.io/ | Durable workflow orchestration |
| NATS | https://nats.io/ | Event streaming (JetStream) |
| ConnectRPC | https://connectrpc.com/ | Type-safe RPC (protobuf) |
| Railway | https://railway.app/ | PaaS hosting |
| Fly.io | https://fly.io/ | PaaS with GPU support |
| Cloudflare R2 | https://developers.cloudflare.com/r2/ | S3-compatible object storage |
