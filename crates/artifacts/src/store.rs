//! The stage cache: an on-disk mapping from invocation keys to verified
//! output-tree manifests.
//!
//! Layout, all plain files inspectable with ordinary tools:
//!
//! ```text
//! <root>/staging/<stage>-<pid>-<nanos>-<n>/   unique, caller-owned until promotion
//! <root>/objects/<digest[0..2]>/<digest[2..]>/   content-addressed output trees
//! <root>/index/<key[0..2]>/<key[2..]>.json    stage entries: key → manifest
//! ```
//!
//! The invariants the layout exists to keep:
//!
//! * an entry is written **only after** its object tree has been fully staged,
//!   validated, digested and moved into place, so a key can never map to bytes
//!   that were not completely produced;
//! * the object move and the index write are each a single `rename`, so a crash
//!   leaves at worst an unreferenced object (a future promotion reuses it) and
//!   never a half-written entry;
//! * a lookup re-digests every byte the manifest names before reporting a hit,
//!   so a hit is evidence about the disk *now*, not about the day it was
//!   cached.

use std::path::{Path, PathBuf};
use std::sync::atomic::{AtomicU64, Ordering};
use std::time::{SystemTime, UNIX_EPOCH};

use crate::invocation::{InvocationKey, InvocationSpec};
use crate::manifest::{ManifestError, OutputTreeManifest, TreeDigest, is_safe_relative_path};

/// The format tag every serialized stage entry carries.
pub const STAGE_ENTRY_FORMAT: &str = "nadir.stage-entry.v1";

/// The completed record for one invocation key: the request it was promoted
/// under, and the verified manifest of the bytes that were produced.
///
/// This is the document AC "each completed stage entry maps to a manifest" is
/// about: the manifest is embedded, and its digest is recorded beside it so a
/// reader can recompute it.
#[derive(Debug, Clone, PartialEq, Eq, serde::Serialize, serde::Deserialize)]
pub struct StageEntry {
    /// Format tag, checked on read; anything else is schema-incompatible.
    pub format: String,
    /// The stage that produced this entry, e.g. `features`.
    pub stage: String,
    /// The output contract version the stage promised.
    pub contract_version: String,
    /// The request identity this entry was promoted under.
    pub invocation_key: InvocationKey,
    /// The produced-bytes identity of the manifest below.
    pub tree_digest: TreeDigest,
    /// The verified manifest of the output tree.
    pub manifest: OutputTreeManifest,
}

/// A verified cache hit.
///
/// Carries the manifest and tree digest — the facts a caller reports or copies
/// from — and deliberately no filesystem path: the cache's object directory is
/// an implementation detail that must never be handed out as a user output.
/// Use [`StageCache::materialize`] to produce user-owned copies.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct StageHit {
    stage: String,
    contract_version: String,
    tree_digest: TreeDigest,
    manifest: OutputTreeManifest,
}

impl StageHit {
    /// The stage that produced the cached bytes.
    pub fn stage(&self) -> &str {
        &self.stage
    }

    /// The output contract version the stage promised.
    pub fn contract_version(&self) -> &str {
        &self.contract_version
    }

    /// The produced-bytes identity of the hit.
    pub fn tree_digest(&self) -> TreeDigest {
        self.tree_digest
    }

    /// The verified manifest of the hit's bytes.
    pub fn manifest(&self) -> &OutputTreeManifest {
        &self.manifest
    }
}

/// The result of a cache lookup.
///
/// `Invalid` is distinct from `Miss` on purpose: a miss means "no completed
/// entry exists, run the stage"; an invalid entry means "an entry exists but
/// its bytes or record cannot be trusted", which callers must be able to
/// report rather than silently recompute through.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Lookup {
    /// No completed entry exists for the key.
    Miss,
    /// A completed entry whose referenced bytes just revalidated.
    Hit(StageHit),
    /// An entry exists but is not trustworthy.
    Invalid(EntryInvalid),
}

