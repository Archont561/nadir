---
type: Domain Guide
title: Photogrammetry Pipeline Stages
description: "Full walkthrough from drone images to mapping products with math and diagrams"
purpose: Full walkthrough from drone images to mapping products with math and diagrams
last_updated: 2025-02-23
status: stable
related:
  - engine-assignment.md
  - ../architecture/five-domains.md
  - ../architecture/pipeline-dag.md
---

# Photogrammetry Pipeline Stages

## TL;DR

Photogrammetry turns overlapping 2D photographs into a geometrically
consistent 3D representation, then projects that into map products. The
pipeline has ~16 stages grouped into 3 computational sub-problems: image
understanding (features/matching), 3D reconstruction (SfM/MVS), and
geospatial production (DSM/ortho). This file walks through every stage
with the underlying math and data flow.

## The Big Picture

```
                    DRONE IMAGES
                         │
                         ▼
              ┌─────────────────────┐
              │ 1. Image ingestion  │
              │ EXIF / GPS / RTK    │
              └──────────┬──────────┘
                         ▼
              ┌─────────────────────┐
              │ 2. Camera model     │
              │ intrinsics/lens     │
              └──────────┬──────────┘
                         ▼
              ┌─────────────────────┐
              │ 3. Feature          │
              │    extraction       │
              └──────────┬──────────┘
                         ▼
              ┌─────────────────────┐
              │ 4. Feature matching │
              └──────────┬──────────┘
                         ▼
              ┌─────────────────────┐
              │ 5. SfM              │
              │ poses + sparse 3D   │
              └──────────┬──────────┘
                         ▼
              ┌─────────────────────┐
              │ 6. Georeferencing   │
              │ GPS / RTK / GCP     │
              └──────────┬──────────┘
                         ▼
              ┌─────────────────────┐
              │ 7. Dense matching   │
              │ (MVS)               │
              └──────────┬──────────┘
                         │
                    ┌────┴─────┐
                    ▼          ▼
             ┌──────────┐  ┌──────────┐
             │ 8. DSM   │  │ 9. Mesh  │
             └────┬─────┘  └────┬─────┘
                  │              │
                  ▼              ▼
             ┌──────────┐  ┌──────────┐
             │ 10. DTM  │  │ Texture  │
             └────┬─────┘  └──────────┘
                  │
                  ▼
             ORTHOMOSAIC
```

## Three Computational Sub-Problems

### A. Image Understanding (stages 1–4)

Primarily computer vision. Extract distinctive points from images and find
correspondences between overlapping photographs.

### B. 3D Reconstruction (stages 5–7)

The hard photogrammetry core. Estimate camera positions and reconstruct
dense 3D geometry from 2D correspondences.

### C. Geospatial Production (stages 8–12)

GIS/raster/point-cloud processing. Turn 3D geometry into georeferenced map
products.

## Stage 1: Image Ingestion

**Input:** Directory of drone photographs
**Output:** `ImageMetadataSet` (images + EXIF + GPS + camera info)

Each photograph contains:
- Pixels (JPEG/TIFF/PNG)
- EXIF: focal length, camera model, timestamp
- GPS: latitude, longitude, altitude
- RTK: high-precision position (optional)

```rust
pub struct ImageData {
    pub path: PathBuf,
    pub width: u32,
    pub height: u32,
    pub gps: Option<GeoPosition>,
    pub intrinsics: CameraIntrinsics,
}
```

Key validation checks:
- Minimum image count (≥ 3, practically ≥ 20)
- GPS coverage (% of images with coordinates)
- Camera model consistency
- Corrupt/unreadable file detection

## Stage 2: Camera Calibration

**Input:** `ImageMetadataSet`
**Output:** `CameraModel` per unique camera

A camera maps 3D points to pixels via the pinhole model:

```
u = f × X/Z
v = f × Y/Z
```

With distortion parameters:

```rust
pub struct CameraModel {
    pub focal_length: f64,
    pub principal_point: Point2<f64>,
    pub distortion: DistortionModel,  // radial + tangential
}
```

**Intrinsics** describe the camera (focal length, distortion).
**Extrinsics** describe where the camera was (position, rotation).

COLMAP handles calibration internally during feature extraction. The
`--ImageReader.camera_model` flag selects the model (SIMPLE_PINHOLE,
PINHOLE, SIMPLE_RADIAL, RADIAL, OPENCV, FULL_OPENCV).

## Stage 3: Feature Extraction

**Input:** `ImageSet`
**Output:** `Features` (keypoints + descriptors per image)

The system finds visually distinctive points in each image:

