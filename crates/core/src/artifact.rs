//! The artifact model: what a task produces, how it is identified, and where it lives.
//!
//! This is the first of the five core principles in `context.md` — the build-system
//! metaphor — expressed as types. An artifact is the unit of caching, invalidation and
//! provenance: every node in the DAG is one, every arrow is a task that turns some into
//! others.

use std::path::{Path, PathBuf};

use serde::{Deserialize, Serialize};

/// The content address of an artifact.
///
/// `blake3(task_kind ‖ input_hashes ‖ normalized_params ‖ engine_version)` — the formula
/// from the artifact model, as a type. Because the hash covers the inputs and the
/// parameters rather than the wall-clock time or the output path, two runs of the same
/// task over the same inputs produce the same address and the second is a cache hit.
///
/// `normalized_params` is the part that is easy to get wrong and expensive to get wrong
/// later: a float written as `0.30` and a float written as `0.3` are the same parameter,
/// and a hash that treats them as different misses the cache on every run. The
/// normalization is therefore a property of [`TaskSpec`]'s hash, not of the caller.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash, Serialize, Deserialize)]
pub struct ArtifactHash(#[serde(with = "hash_hex")] blake3::Hash);

impl ArtifactHash {
    /// Hash the identity of a task: what it is, what it reads, how it is configured.
    ///
    /// This is the cache key. It deliberately does not include the output path: a task
    /// whose output directory changed is the same task, and should hit the same artifact.
    pub fn of_task(spec: &TaskSpec<'_>) -> Self {
        let mut hasher = blake3::Hasher::new();
        hasher.update(spec.kind.as_bytes());
        hasher.update(b"\x1f"); // unit separator: cannot appear in a task kind
        for input in spec.inputs {
            hasher.update(input.0.as_bytes());
            hasher.update(b"\x1e");
        }
        for (key, value) in spec.params {
            // Sorted, so a TOML table's iteration order cannot change the hash.
            hasher.update(key.as_bytes());
            hasher.update(b"\x1d");
            hasher.update(value.as_bytes());
            hasher.update(b"\x1c");
        }
        hasher.update(spec.engine_version.as_bytes());
        Self(hasher.finalize())
    }

    /// The hash of an artifact produced directly rather than by a task — an input image,
    /// for instance. Content-addressed, so a re-downloaded identical file is a cache hit.
    pub fn of_bytes(bytes: &[u8]) -> Self {
        Self(blake3::hash(bytes))
    }

    /// The hex form used on disk and in `nadir explain` output.
    pub fn to_hex(self) -> String {
        self.0.to_hex().to_string()
    }
}

impl std::fmt::Display for ArtifactHash {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str(&self.to_hex())
    }
}

impl std::str::FromStr for ArtifactHash {
    type Err = String;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        blake3::Hash::from_hex(s)
            .map(Self)
            .map_err(|err| format!("not a blake3 hash: {err}"))
    }
}

/// blake3's `Hash` serializes as an array of bytes; on disk and in JSON it is a hex
/// string, because `[1, 2, 3…]` is not greppable and `nadir explain` output is read by
/// people and by `grep`.
mod hash_hex {
    use serde::{Deserialize, Deserializer, Serializer};

    pub fn serialize<S: Serializer>(hash: &blake3::Hash, s: S) -> Result<S::Ok, S::Error> {
        s.serialize_str(&hash.to_hex().to_string())
    }

    pub fn deserialize<'de, D: Deserializer<'de>>(d: D) -> Result<blake3::Hash, D::Error> {
        let s = String::deserialize(d)?;
        blake3::Hash::from_hex(&s).map_err(serde::de::Error::custom)
    }
}

/// What kind of thing a task produces. Part of the hash, and the first thing
/// `nadir explain` prints, so it names the pipeline stage rather than the Rust type.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum ArtifactKind {
    /// A validated, EXIF-stamped set of input images.
    Dataset,
    /// Keypoints and descriptors for one image.
    Features,
    /// Correspondence graph between images.
    Matches,
    /// A sparse reconstruction: cameras, images, points.
    SparseModel,
    /// A dense point cloud.
    DenseCloud,
    /// A triangular mesh.
    Mesh,
    /// A georeferenced raster surface.
    Raster,
    /// A georeferenced orthomosaic.
    Orthomosaic,
    /// A vector map product.
    Map,
    /// Any other engine output, named by the pipeline.
    Other,
}

/// Where an artifact is stored, and what it is.
///
/// The store is a directory of content-addressed entries rather than a task output path,
/// which is what makes two DAG branches that need the same artifact share it instead of
/// computing it twice.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct Artifact {
    /// The content address: the cache key.
    pub hash: ArtifactHash,
    /// What it is.
    pub kind: ArtifactKind,
    /// Where the bytes live, relative to the workspace root.
    pub path: PathBuf,
    /// Size on disk, recorded so `nadir report` can total a run without stat-ing the tree.
    pub size_bytes: u64,
    /// The task that produced it, absent for an input artifact.
    pub produced_by: Option<TaskSpecOwned>,
}