/// Every way a present-but-untrustworthy entry fails.
#[derive(Debug, Clone, PartialEq, Eq, thiserror::Error)]
pub enum EntryInvalid {
    /// The entry document does not parse, or carries a foreign format tag.
    #[error("stage entry schema is incompatible: {detail}")]
    Schema {
        /// What was wrong with the document.
        detail: String,
    },
    /// The entry is filed under one key but records another.
    #[error("stage entry records key {recorded} but is filed under {requested}")]
    KeyMismatch {
        /// The key the lookup was for.
        requested: String,
        /// The key the entry records.
        recorded: String,
    },
    /// The recorded tree digest does not match the embedded manifest.
    #[error("stage entry records tree digest {recorded} but its manifest digests to {computed}")]
    TreeDigestMismatch {
        /// The digest the entry claims.
        recorded: String,
        /// The digest the embedded manifest actually has.
        computed: String,
    },
    /// The referenced object tree is absent from the store.
    #[error("the entry's output tree is missing from the store")]
    TreeMissing,
    /// The object tree failed manifest validation.
    #[error("output tree failed manifest validation: {0}")]
    Manifest(#[from] ManifestError),
}

/// Failures of store operations (opening, staging, promoting, materializing).
#[derive(Debug, Clone, PartialEq, Eq, thiserror::Error)]
pub enum StageCacheError {
    /// The staged tree lacks a file the stage contract declares.
    #[error("staged output is missing declared file `{}`", path.display())]
    MissingExpected {
        /// The declared-but-absent file.
        path: PathBuf,
    },
    /// The staged tree contains a file the stage contract does not declare.
    #[error("staged output contains undeclared file `{}`", path.display())]
    UndeclaredFile {
        /// The undeclared file.
        path: PathBuf,
    },
    /// An expected path is not a safe relative path.
    #[error("declared output `{}` is not a safe relative path", path.display())]
    UnsafeExpectedPath {
        /// The offending declaration.
        path: PathBuf,
    },
    /// The stage name is not a usable single path component.
    #[error("stage name `{stage}` is not a safe directory component")]
    UnsafeStageName {
        /// The offending stage name.
        stage: String,
    },
    /// The directory offered for promotion is not one this store created.
    #[error("`{}` is not a staging directory of this store; promotion refuses to move foreign directories", path.display())]
    ForeignStaging {
        /// The foreign directory.
        path: PathBuf,
    },
    /// An entry exists for this key with different bytes.
    #[error(
        "invocation key {invocation_key} is already recorded with tree digest {recorded}; refusing to replace it with {computed}"
    )]
    ConflictingEntry {
        /// The key both trees were promoted under.
        invocation_key: String,
        /// The digest already recorded.
        recorded: String,
        /// The digest the new run produced.
        computed: String,
    },
    /// The staged tree could not be manifested.
    #[error("staged tree failed manifesting: {0}")]
    Manifest(#[from] ManifestError),
    /// An io failure at a named path.
    #[error("io error at `{}`: {error}", path.display())]
    Io {
        /// Where the failure happened.
        path: PathBuf,
        /// The failure.
        error: std::io::ErrorKind,
    },
    /// Serializing an entry failed; the store's own types are broken.
    #[error("stage entry serialization failed: {0}")]
    EntrySerialization(String),
}

impl From<std::io::Error> for StageCacheError {
    fn from(error: std::io::Error) -> Self {
        StageCacheError::Io {
            path: PathBuf::new(),
            error: error.kind(),
        }
    }
}

/// The outcome of a successful promotion.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Promoted {
    /// The request identity the entry is filed under.
    pub invocation_key: InvocationKey,
    /// The produced-bytes identity of the promoted tree.
    pub tree_digest: TreeDigest,
    /// The verified manifest of the promoted tree.
    pub manifest: OutputTreeManifest,
}

/// Monotonic per-process suffix so concurrent staging requests never collide.
static STAGING_COUNTER: AtomicU64 = AtomicU64::new(0);

