---
type: Architecture
title: Artifact Model
description: "Artifact types, content-based blake3 hashing, caching, invalidation, provenance"
purpose: Artifact types, content-based blake3 hashing, caching, invalidation, provenance
last_updated: 2025-02-23
status: stable
related:
  - overview.md
  - pipeline-dag.md
  - ../../crates/core/README.md
  - ../../backlog/docs/decisions/001-step-scoped-vs-odm-monolith.md
---

# Artifact Model

## TL;DR

Artifacts are the nodes in the pipeline DAG. Each artifact has a kind, a
content hash, a location, and provenance. Tasks are cached by hashing their
inputs + params + engine version. Changing one parameter invalidates only
downstream artifacts. This gives incremental processing, resume, and
reproducibility as architectural properties.

## The Artifact-First Principle

Do not make filesystem paths the cross-task API.

**Bad:**
```rust
struct TaskInput {
    input_path: String,  // "/mnt/worker-12/project/output.laz"
}
```

**Good:**
```rust
struct TaskInput {
    artifact: ArtifactRef,  // globally addressable, location-agnostic
}
```

An artifact is a **logically meaningful processing result** with identity,
not just a file on disk. An artifact may physically consist of one file,
many files, a directory, or a database.

## Artifact Kinds

```rust
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum ArtifactKind {
    // Domain 1: Dataset
    ImageSet,
    ImageMetadata,

    // Domain 2: Reconstruction
    Features,
    FeatureMatches,
    CameraModel,
    CameraPoses,
    SparsePointCloud,
    DensePointCloud,
    Mesh,
    TexturedMesh,

    // Domain 3: Geometry
    GeoreferencedScene,

    // Domain 4: Surface
    FilteredPointCloud,
    ClassifiedPointCloud,
    Dsm,
    Dtm,

    // Domain 5: Cartography
    Orthomosaic,
    Report,
}
```

## The Artifact Struct

```rust
pub struct Artifact {
    pub id: ArtifactId,
    pub kind: ArtifactKind,
    pub location: ArtifactLocation,
    pub content_hash: ArtifactHash,
    pub size_bytes: u64,
    pub metadata: ArtifactMetadata,
    pub provenance: Provenance,
    pub created_at: DateTime<Utc>,
}

pub struct ArtifactRef {
    pub id: ArtifactId,
    pub kind: ArtifactKind,
    pub content_hash: ArtifactHash,
}

pub enum ArtifactLocation {
    WorkerLocal { worker_id: String, path: PathBuf },
    SharedVolume { path: PathBuf },
    ObjectStorage { uri: String },  // s3://bucket/artifacts/123
}

pub struct ArtifactMetadata {
    pub crs: Option<String>,
    pub bounds: Option<BoundingBox>,
    pub resolution: Option<f64>,
    pub point_count: Option<u64>,
    pub image_count: Option<usize>,
}
```

## Content-Based Hashing

Every task result is identified by a deterministic hash:

```
TaskHash = blake3(
    task_kind
    ‖ input_artifact_hashes (sorted)
    ‖ normalized_parameters (deterministic JSON)
    ‖ engine_name
    ‖ engine_version
)
```

### Implementation

```rust
pub fn compute_task_hash(
    task_kind: &str,
    input_hashes: &[&str],
    normalized_params_json: &str,
    engine_name: &str,
    engine_version: &str,
) -> String {
    let mut hasher = blake3::Hasher::new();
    hasher.update(task_kind.as_bytes());

    // Sort input hashes for determinism
    let mut sorted_inputs: Vec<&str> = input_hashes.to_vec();
    sorted_inputs.sort();
    for h in sorted_inputs {
        hasher.update(h.as_bytes());
    }

    hasher.update(normalized_params_json.as_bytes());
    hasher.update(engine_name.as_bytes());
    hasher.update(engine_version.as_bytes());

    hasher.finalize().to_hex().to_string()
}
```

