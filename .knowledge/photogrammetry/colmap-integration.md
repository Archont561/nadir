---
type: Domain Guide
title: COLMAP Integration
description: "CLI commands, binary format, adapter code, progress parsing, resume behavior"
purpose: CLI commands, binary format, adapter code, progress parsing, resume behavior
last_updated: 2025-02-23
status: stable
related:
  - engine-assignment.md
  - openmvs-integration.md
  - ../architecture/engine-registry.md
  - ../architecture/engine-registry.md
---

# COLMAP Integration

## TL;DR

COLMAP is the primary engine for feature extraction, matching, and Structure
from Motion. Nadir invokes COLMAP as a subprocess via `tokio::process`,
parses its binary output format (`cameras.bin`, `images.bin`, `points3D.bin`),
and converts results into Nadir artifact types. COLMAP's CLI is essentially
a task API — each command maps to a Nadir trait method.

## COLMAP CLI Command Map

| Nadir Task | COLMAP Command | Input | Output |
|---|---|---|---|
| `CreateDatabase` | `database_creator` | workspace | `database.db` |
| `ExtractFeatures` | `feature_extractor` | images + DB | features in DB |
| `MatchExhaustive` | `exhaustive_matcher` | DB | matches in DB |
| `MatchSequential` | `sequential_matcher` | DB | matches in DB |
| `MatchSpatial` | `spatial_matcher` | DB + GPS | matches in DB |
| `MatchVocabTree` | `vocab_tree_matcher` | DB + vocab | matches in DB |
| `VerifyMatches` | `geometric_verifier` | DB | verified matches |
| `SparseReconstruction` | `mapper` | DB + images | sparse model dir |
| `HierarchicalSfM` | `hierarchical_mapper` | DB + images | sparse models |
| `GlobalSfM` | `global_mapper` | DB | sparse model |
| `PosePriorSfM` | `pose_prior_mapper` | DB + priors | sparse model |
| `Triangulate` | `point_triangulator` | model + DB | sparse points |
| `BundleAdjust` | `bundle_adjuster` | model | optimized model |
| `Georeference` | `model_aligner` | model + GPS | aligned model |
| `Undistort` | `image_undistorter` | model + images | dense workspace |
| `DenseMVS` | `patch_match_stereo` | dense workspace | depth maps |
| `FuseDepthMaps` | `stereo_fusion` | depth maps | dense PLY |
| `PoissonMesh` | `poisson_mesher` | dense PLY | mesh |
| `DelaunayMesh` | `delaunay_mesher` | dense workspace | mesh |
| `TextureMesh` | `mesh_texturer` | mesh + images | textured mesh |
| `ConvertModel` | `model_converter` | COLMAP model | PLY/other |
| `AnalyzeModel` | `model_analyzer` | model | statistics |

## The Standard COLMAP Pipeline

The explicit pipeline (as opposed to `automatic_reconstructor`):

```bash
# 1. Feature extraction
colmap feature_extractor \
    --database_path database.db \
    --image_path images/ \
    --ImageReader.camera_model OPENCV \
    --SiftExtraction.max_num_features 8192 \
    --SiftExtraction.use_gpu true

# 2. Feature matching
colmap exhaustive_matcher \
    --database_path database.db \
    --SiftMatching.use_gpu true

# 3. Sparse reconstruction
colmap mapper \
    --database_path database.db \
    --image_path images/ \
    --output_path sparse/ \
    --Mapper.ba_max_num_iterations 100

# 4. (Optional) Model alignment with GPS
colmap model_aligner \
    --input_path sparse/0 \
    --output_path sparse/aligned \
    --ref_images_path gps.txt \
    --robust_alignment true

# 5. Undistort for dense reconstruction
colmap image_undistorter \
    --image_path images/ \
    --input_path sparse/0 \
    --output_path dense/ \
    --output_type COLMAP

# 6. Dense MVS
colmap patch_match_stereo \
    --workspace_path dense/ \
    --workspace_format COLMAP \
    --PatchMatchStereo.max_image_size 2000

# 7. Depth map fusion
colmap stereo_fusion \
    --workspace_path dense/ \
    --workspace_format COLMAP \
    --output_path dense/fused.ply
```

## The Rust Adapter

### Engine struct