/// The V0 stage cache.
///
/// # Example
///
/// The full lifecycle a stage adapter performs: stage output into a
/// store-created directory, promote it under the invocation's identity, and on
/// a later identical request receive a byte-revalidated hit whose bytes are
/// copied — never the cache directory itself — to the user's output path.
///
/// ```
/// use nadir_artifacts::{
///     ArtifactHash, InvocationKey, InvocationSpec, Lookup, NamedDigest, RuntimeIdentity,
///     StageCache,
/// };
///
/// # fn main() -> Result<(), Box<dyn std::error::Error>> {
/// let root = std::env::temp_dir().join(format!(
///     "nadir-artifacts-doctest-{}-{}",
///     std::process::id(),
///     std::time::SystemTime::now().duration_since(std::time::UNIX_EPOCH)?.as_nanos(),
/// ));
/// let store = StageCache::open(&root)?;
///
/// let spec = InvocationSpec::new(
///     "features",
///     "colmap-feature-extractor/1",
///     "colmap.database/1",
///     [NamedDigest::new("imageset", ArtifactHash::of_bytes(b"images"))],
///     [("max_features", "8192")],
///     RuntimeIdentity::new(
///         "colmap",
///         "pixi/linux-64/cpu",
///         "3.13.0",
///         ArtifactHash::of_bytes(b"colmap-executable"),
///         [("gpu", "disabled")],
///     ),
/// );
///
/// let staging = store.staging_dir("features")?;
/// std::fs::write(staging.join("database.db"), b"features bytes")?;
/// let promoted = store.promote(&spec, &staging, &["database.db"])?;
///
/// // The identical request later: no staging, no engine, bytes revalidated.
/// let key = InvocationKey::of_spec(&spec);
/// match store.lookup(&key)? {
///     Lookup::Hit(hit) => {
///         assert_eq!(hit.tree_digest(), promoted.tree_digest);
///         store.materialize(&hit, &root.join("outputs/features"))?;
///     }
///     _ => panic!("a promoted entry must hit"),
/// }
///
/// std::fs::remove_dir_all(&root)?;
/// # Ok(())
/// # }
/// ```
#[derive(Debug, Clone)]
pub struct StageCache {
    root: PathBuf,
}

impl StageCache {
    /// Open (creating if needed) a stage cache rooted at `root`.
    pub fn open(root: impl Into<PathBuf>) -> Result<Self, StageCacheError> {
        let root = root.into();
        for dir in ["staging", "objects", "index"] {
            let dir = root.join(dir);
            std::fs::create_dir_all(&dir).map_err(|error| StageCacheError::Io {
                path: dir,
                error: error.kind(),
            })?;
        }
        Ok(Self { root })
    }

    /// Create a unique staging directory for a stage to write its output into.
    ///
    /// The directory belongs to the caller until [`StageCache::promote`]
    /// consumes it; nothing else reads it, and an abandoned one is simply an
    /// unreferenced directory (detection and cleanup are V0.2 `nadir cache`
    /// work, not store work).
    pub fn staging_dir(&self, stage: &str) -> Result<PathBuf, StageCacheError> {
        if !is_safe_stage_name(stage) {
            return Err(StageCacheError::UnsafeStageName {
                stage: stage.to_owned(),
            });
        }

        let pid = std::process::id();
        loop {
            let nanos = SystemTime::now()
                .duration_since(UNIX_EPOCH)
                .map(|since| since.as_nanos())
                .unwrap_or_default();
            let counter = STAGING_COUNTER.fetch_add(1, Ordering::Relaxed);
            let candidate = self
                .root
                .join("staging")
                .join(format!("{stage}-{pid}-{nanos}-{counter}"));

            // `create_dir` is atomic: if it succeeds, the name was free an
            // instant ago and is now exclusively ours.
            match std::fs::create_dir(&candidate) {
                Ok(()) => return Ok(candidate),
                Err(error) if error.kind() == std::io::ErrorKind::AlreadyExists => continue,
                Err(error) => {
                    return Err(StageCacheError::Io {
                        path: candidate,
                        error: error.kind(),
                    });
                }
            }
        }
    }

