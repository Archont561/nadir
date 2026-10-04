//! Engine trait vocabulary shared by planners, executors and adapters.
//!
//! Nadir keeps engine-specific command-line flags inside adapters. The public surface here
//! names the stable domain operations, the config each operation receives, the artifacts it
//! reads and writes, and the preflight metadata a registry can query without running a
//! stage.

use std::future::Future;
use std::path::{Path, PathBuf};
use std::pin::Pin;

use serde::{Deserialize, Serialize};

use crate::artifact::Artifact;

/// Result type shared by every engine trait method.
pub type EngineResult<T> = Result<T, EngineError>;

/// Object-safe future returned by stage traits.
///
/// Traits return a boxed future instead of using `async fn` directly so registries can hold
/// `dyn FeatureExtractor`, `dyn SparseReconstructor`, and the other stage trait objects
/// while adapters remain free to perform asynchronous subprocess work.
pub type EngineFuture<'a, T> = Pin<Box<dyn Future<Output = EngineResult<T>> + Send + 'a>>;

/// Callback an adapter uses to report progress without owning the UI.
pub type ProgressSink<'a> = &'a (dyn Fn(Progress) + Send + Sync + 'a);

/// The base trait all stage traits extend: what this adapter is, and what it can do.
///
/// `version` is part of artifact identity. A caller can query the name, version and
/// capabilities before deciding whether to execute any stage.
pub trait Engine: Send + Sync + 'static {
    /// The engine name as it appears in a pipeline definition.
    fn name(&self) -> &str;

    /// Version string that participates in task hashing.
    fn version(&self) -> &str;

    /// Stage capabilities and hardware requirements advertised at preflight time.
    fn capabilities(&self) -> EngineCapabilities;
}

/// Stage capabilities and hardware requirements advertised by an engine.
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub struct EngineCapabilities {
    /// Whether the engine can extract image features.
    pub feature_extraction: bool,
    /// Whether the engine can match extracted features.
    pub feature_matching: bool,
    /// Whether the engine can run sparse reconstruction.
    pub sfm: bool,
    /// Whether the engine can produce dense point clouds.
    pub dense_reconstruction: bool,
    /// Whether the engine can build meshes.
    pub meshing: bool,
    /// Whether the engine can texture meshes.
    pub texturing: bool,
    /// Whether the engine can georeference a scene.
    pub georeferencing: bool,
    /// Whether the engine can generate elevation rasters.
    pub dem: bool,
    /// Whether the engine can orthorectify and mosaic imagery.
    pub orthomosaic: bool,
    /// Whether the advertised capabilities require GPU hardware.
    pub requires_gpu: bool,
    /// Whether the engine can use CUDA when present.
    pub supports_cuda: bool,
}

/// Progress a long-running engine reports back while it works.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(tag = "kind", rename_all = "snake_case")]
pub enum Progress {
    /// A fraction in `0.0..=1.0`, where the engine knows its denominator.
    Fraction {
        /// Completed fraction.
        completed: f64,
    },
    /// A named stage with a one-based index and optional total.
    Stage {
        /// Human-readable stage name.
        name: String,
        /// One-based stage index.
        index: usize,
        /// Total stages, or zero when the engine does not know.
        total: usize,
    },
    /// A free-form log line.
    Message {
        /// Message text.
        text: String,
    },
}

impl Progress {
    /// Return a renderable fraction when the progress event knows one.
    pub fn fraction(&self) -> Option<f64> {
        match self {
            Progress::Fraction { completed } => Some(*completed),
            Progress::Stage { index, total, .. } if *total > 0 => {
                Some(*index as f64 / *total as f64)
            }
            _ => None,
        }
    }
}

/// Shared error vocabulary for engine adapters.
#[derive(Debug, thiserror::Error)]
pub enum EngineError {
    /// The engine binary is not installed or not visible on `PATH`.
    #[error("{engine} is not installed or not on PATH")]
    NotInstalled {
        /// Engine name.
        engine: &'static str,
    },
    /// The engine ran and exited unsuccessfully.
    #[error("{engine} failed in stage {stage}: {message}")]
    Failed {
        /// Engine name.
        engine: &'static str,
        /// Stage name.
        stage: &'static str,
        /// Adapter-provided failure detail.
        message: String,
    },
    /// The adapter could not parse the engine output.
    #[error("could not parse {engine} output in stage {stage}: {message}")]
    Parse {
        /// Engine name.
        engine: &'static str,
        /// Stage name.
        stage: &'static str,
        /// Parser failure detail.
        message: String,
    },
    /// The engine exceeded a resource limit.
    #[error("{engine} exceeded its {resource} budget in stage {stage}")]
    BudgetExceeded {
        /// Engine name.
        engine: &'static str,
        /// Stage name.
        stage: &'static str,
        /// Resource that exceeded its budget.
        resource: &'static str,
    },
}