```rust
// crates/nadir-reconstruction/src/colmap/mod.rs

use std::path::{Path, PathBuf};
use nadir_process::{ProcessRunner, ProcessSpec, ProgressParser};
use tokio_util::sync::CancellationToken;

pub struct ColmapEngine {
    binary: PathBuf,
    version: String,
}

impl ColmapEngine {
    pub fn find() -> anyhow::Result<Self> {
        let binary = which::which("colmap")
            .or_else(|_| {
                // Check common Pixi/conda paths
                let pixi_path = std::env::var("CONDA_PREFIX")
                    .map(|p| PathBuf::from(p).join("bin/colmap"))
                    .ok();
                pixi_path.filter(|p| p.exists())
                    .ok_or_else(|| anyhow::anyhow!("COLMAP not found in PATH"))
            })?;

        let version = Self::detect_version(&binary)?;
        Ok(Self { binary, version })
    }

    fn detect_version(binary: &Path) -> anyhow::Result<String> {
        let output = std::process::Command::new(binary)
            .arg("help")
            .output()?;
        let stdout = String::from_utf8_lossy(&output.stdout);
        // Parse "COLMAP 3.9.1" from help output
        let version = stdout.lines()
            .find(|l| l.contains("COLMAP"))
            .and_then(|l| l.split_whitespace().nth(1))
            .unwrap_or("unknown")
            .to_string();
        Ok(version)
    }
}
```

### Feature extraction implementation

```rust
impl FeatureExtractor for ColmapEngine {
    fn extract(
        &self,
        images: &ImageSet,
        cfg: &FeatureConfig,
        ctx: &TaskContext,
    ) -> Result<Features> {
        let workspace = ctx.workspace();
        let db_path = workspace.join("database.db");
        let image_path = images.base_dir();

        let mut args = vec![
            "feature_extractor".into(),
            "--database_path".into(), db_path.to_string_lossy().into(),
            "--image_path".into(), image_path.to_string_lossy().into(),
            "--ImageReader.camera_model".into(), "OPENCV".into(),
            "--SiftExtraction.max_num_features".into(),
                cfg.max_features.to_string(),
        ];

        match cfg.detector {
            Detector::Sift => {
                args.push("--SiftExtraction.use_gpu".into());
                args.push("true".into());
            }
            Detector::Orb => {
                // COLMAP doesn't natively support ORB via CLI
                // Would need OpenCV adapter for this
                anyhow::bail!("ORB not supported by COLMAP, use opencv engine");
            }
            _ => {}
        }

        let spec = ProcessSpec {
            program: self.binary.clone(),
            args,
            current_dir: workspace.clone(),
            timeout: None,
        };

        let cancel = ctx.cancellation_token();
        let events = ctx.event_bus();

        // Run with progress parsing
        tokio::runtime::Handle::current().block_on(async {
            ProcessRunner::run(
                spec,
                Some(ColmapProgressParser),
                cancel,
                |pct| events.emit_progress("features", pct),
            ).await
        })?;

        // Features are stored in the SQLite database
        Ok(Features::from_database(&db_path))
    }
}
```

### SfM implementation

```rust
impl SparseReconstructor for ColmapEngine {
    fn reconstruct(
        &self,
        matches: &Matches,
        cfg: &SfMConfig,
        ctx: &TaskContext,
    ) -> Result<SfmOutput> {
        let workspace = ctx.workspace();
        let db_path = workspace.join("database.db");
        let image_path = ctx.input_path("ingest");
        let sparse_path = workspace.join("sparse");
        std::fs::create_dir_all(&sparse_path)?;

        let spec = ProcessSpec {
            program: self.binary.clone(),
            args: vec![
                "mapper".into(),
                "--database_path".into(), db_path.to_string_lossy().into(),
                "--image_path".into(), image_path.to_string_lossy().into(),
                "--output_path".into(), sparse_path.to_string_lossy().into(),
                "--Mapper.ba_max_num_iterations".into(),
                    cfg.max_iterations.to_string(),
                "--Mapper.filter_min_tri_angle".into(), "1.5".into(),
                "--Mapper.ba_global_max_refinements".into(), "5".into(),
                "--Mapper.ba_global_max_num_iterations".into(), "50".into(),
                "--Mapper.init_min_tri_angle".into(), "4".into(),
            ],
            current_dir: workspace.clone(),
            timeout: None,
        };

        ProcessRunner::run(
            spec,
            Some(ColmapProgressParser),
            ctx.cancellation_token(),
            |pct| ctx.emit_progress("sfm", pct),
        ).await?;

        // Parse the best reconstruction (subdirectory "0")
        let model_dir = sparse_path.join("0");
        if !model_dir.exists() {
            anyhow::bail!("COLMAP mapper produced no reconstruction");
        }

        let (cameras, poses, sparse) = parse_colmap_model(&model_dir)?;

        Ok(SfmOutput {
            camera_models: ctx.register(cameras, ArtifactKind::CameraModel)?,
            camera_poses: ctx.register(poses, ArtifactKind::CameraPoses)?,
            sparse_cloud: ctx.register(sparse, ArtifactKind::SparsePointCloud)?,
        })
    }
}
```

## COLMAP Binary Format Parser

