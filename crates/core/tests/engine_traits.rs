//! Integration tests for the public engine trait vocabulary.

use std::future::Future;
use std::path::PathBuf;
use std::pin::Pin;
use std::sync::Mutex;
use std::sync::atomic::{AtomicUsize, Ordering};
use std::task::{Context as TaskContext, Poll, Waker};

use nadir_core::artifact::{Artifact, ArtifactHash, ArtifactKind};
use nadir_core::engine::{
    Context, Engine, EngineCapabilities, EngineError, EngineFuture, FeatureConfig,
    FeatureExtractor, Produced, Progress, ProgressSink, TRAIT_NAMES, covered_capabilities,
};
use serde_json::json;

#[derive(Default)]
struct MockFeatureEngine {
    runs: AtomicUsize,
}

impl Engine for MockFeatureEngine {
    fn name(&self) -> &str {
        "mock-features"
    }

    fn version(&self) -> &str {
        "mock-features 1.0.0"
    }

    fn capabilities(&self) -> EngineCapabilities {
        EngineCapabilities {
            feature_extraction: true,
            ..EngineCapabilities::default()
        }
    }
}

impl FeatureExtractor for MockFeatureEngine {
    fn extract<'a>(
        &'a self,
        images: &'a Artifact,
        cfg: &'a FeatureConfig,
        ctx: &'a Context,
        progress: ProgressSink<'a>,
    ) -> EngineFuture<'a, Produced> {
        Box::pin(async move {
            self.runs.fetch_add(1, Ordering::SeqCst);
            progress(Progress::Stage {
                name: format!("extract {} features", cfg.max_features),
                index: 1,
                total: 1,
            });
            Ok(Produced::one(Artifact {
                hash: ArtifactHash::of_bytes(
                    format!("{}:{}", images.hash, ctx.scratch.display()).as_bytes(),
                ),
                kind: ArtifactKind::Features,
                path: ctx.scratch.join("features.json"),
                size_bytes: cfg.max_features as u64,
                produced_by: None,
            }))
        })
    }
}

#[test]
fn feature_extractor_is_object_safe_and_executable_through_public_surface() {
    let mock = MockFeatureEngine::default();
    let extractor: &dyn FeatureExtractor = &mock;
    let images = input_artifact();
    let cfg = FeatureConfig {
        max_features: 8192,
        detection_threshold: 0.01,
    };
    let ctx = Context::new("workspace/run-001");
    let events = Mutex::new(Vec::new());
    let progress = |event| events.lock().expect("progress mutex").push(event);

    assert_eq!(extractor.name(), "mock-features");
    assert_eq!(extractor.version(), "mock-features 1.0.0");
    assert_eq!(covered_capabilities(&extractor.capabilities()), 1);
    assert_eq!(
        mock.runs.load(Ordering::SeqCst),
        0,
        "preflight must not run a stage"
    );

    let produced =
        poll_ready(extractor.extract(&images, &cfg, &ctx, &progress)).expect("mock extraction");

    assert_eq!(mock.runs.load(Ordering::SeqCst), 1);
    assert_eq!(produced.artifacts.len(), 1);
    assert_eq!(produced.artifacts[0].kind, ArtifactKind::Features);
    assert_eq!(
        produced.artifacts[0].path,
        PathBuf::from("workspace/run-001/scratch/features.json")
    );
    assert_eq!(
        *events.lock().expect("progress mutex"),
        vec![Progress::Stage {
            name: "extract 8192 features".to_owned(),
            index: 1,
            total: 1,
        }]
    );
}

#[test]
fn engine_capabilities_and_progress_are_stable_public_json() {
    assert_eq!(TRAIT_NAMES.len(), 16);

    let caps = EngineCapabilities {
        feature_extraction: true,
        requires_gpu: true,
        supports_cuda: true,
        ..EngineCapabilities::default()
    };
    assert_eq!(covered_capabilities(&caps), 1);
    assert_eq!(
        serde_json::to_value(caps).expect("capabilities serialize"),
        json!({
            "feature_extraction": true,
            "feature_matching": false,
            "sfm": false,
            "dense_reconstruction": false,
            "meshing": false,
            "texturing": false,
            "georeferencing": false,
            "dem": false,
            "orthomosaic": false,
            "requires_gpu": true,
            "supports_cuda": true
        })
    );

    assert_eq!(
        serde_json::to_value(Progress::Fraction { completed: 0.25 }).expect("progress serializes"),
        json!({ "kind": "fraction", "completed": 0.25 })
    );
    assert_eq!(
        EngineError::NotInstalled { engine: "colmap" }.to_string(),
        "colmap is not installed or not on PATH"
    );
}

fn input_artifact() -> Artifact {
    Artifact {
        hash: ArtifactHash::of_bytes(b"images"),
        kind: ArtifactKind::Dataset,
        path: PathBuf::from("images"),
        size_bytes: 3,
        produced_by: None,
    }
}

fn poll_ready<T>(mut future: EngineFuture<'_, T>) -> Result<T, EngineError> {
    let waker = Waker::noop();
    let mut context = TaskContext::from_waker(waker);
    match Pin::new(&mut future).poll(&mut context) {
        Poll::Ready(result) => result,
        Poll::Pending => panic!("mock future should be ready immediately"),
    }
}