/// A handle an engine receives to reach the workspace.
#[derive(Debug, Clone)]
pub struct Context {
    /// Workspace root: where artifacts, caches and adapter scratch space live.
    pub workspace: PathBuf,
    /// Per-run scratch directory. Adapters may write temporary files here.
    pub scratch: PathBuf,
}

impl Context {
    /// Build a context rooted at `workspace`.
    pub fn new(workspace: impl Into<PathBuf>) -> Self {
        let workspace = workspace.into();
        let scratch = workspace.join("scratch");
        Self { workspace, scratch }
    }

    /// Create the scratch directory, failing at the start of a stage rather than at the
    /// first deep write inside one.
    pub fn ensure_scratch(&self) -> std::io::Result<&Path> {
        std::fs::create_dir_all(&self.scratch)?;
        Ok(&self.scratch)
    }
}

/// The outcome every stage returns: artifacts produced and adapter provenance.
#[derive(Debug, Clone)]
pub struct Produced {
    /// Artifacts this call produced, in the order the trait documents.
    pub artifacts: Vec<Artifact>,
    /// Provenance worth recording for `nadir explain`.
    pub provenance: Provenance,
}

impl Produced {
    /// Build a produced result with one artifact and empty provenance.
    pub fn one(artifact: Artifact) -> Self {
        Self {
            artifacts: vec![artifact],
            provenance: Provenance::default(),
        }
    }
}

/// Adapter provenance recorded alongside produced artifacts.
#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize, Deserialize)]
pub struct Provenance {
    /// Engine binary and version as reported by the adapter.
    pub engine_version: String,
    /// Exact command line used to reproduce a subprocess invocation.
    pub command: Vec<String>,
    /// Runtime duration in milliseconds.
    pub duration_ms: u64,
    /// Peak resident memory when the platform can report it.
    pub peak_rss_bytes: Option<u64>,
}

// ── Domain 1: Dataset ────────────────────────────────────────────────────────

/// Configuration for ingesting an image directory.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct IngestConfig {
    /// Image extensions to accept.
    pub extensions: Vec<String>,
    /// Whether GPS EXIF is mandatory.
    pub require_gps: bool,
    /// Whether unreadable EXIF should fail the whole ingest.
    pub strict_exif: bool,
}

/// What dataset inspection reports without producing artifacts.
#[derive(Debug, Clone, Default, PartialEq, Serialize, Deserialize)]
pub struct DatasetReport {
    /// Number of usable images.
    pub images: usize,
    /// Camera models encountered in the dataset.
    pub cameras: Vec<String>,
    /// Number of images carrying GPS data.
    pub images_with_gps: usize,
    /// Whether RTK-quality positioning is present.
    pub has_rtk: bool,
    /// Estimated ground sample distance in metres per pixel.
    pub gsd_metres: Option<f64>,
    /// Warnings or validation findings.
    pub issues: Vec<String>,
}

/// Provides image datasets to the pipeline.
pub trait ImageProvider: Engine {
    /// Ingest a directory of images into a validated dataset.
    fn ingest<'a>(
        &'a self,
        path: &'a Path,
        cfg: &'a IngestConfig,
        ctx: &'a Context,
        progress: ProgressSink<'a>,
    ) -> EngineFuture<'a, Produced>;

    /// Report a dataset's geometry without producing artifacts.
    fn inspect<'a>(&'a self, path: &'a Path) -> EngineFuture<'a, DatasetReport>;
}

// ── Domain 2: Reconstruction ────────────────────────────────────────────────

/// Configuration for feature extraction.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct FeatureConfig {
    /// Maximum number of features per image.
    pub max_features: usize,
    /// Detection threshold. Lower finds more features and more noise.
    pub detection_threshold: f64,
}

/// Configuration for feature matching.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MatchingConfig {
    /// Maximum descriptor distance accepted as a match.
    pub max_distance: f64,
    /// Whether to run a second guided-matching pass.
    pub guided_matching: bool,
    /// Worker threads requested by the adapter.
    pub num_threads: usize,
}

/// Configuration for sparse reconstruction.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SfMConfig {
    /// Minimum inliers for an accepted registration.
    pub min_num_inliers: usize,
    /// Number of initial image registration pairs to try.
    pub init_image_reg_pairs: usize,
    /// Whether bundle adjustment may refine focal length.
    pub ba_refine_focal_length: bool,
    /// Triangulation threshold as a multiple of image pixel scale.
    pub triangulation_threshold: f64,
}