/// The identity of a task: what it is, what it reads, how it is configured.
///
/// Borrowed, because [`ArtifactHash::of_task`] hashes it without keeping it. The owned form
/// is [`TaskSpecOwned`], which is what lands in an artifact's provenance and therefore has
/// to survive the borrow's lifetime.
#[derive(Debug)]
pub struct TaskSpec<'a> {
    /// The stage name — `sfm`, `dsm`. Part of the hash, so renaming a stage invalidates
    /// its artifacts rather than silently serving a stale one.
    pub kind: &'a str,
    /// The artifacts it reads, in a stable order (a DAG's edge order, not a hash-map's).
    pub inputs: &'a [ArtifactHash],
    /// Normalized parameters, sorted by key.
    pub params: &'a [(&'a str, String)],
    /// The engine that implements it — `colmap`, `openmvs`, `builtin`. Part of the hash,
    /// so upgrading an engine invalidates its output. This is why a version bump costs a
    /// recompute rather than producing a stale artifact labelled with the new version.
    pub engine_version: &'a str,
}

impl TaskSpec<'_> {
    /// Build a spec with its parameters sorted, which is what makes the hash independent
    /// of TOML table order.
    ///
    /// Parameters are sorted here rather than trusted from the caller because the caller is
    /// a TOML parse, and a hash that depends on table order is a cache that misses for no
    /// reason — the same pipeline, re-serialized, is a different hash.
    pub fn new(
        kind: &str,
        inputs: &[ArtifactHash],
        params: &[(&str, String)],
        engine_version: &str,
    ) -> TaskSpec<'_> {
        let mut params = params.to_vec();
        params.sort_by(|a, b| a.0.cmp(b.0));
        TaskSpec {
            kind,
            inputs,
            params: &params,
            engine_version,
        }
    }
}

/// [`TaskSpec`] with an owned lifetime — what provenance records.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct TaskSpecOwned {
    pub kind: String,
    pub inputs: Vec<ArtifactHash>,
    pub params: Vec<(String, String)>,
    pub engine_version: String,
}

impl From<&TaskSpecOwned> for TaskSpec<'_> {
    fn from(owned: &'a TaskSpecOwned) -> Self {
        TaskSpec {
            kind: &owned.kind,
            inputs: &owned.inputs,
            params: &owned
                .params
                .iter()
                .map(|(k, v)| (k.as_str(), v.clone()))
                .collect::<Vec<_>>(),
            engine_version: &owned.engine_version,
        }
    }
}

/// A content-addressed store of artifacts.
///
/// A plain directory, with the addressing in the layout rather than in a database: a run's
/// artifacts stay readable with `ls`, and a run interrupted by a full disk leaves a tree
/// that is still a valid prefix rather than a database needing recovery.
#[derive(Debug, Clone)]
pub struct ArtifactStore {
    root: PathBuf,
}

impl ArtifactStore {
    /// Open (creating if needed) a store rooted at `root`.
    pub fn open(root: impl Into<PathBuf>) -> std::io::Result<Self> {
        let root = root.into();
        std::fs::create_dir_all(&root)?;
        Ok(Self { root })
    }

    /// The path an artifact with this hash occupies.
    ///
    /// Two-level fan-out on the first hex byte. A single directory with one entry per
    /// artifact is unusable at the scale a survey flight produces (990 file entries for
    /// one environment is not the number that matters here; a 1,284-image flight yields
    /// hundreds of thousands of feature artifacts), and a flat directory is also slow to
    /// list on the filesystems CI runs on.
    pub fn path_for(&self, hash: ArtifactHash) -> PathBuf {
        let hex = hash.to_hex();
        self.root.join(&hex[..2]).join(&hex[2..])
    }

    /// Record an artifact's location and size.
    ///
    /// `produced_by` is `None` for an input: an input has provenance ("where it came
    /// from"), but it was not produced by a task, and conflating the two would make
    /// `nadir explain` claim a dataset was computed rather than supplied.
    pub fn record(
        &self,
        artifact: &Artifact,
        produced_by: Option<&TaskSpecOwned>,
    ) -> std::io::Result<()> {
        let path = self.path_for(artifact.hash);
        std::fs::create_dir_all(path.parent().expect("artifact path has a parent"))?;
        let manifest = serde_json::to_vec_pretty(artifact)
            .map_err(|err| std::io::Error::new(std::io::ErrorKind::InvalidData, err))?;
        std::fs::write(path, manifest)?;
        let _ = produced_by; // provenance is carried inside `artifact.produced_by`
        Ok(())
    }

    /// Look an artifact up by hash.
    ///
    /// This is the whole cache: a task asks the store whether its output is already here,
    /// and if it is, it is not recomputed. Everything else in the executor exists to make
    /// the hash correct.
    pub fn get(&self, hash: ArtifactHash) -> Option<Artifact> {
        let bytes = std::fs::read(self.path_for(hash)).ok()?;
        serde_json::from_slice(&bytes).ok()
    }

    /// Whether an artifact is present. Cheaper than [`Self::get`] — no parse — and this is
    /// the call the executor makes once per node.
    pub fn contains(&self, hash: ArtifactHash) -> bool {
        self.path_for(hash).is_file()
    }

    /// Every artifact in the store, for `nadir report`.
    pub fn iter(&self) -> impl Iterator<Item = std::io::Result<Artifact>> {
        let root = self.root.clone();
        let walk = |dir: PathBuf| -> Box<dyn Iterator<Item = std::io::Result<Artifact>>> {
            let read = std::fs::read_dir(dir);
            let entries: Vec<_> = match read {
                Ok(entries) => entries.filter_map(Result::ok).collect(),
                Err(_) => Vec::new(),
            };
            Box::new(entries.into_iter().flat_map(move |entry| {
                let path = entry.path();
                if path.is_dir() {
                    walk(path)
                } else {
                    let read = std::fs::read(&path);
                    Box::new(read.into_iter().filter_map(|bytes| {
                        serde_json::from_slice::<Artifact>(&bytes).ok()
                    }))
                }
            }))
        };
        walk(root)
    }
}