    /// Validate, digest and atomically promote a staged output tree.
    ///
    /// The staged directory must be one this store created (see
    /// [`StageCache::staging_dir`]). Its contents must exactly match the
    /// stage's declared output paths: missing or undeclared files refuse the
    /// promotion, and so do symlinks and other irregular entries. Only then is
    /// the manifest built, the tree moved into the content-addressed object
    /// store, and the entry written — in that order, each atomically.
    pub fn promote(
        &self,
        spec: &InvocationSpec,
        staging: &Path,
        expected: &[impl AsRef<Path>],
    ) -> Result<Promoted, StageCacheError> {
        for path in expected {
            let path = path.as_ref();
            if !is_safe_relative_path(path) {
                return Err(StageCacheError::UnsafeExpectedPath {
                    path: path.to_path_buf(),
                });
            }
        }

        let staging = self.owned_staging_path(staging)?;
        let manifest = OutputTreeManifest::build_from_dir(&staging)?;

        let declared: Vec<PathBuf> = expected
            .iter()
            .map(|path| path.as_ref().to_path_buf())
            .collect();
        for declared_path in &declared {
            if !manifest
                .files
                .iter()
                .any(|file| &file.path == declared_path)
            {
                return Err(StageCacheError::MissingExpected {
                    path: declared_path.clone(),
                });
            }
        }
        for file in &manifest.files {
            if !declared.contains(&file.path) {
                return Err(StageCacheError::UndeclaredFile {
                    path: file.path.clone(),
                });
            }
        }

        let invocation_key = InvocationKey::of_spec(spec);
        let tree_digest = manifest.digest();

        // Same key, already recorded with different bytes: a reproducibility
        // violation. Trust the first verified entry and refuse the rewrite.
        // (An unreadable existing entry does not block promotion; overwriting
        // it is the self-healing path, since it was never a valid hit.)
        if let Ok(Some(Ok(entry))) = self.read_entry(&invocation_key)
            && entry.tree_digest != tree_digest
        {
            return Err(StageCacheError::ConflictingEntry {
                invocation_key: invocation_key.to_string(),
                recorded: entry.tree_digest.to_string(),
                computed: tree_digest.to_string(),
            });
        }

        let object_dir = self.object_path(&tree_digest);
        if !object_dir.exists() {
            // `rename` is atomic but not creative: the content-addressed shard
            // directory has to exist before the tree can move into it.
            let shard = object_dir
                .parent()
                .expect("object paths live one level under objects/")
                .to_path_buf();
            std::fs::create_dir_all(&shard).map_err(|error| StageCacheError::Io {
                path: shard,
                error: error.kind(),
            })?;
            match std::fs::rename(&staging, &object_dir) {
                Ok(()) => {}
                Err(_) if object_dir.exists() => {
                    // Lost a race with a concurrent promotion of the same
                    // bytes: another writer moved an identical tree into place
                    // between our check and our rename. Fall through to the
                    // verify-and-dedupe path below.
                }
                Err(error) => {
                    return Err(StageCacheError::Io {
                        path: staging.clone(),
                        error: error.kind(),
                    });
                }
            }
        }

        if staging.exists() {
            // The bytes are already in the store (a previous or concurrent
            // promotion of the same tree, possibly under another key). Verify
            // the stored tree, then discard the redundant staging copy.
            manifest.validate_tree(&object_dir)?;
            std::fs::remove_dir_all(&staging).map_err(|error| StageCacheError::Io {
                path: staging.clone(),
                error: error.kind(),
            })?;
        }

        let entry = StageEntry {
            format: STAGE_ENTRY_FORMAT.to_owned(),
            stage: spec.stage.clone(),
            contract_version: spec.contract_version.clone(),
            invocation_key,
            tree_digest,
            manifest: manifest.clone(),
        };
        self.write_entry(&entry)?;

        Ok(Promoted {
            invocation_key,
            tree_digest,
            manifest,
        })
    }