COLMAP outputs binary files in its sparse model directory. Nadir must parse
these to convert into Nadir artifact types.

### cameras.bin

```
Format:
  num_cameras: uint64
  For each camera:
    camera_id: uint32
    model_id: int32       (0=SIMPLE_PINHOLE, 1=PINHOLE, 2=SIMPLE_RADIAL,
                           3=RADIAL, 4=OPENCV, 5=OPENCV_FISHEYE, 6=FULL_OPENCV)
    width: uint64
    height: uint64
    params: double[]      (length depends on model)
```

```rust
fn parse_cameras_bin(path: &Path) -> Result<Vec<CameraModel>> {
    let data = std::fs::read(path)?;
    let mut cursor = std::io::Cursor::new(&data);

    let num_cameras = cursor.read_u64::<LittleEndian>()?;
    let mut cameras = Vec::with_capacity(num_cameras as usize);

    for _ in 0..num_cameras {
        let camera_id = cursor.read_u32::<LittleEndian>()?;
        let model_id = cursor.read_i32::<LittleEndian>()?;
        let width = cursor.read_u64::<LittleEndian>()? as u32;
        let height = cursor.read_u64::<LittleEndian>()? as u32;

        let num_params = num_params_for_model(model_id);
        let mut params = Vec::with_capacity(num_params);
        for _ in 0..num_params {
            params.push(cursor.read_f64::<LittleEndian>()?);
        }

        cameras.push(CameraModel {
            id: camera_id.into(),
            width,
            height,
            focal_length: params[0],
            principal_point: Point2::new(
                params.get(1).copied().unwrap_or(width as f64 / 2.0),
                params.get(2).copied().unwrap_or(height as f64 / 2.0),
            ),
            distortion: extract_distortion(model_id, &params),
        });
    }

    Ok(cameras)
}

fn num_params_for_model(model_id: i32) -> usize {
    match model_id {
        0 => 3,  // SIMPLE_PINHOLE: f, cx, cy
        1 => 4,  // PINHOLE: fx, fy, cx, cy
        2 => 4,  // SIMPLE_RADIAL: f, cx, cy, k
        3 => 5,  // RADIAL: f, cx, cy, k1, k2
        4 => 8,  // OPENCV: fx, fy, cx, cy, k1, k2, p1, p2
        5 => 8,  // OPENCV_FISHEYE: fx, fy, cx, cy, k1, k2, k3, k4
        6 => 12, // FULL_OPENCV: fx, fy, cx, cy, k1, k2, p1, p2, k3, k4, k5, k6
        _ => panic!("unknown camera model: {model_id}"),
    }
}
```

### images.bin

```
Format:
  num_images: uint64
  For each image:
    image_id: uint32
    qw, qx, qy, qz: double   (rotation quaternion, wxyz)
    tx, ty, tz: double        (translation)
    camera_id: uint32
    name: string (null-terminated)
    num_points2D: uint64
    For each point2D:
      x, y: double
      point3D_id: int64       (-1 if not triangulated)
```

```rust
fn parse_images_bin(path: &Path) -> Result<Vec<CameraPose>> {
    let data = std::fs::read(path)?;
    let mut cursor = std::io::Cursor::new(&data);

    let num_images = cursor.read_u64::<LittleEndian>()?;
    let mut poses = Vec::with_capacity(num_images as usize);

    for _ in 0..num_images {
        let image_id = cursor.read_u32::<LittleEndian>()?;
        let qw = cursor.read_f64::<LittleEndian>()?;
        let qx = cursor.read_f64::<LittleEndian>()?;
        let qy = cursor.read_f64::<LittleEndian>()?;
        let qz = cursor.read_f64::<LittleEndian>()?;
        let tx = cursor.read_f64::<LittleEndian>()?;
        let ty = cursor.read_f64::<LittleEndian>()?;
        let tz = cursor.read_f64::<LittleEndian>()?;
        let camera_id = cursor.read_u32::<LittleEndian>()?;

        // Read null-terminated image name
        let mut name_bytes = Vec::new();
        loop {
            let b = cursor.read_u8()?;
            if b == 0 { break; }
            name_bytes.push(b);
        }
        let _name = String::from_utf8(name_bytes)?;

        // Read 2D points (skip for pose extraction)
        let num_points2d = cursor.read_u64::<LittleEndian>()?;
        for _ in 0..num_points2d {
            let _x = cursor.read_f64::<LittleEndian>()?;
            let _y = cursor.read_f64::<LittleEndian>()?;
            let _point3d_id = cursor.read_i64::<LittleEndian>()?;
        }

        poses.push(CameraPose {
            image_id: image_id.into(),
            camera_id: camera_id.into(),
            position: Point3::new(tx, ty, tz),
            rotation: UnitQuaternion::from_quaternion(
                Quaternion::new(qw, qx, qy, qz)
            ),
        });
    }

    Ok(poses)
}
```

