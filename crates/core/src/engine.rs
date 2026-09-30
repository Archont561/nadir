//! The 16 stable traits: the API boundary between the pipeline and its engines
//! (ADR-007, `architecture/engine-registry.md`).
//!
//! The pipeline knows only these method signatures. Engine-specific CLI flags live inside
//! adapters, and the pipeline never sees them — which is what makes engine swapping, stage
//! caching and parallel DAG branches possible rather than aspirational.
//!
//! Every trait extends [`Engine`], so the registry can ask any adapter what it is and what
//! version it is without knowing which trait it implements. That is not decoration: the
//! engine version is part of the artifact hash, and the hash is what makes caching correct.
//!
//! Signatures are `async` here and synchronous in the KB, because the adapters are
//! subprocesses and a synchronous `fn` that blocks a thread for 40 minutes is a design
//! that only works until the DAG runs two branches at once (ADR-005). The trait *surface*
//! — the names, the config structs, the one-method-per-stage shape — is the KB's.

use std::path::{Path, PathBuf};

use async_trait::async_trait;
use serde::{Deserialize, Serialize};

use crate::artifact::{Artifact, ArtifactHash};

/// The base trait all sixteen extend: what this adapter is, and what it can do.
///
/// `version` is not a diagnostic string. It is an input to the artifact hash
/// ([`crate::artifact::ArtifactHash::of_task`]), so upgrading an adapter invalidates
/// everything it produced. That is the difference between a cache and a bug report.
pub trait Engine: Send + Sync + 'static {
    /// The engine's name, as it appears in a pipeline's `engine = "colmap"` field.
    fn name(&self) -> &str;

    /// The version string that goes into the artifact hash. Must change whenever the
    /// adapter's output could change — which includes a change in the engine's own
    /// version, not just the adapter's code.
    fn version(&self) -> &str;

    /// What this engine can do. The registry consults it to fail at plan time rather than
    /// 40 minutes into a run.
    fn capabilities(&self) -> EngineCapabilities;
}

/// Which stages an engine implements.
///
/// A bag of booleans rather than a set of enum variants: it is derived data about an
/// adapter, the registry intersects it with what a pipeline needs, and the report names
/// the missing capability directly.
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub struct EngineCapabilities {
    pub feature_extraction: bool,
    pub feature_matching: bool,
    pub sfm: bool,
    pub dense_reconstruction: bool,
    pub meshing: bool,
    pub texturing: bool,
    pub georeferencing: bool,
    pub dem: bool,
    pub orthomosaic: bool,
    /// Cannot run without a GPU. Checked at plan time — a GPU stage that fails at run time
    /// fails after the two hours of SfM that precede it.
    pub requires_gpu: bool,
    pub supports_cuda: bool,
}

/// Progress a long-running engine reports back while it works.
///
/// Engines report a fraction where they can (PDAL knows its own point count) and a stage
/// where they cannot (COLMAP's mapper prints stages, not percentages). Reporting "stage 3
/// of 5" rather than fabricating a percentage is the honest mapping, and `indicatif`
/// renders it as a spinner with a label.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(tag = "kind", rename_all = "snake_case")]
pub enum Progress {
    /// A fraction in `0.0..=1.0`, where the engine knows its denominator.
    Fraction { completed: f64 },
    /// A named stage with no denominator. `index`/`total` are 1-based; `total == 0` means
    /// the engine does not know.
    Stage { name: String, index: usize, total: usize },
    /// A free-form line for the log. Not progress: the caller decides whether to show it.
    Message { text: String },
}