### The hash chain

Each artifact's hash depends on everything upstream:

```
ImageSet hash
    = blake3("ingest_images" + file_checksums)
    = a1b2c3...

Features hash
    = blake3("extract_features" + a1b2c3 + {sift, 8192} + "colmap" + "3.9.1")
    = d4e5f6...

Matches hash
    = blake3("match_features" + d4e5f6 + {exhaustive, 0.8} + "colmap" + "3.9.1")
    = g7h8i9...

SfM hash
    = blake3("sparse_reconstruction" + g7h8i9 + {100, 30} + "colmap" + "3.9.1")
    = j0k1l2...

Dense hash
    = blake3("dense_reconstruction" + j0k1l2 + {high, 1} + "openmvs" + "2.2.0")
    = m3n4o5...

DSM hash
    = blake3("generate_dsm" + m3n4o5 + {0.05, standard} + "builtin" + "0.1.0")
    = p6q7r8...

Ortho hash
    = blake3("orthorectify" + j0k1l2 + p6q7r8 + a1b2c3 + {0.03, cog} + "builtin" + "0.1.0")
    = s9t0u1...
```

## Incremental Invalidation

When a parameter changes, only downstream artifacts are invalidated.

### Example: change DSM resolution from 0.05 to 0.10

```
ingest       ✓ a1b2c3  (unchanged)
features     ✓ d4e5f6  (unchanged)
matching     ✓ g7h8i9  (unchanged)
sfm          ✓ j0k1l2  (unchanged)
georef       ✓ k2m3n4  (unchanged)
dense        ✓ m3n4o5  (unchanged)
filter       ✓ l5p6q7  (unchanged)
dsm          ✗ STALE   (resolution changed: 0.05 → 0.10)
dtm          ✗ STALE   (depends on filter → dense, but DTM config may also change)
ortho        ✗ STALE   (depends on DSM hash which changed)
mesh         ✓ cached  (independent branch, doesn't depend on DSM)
texture      ✓ cached  (depends on mesh, not DSM)
```

Only 3 of 12 stages recompute. The other 9 are instant cache hits. This is
the single strongest argument for the step-scoped architecture.

### Example: change feature detector from SIFT to ORB

```
ingest       ✓ cached
features     ✗ STALE   (detector changed)
matching     ✗ STALE   (input hash changed)
sfm          ✗ STALE   (input hash changed)
georef       ✗ STALE   (input hash changed)
dense        ✗ STALE   (input hash changed)
EVERYTHING DOWNSTREAM  ✗ STALE
```

This is correct: changing the feature detector affects the entire pipeline.

## The Artifact Store

```rust
pub struct ArtifactStore {
    base_dir: PathBuf,
}

impl ArtifactStore {
    pub fn new(base_dir: impl Into<PathBuf>) -> Self {
        Self { base_dir: base_dir.into() }
    }

    pub fn exists(&self, task_hash: &str) -> bool {
        self.cache_path(task_hash).exists()
    }

    pub fn cache_path(&self, task_hash: &str) -> PathBuf {
        // Two-level directory structure to avoid too many files in one dir
        let prefix = &task_hash[..2];
        self.base_dir.join("cache").join(prefix).join(task_hash)
    }

    pub fn register(&self, artifact: Artifact) -> Result<ArtifactRef> {
        let cache_dir = self.cache_path(&artifact.content_hash);
        std::fs::create_dir_all(&cache_dir)?;

        // Move or symlink artifact files into cache
        // Write metadata JSON
        let meta_path = cache_dir.join("metadata.json");
        std::fs::write(&meta_path, serde_json::to_string_pretty(&artifact)?)?;

        Ok(ArtifactRef {
            id: artifact.id,
            kind: artifact.kind,
            content_hash: artifact.content_hash,
        })
    }

    pub fn resolve(&self, artifact_ref: &ArtifactRef) -> Result<PathBuf> {
        let cache_dir = self.cache_path(&artifact_ref.content_hash);
        if cache_dir.exists() {
            Ok(cache_dir)
        } else {
            anyhow::bail!("artifact not in cache: {}", artifact_ref.content_hash)
        }
    }
}
```

