---
title: Geospatial Stack
purpose: GDAL, PROJ, PDAL, nalgebra, geo — what each does, Rust bindings, usage patterns
last_updated: 2025-02-23
status: current
related:
  - engine-assignment.md
  - ../architecture/five-domains.md
  - ../decisions/006-pixi-for-dependency-management.md
---

# Geospatial Stack

## TL;DR

Nadir uses five geospatial libraries. GDAL handles raster I/O and warping.
PROJ handles coordinate reference system math. PDAL handles point cloud
filtering and classification. nalgebra handles 3D linear algebra. The geo
crate handles geospatial types and algorithms. GDAL and PROJ are C bindings
(Tier 2). PDAL is a subprocess (Tier 3). nalgebra and geo are pure Rust
(Tier 1).

## GDAL (Geospatial Data Abstraction Library)

**Tier:** 2 (C bindings via `gdal` crate)
**Pixi:** `gdal = ">=3.9"`
**Used for:** GeoTIFF/COG creation, raster reprojection, geotransform
management, overview generation, CRS metadata.

### Rust crate: `gdal`

```rust
use gdal::{Dataset, DriverManager};
use gdal::raster::Buffer;

// Read a GeoTIFF
let dataset = Dataset::open("dsm.tif")?;
let band = dataset.rasterband(1)?;
let mut buffer = vec![0f32; width * height];
band.read_into_slice((0, 0), (width, height), (width, height), &mut buffer, None)?;

// Create a COG
let driver = DriverManager::get_driver_by_name("GTiff")?;
let options = ["TILED=YES", "COMPRESS=DEFLATE", "PREDICTOR=2",
               "BIGTIFF=IF_SAFER", "BLOCKXSIZE=512", "BLOCKYSIZE=512"];
let mut output = driver.create_with_band_type_with_options::<f32, _>(
    "output.tif", width, height, 1, &options,
)?;
output.set_geo_transform(&[min_x, res, 0.0, max_y, 0.0, -res])?;
output.set_spatial_ref(&SpatialRef::from_epsg(32632)?)?;
output.rasterband(1)?.write((0, 0), (width, height), &buffer)?;
output.build_overviews("NEAREST", &[2, 4, 8, 16], &[])?;
```

### When to use GDAL vs CLI

| Operation | Library API | CLI (`gdalwarp`) |
|---|---|---|
| Read/write GeoTIFF | ✅ | ❌ |
| Create COG | ✅ | ❌ |
| Set CRS/geotransform | ✅ | ❌ |
| Reproject raster | ⚠️ Complex | ✅ Simpler |
| Mosaic rasters | ⚠️ Complex | ✅ `gdal_merge.py` |
| Format conversion | ⚠️ | ✅ `gdal_translate` |

Use the library API for most operations. Use the CLI for complex
reprojection and mosaicking until native Rust implementations exist.

### GDAL 3.11+ unified CLI

GDAL 3.11 introduced `gdal raster info`, `gdal raster convert`,
`gdal raster reproject` as a unified CLI alongside the traditional
`gdalinfo`, `gdal_translate`, `gdalwarp` utilities.

## PROJ

**Tier:** 2 (C bindings via `proj` crate)
**Pixi:** `proj = ">=9.4"`
**Used for:** CRS transformations, datum shifts, coordinate conversions.

### Rust crate: `proj`

```rust
use proj::Proj;

// WGS84 → UTM Zone 32N
let transformer = Proj::new_known_crs("EPSG:4326", "EPSG:32632", None)
    .ok_or_else(|| anyhow!("invalid CRS"))?;

let (easting, northing, alt) = transformer.convert((lon, lat, alt))?;
```

### Key operations

| Operation | PROJ function | Example |
|---|---|---|
| CRS → CRS | `proj_create_crs_to_crs` | WGS84 → UTM |
| Datum shift | Helmert/NTv2 | NAD27 → NAD83 |
| Projection | Forward/inverse | lon/lat → easting/northing |
| Compound CRS | Horizontal + vertical | UTM + NAVD88 |

### Important: always use PROJ for CRS math

Do not write your own EPSG transformation code. PROJ handles 7,000+ CRS
definitions, grid-shift corrections, and compound transformations. It is
the international standard and is used by GDAL, QGIS, PostGIS, and every
other geospatial tool.

## PDAL (Point Data Abstraction Library)

