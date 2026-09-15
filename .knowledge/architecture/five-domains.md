---
title: Five Computational Domains
purpose: Detailed breakdown of Dataset, Reconstruction, Geometry, Surface, Cartography
last_updated: 2025-02-23
status: current
related:
  - overview.md
  - engine-registry.md
  - ../photogrammetry/engine-assignment.md
  - ../photogrammetry/pipeline-stages.md
---

# Five Computational Domains

## TL;DR

The photogrammetry pipeline decomposes into five domains. Each domain maps to
a Rust crate group, a set of traits, and one or more engine implementations.
The domains are: **Dataset** (image ingest), **Reconstruction** (3D geometry),
**Geometry** (coordinate systems), **Surface** (elevation models), and
**Cartography** (map products).

## Domain 1: Dataset

**Crate:** `nadir-dataset`
**Responsibility:** Image discovery, EXIF/GPS extraction, camera model
identification, dataset validation, preflight inspection.
**Engines:** Rust native only. No external tools needed.

### Key types

```rust
pub struct ImageData {
    pub path: PathBuf,
    pub width: u32,
    pub height: u32,
    pub gps: Option<GeoPosition>,
    pub timestamp: Option<DateTime<Utc>>,
    pub intrinsics: CameraIntrinsics,
}

pub struct CameraIntrinsics {
    pub focal_length: Option<f64>,
    pub sensor_width: Option<f64>,
    pub width: u32,
    pub height: u32,
}

pub struct GeoPosition {
    pub lat: f64,
    pub lon: f64,
    pub alt: f64,
}

pub struct ImageMetadataSet {
    pub images: Vec<ImageData>,
}
```

### Key operations

| Operation | Implementation | Dependencies |
|---|---|---|
| Image discovery | `std::fs::read_dir` + extension filter | None |
| EXIF parsing | `kamadak-exif` crate | None |
| GPS extraction | EXIF GPS tag parsing | `kamadak-exif` |
| Image dimensions | `image::image_dimensions` | `image` crate |
| Focal length | EXIF FocalLength tag | `kamadak-exif` |
| Preflight inspection | GSD + overlap estimation | `nalgebra` |

### Preflight report

The preflight stage runs before any heavy compute:

```rust
pub struct PreflightReport {
    pub total_images: usize,
    pub valid_images: usize,
    pub corrupt_images: Vec<String>,
    pub camera_models: Vec<CameraSummary>,
    pub gps_status: GpsCoverageSummary,
    pub estimated_gsd_cm: f64,
    pub estimated_overlap: OverlapSummary,
    pub issues: Vec<PreflightIssue>,
}
```

GSD formula: `GSD = (altitude × sensor_width) / (focal_length × image_width)`

## Domain 2: Reconstruction

**Crates:** `nadir-reconstruction`
**Responsibility:** Feature extraction, feature matching, Structure from
Motion (SfM), bundle adjustment, dense multi-view stereo (MVS), mesh
generation, texturing.
**Engines:** COLMAP (features, matching, SfM), OpenMVS (dense, mesh, texture).

This is the computationally hardest domain. It contains two sub-problems:

### 2A: Image Understanding (features + matching)

| Operation | Trait | Engines |
|---|---|---|
| Feature extraction | `FeatureExtractor` | COLMAP (SIFT), OpenCV (ORB/AKAZE), future Rust |
| Feature matching | `FeatureMatcher` | COLMAP (exhaustive/spatial/vocab_tree) |

### 2B: 3D Reconstruction (SfM + MVS)

| Operation | Trait | Engines |
|---|---|---|
| Sparse reconstruction | `SparseReconstructor` | COLMAP (incremental/hierarchical/global) |
| Bundle adjustment | (inside SfM) | COLMAP (Ceres Solver) |
| Dense reconstruction | `DenseReconstructor` | OpenMVS, COLMAP (PatchMatch) |
| Mesh generation | `MeshGenerator` | OpenMVS (Poisson/Delaunay) |
| Texturing | `TextureGenerator` | OpenMVS |