impl Progress {
    /// The fraction to render, where one is known. `None` means indeterminate, which
    /// `indicatif` draws as a spinner rather than a bar that lies about its position.
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

/// Something went wrong inside an engine adapter.
///
/// One type for the whole trait surface on purpose. A caller that has to handle sixteen
/// error enums cannot usefully match on any of them; it can only log and give up, which is
/// what it does anyway. The variants carry which engine and which stage, which is what
/// `nadir report` needs.
#[derive(Debug, thiserror::Error)]
pub enum EngineError {
    /// The engine's binary is not on PATH. Split out because it is an environment problem
    /// with a different remedy than a failure inside the engine, and because `nadir engines`
    /// can preflight every adapter for this one case.
    #[error("{engine} is not installed or not on PATH")]
    NotInstalled { engine: &'static str },
    /// The engine ran and exited non-zero.
    #[error("{engine} failed in stage {stage}: {message}")]
    Failed {
        engine: &'static str,
        stage: &'static str,
        message: String,
    },
    /// The engine's output could not be parsed. Common when an engine's output format
    /// changes between versions — which is why the engine version is part of the artifact
    /// hash and not only a log line.
    #[error("could not parse {engine} output in stage {stage}: {message}")]
    Parse {
        engine: &'static str,
        stage: &'static str,
        message: String,
    },
    /// The engine exceeded its resource budget and was stopped.
    #[error("{engine} exceeded its {resource} budget in stage {stage}")]
    BudgetExceeded {
        engine: &'static str,
        stage: &'static str,
        resource: &'static str,
    },
}

/// A handle an engine receives to reach the workspace.
///
/// Cancellation goes through the context rather than through killing the process directly,
/// so an adapter can flush its own state and exit with a resumable workspace — which is
/// what makes `nadir resume` possible at all.
#[derive(Debug, Clone)]
pub struct Context {
    /// Workspace root: where artifacts, caches and adapter scratch space live.
    pub workspace: PathBuf,
    /// Per-run scratch. Adapters write here; it is safe to delete between runs.
    pub scratch: PathBuf,
}

impl Context {
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

/// The outcome every trait returns: artifacts produced, and how they were made.
#[derive(Debug, Clone)]
pub struct Produced {
    /// The artifacts this call produced, in the order the trait documents.
    pub artifacts: Vec<Artifact>,
    /// Anything worth recording for `nadir explain`.
    pub provenance: Provenance,
}

impl Produced {
    /// A result with one artifact and no provenance filled in yet — what an adapter
    /// returns before it has read the engine's version.
    pub fn one(artifact: Artifact) -> Self {
        Self {
            artifacts: vec![artifact],
            provenance: Provenance::default(),
        }
    }
}

/// What an adapter records about how it ran an engine.
#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize, Deserialize)]
pub struct Provenance {
    /// The engine binary and version, as reported. Part of the artifact hash.
    pub engine_version: String,
    /// The exact argv, so a failure is reproducible by hand.
    pub command: Vec<String>,
    pub duration_ms: u64,
    /// Peak resident memory, where the platform reports it. `None` on Linux without
    /// `wait4`, and that is preferable to reporting a number nobody measured.
    pub peak_rss_bytes: Option<u64>,
}

// ── Domain 1: Dataset (1 trait) ──────────────────────────────────────────────

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct IngestConfig {
    /// Image extensions to accept.
    pub extensions: Vec<String>,
    /// Require GPS in EXIF. Off by default: a flight without RTK still processes, it just
    /// georeferences from the SfM GPS solution instead — worse, and a warning rather than
    /// a refusal.
    pub require_gps: bool,
    /// Reject an image whose EXIF cannot be read. Off: one corrupt file among 1,284 is a
    /// warning and a dropped image, not a failed run.
    pub strict_exif: bool,
}

/// What `nadir inspect` reports.
#[derive(Debug, Clone, Default, PartialEq, Serialize, Deserialize)]
pub struct DatasetReport {
    pub images: usize,
    pub cameras: Vec<String>,
    pub images_with_gps: usize,
    pub has_rtk: bool,
    /// Ground sample distance in metres per pixel, estimated from flight geometry.
    pub gsd_metres: Option<f64>,
    pub issues: Vec<String>,
}

#[async_trait]
pub trait ImageProvider: Engine {
    /// Ingest a directory of images into a validated dataset.
    async fn ingest(
        &self,
        path: &Path,
        cfg: &IngestConfig,
        ctx: &Context,
        progress: &dyn Fn(Progress),
    ) -> Result<Produced, EngineError>;

