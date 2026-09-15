---
title: OpenMVS Integration
purpose: Tool chain, MVS scene format, COLMAP→OpenMVS conversion, adapter code
last_updated: 2025-02-23
status: current
related:
  - colmap-integration.md
  - engine-assignment.md
  - ../architecture/engine-registry.md
  - ../implementation/process-runner.md
---

# OpenMVS Integration

## TL;DR

OpenMVS handles dense multi-view stereo, mesh generation, and texturing.
It consumes COLMAP's sparse reconstruction via `InterfaceCOLMAP`, produces
a `.mvs` scene file, then runs `DensifyPointCloud`, `ReconstructMesh`,
and `TextureMesh` as separate subprocesses. Nadir wraps each tool behind
a Rust trait and manages the data format conversion.

## The OpenMVS Tool Chain

```
COLMAP sparse model
        │
        ▼
InterfaceCOLMAP          ← converts COLMAP → OpenMVS .mvs scene
        │
        ▼
scene.mvs
        │
   ┌────┼────────┐
   ▼    ▼        ▼
Densify  Reconstruct  Refine
PointCloud  Mesh      Mesh
   │    │        │
   ▼    ▼        ▼
dense  mesh    refined
cloud  .obj    mesh
   │    │        │
   │    ▼        ▼
   │  TextureMesh
   │    │
   │    ▼
   │  textured
   │  mesh
   ▼
(point cloud
 artifact)
```

### The four binaries

| Binary | Purpose | Input | Output |
|---|---|---|---|
| `InterfaceCOLMAP` | Convert COLMAP model to MVS scene | COLMAP sparse dir + images | `scene.mvs` |
| `DensifyPointCloud` | Multi-view stereo densification | `scene.mvs` | `scene_dense.mvs` + `scene_dense.ply` |
| `ReconstructMesh` | Surface reconstruction | `scene_dense.mvs` | `scene_dense_mesh.mvs` + `.obj` |
| `RefineMesh` | Mesh refinement | `scene_dense_mesh.mvs` | `scene_dense_mesh_refined.mvs` |
| `TextureMesh` | Texture projection | `scene_dense_mesh.mvs` + images | `scene_dense_mesh_textured.obj` |

## The Rust Adapter

### Engine struct

```rust
// crates/nadir-reconstruction/src/openmvs/mod.rs

use std::path::{Path, PathBuf};
use nadir_process::{ProcessRunner, ProcessSpec};
use tokio_util::sync::CancellationToken;

pub struct OpenMvsEngine {
    interface_colmap: PathBuf,
    densify: PathBuf,
    mesh: PathBuf,
    refine: PathBuf,
    texture: PathBuf,
    version: String,
}

impl OpenMvsEngine {
    pub fn find() -> anyhow::Result<Self> {
        let bin_dir = Self::find_bin_dir()?;
        Ok(Self {
            interface_colmap: bin_dir.join("InterfaceCOLMAP"),
            densify: bin_dir.join("DensifyPointCloud"),
            mesh: bin_dir.join("ReconstructMesh"),
            refine: bin_dir.join("RefineMesh"),
            texture: bin_dir.join("TextureMesh"),
            version: Self::detect_version(&bin_dir)?,
        })
    }

    fn find_bin_dir() -> anyhow::Result<PathBuf> {
        // Check PATH, then Pixi/conda, then common install locations
        which::which("InterfaceCOLMAP")
            .map(|p| p.parent().unwrap().to_path_buf())
            .or_else(|_| {
                let conda = std::env::var("CONDA_PREFIX")
                    .map(|p| PathBuf::from(p).join("bin"))
                    .ok();
                conda.filter(|p| p.join("InterfaceCOLMAP").exists())
                    .ok_or_else(|| anyhow::anyhow!("OpenMVS not found"))
            })
    }
}
```

### COLMAP → OpenMVS conversion

This is the critical data format bridge between the two engines:

```rust
impl OpenMvsEngine {
    pub async fn convert_from_colmap(
        &self,
        colmap_model_dir: &Path,
        image_dir: &Path,
        output_scene: &Path,
        cancel: CancellationToken,
    ) -> anyhow::Result<()> {
        let spec = ProcessSpec {
            program: self.interface_colmap.clone(),
            args: vec![
                "-i".into(), colmap_model_dir.to_string_lossy().into(),
                "-o".into(), output_scene.to_string_lossy().into(),
                "--image-folder".into(), image_dir.to_string_lossy().into(),
            ],
            current_dir: output_scene.parent().unwrap().to_path_buf(),
            timeout: None,
        };

        ProcessRunner::run(spec, None::<NullParser>, cancel, |_| {}).await
    }
}
```