### Key types

```rust
pub struct CameraPose {
    pub image_id: ImageId,
    pub camera_id: CameraId,
    pub position: Point3<f64>,
    pub rotation: UnitQuaternion<f64>,
}

pub struct SparsePoint {
    pub position: Point3<f64>,
    pub color: Color,
    pub error: f64,
    pub observations: Vec<Observation>,
}

pub struct SparsePointCloud {
    pub points: Vec<SparsePoint>,
}

pub struct SfmOutput {
    pub camera_poses: ArtifactRef,
    pub sparse_cloud: ArtifactRef,
    pub camera_models: ArtifactRef,
}
```

### COLMAP binary format

The adapter must parse COLMAP's output format:

```
cameras.bin   → camera_id, model, width, height, params[]
images.bin    → image_id, qw, qx, qy, qz, tx, ty, tz, camera_id, name
points3D.bin  → point3D_id, x, y, z, r, g, b, error, track[]
```

See [colmap-integration.md](../photogrammetry/colmap-integration.md).

## Domain 3: Geometry

**Crate:** `nadir-geometry`
**Responsibility:** Coordinate reference system (CRS) transformations,
georeferencing (aligning SfM reconstruction to real-world coordinates),
GCP integration, datum transformations.
**Engines:** `nalgebra` (transforms), `proj` crate (CRS), Rust native (GCP).

### Key operations

| Operation | Implementation | Dependencies |
|---|---|---|
| Similarity transform | Umeyama/Procrustes (7-param Helmert) | `nalgebra` SVD |
| CRS detection | Auto-select UTM zone from GPS | `proj` crate |
| CRS transformation | WGS84 → UTM → output CRS | `proj` crate |
| GCP alignment | Constrained bundle adjustment | `nalgebra` |
| Georeferencing | Apply similarity transform to poses + points | `nalgebra` |

### Key types

```rust
pub struct SimilarityTransform {
    pub scale: f64,
    pub rotation: Matrix3<f64>,
    pub translation: Vector3<f64>,
}

pub struct GeorefOutput {
    pub poses: ArtifactRef,
    pub sparse_cloud: ArtifactRef,
    pub transform: SimilarityTransform,
    pub crs: Crs,
}
```

### The math

The georeferencing transform maps SfM coordinates to real-world:

```
X_real = s × R × X_sfm + t
```

Where `s` is scale, `R` is rotation (3×3), `t` is translation. Estimated
via Umeyama algorithm from GPS/RTK correspondences using SVD of the
cross-covariance matrix.

For CRS-to-CRS transforms (e.g., WGS84 → UTM), delegate to PROJ:

```rust
let proj = Proj::new_known_crs("EPSG:4326", "EPSG:32632", None)?;
let (x, y, z) = proj.convert((lon, lat, alt))?;
```

## Domain 4: Surface

**Crate:** `nadir-surface`
**Responsibility:** Point cloud filtering, ground classification, DSM
rasterization, DTM interpolation, mesh operations.
**Engines:** PDAL (filtering, classification), Rust native (DSM/DTM
rasterization), OpenMVS (mesh).

### Key operations

| Operation | Trait | Engines |
|---|---|---|
| Outlier filtering | `PointCloudFilter` | PDAL, Rust (statistical) |
| Ground classification | `PointCloudClassifier` | PDAL (SMRF), Rust (CSF) |
| DSM generation | `SurfaceGenerator::generate_dsm` | Rust (IDW rasterizer) |
| DTM generation | `SurfaceGenerator::generate_dtm` | Rust (IDW from ground) |
| Mesh generation | `MeshGenerator` | OpenMVS |

### Key types