    /// Look up a completed entry, revalidating every byte before any hit.
    ///
    /// A hit proves the manifest's bytes are on disk *now*; it launches no
    /// engine subprocess because the store has no engine to launch — it only
    /// reads and digests. The returned [`StageHit`] carries no store path; use
    /// [`StageCache::materialize`] to produce user-owned outputs.
    pub fn lookup(&self, key: &InvocationKey) -> Result<Lookup, StageCacheError> {
        let Some(parsed) = self.read_entry(key)? else {
            return Ok(Lookup::Miss);
        };
        let entry = match parsed {
            Ok(entry) => entry,
            Err(invalid) => return Ok(Lookup::Invalid(invalid)),
        };

        if entry.invocation_key != *key {
            return Ok(Lookup::Invalid(EntryInvalid::KeyMismatch {
                requested: key.to_string(),
                recorded: entry.invocation_key.to_string(),
            }));
        }

        let computed = entry.manifest.digest();
        if computed != entry.tree_digest {
            return Ok(Lookup::Invalid(EntryInvalid::TreeDigestMismatch {
                recorded: entry.tree_digest.to_string(),
                computed: computed.to_string(),
            }));
        }

        let object_dir = self.object_path(&entry.tree_digest);
        if !object_dir.is_dir() {
            return Ok(Lookup::Invalid(EntryInvalid::TreeMissing));
        }

        if let Err(error) = entry.manifest.validate_tree(&object_dir) {
            return Ok(Lookup::Invalid(EntryInvalid::Manifest(error)));
        }

        Ok(Lookup::Hit(StageHit {
            stage: entry.stage,
            contract_version: entry.contract_version,
            tree_digest: entry.tree_digest,
            manifest: entry.manifest,
        }))
    }

    /// Copy a hit's bytes to a user-owned output directory.
    ///
    /// The destination is user territory; the store's object directory is
    /// never the answer to "where is my output". Recorded permission bits
    /// travel with the copy.
    pub fn materialize(&self, hit: &StageHit, dest: &Path) -> Result<(), StageCacheError> {
        let object_dir = self.object_path(&hit.tree_digest);
        if !object_dir.is_dir() {
            return Err(StageCacheError::Io {
                path: object_dir,
                error: std::io::ErrorKind::NotFound,
            });
        }

        std::fs::create_dir_all(dest).map_err(|error| StageCacheError::Io {
            path: dest.to_path_buf(),
            error: error.kind(),
        })?;

        for file in &hit.manifest.files {
            let source = object_dir.join(&file.path);
            let target = dest.join(&file.path);
            if let Some(parent) = target.parent() {
                std::fs::create_dir_all(parent).map_err(|error| StageCacheError::Io {
                    path: parent.to_path_buf(),
                    error: error.kind(),
                })?;
            }
            std::fs::copy(&source, &target).map_err(|error| StageCacheError::Io {
                path: source.clone(),
                error: error.kind(),
            })?;
            #[cfg(unix)]
            {
                use std::os::unix::fs::PermissionsExt;
                std::fs::set_permissions(&target, std::fs::Permissions::from_mode(file.mode))
                    .map_err(|error| StageCacheError::Io {
                        path: target.clone(),
                        error: error.kind(),
                    })?;
            }
        }

        Ok(())
    }

    /// Verify `staging` is a directory this store created, and return its
    /// canonical path.
    ///
    /// Promotion moves the directory; a path the store did not issue must
    /// therefore be refused, or `promote` would eat a caller's arbitrary
    /// directory.
    fn owned_staging_path(&self, staging: &Path) -> Result<PathBuf, StageCacheError> {
        let staging_root =
            self.root
                .join("staging")
                .canonicalize()
                .map_err(|error| StageCacheError::Io {
                    path: self.root.join("staging"),
                    error: error.kind(),
                })?;
        let canonical = staging
            .canonicalize()
            .map_err(|error| StageCacheError::Io {
                path: staging.to_path_buf(),
                error: error.kind(),
            })?;
        if !canonical.starts_with(&staging_root) {
            return Err(StageCacheError::ForeignStaging {
                path: staging.to_path_buf(),
            });
        }
        Ok(canonical)
    }

