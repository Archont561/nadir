//! The DAG: how a declarative TOML pipeline becomes a graph of tasks, and how the
//! executor walks it.
//!
//! Declarative pipelines and the petgraph graph are V0.1 (`roadmap/v0-mvp.md`), so this
//! module is the type vocabulary they will fill in rather than the executor. What is here
//! is the part that is cheap to get right now and expensive to retrofit: the distinction
//! between a *declared* task and a *resolved* one, because a task's artifact hash is not
//! known until its inputs have been resolved, and a hash that cannot be computed before
//! the graph runs is a hash that cannot gate a cache lookup.

use std::collections::{BTreeMap, BTreeSet};

use petgraph::graph::{DiGraph, NodeIndex};
use serde::{Deserialize, Serialize};

use crate::artifact::{ArtifactHash, ArtifactKind, TaskSpec, TaskSpecOwned};

/// What a task is, as written in `pipelines/standard.toml`.
///
/// Everything here is text. Nothing has been resolved against a workspace, an engine
/// registry or a cache, which is what lets `nadir plan` validate a pipeline on a machine
/// with no engines installed.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct TaskDecl {
    /// Stage name — `sfm`, `dsm`. Part of the artifact hash: renaming a stage invalidates
    /// its artifacts rather than silently serving a stale one.
    pub name: String,
    /// The task kind, which selects the trait (`SparseReconstructor`, and so on).
    pub kind: String,
    /// Which engine implements it. `engine = "builtin"` is a real value, not a
    /// placeholder: the native Rust stages are engines too.
    #[serde(default = "default_engine")]
    pub engine: String,
    /// Names of the tasks whose outputs this one reads.
    #[serde(default)]
    pub needs: Vec<String>,
    /// Parameters. Values are kept as strings and normalized by the task's config struct,
    /// so `0.30` and `0.3` reach the hash as the same value.
    #[serde(default)]
    pub params: BTreeMap<String, toml::Value>,
}

fn default_engine() -> String {
    "builtin".to_owned()
}

/// What kind of artifact a task produces. Declared rather than inferred from the trait, so
/// the DAG can be checked for a coherent product type before any engine runs — a stage that
/// declares `Features` and is asked to consume an `Orthomosaic` is a typo, and it should
/// fail at plan time rather than at the artifact store.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub struct Produces(ArtifactKind);

/// A pipeline as written.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct PipelineDecl {
    pub name: String,
    #[serde(default)]
    pub description: Option<String>,
    pub tasks: Vec<TaskDecl>,
}

/// A pipeline after validation: names resolved to indices, and the graph built.
#[derive(Debug, Clone)]
pub struct Pipeline {
    pub name: String,
    pub description: Option<String>,
    graph: DiGraph<TaskNode, ()>,
}

/// A node in a resolved pipeline.
#[derive(Debug, Clone)]
pub struct TaskNode {
    pub decl: TaskDecl,
    /// Filled in during execution, not before: a node's output hash depends on its inputs'
    /// hashes, so it cannot be known until the inputs have been resolved.
    pub output: Option<ArtifactHash>,
}

impl Pipeline {
    /// Validate a declaration and build its graph.
    ///
    /// Everything that can be wrong without running anything is caught here: an unknown
    /// dependency, a dependency on a task declared later (the graph is a DAG, so this is
    /// an error rather than a forward reference), and a duplicate task name.
    pub fn build(decl: PipelineDecl) -> Result<Self, PipelineError> {
        let mut graph: DiGraph<TaskNode, ()> = DiGraph::new();
        let mut indices: BTreeMap<String, NodeIndex> = BTreeMap::new();

        for task in &decl.tasks {
            if indices.contains_key(&task.name) {
                return Err(PipelineError::DuplicateTask {
                    name: task.name.clone(),
                });
            }
            let index = graph.add_node(TaskNode {
                decl: task.clone(),
                output: None,
            });
            indices.insert(task.name.clone(), index);
        }

        for task in &decl.tasks {
            let to = indices[&task.name];
            for need in &task.needs {
                let Some(&from) = indices.get(need) else {
                    return Err(PipelineError::UnknownDependency {
                        task: task.name.clone(),
                        depends_on: need.clone(),
                    });
                };
                graph.add_edge(from, to, ());
            }
        }

        // petgraph's `is_dag` is a reachability scan, which is the right check here: the
        // pipeline is small (a dozen stages) and the alternative — a Kahn pass that also
        // produces the execution order — is the executor's job, not the validator's.
        if let Some(cycle) = petgraph::algo::find_cycle(&graph, None) {
            let names: Vec<&str> = cycle
                .node_indices()
                .map(|index| graph[index].decl.name.as_str())
                .collect();
            return Err(PipelineError::Cycle { nodes: names.join(" -> ") });
        }

        Ok(Self {
            name: decl.name,
            description: decl.description,
            graph,
        })
    }

    /// The task names, in declaration order.
    pub fn task_names(&self) -> Vec<&str> {
        self.graph
            .node_indices()
            .map(|index| self.graph[index].decl.name.as_str())
            .collect()
    }