### Dense reconstruction

```rust
impl DenseReconstructor for OpenMvsEngine {
    fn densify(
        &self,
        scene: &SparseReconstruction,
        cfg: &DenseConfig,
        ctx: &TaskContext,
    ) -> Result<DensePointCloud> {
        let workspace = ctx.workspace();
        let colmap_dir = ctx.resolve_artifact(&scene.model_ref)?;
        let image_dir = ctx.resolve_artifact(&scene.images_ref)?;
        let scene_mvs = workspace.join("scene.mvs");
        let dense_ply = workspace.join("scene_dense.ply");

        // Step 1: Convert COLMAP → MVS
        self.convert_from_colmap(
            &colmap_dir, &image_dir, &scene_mvs, ctx.cancellation_token()
        ).await?;

        // Step 2: Densify
        let mut args = vec![
            "-i".into(), scene_mvs.to_string_lossy().into(),
            "-o".into(), dense_ply.to_string_lossy().into(),
            "--resolution-level".into(), cfg.resolution_level.to_string(),
            "--number-views".into(), "4".into(),
            "--min-resolution".into(), "640".into(),
            "--max-resolution".into(), "3200".into(),
            "--archive-type".into(), "0".into(),  // no compression
        ];

        match cfg.quality {
            Quality::Low => {
                args.push("--resolution-level".into());
                args.push("2".into());
            }
            Quality::High => {
                args.push("--number-views".into());
                args.push("6".into());
                args.push("--min-resolution".into());
                args.push("1280".into());
            }
            _ => {}
        }

        let spec = ProcessSpec {
            program: self.densify.clone(),
            args,
            current_dir: workspace.clone(),
            timeout: Some(std::time::Duration::from_secs(3600 * 8)),
        };

        ProcessRunner::run(
            spec,
            Some(OpenMvsProgressParser),
            ctx.cancellation_token(),
            |pct| ctx.emit_progress("dense", pct),
        ).await?;

        // Step 3: Parse dense PLY
        let cloud = parse_ply_point_cloud(&dense_ply)?;
        Ok(cloud)
    }
}
```

### Mesh generation

```rust
impl MeshGenerator for OpenMvsEngine {
    fn mesh(
        &self,
        cloud: &DensePointCloud,
        cfg: &MeshConfig,
        ctx: &TaskContext,
    ) -> Result<Mesh> {
        let workspace = ctx.workspace();
        let scene_mvs = workspace.join("scene_dense.mvs");
        let mesh_obj = workspace.join("scene_dense_mesh.obj");

        let spec = ProcessSpec {
            program: self.mesh.clone(),
            args: vec![
                "-i".into(), scene_mvs.to_string_lossy().into(),
                "-o".into(), mesh_obj.to_string_lossy().into(),
                "--archive-type".into(), "0".into(),
            ],
            current_dir: workspace.clone(),
            timeout: Some(std::time::Duration::from_secs(3600 * 4)),
        };

        ProcessRunner::run(
            spec, None::<NullParser>, ctx.cancellation_token(), |_| {}
        ).await?;

        Ok(Mesh::from_obj(&mesh_obj)?)
    }
}
```

### Texturing

```rust
impl TextureGenerator for OpenMvsEngine {
    fn texture(
        &self,
        mesh: &Mesh,
        images: &ImageSet,
        cfg: &TextureConfig,
        ctx: &TaskContext,
    ) -> Result<TexturedMesh> {
        let workspace = ctx.workspace();
        let scene_mvs = workspace.join("scene_dense_mesh.mvs");
        let textured_obj = workspace.join("scene_dense_mesh_textured.obj");

        let spec = ProcessSpec {
            program: self.texture.clone(),
            args: vec![
                "-i".into(), scene_mvs.to_string_lossy().into(),
                "-o".into(), textured_obj.to_string_lossy().into(),
                "--export-type".into(), "obj".into(),
                "--decimate".into(), "0.5".into(),
            ],
            current_dir: workspace.clone(),
            timeout: Some(std::time::Duration::from_secs(3600 * 2)),
        };

        ProcessRunner::run(
            spec, None::<NullParser>, ctx.cancellation_token(), |_| {}
        ).await?;

        Ok(TexturedMesh::from_obj(&textured_obj)?)
    }
}
```

