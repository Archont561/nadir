---
type: Architecture
title: Pipeline & DAG Executor
description: "Declarative TOML pipelines, petgraph DAG, topological executor, concurrent branches"
purpose: Declarative TOML pipelines, petgraph DAG, topological executor, concurrent branches
last_updated: 2025-02-23
status: stable
related:
  - overview.md
  - artifact-model.md
  - engine-registry.md
  - ../implementation/config-model.md
  - ../decisions/001-step-scoped-vs-odm-monolith.md
---

# Pipeline & DAG Executor

## TL;DR

Pipelines are declarative TOML files describing a DAG of tasks. Each task
specifies a `task` kind (photogrammetric concept), an `engine` (implementation
choice), and `depends_on` (upstream dependencies). The executor builds a
`petgraph` DAG, topologically sorts it, checks the artifact cache before each
step, and runs independent branches concurrently via Tokio.

## The Pipeline Definition Format

Pipelines are TOML files stored in `pipelines/`:

```toml
# pipelines/standard.toml
name = "standard"
version = "1.0.0"
description = "Full photogrammetry: orthomosaic + DSM + DTM + 3D model"

[[steps]]
id = "ingest"
task = "ingest_images"
engine = "builtin"

[[steps]]
id = "features"
task = "extract_features"
engine = "colmap"
depends_on = ["ingest"]
[steps.params]
detector = "sift"
max_features = 8192

[[steps]]
id = "matching"
task = "match_features"
engine = "colmap"
depends_on = ["features"]
[steps.params]
method = "exhaustive"

[[steps]]
id = "sfm"
task = "sparse_reconstruction"
engine = "colmap"
depends_on = ["matching"]

[[steps]]
id = "georef"
task = "georeference"
engine = "builtin"
depends_on = ["sfm", "ingest"]

[[steps]]
id = "dense"
task = "dense_reconstruction"
engine = "openmvs"
depends_on = ["georef"]
[steps.params]
resolution_level = 1

[[steps]]
id = "filter"
task = "filter_point_cloud"
engine = "builtin"
depends_on = ["dense"]

[[steps]]
id = "dsm"
task = "generate_dsm"
engine = "builtin"
depends_on = ["filter"]
[steps.params]
resolution = 0.05

[[steps]]
id = "dtm"
task = "generate_dtm"
engine = "builtin"
depends_on = ["filter"]
[steps.params]
resolution = 0.05

[[steps]]
id = "mesh"
task = "generate_mesh"
engine = "openmvs"
depends_on = ["filter"]
optional = true

[[steps]]
id = "ortho"
task = "orthorectify"
engine = "builtin"
depends_on = ["georef", "dsm", "ingest"]
[steps.params]
resolution = 0.03
format = "cog"
```

### Key fields

| Field | Required | Description |
|---|---|---|
| `id` | ✅ | Unique step identifier within the pipeline |
| `task` | ✅ | Photogrammetric concept (maps to `TaskKind` enum) |
| `engine` | ✅ | Implementation choice (resolved by `EngineRegistry`) |
| `depends_on` | ❌ | List of upstream step IDs (empty = root step) |
| `params` | ❌ | Step-specific configuration (maps to config structs) |
| `optional` | ❌ | If true, skip if engine not available |

### The critical rule

The `task` field references a **concept**, not an engine. The `engine` field
selects the implementation. This means swapping engines requires changing
only one field:

```toml
# Use COLMAP for dense instead of OpenMVS
[[steps]]
id = "dense"
task = "dense_reconstruction"   # ← same concept
engine = "colmap"               # ← different implementation
```

## The TaskKind Enum

```rust
#[derive(Debug, Clone, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum TaskKind {
    // Domain 1
    IngestImages,
    // Domain 2
    ExtractFeatures,
    MatchFeatures,
    SparseReconstruction,
    DenseReconstruction,
    GenerateMesh,
    TextureMesh,
    // Domain 3
    Georeference,
    // Domain 4
    FilterPointCloud,
    ClassifyPointCloud,
    GenerateDsm,
    GenerateDtm,
    // Domain 5
    Orthorectify,
    Mosaic,
    GenerateReport,
}
```

## DAG Construction

The pipeline TOML is parsed into a `petgraph::DiGraph`:

