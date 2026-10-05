---
type: Architecture
title: Engine Registry & Trait API
description: "The 16 stable traits, config structs, engine resolution, and adapter pattern"
purpose: The 16 stable traits, config structs, engine resolution, and adapter pattern
last_updated: 2025-02-23
status: stable
related:
  - overview.md
  - five-domains.md
  - ../../backlog/docs/decisions/007-engine-trait-api-surface.md
  - ../photogrammetry/engine-assignment.md
---

# Engine Registry & Trait API

> **V0 clarification (ADR-012):** the 16-trait registry is the long-term
> architectural vocabulary. V0 qualifies only the CPU-only COLMAP sparse path
> and its private workspace contracts; it does not expose selectable engines or
> raw adapter flags. See
> [V0 strict sparse reconstruction](./v0-strict-sparse-profile.md).

## TL;DR

The photogrammetry API surface is exactly **16 stable Rust traits**. Each
trait has a small method signature using domain types. Configuration structs
have 5–10 fields. Engine-specific CLI flags live inside adapters and are
never exposed to the pipeline. The `EngineRegistry` maps `(TaskKind,
engine_name)` to trait implementations at startup.

## The 16 Traits

### Domain 1: Dataset (1 trait)

```rust
pub trait ImageProvider: Engine {
    fn ingest(&self, path: &Path) -> Result<ImageSet>;
}
```

### Domain 2: Reconstruction (6 traits)

```rust
pub trait FeatureExtractor: Engine {
    fn extract(
        &self,
        images: &ImageSet,
        cfg: &FeatureConfig,
    ) -> Result<Features>;
}

pub trait FeatureMatcher: Engine {
    fn match_features(
        &self,
        features: &Features,
        cfg: &MatchingConfig,
    ) -> Result<Matches>;
}

pub trait SparseReconstructor: Engine {
    fn reconstruct(
        &self,
        matches: &Matches,
        cfg: &SfMConfig,
    ) -> Result<SfmOutput>;
    // SfmOutput = { camera_poses, sparse_cloud, camera_models }
}

pub trait DenseReconstructor: Engine {
    fn densify(
        &self,
        scene: &SparseReconstruction,
        cfg: &DenseConfig,
    ) -> Result<DensePointCloud>;
}

pub trait MeshGenerator: Engine {
    fn mesh(
        &self,
        cloud: &DensePointCloud,
        cfg: &MeshConfig,
    ) -> Result<Mesh>;
}

pub trait TextureGenerator: Engine {
    fn texture(
        &self,
        mesh: &Mesh,
        images: &ImageSet,
        cfg: &TextureConfig,
    ) -> Result<TexturedMesh>;
}
```

### Domain 3: Geometry (2 traits)

```rust
pub trait Georeferencer: Engine {
    fn georeference(
        &self,
        scene: &SparseReconstruction,
        gps: &GpsData,
        cfg: &GeorefConfig,
    ) -> Result<GeoreferencedScene>;
}

pub trait CoordinateTransformer: Engine {
    fn transform(&self, point: GeoPoint) -> Result<GeoPoint>;
    fn transform_many(&self, points: &mut [GeoPoint]) -> Result<()>;
}
```

### Domain 4: Surface (3 traits)

```rust
pub trait PointCloudFilter: Engine {
    fn filter(
        &self,
        cloud: &DensePointCloud,
        cfg: &FilterConfig,
    ) -> Result<DensePointCloud>;
}

pub trait PointCloudClassifier: Engine {
    fn classify(
        &self,
        cloud: &DensePointCloud,
        cfg: &ClassifyConfig,
    ) -> Result<ClassifiedPointCloud>;
}

pub trait SurfaceGenerator: Engine {
    fn generate_dsm(
        &self,
        cloud: &DensePointCloud,
        cfg: &DemConfig,
    ) -> Result<Raster>;

    fn generate_dtm(
        &self,
        cloud: &ClassifiedPointCloud,
        cfg: &DemConfig,
    ) -> Result<Raster>;
}
```

### Domain 5: Cartography (2 traits)

```rust
pub trait Orthorectifier: Engine {
    fn orthorectify(
        &self,
        images: &ImageSet,
        scene: &GeoreferencedScene,
        surface: &Raster,
        cfg: &OrthoConfig,
    ) -> Result<Raster>;
}

pub trait Mosaicker: Engine {
    fn mosaic(
        &self,
        orthos: &[Raster],
        cfg: &MosaicConfig,
    ) -> Result<Raster>;
}
```

### I/O (2 traits)

```rust
pub trait RasterStore: Engine {
    fn read(&self, path: &Path) -> Result<Raster>;
    fn write(&self, raster: &Raster, path: &Path) -> Result<()>;
}

pub trait PointCloudStore: Engine {
    fn read(&self, path: &Path) -> Result<DensePointCloud>;
    fn write(&self, cloud: &DensePointCloud, path: &Path) -> Result<()>;
}
```

## The Base Engine Trait

All 16 traits extend a common base:

```rust
pub trait Engine: Send + Sync + 'static {
    fn name(&self) -> &str;
    fn version(&self) -> &str;
    fn capabilities(&self) -> EngineCapabilities;
}

pub struct EngineCapabilities {
    pub feature_extraction: bool,
    feature_matching: bool,
    sfm: bool,
    dense_reconstruction: bool,
    meshing: bool,
    texturing: bool,
    georeferencing: bool,
    dem: bool,
    orthomosaic: bool,
    requires_gpu: bool,
    supports_cuda: bool,
}
```

## Config Structs

Small, domain-level. No engine-specific flags.

```rust
pub struct FeatureConfig {
    pub detector: Detector,       // Sift | Orb | Akaze
    pub max_features: u32,        // 8192
}

pub struct MatchingConfig {
    pub method: MatchMethod,      // Exhaustive | VocabTree | Sequential | Spatial
    pub ratio_test: f64,          // 0.8
}

pub struct SfmConfig {
    pub max_iterations: u32,      // 100
    pub min_inliers: u32,         // 30
}

pub struct DenseConfig {
    pub quality: Quality,         // Low | Medium | High
    pub resolution_level: u32,    // 1
}

pub struct DemConfig {
    pub resolution: f64,          // 0.05 (meters)
    pub gap_filling: GapFilling,  // None | Standard | Aggressive
}

pub struct OrthoConfig {
    pub resolution: f64,          // 0.03
    pub format: OutputFormat,     // GeoTiff | Cog
}

pub struct FilterConfig {
    pub outlier_k: u32,           // 16
    pub outlier_std: f64,         // 2.0
}

pub struct GeorefConfig {
    pub target_crs: Option<Crs>,  // None = auto-detect
    pub method: GeorefMethod,     // Similarity | Rigid
}
```

### Escape hatch for advanced users

```rust
pub struct EngineOverrides {
    pub odm: HashMap<String, String>,      // raw ODM flags
    pub colmap: HashMap<String, String>,   // raw COLMAP flags
    pub openmvs: HashMap<String, String>,  // raw OpenMVS flags
}
```

Used in config:

```toml
[steps.params]
detector = "sift"
max_features = 8192

[steps.params.engine_overrides.colmap]
use_gpu = "true"
gpu_index = "0"
```

## The Engine Registry

Maps `(TaskKind, engine_name)` to trait implementations.

```rust
pub struct EngineRegistry {
    // Domain 1
    ingesters: HashMap<String, Arc<dyn ImageProvider>>,
    // Domain 2
    feature_extractors: HashMap<String, Arc<dyn FeatureExtractor>>,
    matchers: HashMap<String, Arc<dyn FeatureMatcher>>,
    sfm: HashMap<String, Arc<dyn SparseReconstructor>>,
    dense: HashMap<String, Arc<dyn DenseReconstructor>>,
    meshers: HashMap<String, Arc<dyn MeshGenerator>>,
    texturers: HashMap<String, Arc<dyn TextureGenerator>>,
    // Domain 3
    georeferencers: HashMap<String, Arc<dyn Georeferencer>>,
    transformers: HashMap<String, Arc<dyn CoordinateTransformer>>,
    // Domain 4
    pc_filters: HashMap<String, Arc<dyn PointCloudFilter>>,
    pc_classifiers: HashMap<String, Arc<dyn PointCloudClassifier>>,
    surface: HashMap<String, Arc<dyn SurfaceGenerator>>,
    // Domain 5
    orthorectifiers: HashMap<String, Arc<dyn Orthorectifier>>,
    mosaickers: HashMap<String, Arc<dyn Mosaicker>>,
    // I/O
    raster_stores: HashMap<String, Arc<dyn RasterStore>>,
    pc_stores: HashMap<String, Arc<dyn PointCloudStore>>,
}
```

### Construction

```rust
impl EngineRegistry {
    pub fn from_config(config: &NadirConfig) -> Result<Self> {
        let mut reg = Self::empty();

        // Domain 1: always Rust
        reg.register_ingester("builtin", RustImageIngester);

        // Domain 2: based on config
        match config.features.engine.as_str() {
            "colmap" => {
                let colmap = ColmapEngine::find()?;
                reg.register_feature_extractor("colmap", colmap.clone());
                reg.register_matcher("colmap", colmap.clone());
                reg.register_sfm("colmap", colmap.clone());
            }
            "opencv" => {
                reg.register_feature_extractor("opencv", OpencvExtractor);
            }
            other => bail!("unknown feature engine: {other}"),
        }

        match config.dense.engine.as_str() {
            "openmvs" => {
                let mvs = OpenMvsEngine::find()?;
                reg.register_dense("openmvs", mvs.clone());
                reg.register_mesher("openmvs", mvs.clone());
                reg.register_texturer("openmvs", mvs);
            }
            "colmap" => {
                let colmap = ColmapEngine::find()?;
                reg.register_dense("colmap", colmap);
            }
            other => bail!("unknown dense engine: {other}"),
        }

        // Domain 3: always PROJ + nalgebra
        reg.register_georeferencer("builtin", RustGeoreferencer);
        reg.register_transformer("proj", ProjTransformer);

        // Domain 4: Rust + optional PDAL
        reg.register_surface("builtin", RustSurfaceGenerator);
        reg.register_pc_filter("builtin", RustPointCloudFilter);
        if let Ok(pdal) = PdalEngine::find() {
            reg.register_pc_filter("pdal", pdal.clone());
            reg.register_pc_classifier("pdal", pdal);
        }

        // Domain 5: Rust + GDAL
        reg.register_orthorectifier("builtin", RustOrthorectifier);
        reg.register_raster_store("gdal", GdalRasterStore);

        Ok(reg)
    }
}
```