### points3D.bin

```
Format:
  num_points: uint64
  For each point:
    point3D_id: uint64
    x, y, z: double
    r, g, b: uint8
    error: double
    track_length: uint64
    For each track element:
      image_id: uint32
      point2D_idx: uint32
```

```rust
fn parse_points3d_bin(path: &Path) -> Result<SparsePointCloud> {
    let data = std::fs::read(path)?;
    let mut cursor = std::io::Cursor::new(&data);

    let num_points = cursor.read_u64::<LittleEndian>()?;
    let mut points = Vec::with_capacity(num_points as usize);

    for _ in 0..num_points {
        let _point3d_id = cursor.read_u64::<LittleEndian>()?;
        let x = cursor.read_f64::<LittleEndian>()?;
        let y = cursor.read_f64::<LittleEndian>()?;
        let z = cursor.read_f64::<LittleEndian>()?;
        let r = cursor.read_u8()?;
        let g = cursor.read_u8()?;
        let b = cursor.read_u8()?;
        let error = cursor.read_f64::<LittleEndian>()?;

        let track_length = cursor.read_u64::<LittleEndian>()?;
        let mut observations = Vec::with_capacity(track_length as usize);
        for _ in 0..track_length {
            let image_id = cursor.read_u32::<LittleEndian>()?;
            let point2d_idx = cursor.read_u32::<LittleEndian>()?;
            observations.push(Observation {
                image_id: image_id.into(),
                point2d_idx,
            });
        }

        points.push(SparsePoint {
            position: Point3::new(x, y, z),
            color: Color::rgb(r, g, b),
            error,
            observations,
        });
    }

    Ok(SparsePointCloud { points })
}
```

## Progress Parsing

COLMAP emits progress to stdout in recognizable patterns:

```rust
struct ColmapProgressParser;

impl ProgressParser for ColmapProgressParser {
    fn parse_line(&self, line: &str) -> Option<f32> {
        // Feature extraction: "Processed file [123/1284]"
        if let Some(caps) = regex::Regex::new(r"Processed file \[(\d+)/(\d+)\]")
            .ok()?.captures(line)
        {
            let current: f32 = caps[1].parse().ok()?;
            let total: f32 = caps[2].parse().ok()?;
            return Some(current / total);
        }

        // Matching: "Matching image [456/1284]"
        if let Some(caps) = regex::Regex::new(r"Matching image \[(\d+)/(\d+)\]")
            .ok()?.captures(line)
        {
            let current: f32 = caps[1].parse().ok()?;
            let total: f32 = caps[2].parse().ok()?;
            return Some(current / total);
        }

        // SfM: "Registering image #12 (45)"
        if line.contains("Registering image") {
            // Coarse progress — would need total image count for precise %
            return Some(0.5); // placeholder
        }

        None
    }
}
```

## Resume Behavior

COLMAP supports resuming several operations:

| Command | Resume support | How |
|---|---|---|
| `feature_extractor` | ✅ | Skips images already in database |
| `exhaustive_matcher` | ✅ | Skips pairs already matched |
| `mapper` | ✅ | Continues from existing reconstruction |
| `patch_match_stereo` | ✅ | Skips images with existing depth maps |
| `stereo_fusion` | ⚠️ | Reruns from scratch |

Nadir's artifact cache handles resume at the task level. COLMAP's native
resume handles it within a task. The two layers complement each other:

```
Nadir cache: "SfM task already completed → skip entirely"
COLMAP resume: "SfM task crashed at image 800/1284 → continue from 800"
```

## COLMAP Database (SQLite)

The `database.db` file is a SQLite database containing:

| Table | Content |
|---|---|
| `cameras` | Camera models and intrinsics |
| `images` | Image metadata and poses |
| `keypoints` | Feature keypoints per image |
| `descriptors` | Feature descriptors per image |
| `matches` | Pairwise feature matches |
| `two_view_geometries` | Verified geometric matches |

Nadir can query this database directly for advanced operations:

```rust
use rusqlite::Connection;

fn get_match_count(db_path: &Path) -> Result<usize> {
    let conn = Connection::open(db_path)?;
    let count: usize = conn.query_row(
        "SELECT COUNT(*) FROM matches WHERE rows > 0",
        [],
        |row| row.get(0),
    )?;
    Ok(count)
}
```

## See Also

- [OpenMVS integration](./openmvs-integration.md) — the next stage after COLMAP
- [Engine assignment](./engine-assignment.md) — why COLMAP for SfM
- [Process runner](../architecture/engine-registry.md) — subprocess management
- [Engine registry](../architecture/engine-registry.md) — trait implementation