```rust
pub struct Point {
    pub position: Point3<f64>,
    pub color: Option<Rgb<u8>>,
    pub classification: Option<u8>,
    pub confidence: Option<f32>,
}

pub struct DensePointCloud {
    pub points: Vec<Point>,
    pub crs: Option<Crs>,
    pub bounds: BoundingBox,
}

pub struct RasterGrid {
    pub width: usize,
    pub height: usize,
    pub data: Vec<f32>,
    pub nodata_value: f32,
    pub bounds: GridBounds,
}
```

### DSM rasterization

The DSM is generated via Inverse Distance Weighting (IDW) from the dense
point cloud, parallelized with `rayon`:

```rust
pub fn rasterize_idw(
    points: &[[f64; 3]],
    bounds: GridBounds,
    resolution: f64,
) -> RasterGrid {
    // Parallel iteration over grid rows
    data.par_chunks_mut(width).enumerate().for_each(|(row, slice)| {
        for col in 0..width {
            // IDW interpolation from nearby points
        }
    });
}
```

### PDAL integration

PDAL is invoked via JSON pipeline files:

```json
[
  "input.laz",
  { "type": "filters.outlier", "method": "statistical" },
  { "type": "filters.smrf", "cell": 1.0, "slope": 0.2 },
  "output.laz"
]
```

```bash
pdal pipeline pipeline.json
```

## Domain 5: Cartography

**Crate:** `nadir-cartography`
**Responsibility:** Orthorectification, mosaicking, tiling, GeoTIFF/COG
export, raster reprojection.
**Engines:** Rust native (orthorectification, mosaicking), GDAL (GeoTIFF
I/O, reprojection, COG creation).

### Key operations

| Operation | Trait | Engines |
|---|---|---|
| Orthorectification | `Orthorectifier` | Rust (ray-surface intersection) |
| Mosaicking | `Mosaicker` | Rust (seam blending) |
| GeoTIFF export | `RasterStore` | GDAL crate |
| COG creation | `RasterStore` | GDAL crate (tiled + overviews) |
| Reprojection | (via `CoordinateTransformer`) | PROJ |

### GeoTIFF/COG export

```rust
pub fn write_dsm_cog(
    grid: &RasterGrid,
    epsg_code: u32,
    output_path: &Path,
) -> Result<()> {
    let driver = DriverManager::get_driver_by_name("GTiff")?;
    let options = ["TILED=YES", "COMPRESS=DEFLATE", "PREDICTOR=2",
                   "BIGTIFF=IF_SAFER", "BLOCKXSIZE=512", "BLOCKYSIZE=512"];
    let mut dataset = driver.create_with_band_type_with_options::<f32, _>(
        output_path, grid.width, grid.height, 1, &options,
    )?;
    dataset.set_geo_transform(&geo_transform)?;
    dataset.set_spatial_ref(&SpatialRef::from_epsg(epsg_code)?)?;
    band.write((0, 0), (grid.width, grid.height), &buffer)?;
    dataset.build_overviews("NEAREST", &[2, 4, 8, 16], &[])?;
    Ok(())
}
```

## The Domain Interaction Diagram

```
         Domain 1: DATASET
         ImageMetadataSet
              │
              ▼
         Domain 2: RECONSTRUCTION
         Features → Matches → SfM → Dense
              │
              ▼
         Domain 3: GEOMETRY
         GeoreferencedScene (SfM → UTM)
              │
         ┌────┴────┐
         ▼         ▼
    Domain 4     Domain 4
    DSM/DTM      Mesh/Texture
         │         │
         ▼         │
    Domain 5       │
    Orthomosaic    │
         │         │
         ▼         ▼
      GeoTIFF    OBJ/PLY
      COG        + textures
```

## See Also

- [Overview](./overview.md) — big picture
- [Engine registry](./engine-registry.md) — trait definitions per domain
- [Engine assignment](../photogrammetry/engine-assignment.md) — which tool does what
- [Pipeline stages](../photogrammetry/pipeline-stages.md) — full walkthrough