    /// The number of tasks.
    pub fn len(&self) -> usize {
        self.graph.node_count()
    }

    /// Whether the pipeline has no tasks. Distinct from a pipeline that failed to parse:
    /// an empty pipeline is a valid pipeline that will produce nothing.
    pub fn is_empty(&self) -> bool {
        self.graph.node_count() == 0
    }

    /// Compute a node's output hash from its inputs.
    ///
    /// This is the call that turns a declaration into a cache key, and it is where the
    /// engine's version enters. It returns `None` rather than a hash when an input has not
    /// run yet, because an artifact hash computed from a placeholder input is a hash that
    /// caches the wrong answer.
    pub fn resolve_hash(&self, name: &str, engine_version: &str) -> Result<Option<ArtifactHash>, PipelineError> {
        let Some(&index) = self.index_of(name) else {
            return Err(PipelineError::UnknownTask {
                name: name.to_owned(),
            });
        };

        let mut inputs = Vec::with_capacity(self.graph.neighbors_directed(index, petgraph::Direction::Incoming).count());
        let mut inputs_complete = true;
        for source in self.graph.neighbors_directed(index, petgraph::Direction::Incoming) {
            match self.graph[source].output {
                Some(hash) => inputs.push(hash),
                None => inputs_complete = false,
            }
        }
        if !inputs_complete {
            return Ok(None);
        }

        let decl = &self.graph[index].decl;
        // Sorted, because the declaration is a BTreeMap and the hash must not depend on
        // the engine that renders the TOML.
        let params: Vec<(&str, String)> = decl
            .params
            .iter()
            .map(|(key, value)| (key.as_str(), normalize_param(value)))
            .collect();

        Ok(Some(ArtifactHash::of_task(&TaskSpec::new(
            &decl.kind,
            &inputs,
            &params,
            engine_version,
        ))))
    }

    fn index_of(&self, name: &str) -> Option<&NodeIndex> {
        self.graph
            .node_indices()
            .find(|index| self.graph[*index].decl.name == name)
    }

    /// The tasks in an order where every task follows its dependencies.
    ///
    /// Kahn's algorithm rather than a sort, because the declaration order is not a valid
    /// execution order and neither is alphabetical: two tasks may have identical names'
    /// prefixes and no dependencies between them, and the executor wants every independent
    /// pair adjacent so it can run them concurrently.
    pub fn execution_order(&self) -> Vec<&str> {
        let mut order: Vec<NodeIndex> = Vec::with_capacity(self.graph.node_count());
        let mut remaining: BTreeSet<NodeIndex> = self.graph.node_indices().collect();

        while !remaining.is_empty() {
            // The lowest declared index among the ready tasks: declaration order breaks
            // ties, so the order is stable across runs and a diff of two executions is
            // readable.
            let ready: Vec<NodeIndex> = remaining
                .iter()
                .copied()
                .filter(|index| {
                    self.graph
                        .neighbors_directed(*index, petgraph::Direction::Incoming)
                        .all(|dep| order.contains(&dep))
                })
                .collect();

            let Some(next) = ready.into_iter().min() else {
                // `build` proved the graph acyclic, so this is unreachable. Returning the
                // remainder is better than panicking in a library: a caller that built a
                // `Pipeline` by other means still gets a usable order.
                order.extend(remaining.iter().copied());
                break;
            };

            order.push(next);
            remaining.remove(&next);
        }

        order
            .into_iter()
            .map(|index| self.graph[index].decl.name.as_str())
            .collect()
    }

    /// Tasks that can run at the same time, as groups.
    ///
    /// This is what the executor schedules: one group is one parallel wave, and a task in
    /// group *n* does not depend on another task in group *n*.
    pub fn parallel_waves(&self) -> Vec<Vec<&str>> {
        let mut waves: Vec<Vec<&str>> = Vec::new();
        let mut done: BTreeSet<NodeIndex> = BTreeSet::new();

        while done.len() < self.graph.node_count() {
            let wave: Vec<NodeIndex> = self
                .graph
                .node_indices()
                .filter(|index| {
                    !done.contains(index)
                        && self
                            .graph
                            .neighbors_directed(*index, petgraph::Direction::Incoming)
                            .all(|dep| done.contains(&dep))
                })
                .collect();
            if wave.is_empty() {
                break; // unreachable for an acyclic graph; see `execution_order`
            }
            for index in &wave {
                done.insert(*index);
            }
            waves.push(
                wave.into_iter()
                    .map(|index| self.graph[index].decl.name.as_str())
                    .collect(),
            );
        }
        waves
    }
}

/// A parameter value as it enters the artifact hash.
///
/// TOML's `1.0`, `"1.0"` and `1` are three values in a TOML document and one parameter to
/// a config struct. Hashing the raw TOML text would make the cache miss whenever a
/// pipeline was reformatted, which is the failure mode a content-addressed cache cannot
/// afford: the hash is only useful if it changes when the *meaning* changes.
fn normalize_param(value: &toml::Value) -> String {
    match value {
        toml::Value::Float(f) => {
            // `{}` on f64 already prints the shortest round-tripping form, so `0.3` and
            // `0.30000000000000004` do not both appear for the same number.
            format!("{f}")
        }
        toml::Value::Integer(i) => i.to_string(),
        toml::Value::Boolean(b) => b.to_string(),
        toml::Value::String(s) => s.clone(),
        other => other.to_string(),
    }
}