```
Image A:
    feature 1 → (183, 921)  descriptor: [0.12, 0.87, ...]
    feature 2 → (451, 782)  descriptor: [0.45, 0.23, ...]
    ...
```

Each feature = pixel location + visual fingerprint (128-dim SIFT vector).

Algorithms: SIFT (default, best quality), ORB (fast), AKAZE (balanced).
COLMAP also supports learned features (SuperPoint, DISK).

Typical output: 5,000–10,000 features per image.

## Stage 4: Feature Matching

**Input:** `Features`
**Output:** `FeatureMatches` (pairwise correspondences)

Compare overlapping images to find which features correspond:

```
A feature 183 → B feature 102
A feature 451 → B feature 372
```

Matching strategies:
- **Exhaustive:** All pairs. O(N²). Best for < 300 images.
- **Sequential:** Adjacent images only. For linear flight paths.
- **Spatial:** GPS-based nearest neighbors. For 300–3,000 images.
- **Vocabulary tree:** Visual word index. O(N). For > 3,000 images.

The matching graph can be huge:

```
IMG1 ─── IMG2 ─── IMG3
 │        │        │
 └────────IMG4─────┘
          │
         IMG5
```

GPS priors dramatically reduce the search space. If two photos were taken
50 km apart, they don't need to be compared.

## Stage 5: Structure from Motion (SfM)

**Input:** `FeatureMatches` + `ImageSet`
**Output:** `CameraPoses` + `SparsePointCloud`

**This is the first magical stage.** From 2D correspondences, estimate
camera positions and 3D point positions simultaneously.

### Triangulation

The same physical point appears at different pixels in different images.
Given camera geometry, triangulate its 3D position:

```
Camera A             Camera B
     \                  /
      \                /
       \     ●        /
        \   / \      /
         \ /   \    /
          ●─────●
         physical point
```

### Bundle Adjustment

SfM doesn't stop after triangulation. Optimize the entire reconstruction
by minimizing reprojection error:

```
observed pixel ●
                × predicted pixel
              ← error →
```

```
minimize Σ reprojection_error²
  over all cameras + all 3D points + camera parameters
```

This is a large sparse nonlinear least-squares problem. COLMAP uses Ceres
Solver internally. This is one of the most computationally difficult parts
of photogrammetry and a key reason to use mature engines.

### Output

A sparse reconstruction:

```rust
pub struct CameraPose {
    pub position: Point3<f64>,       // X, Y, Z in SfM coords
    pub rotation: UnitQuaternion<f64>,
}

pub struct SparsePoint {
    pub position: Point3<f64>,
    pub color: Color,
    pub error: f64,                  // reprojection error
    pub observations: Vec<Observation>,
}
```

Typical output: 100K–10M sparse 3D points.

## Stage 6: Georeferencing

**Input:** `SparseReconstruction` + `GpsData` (+ optional GCPs)
**Output:** `GeoreferencedScene` (poses + points in real-world CRS)

SfM reconstruction is in an arbitrary coordinate system. It doesn't know
north, east, or absolute scale. Georeferencing aligns it to real-world
coordinates using GPS/RTK/GCP.

### The transform

Estimate a 7-parameter Helmert similarity transform:

```
X_real = s × R × X_sfm + t
```

Where: s = scale, R = 3×3 rotation, t = translation.

Estimated via the Umeyama/Procrustes algorithm:
1. Compute centroids of both point sets
2. Center both point sets
3. SVD of cross-covariance matrix
4. Extract R, t, s

### CRS transformation

After alignment, convert from WGS84 to the project CRS (e.g., UTM):

```
WGS84 (lon, lat, alt) → UTM (easting, northing, elevation)
```

Uses PROJ for the datum transformation.

### GCP refinement

Ground Control Points provide survey-grade accuracy. A GCP file maps
pixel coordinates in specific images to known real-world positions:

```
image_name, pixel_x, pixel_y, easting, northing, elevation
IMG_0042.JPG, 2048, 1536, 582341.23, 4512387.45, 142.30
```

GCPs constrain the bundle adjustment for sub-centimeter accuracy.

## Stage 7: Dense Reconstruction (MVS)

**Input:** `GeoreferencedScene` + `ImageSet`
**Output:** `DensePointCloud`

SfM produces thousands of points. MVS produces millions or billions.

Multi-View Stereo estimates depth for every pixel that can be reliably
reconstructed:

```
Image A ────────┐
Image B ────────┼──► estimate depth per pixel
Image C ────────┘
```

Instead of "here is one distinctive point," MVS asks "what is the surface
depth for every patch?"