/// Configuration for dense reconstruction.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DenseConfig {
    /// Number of source views used for depth fusion.
    pub num_views: usize,
    /// Minimum accepted depth.
    pub depth_min: f64,
    /// Maximum accepted depth.
    pub depth_max: f64,
    /// Consistency threshold for accepting dense points.
    pub consistency_threshold: f64,
}

/// Configuration for mesh generation.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MeshConfig {
    /// Reconstruction depth.
    pub depth: u8,
    /// Surface scale parameter.
    pub scale: f64,
    /// Whether to use linear fitting.
    pub linear_fit: bool,
}

/// Configuration for mesh texturing.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TextureConfig {
    /// Output texture resolution.
    pub resolution: u32,
    /// Whether to apply global seam leveling before blending.
    pub global_seam_leveling: bool,
    /// Worker threads requested by the adapter.
    pub num_threads: usize,
}

/// Extracts image features.
pub trait FeatureExtractor: Engine {
    /// Extract feature descriptors from an image dataset artifact.
    fn extract<'a>(
        &'a self,
        images: &'a Artifact,
        cfg: &'a FeatureConfig,
        ctx: &'a Context,
        progress: ProgressSink<'a>,
    ) -> EngineFuture<'a, Produced>;
}

/// Matches image features.
pub trait FeatureMatcher: Engine {
    /// Match feature descriptors into an image correspondence graph.
    fn match_features<'a>(
        &'a self,
        features: &'a Artifact,
        cfg: &'a MatchingConfig,
        ctx: &'a Context,
        progress: ProgressSink<'a>,
    ) -> EngineFuture<'a, Produced>;
}

/// Reconstructs sparse scenes from matched features.
pub trait SparseReconstructor: Engine {
    /// Build a sparse reconstruction from matched features.
    fn reconstruct<'a>(
        &'a self,
        matches: &'a Artifact,
        cfg: &'a SfMConfig,
        ctx: &'a Context,
        progress: ProgressSink<'a>,
    ) -> EngineFuture<'a, Produced>;
}

/// Produces dense point clouds from sparse scenes.
pub trait DenseReconstructor: Engine {
    /// Produce a dense point cloud from a sparse scene.
    fn densify<'a>(
        &'a self,
        scene: &'a Artifact,
        cfg: &'a DenseConfig,
        ctx: &'a Context,
        progress: ProgressSink<'a>,
    ) -> EngineFuture<'a, Produced>;
}

/// Builds meshes from dense point clouds.
pub trait MeshGenerator: Engine {
    /// Build a mesh from a dense point cloud.
    fn mesh<'a>(
        &'a self,
        cloud: &'a Artifact,
        cfg: &'a MeshConfig,
        ctx: &'a Context,
        progress: ProgressSink<'a>,
    ) -> EngineFuture<'a, Produced>;
}

/// Textures meshes with source imagery.
pub trait TextureGenerator: Engine {
    /// Project source images onto a mesh.
    fn texture<'a>(
        &'a self,
        mesh: &'a Artifact,
        images: &'a Artifact,
        cfg: &'a TextureConfig,
        ctx: &'a Context,
        progress: ProgressSink<'a>,
    ) -> EngineFuture<'a, Produced>;
}

// ── Domain 3: Geometry ──────────────────────────────────────────────────────

/// Configuration for georeferencing.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct GeorefConfig {
    /// Ground control points. Empty means derive from the SfM GPS solution.
    pub ground_control_points: Vec<GroundControlPoint>,
    /// Target CRS as an EPSG code or a PROJ string.
    pub target_crs: String,
}

/// A ground control point linking an image pixel to a real-world coordinate.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct GroundControlPoint {
    /// Source image path.
    pub image: PathBuf,
    /// Pixel x coordinate.
    pub x_pixel: f64,
    /// Pixel y coordinate.
    pub y_pixel: f64,
    /// Target easting.
    pub easting: f64,
    /// Target northing.
    pub northing: f64,
    /// Target elevation when known.
    pub elevation: Option<f64>,
}

/// Georeferences reconstructed scenes.
pub trait Georeferencer: Engine {
    /// Align a scene into the requested coordinate reference system.
    fn georeference<'a>(
        &'a self,
        scene: &'a Artifact,
        gps: &'a Artifact,
        cfg: &'a GeorefConfig,
        ctx: &'a Context,
    ) -> EngineFuture<'a, Produced>;
}

