//! Pipeline declaration and deterministic DAG validation.
//!
//! A pipeline declaration is text-shaped data: task names, task kinds, engine names,
//! dependency names and normalized parameters. Building a [`Pipeline`] resolves those names
//! into a typed DAG that can be inspected without exposing graph internals.

use std::collections::{BTreeMap, BTreeSet};

use serde::{Deserialize, Serialize};
use serde_json::Value;

use crate::artifact::{ArtifactHash, ArtifactKind, TaskSpec};

/// What a task is as written in a pipeline definition.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct TaskDecl {
    /// Unique task name within a pipeline.
    #[serde(alias = "id")]
    pub name: String,
    /// Domain operation implemented by an engine.
    #[serde(alias = "task")]
    pub kind: String,
    /// Engine implementation selected for this task.
    #[serde(default = "default_engine")]
    pub engine: String,
    /// Names of upstream tasks whose artifacts this task reads, in declared order.
    #[serde(default, alias = "depends_on")]
    pub needs: Vec<String>,
    /// Task parameters before conversion into a stage-specific config struct.
    #[serde(default)]
    pub params: BTreeMap<String, Value>,
}

fn default_engine() -> String {
    "builtin".to_owned()
}

/// What kind of artifact a task produces.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub struct Produces {
    /// Artifact kind produced by the task.
    pub kind: ArtifactKind,
}

/// A pipeline as declared by a caller or deserialized from a manifest.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct PipelineDecl {
    /// Pipeline name.
    pub name: String,
    /// Human-readable pipeline description.
    #[serde(default)]
    pub description: Option<String>,
    /// Declared tasks. `steps` is accepted as a manifest alias.
    #[serde(alias = "steps")]
    pub tasks: Vec<TaskDecl>,
}

/// A pipeline after validation: task names resolved to deterministic node indices.
#[derive(Debug, Clone)]
pub struct Pipeline {
    /// Pipeline name.
    pub name: String,
    /// Human-readable pipeline description.
    pub description: Option<String>,
    nodes: Vec<TaskNode>,
    indices: BTreeMap<String, usize>,
    dependencies: Vec<Vec<usize>>,
    dependents: Vec<Vec<usize>>,
}

/// A node in a resolved pipeline.
#[derive(Debug, Clone)]
pub struct TaskNode {
    /// Original task declaration.
    pub decl: TaskDecl,
    /// Artifact hash produced by the task after execution.
    pub output: Option<ArtifactHash>,
}

impl Pipeline {
    /// Validate a declaration and build its DAG.
    ///
    /// Duplicate task names, missing dependencies and cycles are rejected before any
    /// engine runs. Declaration order is preserved for public reporting and for
    /// deterministic tie-breaking between independent tasks.
    pub fn build(decl: PipelineDecl) -> Result<Self, PipelineError> {
        let mut nodes = Vec::with_capacity(decl.tasks.len());
        let mut indices = BTreeMap::new();

        for task in decl.tasks {
            if indices.contains_key(&task.name) {
                return Err(PipelineError::DuplicateTask { name: task.name });
            }
            let index = nodes.len();
            indices.insert(task.name.clone(), index);
            nodes.push(TaskNode {
                decl: task,
                output: None,
            });
        }

        let mut dependencies = vec![Vec::new(); nodes.len()];
        let mut dependents = vec![Vec::new(); nodes.len()];

        for (to, node) in nodes.iter().enumerate() {
            for need in &node.decl.needs {
                let Some(&from) = indices.get(need) else {
                    return Err(PipelineError::UnknownDependency {
                        task: node.decl.name.clone(),
                        depends_on: need.clone(),
                    });
                };
                dependencies[to].push(from);
                dependents[from].push(to);
            }
        }

        if let Some(cycle) = find_cycle(&nodes, &dependents) {
            return Err(PipelineError::Cycle {
                nodes: cycle.join(" -> "),
            });
        }

        Ok(Self {
            name: decl.name,
            description: decl.description,
            nodes,
            indices,
            dependencies,
            dependents,
        })
    }

    /// The task names in declaration order.
    pub fn task_names(&self) -> Vec<&str> {
        self.nodes
            .iter()
            .map(|node| node.decl.name.as_str())
            .collect()
    }

    /// Return a task declaration by name.
    pub fn task(&self, name: &str) -> Result<&TaskDecl, PipelineError> {
        let index = self.index_of(name)?;
        Ok(&self.nodes[index].decl)
    }

    /// The number of tasks in the pipeline.
    pub fn len(&self) -> usize {
        self.nodes.len()
    }

    /// Whether the pipeline has no tasks.
    pub fn is_empty(&self) -> bool {
        self.nodes.is_empty()
    }

    /// Record a task output hash after execution.
    pub fn record_output(&mut self, name: &str, hash: ArtifactHash) -> Result<(), PipelineError> {
        let index = self.index_of(name)?;
        self.nodes[index].output = Some(hash);
        Ok(())
    }

    /// Compute a task output hash from resolved input hashes and normalized parameters.
    ///
    /// Returns `Ok(None)` until every upstream dependency has a recorded output hash.
    pub fn resolve_hash(
        &self,
        name: &str,
        engine_version: &str,
    ) -> Result<Option<ArtifactHash>, PipelineError> {
        let index = self.index_of(name)?;
        let mut inputs = Vec::with_capacity(self.dependencies[index].len());
        for source in &self.dependencies[index] {
            let Some(hash) = self.nodes[*source].output else {
                return Ok(None);
            };
            inputs.push(hash);
        }

        let decl = &self.nodes[index].decl;
        let params = decl
            .params
            .iter()
            .map(|(key, value)| (key.as_str(), normalize_param(value)));

        Ok(Some(ArtifactHash::of_task(&TaskSpec::new(
            &decl.kind,
            inputs,
            params,
            engine_version,
        ))))
    }