    fn entry_path(&self, key: &InvocationKey) -> PathBuf {
        let hex = key.to_string();
        self.root
            .join("index")
            .join(&hex[..2])
            .join(format!("{}.json", &hex[2..]))
    }

    fn object_path(&self, digest: &TreeDigest) -> PathBuf {
        let hex = digest.to_string();
        self.root.join("objects").join(&hex[..2]).join(&hex[2..])
    }

    /// Read and parse an entry; `Ok(None)` when no entry file exists.
    ///
    /// Parse failures and foreign format tags are returned as
    /// [`EntryInvalid::Schema`] wrapped in a hit-or-invalid answer — the caller
    /// decides how to surface them, because "unreadable entry" is not a store
    /// crash.
    fn read_entry(
        &self,
        key: &InvocationKey,
    ) -> Result<Option<Result<StageEntry, EntryInvalid>>, StageCacheError> {
        let path = self.entry_path(key);
        let bytes = match std::fs::read(&path) {
            Ok(bytes) => bytes,
            Err(error) if error.kind() == std::io::ErrorKind::NotFound => return Ok(None),
            Err(error) => {
                return Err(StageCacheError::Io {
                    path,
                    error: error.kind(),
                });
            }
        };

        Ok(Some(Self::parse_entry(&bytes)))
    }

    fn parse_entry(bytes: &[u8]) -> Result<StageEntry, EntryInvalid> {
        let entry: StageEntry =
            serde_json::from_slice(bytes).map_err(|error| EntryInvalid::Schema {
                detail: format!("entry does not parse as {STAGE_ENTRY_FORMAT}: {error}"),
            })?;
        if entry.format != STAGE_ENTRY_FORMAT {
            return Err(EntryInvalid::Schema {
                detail: format!(
                    "entry declares format `{}`, this store reads `{STAGE_ENTRY_FORMAT}`",
                    entry.format
                ),
            });
        }
        Ok(entry)
    }

    /// Write an entry atomically: temp file in the same directory, then rename.
    fn write_entry(&self, entry: &StageEntry) -> Result<(), StageCacheError> {
        let path = self.entry_path(&entry.invocation_key);
        let parent = path
            .parent()
            .expect("entry path is under the store root")
            .to_path_buf();
        std::fs::create_dir_all(&parent).map_err(|error| StageCacheError::Io {
            path: parent.clone(),
            error: error.kind(),
        })?;

        let bytes = serde_json::to_vec_pretty(entry)
            .map_err(|error| StageCacheError::EntrySerialization(error.to_string()))?;

        let nanos = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .map(|since| since.as_nanos())
            .unwrap_or_default();
        let temp = parent.join(format!(
            ".{}.tmp-{}-{}",
            path.file_name()
                .expect("entry path has a file name")
                .to_string_lossy(),
            std::process::id(),
            nanos
        ));
        std::fs::write(&temp, &bytes).map_err(|error| StageCacheError::Io {
            path: temp.clone(),
            error: error.kind(),
        })?;
        std::fs::rename(&temp, &path).map_err(|error| StageCacheError::Io {
            path: temp,
            error: error.kind(),
        })?;
        Ok(())
    }
}

/// A stage name must be a single safe path component.
fn is_safe_stage_name(stage: &str) -> bool {
    if stage.is_empty() || stage == "." || stage == ".." || stage.contains(['/', '\\']) {
        return false;
    }
    let path = Path::new(stage);
    path.components().count() == 1
        && matches!(
            path.components().next(),
            Some(std::path::Component::Normal(_))
        )
}