/// A WGS84-like geodetic point.
#[derive(Debug, Clone, Copy, PartialEq, Serialize, Deserialize)]
pub struct GeoPoint {
    /// Longitude in degrees.
    pub longitude: f64,
    /// Latitude in degrees.
    pub latitude: f64,
    /// Elevation in metres.
    pub elevation: f64,
}

/// Transforms coordinates between reference systems.
pub trait CoordinateTransformer: Engine {
    /// Transform one point.
    fn transform<'a>(&'a self, point: GeoPoint) -> EngineFuture<'a, GeoPoint>;

    /// Transform many points in place.
    fn transform_many<'a>(&'a self, points: &'a mut [GeoPoint]) -> EngineFuture<'a, ()>;
}

// ── Domain 4: Surface ───────────────────────────────────────────────────────

/// Configuration for point cloud filtering.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct FilterConfig {
    /// Remove statistical outliers beyond this many standard deviations.
    pub outlier_k: u32,
}

/// Configuration for point cloud classification.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ClassifyConfig {
    /// Whether to classify ground points for DTM generation.
    pub classify_ground: bool,
}

/// Configuration for elevation raster generation.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DemConfig {
    /// Output resolution in metres.
    pub resolution_metres: f64,
    /// Whether to fill holes by interpolation.
    pub fill_holes: bool,
    /// Maximum hole radius eligible for interpolation.
    pub max_hole_radius_metres: f64,
}

/// Filters point clouds.
pub trait PointCloudFilter: Engine {
    /// Remove outliers from a point cloud.
    fn filter<'a>(
        &'a self,
        cloud: &'a Artifact,
        cfg: &'a FilterConfig,
        ctx: &'a Context,
        progress: ProgressSink<'a>,
    ) -> EngineFuture<'a, Produced>;
}

/// Classifies point clouds.
pub trait PointCloudClassifier: Engine {
    /// Classify point-cloud points into domain classes such as ground.
    fn classify<'a>(
        &'a self,
        cloud: &'a Artifact,
        cfg: &'a ClassifyConfig,
        ctx: &'a Context,
        progress: ProgressSink<'a>,
    ) -> EngineFuture<'a, Produced>;
}

/// Generates elevation rasters.
pub trait SurfaceGenerator: Engine {
    /// Generate a digital surface model.
    fn generate_dsm<'a>(
        &'a self,
        cloud: &'a Artifact,
        cfg: &'a DemConfig,
        ctx: &'a Context,
        progress: ProgressSink<'a>,
    ) -> EngineFuture<'a, Produced>;

    /// Generate a digital terrain model.
    fn generate_dtm<'a>(
        &'a self,
        cloud: &'a Artifact,
        cfg: &'a DemConfig,
        ctx: &'a Context,
        progress: ProgressSink<'a>,
    ) -> EngineFuture<'a, Produced>;
}

// ── Domain 5: Cartography ───────────────────────────────────────────────────

/// Configuration for orthorectification.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct OrthoConfig {
    /// Output resolution in metres.
    pub resolution_metres: f64,
    /// Whether to write Cloud-Optimized GeoTIFFs.
    pub cloud_optimized: bool,
    /// Worker threads requested by the adapter.
    pub num_threads: usize,
}

/// Configuration for mosaicking.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MosaicConfig {
    /// Tile size in pixels.
    pub tile_size_pixels: usize,
    /// XYZ pyramid levels for the web map.
    pub pyramid_levels: Vec<usize>,
}

/// Orthorectifies source imagery.
pub trait Orthorectifier: Engine {
    /// Orthorectify images against a georeferenced scene and surface.
    fn orthorectify<'a>(
        &'a self,
        images: &'a Artifact,
        scene: &'a Artifact,
        surface: &'a Artifact,
        cfg: &'a OrthoConfig,
        ctx: &'a Context,
        progress: ProgressSink<'a>,
    ) -> EngineFuture<'a, Produced>;
}

/// Mosaics orthorectified rasters.
pub trait Mosaicker: Engine {
    /// Build a web-map mosaic from orthorectified rasters.
    fn mosaic<'a>(
        &'a self,
        orthos: &'a [Artifact],
        cfg: &'a MosaicConfig,
        ctx: &'a Context,
        progress: ProgressSink<'a>,
    ) -> EngineFuture<'a, Produced>;
}

// ── I/O ─────────────────────────────────────────────────────────────────────

/// Reads and writes raster artifacts.
pub trait RasterStore: Engine {
    /// Read a raster artifact from storage.
    fn read<'a>(&'a self, path: &'a Path) -> EngineFuture<'a, Produced>;

    /// Write a raster artifact to storage.
    fn write<'a>(&'a self, raster: &'a Artifact, path: &'a Path) -> EngineFuture<'a, ()>;
}