```rust
use petgraph::graph::DiGraph;
use petgraph::algo::toposort;
use std::collections::HashMap;

pub struct PipelineDef {
    pub name: String,
    pub version: String,
    pub description: String,
    pub steps: Vec<StepDef>,
}

pub struct StepDef {
    pub id: String,
    pub task: String,
    pub engine: String,
    pub depends_on: Vec<String>,
    pub params: serde_json::Value,
    pub optional: bool,
}

pub struct PipelineDag {
    graph: DiGraph<String, ()>,
    node_map: HashMap<String, petgraph::graph::NodeIndex>,
}

impl PipelineDag {
    pub fn from_def(def: &PipelineDef) -> Result<Self> {
        let mut graph = DiGraph::new();
        let mut node_map = HashMap::new();

        // Add nodes
        for step in &def.steps {
            let idx = graph.add_node(step.id.clone());
            node_map.insert(step.id.clone(), idx);
        }

        // Add edges (dependency → dependent)
        for step in &def.steps {
            let target = node_map[&step.id];
            for dep in &step.depends_on {
                let source = node_map.get(dep)
                    .ok_or_else(|| anyhow!("unknown dependency: {dep}"))?;
                graph.add_edge(*source, target, ());
            }
        }

        // Verify no cycles
        toposort(&graph, None)
            .map_err(|_| anyhow!("cycle detected in pipeline DAG"))?;

        Ok(Self { graph, node_map })
    }

    pub fn execution_order(&self) -> Vec<&str> {
        toposort(&self.graph, None)
            .unwrap()
            .iter()
            .map(|&idx| self.graph[idx].as_str())
            .collect()
    }

    /// Find steps that can run concurrently (no dependency between them)
    pub fn concurrent_groups(&self) -> Vec<Vec<&str>> {
        // Kahn's algorithm: group nodes by topological level
        let mut in_degree = HashMap::new();
        for idx in self.graph.node_indices() {
            in_degree.insert(idx, self.graph.edges_directed(
                idx, petgraph::Direction::Incoming
            ).count());
        }

        let mut groups = Vec::new();
        let mut ready: Vec<_> = in_degree.iter()
            .filter(|(_, &deg)| deg == 0)
            .map(|(&idx, _)| idx)
            .collect();

        while !ready.is_empty() {
            let group: Vec<&str> = ready.iter()
                .map(|&idx| self.graph[idx].as_str())
                .collect();
            groups.push(group);

            let mut next_ready = Vec::new();
            for &idx in &ready {
                for edge in self.graph.edges_directed(idx, petgraph::Direction::Outgoing) {
                    let target = edge.target();
                    *in_degree.get_mut(&target).unwrap() -= 1;
                    if in_degree[&target] == 0 {
                        next_ready.push(target);
                    }
                }
            }
            ready = next_ready;
        }

        groups
    }
}
```

### Example DAG for `standard.toml`

```
ingest
  │
  ▼
features
  │
  ▼
matching
  │
  ▼
sfm
  │
  ▼
georef ──────┐
  │          │
  ▼          │
dense        │
  │          │
  ▼          │
filter       │
  │          │
  ├──► dsm ──┤
  │          │
  ├──► dtm   │
  │          │
  └──► mesh  │
             │
             ▼
           ortho
```

Concurrent groups:

```
Level 0: [ingest]
Level 1: [features]
Level 2: [matching]
Level 3: [sfm]
Level 4: [georef]
Level 5: [dense]
Level 6: [filter]
Level 7: [dsm, dtm, mesh]    ← concurrent!
Level 8: [ortho]
```

## The Executor

### Sequential execution (V0.1)

```rust
pub struct Executor {
    registry: EngineRegistry,
    artifact_store: ArtifactStore,
    workspace: PathBuf,
}

impl Executor {
    pub async fn execute(&self, pipeline: &PipelineDef) -> Result<RunResult> {
        let dag = PipelineDag::from_def(pipeline)?;
        let order = dag.execution_order();

        for step_id in order {
            let step = pipeline.steps.iter()
                .find(|s| s.id == step_id).unwrap();

            // 1. Compute task hash
            let input_hashes = self.collect_upstream_hashes(step)?;
            let params_json = serde_json::to_string(&step.params)?;
            let task_hash = ArtifactStore::compute_task_hash(
                &step.task, &input_hashes, &params_json, "1.0.0",
            );

            // 2. Check cache
            if self.artifact_store.exists(&task_hash) {
                tracing::info!(task = %step.id, "cached, skipping");
                continue;
            }

            // 3. Resolve engine
            let task_kind: TaskKind = step.task.parse()?;
            let engine = self.registry.resolve(&task_kind, &step.engine)?;

            // 4. Execute
            let span = tracing::info_span!("task",
                id = %step.id, engine = %step.engine
            );
            let _guard = span.enter();

            tracing::info!("executing");
            let ctx = TaskContext::new(step, &self.workspace);
            let artifacts = engine.execute(&ctx).await?;

            // 5. Register artifacts
            for artifact in artifacts {
                self.artifact_store.register(artifact)?;
            }
        }

        Ok(RunResult::success())
    }
}
```