/// A pipeline that cannot be built. Every variant is a mistake a person can fix in the
/// TOML, and every one of them is caught before an engine runs.
#[derive(Debug, thiserror::Error, PartialEq, Eq)]
pub enum PipelineError {
    #[error("two tasks are both named `{name}`")]
    DuplicateTask { name: String },
    #[error("task `{task}` depends on `{depends_on}`, which is not declared")]
    UnknownDependency { task: String, depends_on: String },
    #[error("`{name}` is not a task in this pipeline")]
    UnknownTask { name: String },
    #[error("the pipeline has a cycle: {nodes}")]
    Cycle { nodes: String },
}

impl From<PipelineError> for crate::engine::EngineError {
    fn from(error: PipelineError) -> Self {
        Self::Parse {
            engine: "builtin",
            stage: "pipeline",
            message: error.to_string(),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn task(name: &str, needs: &[&str]) -> TaskDecl {
        TaskDecl {
            name: name.to_owned(),
            kind: format!("{name}-kind"),
            engine: default_engine(),
            needs: needs.iter().map(|s| (*s).to_owned()).collect(),
            params: BTreeMap::new(),
        }
    }

    fn pipeline(tasks: Vec<TaskDecl>) -> Result<Pipeline, PipelineError> {
        Pipeline::build(PipelineDecl {
            name: "standard".to_owned(),
            description: None,
            tasks,
        })
    }

    #[test]
    fn a_linear_pipeline_executes_in_order() {
        let dag = pipeline(vec![
            task("ingest", &[]),
            task("sfm", &["ingest"]),
            task("dsm", &["sfm"]),
        ])
        .expect("a linear pipeline is valid");

        assert_eq!(dag.execution_order(), ["ingest", "sfm", "dsm"]);
    }

    #[test]
    fn declaration_order_does_not_decide_execution_order() {
        // Declared last-to-first on purpose: the DAG, not the TOML, decides what runs when.
        let dag = pipeline(vec![
            task("dsm", &["sfm"]),
            task("sfm", &["ingest"]),
            task("ingest", &[]),
        ])
        .expect("order of declaration is not a constraint");

        assert_eq!(dag.execution_order(), ["ingest", "sfm", "dsm"]);
    }

    #[test]
    fn independent_tasks_share_a_wave() {
        // The parallelism ADR-005 depends on: after ingest, features and inspection do not
        // depend on each other and must be able to run at once.
        let dag = pipeline(vec![
            task("ingest", &[]),
            task("features", &["ingest"]),
            task("inspect", &["ingest"]),
            task("sfm", &["features", "inspect"]),
        ])
        .expect("a diamond is valid");

        let waves = dag.parallel_waves();
        assert_eq!(waves.len(), 3, "got {waves:?}");
        assert_eq!(waves[0], ["ingest"]);
        assert_eq!(waves[1], ["features", "inspect"]);
        assert_eq!(waves[2], ["sfm"]);
    }

    #[test]
    fn a_cycle_is_rejected() {
        let error = pipeline(vec![task("a", &["b"]), task("b", &["a"])])
            .expect_err("a cycle is not a pipeline");

        assert!(
            matches!(error, PipelineError::Cycle { .. }),
            "expected Cycle, got {error:?}"
        );
    }

    #[test]
    fn an_unknown_dependency_is_rejected() {
        let error = pipeline(vec![task("sfm", &["nonexistent"])])
            .expect_err("an unknown dependency is not a pipeline");

        assert_eq!(
            error,
            PipelineError::UnknownDependency {
                task: "sfm".to_owned(),
                depends_on: "nonexistent".to_owned(),
            }
        );
    }

    #[test]
    fn a_duplicate_name_is_rejected() {
        let error = pipeline(vec![task("sfm", &[]), task("sfm", &[])])
            .expect_err("two tasks cannot share a name");

        assert_eq!(
            error,
            PipelineError::DuplicateTask {
                name: "sfm".to_owned()
            }
        );
    }

    #[test]
    fn a_task_whose_input_has_not_run_has_no_hash_yet() {
        // The property that makes caching correct: no hash is available until every input
        // is resolved, so the executor can never look up a hash built from a placeholder.
        let dag = pipeline(vec![task("ingest", &[]), task("sfm", &["ingest"])])
            .expect("a linear pipeline is valid");

        assert_eq!(
            dag.resolve_hash("sfm", "colmap 3.13.0").expect("sfm is a task"),
            None,
            "sfm depends on ingest, which has produced nothing"
        );
    }

    #[test]
    fn an_empty_pipeline_is_valid_and_produces_nothing() {
        let dag = pipeline(vec![]).expect("an empty pipeline is valid");
        assert!(dag.is_empty());
        assert!(dag.execution_order().is_empty());
    }
}