/// Reads and writes point cloud artifacts.
pub trait PointCloudStore: Engine {
    /// Read a point cloud artifact from storage.
    fn read<'a>(&'a self, path: &'a Path) -> EngineFuture<'a, Produced>;

    /// Write a point cloud artifact to storage.
    fn write<'a>(&'a self, cloud: &'a Artifact, path: &'a Path) -> EngineFuture<'a, ()>;
}

/// The sixteen trait names, in the architecture's public order.
pub const TRAIT_NAMES: [&str; 16] = [
    "ImageProvider",
    "FeatureExtractor",
    "FeatureMatcher",
    "SparseReconstructor",
    "DenseReconstructor",
    "MeshGenerator",
    "TextureGenerator",
    "Georeferencer",
    "CoordinateTransformer",
    "PointCloudFilter",
    "PointCloudClassifier",
    "SurfaceGenerator",
    "Orthorectifier",
    "Mosaicker",
    "RasterStore",
    "PointCloudStore",
];

/// Count how many processing-stage capabilities an engine advertises.
pub fn covered_capabilities(caps: &EngineCapabilities) -> usize {
    let flags = [
        caps.feature_extraction,
        caps.feature_matching,
        caps.sfm,
        caps.dense_reconstruction,
        caps.meshing,
        caps.texturing,
        caps.georeferencing,
        caps.dem,
        caps.orthomosaic,
    ];
    flags.iter().filter(|flag| **flag).count()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn a_fraction_reports_itself() {
        assert_eq!(Progress::Fraction { completed: 0.5 }.fraction(), Some(0.5));
    }

    #[test]
    fn a_stage_reports_its_fraction_when_it_knows_its_total() {
        let progress = Progress::Stage {
            name: "mapper".to_owned(),
            index: 2,
            total: 4,
        };
        assert_eq!(progress.fraction(), Some(0.5));
    }

    #[test]
    fn a_message_is_not_progress() {
        let progress = Progress::Message {
            text: "loading image".to_owned(),
        };
        assert_eq!(progress.fraction(), None);
    }

    #[test]
    fn a_stage_that_does_not_know_its_total_is_indeterminate() {
        let progress = Progress::Stage {
            name: "registering".to_owned(),
            index: 1,
            total: 0,
        };
        assert_eq!(progress.fraction(), None);
    }

    #[test]
    fn a_missing_engine_is_distinguishable_from_a_failure() {
        let error = EngineError::NotInstalled { engine: "colmap" };
        assert!(error.to_string().contains("not installed"));
        assert!(!matches!(error, EngineError::Failed { .. }));
    }

    #[test]
    fn a_parse_failure_names_the_engine_and_stage() {
        let error = EngineError::Parse {
            engine: "colmap",
            stage: "mapper",
            message: "unterminated line".to_owned(),
        };
        let message = error.to_string();
        assert!(message.contains("colmap"), "got {message}");
        assert!(message.contains("mapper"), "got {message}");
    }

    #[test]
    fn progress_round_trips_through_json() {
        let progress = Progress::Stage {
            name: "densify".to_owned(),
            index: 3,
            total: 7,
        };
        let json = serde_json::to_string(&progress).expect("serializing progress");
        assert_eq!(
            progress,
            serde_json::from_str(&json).expect("deserializing progress")
        );
    }

    #[test]
    fn a_context_derives_its_scratch_directory_from_the_workspace() {
        let ctx = Context::new("/data/flight_042");
        assert_eq!(ctx.scratch, PathBuf::from("/data/flight_042/scratch"));
    }

    #[test]
    fn the_trait_surface_is_sixteen() {
        assert_eq!(TRAIT_NAMES.len(), 16);
    }

    #[test]
    fn trait_names_are_unique() {
        let mut names = TRAIT_NAMES.to_vec();
        names.sort_unstable();
        names.dedup();
        assert_eq!(names.len(), TRAIT_NAMES.len());
    }

    #[test]
    fn capabilities_count_what_an_engine_can_do() {
        let caps = EngineCapabilities {
            feature_extraction: true,
            feature_matching: true,
            sfm: true,
            ..EngineCapabilities::default()
        };
        assert_eq!(covered_capabilities(&caps), 3);
    }

    #[test]
    fn capabilities_ignore_gpu_flags_when_counting_stages() {
        let caps = EngineCapabilities {
            sfm: true,
            requires_gpu: true,
            supports_cuda: true,
            ..EngineCapabilities::default()
        };
        assert_eq!(covered_capabilities(&caps), 1);
    }
}