### Concurrent execution (V1.0)

Independent DAG branches run in parallel via Tokio:

```rust
pub async fn execute_concurrent(
    &self,
    pipeline: &PipelineDef,
    scheduler: &ResourceScheduler,
) -> Result<RunResult> {
    let dag = PipelineDag::from_def(pipeline)?;
    let groups = dag.concurrent_groups();

    for group in groups {
        // All steps in a group are independent — run concurrently
        let handles: Vec<_> = group.iter().map(|&step_id| {
            let step = pipeline.steps.iter()
                .find(|s| s.id == step_id).unwrap().clone();
            let executor = self.clone();
            let scheduler = scheduler.clone();

            tokio::spawn(async move {
                let req = step.resource_requirements();
                let _guard = scheduler.acquire(&req).await;
                executor.execute_step(&step).await
            })
        }).collect();

        // Wait for all steps in this group to complete
        for handle in handles {
            handle.await??;
        }
    }

    Ok(RunResult::success())
}
```

## Pipeline Variants

Different TOML files for different use cases. Same executor.

### Fast orthophoto

```toml
# pipelines/fast-ortho.toml
name = "fast-ortho"
description = "Quick 2D map, skip 3D and DTM"

[[steps]]
id = "ingest"
task = "ingest_images"
engine = "builtin"

[[steps]]
id = "features"
task = "extract_features"
engine = "colmap"
depends_on = ["ingest"]

[[steps]]
id = "matching"
task = "match_features"
engine = "colmap"
depends_on = ["features"]

[[steps]]
id = "sfm"
task = "sparse_reconstruction"
engine = "colmap"
depends_on = ["matching"]

[[steps]]
id = "georef"
task = "georeference"
engine = "builtin"
depends_on = ["sfm", "ingest"]

[[steps]]
id = "dense"
task = "dense_reconstruction"
engine = "openmvs"
depends_on = ["georef"]

[[steps]]
id = "dsm"
task = "generate_dsm"
engine = "builtin"
depends_on = ["dense"]

[[steps]]
id = "ortho"
task = "orthorectify"
engine = "builtin"
depends_on = ["georef", "dsm", "ingest"]
```

### DEM only

```toml
# pipelines/dem-only.toml
name = "dem-only"
description = "Elevation models only, no orthomosaic"

# ... ingest → features → matching → sfm → georef → dense → classify → dsm → dtm
```

## Adaptive Pipeline Planning (V1.0)

The planner modifies the pipeline based on dataset characteristics:

```rust
pub struct AdaptivePlanner;

impl AdaptivePlanner {
    pub fn plan(
        preflight: &PreflightReport,
        recipe: ProductRecipe,
        base: PipelineDef,
    ) -> PipelineDef {
        let mut planned = base;

        // Adapt matching strategy by image count
        for step in &mut planned.steps {
            if step.task == "match_features" {
                if preflight.total_images < 300 {
                    step.params["method"] = json!("exhaustive");
                } else if preflight.total_images < 3000 {
                    step.params["method"] = json!("spatial");
                } else {
                    step.params["method"] = json!("vocab_tree");
                }
            }
        }

        // Prune steps not needed for the target product
        Self::prune_to_targets(planned, recipe.target_tasks())
    }
}
```

## The `plan` CLI Command

```bash
$ nadir plan ./flight_042 --product orthomosaic

Workflow
────────────────────────────
1. ingest              builtin
2. features            colmap (exhaustive)
3. matching            colmap
4. sfm                 colmap
5. georef              builtin
6. dense               openmvs
7. dsm                 builtin
8. ortho               builtin

Skipped (not needed for orthomosaic):
  - dtm
  - mesh
  - texture

Artifacts
────────────────────────────
input images        ~2.4 GB
features            ~400 MB
matches             ~120 MB
sparse cloud        ~80 MB
dense cloud         ~12 GB
DSM                 ~200 MB
orthomosaic         ~800 MB

Resources
────────────────────────────
CPU                 16 cores
RAM                 32 GB
GPU                 optional
Disk                ~20 GB
Estimated time      ~2h 30m
```

## See Also

- [Artifact model](./artifact-model.md) — caching and hashing
- [Engine registry](./engine-registry.md) — engine resolution
- [Config model](../implementation/config-model.md) — layered configuration
- [QC & adaptive](../implementation/qc-and-adaptive.md) — adaptive planning