### Cache directory structure

```
.cache/
  91/
    91a8b3c4d5e6.../
      metadata.json
      orthomosaic.tif
  4e/
    4e12f5a6b7c8.../
      metadata.json
      dsm.tif
  a2/
    a2cc3d4e5f6a.../
      metadata.json
      dense_cloud.laz
```

## Artifact Lifecycle

```
COMPUTED
    │
    ├──► PERSISTING (async upload to object storage)
    │         │
    │         ▼
    │     PERSISTED
    │
    ▼
  (local use by next task)
    │
    ▼
  EXPIRED (retention policy)
    │
    ▼
  DELETED
```

A task can finish computation before persistence finishes. The next local
task can use the worker-local artifact immediately while the upload happens
in the background.

## Persistence Policies

Do not persist every file by default. ODM produces many intermediate files.

```rust
pub enum PersistencePolicy {
    OutputsOnly,   // orthomosaic, DSM, DTM, point cloud
    Checkpoints,   // outputs + reconstruction + dense cloud
    Debug,         // everything including logs and engine internals
}
```

| Policy | What's persisted | Storage cost |
|---|---|---|
| `outputs-only` | orthomosaic, DSM, DTM, point cloud, mesh | ~10–50 GB |
| `checkpoints` | outputs + reconstruction + dense cloud | ~50–200 GB |
| `debug` | everything including ODM project directory | ~100–500 GB |

## Provenance

Every artifact knows how it was created:

```rust
pub struct Provenance {
    pub task_id: String,
    pub task_kind: TaskKind,
    pub engine_name: String,
    pub engine_version: String,
    pub platform_version: String,
    pub parameters: serde_json::Value,
    pub input_artifact_hashes: Vec<String>,
    pub execution_time_secs: f64,
    pub worker_id: String,
    pub timestamp: DateTime<Utc>,
    pub environment: EnvironmentInfo,
}

pub struct EnvironmentInfo {
    pub os: String,
    pub cpu_count: usize,
    pub ram_gb: u64,
    pub gpu: Option<String>,
}
```

### The `explain` command

```bash
$ nadir explain ./outputs/orthomosaic.tif

Orthomosaic: outputs/orthomosaic.tif
  ├─ Hash:    blake3:s9t0u1...
  ├─ Size:    412 MB
  ├─ Engine:  nadir-cartography v0.1.0 (GDAL 3.9.1)
  ├─ Params:  { resolution: 0.03, format: "cog" }
  ├─ Time:    5m 11s
  │
  ├── DSM: cache/p6q7r8...
  │    ├─ Engine: nadir-surface v0.1.0 (IDW)
  │    ├─ Params: { resolution: 0.05 }
  │    └─ Dense Cloud: cache/m3n4o5...
  │         ├─ Engine: openmvs v2.2.0
  │         └─ SfM: cache/j0k1l2...
  │              ├─ Engine: colmap v3.9.1
  │              └─ 1,284 images registered (98.2%)
  │
  └── Dataset: mission/ (1,284 images, RTK)
```

## Reproducibility

A run should be fully reconstructable. Store:

- Workflow version
- Engine versions (COLMAP, OpenMVS, GDAL, PROJ)
- Platform version (Nadir)
- Normalized configuration
- Input artifact IDs and checksums
- Environment information
- Resource profile

This means: same inputs + same config + same engine versions = same outputs.

## See Also

- [Pipeline & DAG](./pipeline-dag.md) — how tasks consume artifacts
- [Core types](../../crates/core/README.md) — Rust struct definitions
- [Overview](./overview.md) — the artifact-first principle