    fn index_of(&self, name: &str) -> Result<usize, PipelineError> {
        self.indices
            .get(name)
            .copied()
            .ok_or_else(|| PipelineError::UnknownTask {
                name: name.to_owned(),
            })
    }

    /// The tasks in deterministic topological execution order.
    pub fn execution_order(&self) -> Vec<&str> {
        self.topological_order_indices()
            .into_iter()
            .map(|index| self.nodes[index].decl.name.as_str())
            .collect()
    }

    /// Tasks grouped into deterministic parallel waves.
    pub fn parallel_waves(&self) -> Vec<Vec<&str>> {
        let mut waves = Vec::new();
        let (mut in_degree, mut ready) = self.initial_schedule();

        while !ready.is_empty() {
            let wave_indices: Vec<_> = ready.iter().copied().collect();
            ready.clear();

            for index in &wave_indices {
                for dependent in &self.dependents[*index] {
                    in_degree[*dependent] -= 1;
                    if in_degree[*dependent] == 0 {
                        ready.insert(*dependent);
                    }
                }
            }

            waves.push(
                wave_indices
                    .into_iter()
                    .map(|index| self.nodes[index].decl.name.as_str())
                    .collect(),
            );
        }

        waves
    }

    /// Return the mutable state shared by deterministic scheduling views.
    ///
    /// Both public scheduling methods start from the same indegrees and initial ready set;
    /// keeping that setup in one place ensures they retain the same declaration-order
    /// tie-breaker without sharing any mutable state between calls.
    fn initial_schedule(&self) -> (Vec<usize>, BTreeSet<usize>) {
        let in_degree: Vec<usize> = self.dependencies.iter().map(Vec::len).collect();
        let ready = in_degree
            .iter()
            .enumerate()
            .filter_map(|(index, degree)| (*degree == 0).then_some(index))
            .collect();
        (in_degree, ready)
    }

    fn topological_order_indices(&self) -> Vec<usize> {
        let mut order = Vec::with_capacity(self.nodes.len());
        let (mut in_degree, mut ready) = self.initial_schedule();

        while let Some(index) = ready.pop_first() {
            order.push(index);
            for dependent in &self.dependents[index] {
                in_degree[*dependent] -= 1;
                if in_degree[*dependent] == 0 {
                    ready.insert(*dependent);
                }
            }
        }

        order
    }
}

fn normalize_param(value: &Value) -> String {
    match value {
        Value::Null => "null".to_owned(),
        Value::Bool(value) => value.to_string(),
        Value::Number(value) => value.to_string(),
        Value::String(value) => value.clone(),
        Value::Array(_) | Value::Object(_) => {
            serde_json::to_string(value).unwrap_or_else(|_| value.to_string())
        }
    }
}

fn find_cycle(nodes: &[TaskNode], dependents: &[Vec<usize>]) -> Option<Vec<String>> {
    #[derive(Clone, Copy, PartialEq, Eq)]
    enum Visit {
        Unvisited,
        Visiting,
        Done,
    }

    fn visit(
        index: usize,
        nodes: &[TaskNode],
        dependents: &[Vec<usize>],
        state: &mut [Visit],
        stack: &mut Vec<usize>,
    ) -> Option<Vec<String>> {
        state[index] = Visit::Visiting;
        stack.push(index);

        for next in &dependents[index] {
            match state[*next] {
                Visit::Unvisited => {
                    if let Some(cycle) = visit(*next, nodes, dependents, state, stack) {
                        return Some(cycle);
                    }
                }
                Visit::Visiting => {
                    let start = stack
                        .iter()
                        .position(|candidate| candidate == next)
                        .expect("visiting node is on the DFS stack");
                    let mut cycle: Vec<String> = stack[start..]
                        .iter()
                        .map(|cycle_index| nodes[*cycle_index].decl.name.clone())
                        .collect();
                    cycle.push(nodes[*next].decl.name.clone());
                    return Some(cycle);
                }
                Visit::Done => {}
            }
        }

        stack.pop();
        state[index] = Visit::Done;
        None
    }

    let mut state = vec![Visit::Unvisited; nodes.len()];
    let mut stack = Vec::new();

    for index in 0..nodes.len() {
        if state[index] == Visit::Unvisited
            && let Some(cycle) = visit(index, nodes, dependents, &mut state, &mut stack)
        {
            return Some(cycle);
        }
    }

    None
}

/// A pipeline that cannot be built.
#[derive(Debug, Clone, thiserror::Error, PartialEq, Eq)]
pub enum PipelineError {
    /// Two declarations used the same task name.
    #[error("two tasks are both named `{name}`")]
    DuplicateTask {
        /// Duplicated task name.
        name: String,
    },
    /// A task depends on a name that was not declared.
    #[error("task `{task}` depends on `{depends_on}`, which is not declared")]
    UnknownDependency {
        /// Task declaring the dependency.
        task: String,
        /// Missing dependency name.
        depends_on: String,
    },
    /// A requested task name is not present in the resolved pipeline.
    #[error("`{name}` is not a task in this pipeline")]
    UnknownTask {
        /// Missing task name.
        name: String,
    },
    /// The dependency graph contains a cycle.
    #[error("the pipeline has a cycle: {nodes}")]
    Cycle {
        /// Names involved in the cycle, rendered in traversal order.
        nodes: String,
    },
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
    fn independent_tasks_share_a_wave() {
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
        let dag = pipeline(vec![task("ingest", &[]), task("sfm", &["ingest"])])
            .expect("a linear pipeline is valid");

        assert_eq!(
            dag.resolve_hash("sfm", "colmap 3.13.0")
                .expect("sfm is a task"),
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