**Tier:** 3 (subprocess via `tokio::process`)
**Pixi:** `pdal = ">=2.7"`
**Used for:** Point cloud filtering, ground classification, format
conversion, reprojection, statistics.

### JSON pipeline interface

PDAL's killer feature is its JSON pipeline abstraction:

```json
[
  "input.laz",
  {
    "type": "filters.outlier",
    "method": "statistical",
    "mean_k": 16,
    "multiplier": 2.0
  },
  {
    "type": "filters.smrf",
    "cell": 1.0,
    "slope": 0.2,
    "threshold": 0.45,
    "window": 16.0
  },
  {
    "type": "filters.expression",
    "expression": "Classification == 2"
  },
  "ground.laz"
]
```

```bash
pdal pipeline pipeline.json
```

### Nadir's PDAL adapter generates JSON

```rust
pub struct PdalPipeline {
    stages: Vec<serde_json::Value>,
}

impl PdalPipeline {
    pub fn new() -> Self { Self { stages: Vec::new() } }

    pub fn read(mut self, path: &str) -> Self {
        self.stages.push(json!(path));
        self
    }

    pub fn filter(mut self, filter_type: &str, opts: serde_json::Value) -> Self {
        let mut stage = opts;
        stage["type"] = json!(filter_type);
        self.stages.push(stage);
        self
    }

    pub fn write(mut self, path: &str) -> Self {
        self.stages.push(json!(path));
        self
    }
}

// Usage:
let pipeline = PdalPipeline::new()
    .read("dense.laz")
    .filter("filters.outlier", json!({"method": "statistical"}))
    .filter("filters.smrf", json!({"slope": 0.2, "window": 16.0}))
    .write("ground.laz");

pdal_engine.execute(pipeline).await?;
```

### Key PDAL filters

| Filter | Purpose | Nadir use |
|---|---|---|
| `filters.outlier` | Remove noise | Point cloud cleaning |
| `filters.smrf` | Ground classification | DTM generation |
| `filters.range` | Value filtering | Height clipping |
| `filters.reprojection` | CRS transform | Point cloud reprojection |
| `filters.delaunay` | Mesh generation | Future |
| `filters.stats` | Statistics | QC metrics |

## nalgebra

**Tier:** 1 (pure Rust)
**Crate:** `nalgebra = "0.33"`
**Used for:** 3D linear algebra, rotation matrices, quaternions, SVD,
similarity transforms, camera geometry.

### Key operations in Nadir

```rust
use nalgebra::{Matrix3, Point3, Vector3, SVD, UnitQuaternion};

// 7-parameter Helmert transform
pub struct SimilarityTransform {
    pub scale: f64,
    pub rotation: Matrix3<f64>,
    pub translation: Vector3<f64>,
}

// Umeyama alignment via SVD
let svd = SVD::new(covariance_matrix, true, true);
let rotation = u * d * v_t;
let scale = (1.0 / variance) * (d * singular_values).trace();
let translation = centroid_b - scale * (rotation * centroid_a);

// Camera pose application
let world_point = pose.rotation * local_point + pose.position;
```

## geo / geo-types

**Tier:** 1 (pure Rust)
**Crates:** `geo = "0.29"`, `geo-types = "0.7"`
**Used for:** Geospatial types (Point, LineString, Polygon, BoundingBox),
spatial algorithms (intersection, distance, convex hull), spatial indexing.

### Key types

```rust
use geo_types::{Point, Rect, Coord};

let bounds = Rect::new(
    Coord { x: min_lon, y: min_lat },
    Coord { x: max_lon, y: max_lat },
);
```

## The Dependency Diagram

```
                    Nadir Rust Code
                         │
          ┌──────────────┼──────────────┐
          │              │              │
     Tier 1          Tier 2         Tier 3
    (pure Rust)     (C bindings)   (subprocess)
          │              │              │
    ┌─────┼─────┐   ┌────┼────┐    ┌────┤
    │     │     │   │         │    │    │
nalgebra geo  las  gdal     proj  PDAL  COLMAP
image  blake3      (crate) (crate)      OpenMVS
```

## See Also

- [Engine assignment](./engine-assignment.md) — which tool handles which stage
- [Pixi setup](../deployment/pixi-setup.md) — how these are installed
- [Dependency matrix](../references/dependency-matrix.md) — versions and tiers
