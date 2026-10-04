//! Public top-level re-export smoke tests for nadir-core.

use std::collections::BTreeMap;

use nadir_core::{
    ArtifactHash, EngineCapabilities, Pipeline, PipelineDecl, Progress, TRAIT_NAMES, TaskDecl,
    TaskSpec, covered_capabilities,
};

#[test]
fn stable_core_vocabulary_is_available_from_the_crate_root() {
    let input = ArtifactHash::of_bytes(b"images");
    let spec = TaskSpec::new(
        "extract_features",
        [input],
        [("detector", "sift")],
        "colmap 3.13.0",
    );
    assert_eq!(ArtifactHash::of_task(&spec).to_string().len(), 64);

    let caps = EngineCapabilities {
        feature_extraction: true,
        ..EngineCapabilities::default()
    };
    assert_eq!(covered_capabilities(&caps), 1);
    assert_eq!(TRAIT_NAMES.len(), 16);

    let pipeline = Pipeline::build(PipelineDecl {
        name: "standard".to_owned(),
        description: None,
        tasks: vec![TaskDecl {
            name: "ingest".to_owned(),
            kind: "ingest_images".to_owned(),
            engine: "builtin".to_owned(),
            needs: vec![],
            params: BTreeMap::new(),
        }],
    })
    .expect("pipeline builds through crate root re-exports");
    assert_eq!(pipeline.task_names(), ["ingest"]);

    assert_eq!(Progress::Fraction { completed: 0.5 }.fraction(), Some(0.5));
}