Output: 10M–500M points with XYZ + RGB.

COLMAP uses PatchMatch stereo. OpenMVS uses a similar plane-sweep approach.
Both are GPU-accelerated when available.

## Stage 8: Point Cloud Filtering

**Input:** `DensePointCloud`
**Output:** `FilteredPointCloud`

Dense reconstruction produces noise, outliers, and floating points:

```
raw dense cloud → statistical outlier removal → clean cloud
```

Methods:
- Statistical outlier filter (remove points with few neighbors)
- Radius outlier filter (remove isolated points)
- Confidence thresholding (remove low-confidence MVS points)

## Stage 9: Ground Classification

**Input:** `FilteredPointCloud`
**Output:** `ClassifiedPointCloud` (ground / non-ground labels)

For DTM generation, separate ground from vegetation/buildings:

```
Classification labels:
  2 = ground
  3 = low vegetation
  4 = medium vegetation
  5 = high vegetation
  6 = building
```

Algorithms: SMRF (Simple Morphological Filter), CSF (Cloth Simulation
Filter). PDAL provides both. Rust implementations are feasible for V2.

## Stage 10: DSM Generation

**Input:** `DensePointCloud` or `FilteredPointCloud`
**Output:** `Dsm` (GeoTIFF raster)

A Digital Surface Model represents the top visible surface (trees, roofs,
ground). Rasterize the point cloud onto a regular grid:

```
DSM.tif
pixel (i,j):
    elevation = 142.73m
```

Methods: IDW (Inverse Distance Weighting), nearest neighbor, binning.
Resolution typically 2–10× GSD (e.g., 5cm for 2cm GSD).

## Stage 11: DTM Generation

**Input:** `ClassifiedPointCloud` (ground points only)
**Output:** `Dtm` (GeoTIFF raster)

A Digital Terrain Model represents bare ground (no trees, no buildings).
Interpolate from classified ground points.

The relationship: `height_above_ground ≈ DSM - DTM`

## Stage 12: Mesh Generation

**Input:** `DensePointCloud`
**Output:** `Mesh` (triangulated surface)

Turn the point cloud into a triangle mesh:

```
   ●────●
  / \  / \
 /   \/   \
●────●────●
```

Algorithms: Poisson surface reconstruction, Delaunay triangulation.
OpenMVS provides both.

## Stage 13: Texturing

**Input:** `Mesh` + `ImageSet` + `CameraPoses`
**Output:** `TexturedMesh` (OBJ + texture atlas)

Project images onto the mesh surface to create a photorealistic 3D model.

## Stage 14: Orthorectification

**Input:** `ImageSet` + `GeoreferencedScene` + `DSM`
**Output:** Orthorectified images

Remove perspective distortion from each image using camera pose + DSM:

```
                  camera
                    📷
                   /|\
                  / | \
                 /  |  \
                ▼   ▼   ▼
             surface / DSM
────────────────────────────────
```

Every pixel is projected onto the reconstructed surface, producing a
geometrically correct image where distances behave like a map.

## Stage 15: Mosaicking

**Input:** Orthorectified images
**Output:** `Orthomosaic` (single seamless GeoTIFF/COG)

Merge all orthorectified images into one seamless map:

```
IMG1 + IMG2 + IMG3 + ... → single orthomosaic
```

Involves: seam-line computation, color balancing, blending, tiling.

## Stage 16: Report

**Input:** All artifacts + QC metrics
**Output:** `Report` (JSON/PDF)

Summary of the entire processing run: image count, registration rate,
reprojection error, point counts, CRS, product inventory.

## Key Math Reference

### Ground Sampling Distance (GSD)

```
GSD = (altitude × sensor_width) / (focal_length × image_width)
```

Example: 100m altitude, 13.2mm sensor, 8.8mm focal, 5472px width
→ GSD = (100 × 13.2) / (8.8 × 5472) = 0.0274 m/px ≈ 2.7 cm/px

### Reprojection Error

The distance between an observed pixel and the projected 3D point:

```
error = ‖pixel_observed - project(camera, point_3d)‖
```

Good reconstruction: < 1.0 px RMSE. Excellent: < 0.5 px.

### Triangulation

Given two camera poses and corresponding 2D points, solve for the 3D
point using the Direct Linear Transform (DLT) or midpoint method.

## See Also

- [Engine assignment](./engine-assignment.md) — which tool handles each stage
- [Five domains](../architecture/five-domains.md) — domain-level breakdown
- [Pipeline & DAG](../architecture/pipeline-dag.md) — how stages are orchestrated