## The MVS Scene Format

OpenMVS uses a binary `.mvs` scene file containing:

```
Header:
  magic: "MVS" (3 bytes)
  version: uint32
  num_platforms: uint32

For each platform:
  name: string
  num_cameras: uint32
  For each camera:
    K: float[3x3]        (intrinsics)
    R: float[3x3]        (rotation)
    C: float[3]          (center)
  num_images: uint32
  For each image:
    platform_id: uint32
    camera_id: uint32
    image_name: string
    K: float[3x3]
    R: float[3x3]
    C: float[3]
    num_points: uint32
    For each point:
      x, y, z: float
      color: uint8[3]
      num_views: uint32
      For each view:
        image_id: uint32
        confidence: float
```

Nadir doesn't need to parse this format directly — `InterfaceCOLMAP`
handles the conversion from COLMAP's format, and the OpenMVS tools
read/write it internally. Nadir only needs to know the file paths.

## Progress Parsing

OpenMVS emits progress differently from COLMAP:

```rust
struct OpenMvsProgressParser;

impl ProgressParser for OpenMvsProgressParser {
    fn parse_line(&self, line: &str) -> Option<f32> {
        // DensifyPointCloud: "Processed 45/128 images (35%)"
        if let Some(caps) = regex::Regex::new(r"(\d+)%")
            .ok()?.captures(line)
        {
            return caps[1].parse::<f32>().ok().map(|p| p / 100.0);
        }
        None
    }
}
```

## Data Flow: COLMAP → OpenMVS → Nadir

```
COLMAP sparse/0/
  ├── cameras.bin
  ├── images.bin
  └── points3D.bin
        │
        │ InterfaceCOLMAP -i sparse/0 -o scene.mvs --image-folder images/
        ▼
scene.mvs  (OpenMVS binary format)
        │
        │ DensifyPointCloud -i scene.mvs -o scene_dense.ply
        ▼
scene_dense.ply  (dense point cloud)
scene_dense.mvs  (updated scene with depth maps)
        │
        │ ReconstructMesh -i scene_dense.mvs -o scene_mesh.obj
        ▼
scene_dense_mesh.obj  (triangle mesh)
        │
        │ TextureMesh -i scene_dense_mesh.mvs -o scene_textured.obj
        ▼
scene_dense_mesh_textured.obj  (textured mesh)
scene_dense_mesh_textured.mtl  (material file)
scene_dense_mesh_textured_texture*.jpg  (texture atlases)
```

Nadir registers each output as a typed artifact:

```
scene_dense.ply           → ArtifactKind::DensePointCloud
scene_dense_mesh.obj      → ArtifactKind::Mesh
scene_textured.obj + .mtl → ArtifactKind::TexturedMesh
```

## Workspace Management

OpenMVS generates many intermediate files. The workspace layout:

```
/scratch/nadir/runs/run-123/task-dense/
  ├── input/
  │   └── (symlink to COLMAP sparse model)
  ├── work/
  │   ├── scene.mvs
  │   ├── scene_dense.mvs
  │   ├── scene_dense.ply
  │   ├── scene_dense_mesh.mvs
  │   ├── scene_dense_mesh.obj
  │   ├── scene_dense_mesh_textured.mvs
  │   ├── scene_dense_mesh_textured.obj
  │   ├── scene_dense_mesh_textured.mtl
  │   ├── scene_dense_mesh_textured_texture0.jpg
  │   └── depthmaps/          (intermediate, large)
  │       ├── 0000.dmap
  │       ├── 0001.dmap
  │       └── ...
  ├── output/
  │   ├── dense_cloud.laz     (converted from PLY)
  │   └── textured_mesh.zip   (archived for artifact store)
  └── metadata/
      └── provenance.json
```

The `depthmaps/` directory can be very large (tens of GB). It should be
cleaned up after the dense cloud is extracted unless debug retention is
enabled.

## See Also

- [COLMAP integration](./colmap-integration.md) — the upstream engine
- [Engine assignment](./engine-assignment.md) — why OpenMVS for MVS
- [Process runner](../implementation/process-runner.md) — subprocess management
- [Pipeline & DAG](../architecture/pipeline-dag.md) — how dense fits in the DAG
