//! Integration tests for the public pipeline declaration and DAG seam.

use std::collections::BTreeMap;

use nadir_core::pipeline::{Pipeline, PipelineDecl, PipelineError, TaskDecl};
use serde_json::Value;

fn task(name: &str, kind: &str, needs: &[&str]) -> TaskDecl {
    TaskDecl {
        name: name.to_owned(),
        kind: kind.to_owned(),
        engine: "builtin".to_owned(),
        needs: needs.iter().map(|need| (*need).to_owned()).collect(),
        params: BTreeMap::<String, Value>::new(),
    }
}

fn build(tasks: Vec<TaskDecl>) -> Result<Pipeline, PipelineError> {
    Pipeline::build(PipelineDecl {
        name: "standard".to_owned(),
        description: Some("test pipeline".to_owned()),
        tasks,
    })
}

#[test]
fn declaration_resolves_to_dag_without_losing_declaration_order() {
    let pipeline = build(vec![
        task("report", "generate_report", &[]),
        task("ingest", "ingest_images", &[]),
        task("features", "extract_features", &["ingest"]),
        task("metadata", "inspect_dataset", &["ingest"]),
        task("ortho", "orthorectify", &["features", "metadata"]),
    ])
    .expect("valid branching declaration builds");

    assert_eq!(pipeline.len(), 5);
    assert_eq!(
        pipeline.task_names(),
        ["report", "ingest", "features", "metadata", "ortho"]
    );
    assert_eq!(
        pipeline.execution_order(),
        ["report", "ingest", "features", "metadata", "ortho"]
    );
    assert_eq!(
        pipeline.parallel_waves(),
        [
            vec!["report", "ingest"],
            vec!["features", "metadata"],
            vec!["ortho"],
        ]
    );
}

#[test]
fn duplicate_tasks_return_typed_diagnostic_naming_the_task() {
    let error = build(vec![
        task("ingest", "ingest_images", &[]),
        task("ingest", "inspect_dataset", &[]),
    ])
    .expect_err("duplicate task names are invalid");

    assert_eq!(
        error,
        PipelineError::DuplicateTask {
            name: "ingest".to_owned(),
        }
    );
    assert!(error.to_string().contains("ingest"));
}

#[test]
fn missing_dependencies_return_typed_diagnostic_naming_both_tasks() {
    let error = build(vec![task("sfm", "sparse_reconstruction", &["matches"])])
        .expect_err("unknown dependencies are invalid");

    assert_eq!(
        error,
        PipelineError::UnknownDependency {
            task: "sfm".to_owned(),
            depends_on: "matches".to_owned(),
        }
    );
    let message = error.to_string();
    assert!(message.contains("sfm"), "got {message}");
    assert!(message.contains("matches"), "got {message}");
}

#[test]
fn cycles_return_typed_diagnostic_naming_involved_tasks() {
    let error = build(vec![
        task("a", "first", &["c"]),
        task("b", "second", &["a"]),
        task("c", "third", &["b"]),
    ])
    .expect_err("cycles are invalid");

    let PipelineError::Cycle { nodes } = error else {
        panic!("expected cycle error, got {error:?}");
    };
    assert!(nodes.contains("a"), "got {nodes}");
    assert!(nodes.contains("b"), "got {nodes}");
    assert!(nodes.contains("c"), "got {nodes}");
}