### Resolution at runtime

```rust
impl EngineRegistry {
    pub fn resolve(
        &self,
        task: &TaskKind,
        engine_name: &str,
    ) -> Result<Box<dyn TaskExecutor>> {
        match task {
            TaskKind::ExtractFeatures => {
                let e = self.feature_extractors.get(engine_name)
                    .ok_or_else(|| anyhow!("no feature engine: {engine_name}"))?;
                Ok(Box::new(FeatureExtractionTask(e.clone())))
            }
            TaskKind::SparseReconstruction => {
                let e = self.sfm.get(engine_name)
                    .ok_or_else(|| anyhow!("no SfM engine: {engine_name}"))?;
                Ok(Box::new(SfmTask(e.clone())))
            }
            TaskKind::GenerateDsm => {
                let e = self.surface.get(engine_name)
                    .ok_or_else(|| anyhow!("no surface engine: {engine_name}"))?;
                Ok(Box::new(DsmTask(e.clone())))
            }
            // ... one arm per TaskKind
        }
    }
}
```

## The Adapter Pattern

Each engine adapter implements one or more traits. The adapter translates
domain-level config into engine-specific CLI flags. The pipeline never sees
engine-specific flags.

### Example: COLMAP adapter

```rust
pub struct ColmapEngine {
    binary: PathBuf,
}

impl SparseReconstructor for ColmapEngine {
    fn reconstruct(
        &self,
        matches: &Matches,
        cfg: &SfmConfig,
    ) -> Result<SfmOutput> {
        let ws = self.prepare_workspace(matches)?;

        // The ugly COLMAP flags live HERE, hidden from the pipeline
        self.run(&[
            "mapper",
            "--database_path", &ws.db(),
            "--image_path", &ws.images(),
            "--output_path", &ws.sparse(),
            "--Mapper.ba_max_num_iterations", &cfg.max_iterations.to_string(),
            "--Mapper.filter_min_tri_angle", "1.5",
            "--Mapper.ba_global_max_refinements", "5",
        ])?;

        let (cameras, poses, sparse) = parse_colmap_model(&ws.sparse())?;
        Ok(SfmOutput {
            camera_models: ctx.register(cameras)?,
            camera_poses: ctx.register(poses)?,
            sparse_cloud: ctx.register(sparse)?,
        })
    }
}
```

### Example: OpenMVS adapter

```rust
pub struct OpenMvsEngine {
    interface: PathBuf,
    densify: PathBuf,
}

impl DenseReconstructor for OpenMvsEngine {
    fn densify(
        &self,
        scene: &SparseReconstruction,
        cfg: &DenseConfig,
    ) -> Result<DensePointCloud> {
        let mvs_scene = self.export_scene(scene)?;
        self.run(&self.densify, &[
            "-i", &mvs_scene,
            "--resolution-level", &cfg.resolution_level.to_string(),
            "--number-views", "4",
        ])?;
        parse_mvs_ply(&output)
    }
}
```

## Engine Implementations Matrix

| Trait | `builtin` (Rust) | `colmap` | `openmvs` | `pdal` | `gdal` | `proj` | `opencv` |
|---|---|---|---|---|---|---|---|
| ImageProvider | ✅ | | | | | | |
| FeatureExtractor | | ✅ | | | | | ✅ |
| FeatureMatcher | | ✅ | | | | | ✅ |
| SparseReconstructor | | ✅ | | | | | |
| DenseReconstructor | | ✅ | ✅ | | | | |
| MeshGenerator | | | ✅ | | | | |
| TextureGenerator | | | ✅ | | | | |
| Georeferencer | ✅ | | | | | | |
| CoordinateTransformer | | | | | | ✅ | |
| PointCloudFilter | ✅ | | | ✅ | | | |
| PointCloudClassifier | | | | ✅ | | | |
| SurfaceGenerator | ✅ | | | | ✅ | | |
| Orthorectifier | ✅ | | | | ✅ | | |
| Mosaicker | ✅ | | | | ✅ | | |
| RasterStore | | | | | ✅ | | |
| PointCloudStore | ✅ | | | | | | |

## See Also

- [ADR-007: Engine trait API surface](../../backlog/docs/decisions/007-engine-trait-api-surface.md) — why 16 traits
- [Five domains](./five-domains.md) — domain-level detail
- [Engine assignment](../photogrammetry/engine-assignment.md) — which tool does what
- [COLMAP integration](../photogrammetry/colmap-integration.md) — COLMAP adapter detail
- [OpenMVS integration](../photogrammetry/openmvs-integration.md) — OpenMVS adapter detail