    /// Report a dataset's geometry without producing anything.
    async fn inspect(&self, path: &Path) -> Result<DatasetReport, EngineError>;
}

// ── Domain 2: Reconstruction (6 traits) ──────────────────────────────────────

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct FeatureConfig {
    pub max_features: usize,
    /// Detection threshold. Lower finds more features and more noise; this is the knob
    /// `nadir process --quality` turns.
    pub detection_threshold: f64,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MatchingConfig {
    pub max_distance: f64,
    /// Run a second guided-matching pass. Expensive, materially better.
    pub guided_matching: bool,
    pub num_threads: usize,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SfMConfig {
    pub min_num_inliers: usize,
    pub init_image_reg_pairs: usize,
    pub ba_refine_focal_length: bool,
    /// Triangulation threshold as a multiple of the image's pixel scale.
    pub triangulation_threshold: f64,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DenseConfig {
    pub num_views: usize,
    pub depth_min: f64,
    pub depth_max: f64,
    pub consistency_threshold: f64,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MeshConfig {
    pub depth: u8,
    pub scale: f64,
    pub linear_fit: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TextureConfig {
    pub resolution: u32,
    /// Global seam-leveling before blending. Visible in the output at high overlap.
    pub global_seam_leveling: bool,
    pub num_threads: usize,
}

#[async_trait]
pub trait FeatureExtractor: Engine {
    async fn extract(
        &self,
        images: &Artifact,
        cfg: &FeatureConfig,
        ctx: &Context,
        progress: &dyn Fn(Progress),
    ) -> Result<Produced, EngineError>;
}

#[async_trait]
pub trait FeatureMatcher: Engine {
    async fn match_features(
        &self,
        features: &Artifact,
        cfg: &MatchingConfig,
        ctx: &Context,
        progress: &dyn Fn(Progress),
    ) -> Result<Produced, EngineError>;
}

#[async_trait]
pub trait SparseReconstructor: Engine {
    async fn reconstruct(
        &self,
        matches: &Artifact,
        cfg: &SfMConfig,
        ctx: &Context,
        progress: &dyn Fn(Progress),
    ) -> Result<Produced, EngineError>;
}

#[async_trait]
pub trait DenseReconstructor: Engine {
    async fn densify(
        &self,
        scene: &Artifact,
        cfg: &DenseConfig,
        ctx: &Context,
        progress: &dyn Fn(Progress),
    ) -> Result<Produced, EngineError>;
}

#[async_trait]
pub trait MeshGenerator: Engine {
    async fn mesh(
        &self,
        cloud: &Artifact,
        cfg: &MeshConfig,
        ctx: &Context,
        progress: &dyn Fn(Progress),
    ) -> Result<Produced, EngineError>;
}

#[async_trait]
pub trait TextureGenerator: Engine {
    async fn texture(
        &self,
        mesh: &Artifact,
        images: &Artifact,
        cfg: &TextureConfig,
        ctx: &Context,
        progress: &dyn Fn(Progress),
    ) -> Result<Produced, EngineError>;
}

// ── Domain 3: Geometry (2 traits) ────────────────────────────────────────────

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct GeorefConfig {
    /// Ground control points. Empty means "derive from the SfM GPS solution", which is
    /// worse, and `nadir report` says which of the two produced the georeference.
    pub ground_control_points: Vec<GroundControlPoint>,
    /// Target CRS as an EPSG code or a PROJ string.
    pub target_crs: String,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct GroundControlPoint {
    pub image: PathBuf,
    pub x_pixel: f64,
    pub y_pixel: f64,
    pub easting: f64,
    pub northing: f64,
    pub elevation: Option<f64>,
}

#[async_trait]
pub trait Georeferencer: Engine {
    async fn georeference(
        &self,
        scene: &Artifact,
        gps: &Artifact,
        cfg: &GeorefConfig,
        ctx: &Context,
    ) -> Result<Produced, EngineError>;
}

/// A WGS84-ish geodetic point. Kept distinct from `geo-types`' `Point<f64>` so a trait
/// signature says which CRS it is in; a `Point<f64>` at a CRS boundary is the bug this
/// naming prevents.
#[derive(Debug, Clone, Copy, PartialEq, Serialize, Deserialize)]
pub struct GeoPoint {
    pub longitude: f64,
    pub latitude: f64,
    pub elevation: f64,
}

#[async_trait]
pub trait CoordinateTransformer: Engine {
    /// Transform one point.
    async fn transform(&self, point: GeoPoint) -> Result<GeoPoint, EngineError>;

    /// Transform many points in place.
    ///
    /// A separate method rather than a convenience default over `transform`: PROJ can
    /// transform a batch on the worker pool, and the default `transform` one at a time
    /// turns a 4-million-point reprojection into four million sequential calls.
    async fn transform_many(&self, points: &mut [GeoPoint]) -> Result<(), EngineError>;
}

// ── Domain 4: Surface (3 traits) ─────────────────────────────────────────────

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct FilterConfig {
    /// Remove statistical outliers beyond this many standard deviations.
    pub outlier_k: u32,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ClassifyConfig {
    /// Ground classification needed for a DTM, and unnecessary work for a DSM.
    pub classify_ground: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DemConfig {
    pub resolution_metres: f64,
    /// Fill holes by interpolation. Off for a DSM — a hole is missing data — and on for a
    /// DTM, where a hole under vegetation is an estimate and the estimate is the product.
    pub fill_holes: bool,
    pub max_hole_radius_metres: f64,
}

#[async_trait]
pub trait PointCloudFilter: Engine {
    async fn filter(
        &self,
        cloud: &Artifact,
        cfg: &FilterConfig,
        ctx: &Context,
        progress: &dyn Fn(Progress),
    ) -> Result<Produced, EngineError>;
}

#[async_trait]
pub trait PointCloudClassifier: Engine {
    async fn classify(
        &self,
        cloud: &Artifact,
        cfg: &ClassifyConfig,
        ctx: &Context,
        progress: &dyn Fn(Progress),
    ) -> Result<Produced, EngineError>;
}

#[async_trait]
pub trait SurfaceGenerator: Engine {
    async fn generate_dsm(
        &self,
        cloud: &Artifact,
        cfg: &DemConfig,
        ctx: &Context,
        progress: &dyn Fn(Progress),
    ) -> Result<Produced, EngineError>;

    async fn generate_dtm(
        &self,
        cloud: &Artifact,
        cfg: &DemConfig,
        ctx: &Context,
        progress: &dyn Fn(Progress),
    ) -> Result<Produced, EngineError>;
}

// ── Domain 5: Cartography (2 traits) ─────────────────────────────────────────

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct OrthoConfig {
    pub resolution_metres: f64,
    /// Cloud-Optimized GeoTIFF rather than plain GeoTIFF: the product is streamed to a
    /// browser, and a tiled COG is what makes that possible without rewriting the file.
    pub cloud_optimized: bool,
    pub num_threads: usize,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MosaicConfig {
    pub tile_size_pixels: usize,
    /// XYZ pyramid levels for the web map.
    pub pyramid_levels: Vec<usize>,
}

#[async_trait]
pub trait Orthorectifier: Engine {
    async fn orthorectify(
        &self,
        images: &Artifact,
        scene: &Artifact,
        surface: &Artifact,
        cfg: &OrthoConfig,
        ctx: &Context,
        progress: &dyn Fn(Progress),
    ) -> Result<Produced, EngineError>;
}

#[async_trait]
pub trait Mosaicker: Engine {
    async fn mosaic(
        &self,
        orthos: &Artifact,
        cfg: &MosaicConfig,
        ctx: &Context,
        progress: &dyn Fn(Progress),
    ) -> Result<Produced, EngineError>;
}

// ── I/O (2 traits) ───────────────────────────────────────────────────────────

#[async_trait]
pub trait RasterStore: Engine {
    async fn read(&self, path: &Path) -> Result<Produced, EngineError>;
    async fn write(&self, raster: &Artifact, path: &Path) -> Result<(), EngineError>;
}

#[async_trait]
pub trait PointCloudStore: Engine {
    async fn read(&self, path: &Path) -> Result<Produced, EngineError>;
    async fn write(&self, cloud: &Artifact, path: &Path) -> Result<(), EngineError>;
}

/// The sixteen trait names, in the KB's order. ADR-007 says sixteen; this array is the
/// check that keeps the sentence true, because `nadir explain --traits` prints it and the
/// count is part of the architecture's public description.
pub const TRAIT_NAMES: [&str; 16] = [
    // Domain 1: Dataset
    "ImageProvider",
    // Domain 2: Reconstruction
    "FeatureExtractor",
    "FeatureMatcher",
    "SparseReconstructor",
    "DenseReconstructor",
    "MeshGenerator",
    "TextureGenerator",
    // Domain 3: Geometry
    "Georeferencer",
    "CoordinateTransformer",
    // Domain 4: Surface
    "PointCloudFilter",
    "PointCloudClassifier",
    "SurfaceGenerator",
    // Domain 5: Cartography
    "Orthorectifier",
    "Mosaicker",
    // I/O
    "RasterStore",
    "PointCloudStore",
];

/// How many of the sixteen a capability set covers. Used by `nadir plan` to report what a
/// pipeline cannot run with the engines actually installed.
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
        // No fraction, so indicatif draws a spinner rather than a bar that invents a
        // position. That distinction is the reason this is an enum.
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
        assert_eq!(
            progress.fraction(),
            None,
            "an unknown total must be indeterminate, not a division by zero"
        );
    }

    #[test]
    fn a_missing_engine_is_distinguishable_from_a_failure() {
        // The distinction the executor branches on: `NotInstalled` is a preflight finding,
        // `Failed` is a run that started.
        let error = EngineError::NotInstalled { engine: "colmap" };
        assert!(error.to_string().contains("not installed"));
        assert!(!matches!(error, EngineError::Failed { .. }));
    }

    #[test]
    fn a_parse_failure_names_the_engine_and_stage() {
        // `nadir report` reads these two fields to say which adapter needs attention, so
        // they must survive formatting.
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
        assert_eq!(
            TRAIT_NAMES.len(),
            16,
            "ADR-007 fixes this number; a seventeenth trait must edit this array on purpose"
        );
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
        // `requires_gpu`/`supports_cuda` describe the hardware, not a stage, so they must
        // not inflate the count or `nadir plan` would claim coverage that is not there.
        let caps = EngineCapabilities {
            sfm: true,
            requires_gpu: true,
            supports_cuda: true,
            ..EngineCapabilities::default()
        };
        assert_eq!(covered_capabilities(&caps), 1);
    }
}