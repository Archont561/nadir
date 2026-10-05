//! Output-tree manifests: the record of the bytes a stage actually produced.
//!
//! A manifest lists every file in a stage's output tree — its canonical
//! relative path, size, permission bits and BLAKE3 content digest — nothing
//! else. It is pure content identity: no timestamps, no machine names, no
//! invocation facts. Two runs that produce identical bytes therefore produce
//! identical manifests and identical [`TreeDigest`] values, which is what
//! ADR-012's fresh-store determinism evidence compares.
//!
//! [`OutputTreeManifest::validate_tree`] re-derives that record from a
//! directory and reports every way the directory can fail to be the manifest's
//! tree, as a typed [`ManifestError`] rather than a boolean: the caller must be
//! able to tell "mutated bytes" from "extra file" without parsing prose.

use std::path::{Path, PathBuf};

use nadir_core::ArtifactHash;

use crate::hashing::{finalize, update_bytes, update_len, update_str};

/// The format tag every serialized manifest carries.
///
/// A document with any other tag is schema-incompatible by construction, which
/// is how a future manifest format gets detected instead of misread.
pub const MANIFEST_FORMAT: &str = "nadir.output-tree.v1";

/// One file in an output tree, as recorded by a manifest.
#[derive(Debug, Clone, PartialEq, Eq, serde::Serialize, serde::Deserialize)]
pub struct FileEntry {
    /// Canonical relative path within the tree, using `/` separators.
    pub path: PathBuf,
    /// Size of the file in bytes.
    pub size_bytes: u64,
    /// Permission bits recorded at promotion time (e.g. `0o644`).
    pub mode: u32,
    /// BLAKE3 digest of the file's bytes.
    pub digest: ArtifactHash,
}

/// The manifest of a stage's output tree: every file, sorted by path.
#[derive(Debug, Clone, PartialEq, Eq, serde::Serialize, serde::Deserialize)]
pub struct OutputTreeManifest {
    /// Every file in the tree, sorted by canonical relative path.
    pub files: Vec<FileEntry>,
}

impl OutputTreeManifest {
    /// Walk a directory and build its manifest, digesting every file's bytes.
    ///
    /// This is the promotion-time operation: the resulting manifest is exactly
    /// what was staged, no more and no less. Symlinks and other irregular
    /// files are rejected — a tree whose "files" can point outside itself is
    /// not a tree this store will vouch for. Empty directories are invisible:
    /// a manifest is a manifest of bytes.
    pub fn build_from_dir(root: &Path) -> Result<Self, ManifestError> {
        let files = walk_tree(root)?;
        Ok(Self { files })
    }

    /// The produced-bytes identity of this manifest.
    ///
    /// Digests the canonical manifest record (paths, sizes, modes, content
    /// digests) under the `nadir.output-tree.v1` domain. It never touches the
    /// invocation domain, so a tree digest cannot be mistaken for a request
    /// identity by construction.
    pub fn digest(&self) -> TreeDigest {
        TreeDigest::of_manifest(self)
    }

    /// Validate that `root` still is exactly the tree this manifest records.
    ///
    /// Every byte is re-read and re-digested; the manifest's paths are checked
    /// for safety before any filesystem access happens under them. Missing,
    /// extra, mutated, resized or irregular files are typed errors — never a
    /// `false` and never a panic.
    pub fn validate_tree(&self, root: &Path) -> Result<(), ManifestError> {
        for file in &self.files {
            if !is_safe_relative_path(&file.path) {
                return Err(ManifestError::UnsafePath {
                    path: file.path.clone(),
                });
            }
        }

        let actual = walk_tree(root)?;

        let expected_paths: Vec<&Path> = self.files.iter().map(|f| f.path.as_path()).collect();
        let actual_paths: Vec<&Path> = actual.iter().map(|f| f.path.as_path()).collect();

        for expected in &self.files {
            if !actual_paths.contains(&expected.path.as_path()) {
                return Err(ManifestError::MissingFile {
                    path: expected.path.clone(),
                });
            }
        }
        for file in &actual {
            if !expected_paths.contains(&file.path.as_path()) {
                return Err(ManifestError::ExtraFile {
                    path: file.path.clone(),
                });
            }
        }

        for expected in &self.files {
            let found = actual
                .iter()
                .find(|f| f.path == expected.path)
                .expect("presence checked above");
            if found.size_bytes != expected.size_bytes {
                return Err(ManifestError::SizeMismatch {
                    path: expected.path.clone(),
                    expected: expected.size_bytes,
                    found: found.size_bytes,
                });
            }
            if found.digest != expected.digest {
                return Err(ManifestError::DigestMismatch {
                    path: expected.path.clone(),
                    expected: expected.digest.to_string(),
                    found: found.digest.to_string(),
                });
            }
        }

        Ok(())
    }
}

/// The digest of an output-tree manifest: the identity of *produced bytes*.
///
/// A distinct type from [`crate::InvocationKey`] (request identity) on purpose
/// — ADR-012's cache semantics exist precisely because those two identities
/// answer different questions and must never be compared or conflated.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct TreeDigest(ArtifactHash);

impl TreeDigest {
    /// Digest a manifest under the output-tree domain.
    pub fn of_manifest(manifest: &OutputTreeManifest) -> Self {
        let mut buffer = Vec::new();
        update_str(&mut buffer, "domain", MANIFEST_FORMAT);
        update_len(&mut buffer, "files", manifest.files.len());
        for file in &manifest.files {
            update_str(&mut buffer, "file.path", &file.path.to_string_lossy());
            update_bytes(&mut buffer, "file.size", &file.size_bytes.to_le_bytes());
            update_bytes(&mut buffer, "file.mode", &file.mode.to_le_bytes());
            update_bytes(&mut buffer, "file.digest", file.digest.to_hex().as_bytes());
        }
        Self(finalize(&buffer))
    }

    /// The underlying digest, for callers that report or store it.
    pub fn digest(self) -> ArtifactHash {
        self.0
    }
}

impl std::fmt::Display for TreeDigest {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        self.0.fmt(f)
    }
}

impl std::str::FromStr for TreeDigest {
    type Err = String;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        s.parse::<ArtifactHash>()
            .map(Self)
            .map_err(|err| format!("not an output-tree digest (expected blake3 hex): {err}"))
    }
}

impl serde::Serialize for TreeDigest {
    fn serialize<S: serde::Serializer>(&self, serializer: S) -> Result<S::Ok, S::Error> {
        self.0.serialize(serializer)
    }
}

impl<'de> serde::Deserialize<'de> for TreeDigest {
    fn deserialize<D: serde::Deserializer<'de>>(deserializer: D) -> Result<Self, D::Error> {
        ArtifactHash::deserialize(deserializer).map(Self)
    }
}

/// Every way a tree can fail to be the tree a manifest records.
#[derive(Debug, Clone, PartialEq, Eq, thiserror::Error)]
pub enum ManifestError {
    /// A manifest path is not a safe canonical relative path.
    #[error("manifest path `{}` is not a safe relative path", path.display())]
    UnsafePath {
        /// The offending path.
        path: PathBuf,
    },
    /// A file the manifest records is absent from the tree.
    #[error("manifest file `{}` is missing from the tree", path.display())]
    MissingFile {
        /// The absent file.
        path: PathBuf,
    },
    /// The tree contains a file the manifest does not record.
    #[error("tree contains undeclared file `{}`", path.display())]
    ExtraFile {
        /// The undeclared file.
        path: PathBuf,
    },
    /// A file's size differs from the manifest's record.
    #[error("file `{}` is {found} bytes, manifest recorded {expected}", path.display())]
    SizeMismatch {
        /// The file whose size disagrees.
        path: PathBuf,
        /// Size the manifest recorded.
        expected: u64,
        /// Size found in the tree.
        found: u64,
    },
    /// A file's bytes do not match the manifest's digest.
    #[error("file `{}` digests to {found}, manifest recorded {expected}", path.display())]
    DigestMismatch {
        /// The file whose content disagrees.
        path: PathBuf,
        /// Digest the manifest recorded.
        expected: String,
        /// Digest of the bytes found.
        found: String,
    },
    /// The tree contains a symlink or other non-regular file.
    #[error("tree contains irregular file `{}`; symlinks and special files are rejected", path.display())]
    IrregularFile {
        /// The offending entry.
        path: PathBuf,
    },
    /// Reading the tree failed.
    #[error("io error at `{}`: {error}", path.display())]
    Io {
        /// Where the failure happened.
        path: PathBuf,
        /// The failure.
        error: std::io::ErrorKind,
    },
}

/// A manifest path must be relative, non-empty, and free of `..` components.
pub(crate) fn is_safe_relative_path(path: &Path) -> bool {
    if path.as_os_str().is_empty() || path.is_absolute() {
        return false;
    }
    let mut components = path.components();
    components.all(|component| {
        matches!(
            component,
            std::path::Component::Normal(_) | std::path::Component::CurDir
        )
    })
}

/// Walk a tree, digesting every regular file, sorted by relative path.
fn walk_tree(root: &Path) -> Result<Vec<FileEntry>, ManifestError> {
    let mut files = Vec::new();
    walk_into(root, PathBuf::new(), &mut files)?;
    files.sort_by(|left, right| left.path.cmp(&right.path));
    Ok(files)
}

fn walk_into(dir: &Path, prefix: PathBuf, files: &mut Vec<FileEntry>) -> Result<(), ManifestError> {
    let mut entries: Vec<_> = std::fs::read_dir(dir)
        .map_err(|error| ManifestError::Io {
            path: dir.to_path_buf(),
            error: error.kind(),
        })?
        .collect::<std::io::Result<_>>()
        .map_err(|error| ManifestError::Io {
            path: dir.to_path_buf(),
            error: error.kind(),
        })?;
    entries.sort_by_key(|entry| entry.file_name());

    for entry in entries {
        let relative = prefix.join(entry.file_name());
        let path = entry.path();
        let metadata = entry.metadata().map_err(|error| ManifestError::Io {
            path: path.clone(),
            error: error.kind(),
        })?;

        if metadata.is_dir() {
            walk_into(&path, relative, files)?;
        } else if metadata.is_file() {
            let bytes = std::fs::read(&path).map_err(|error| ManifestError::Io {
                path: path.clone(),
                error: error.kind(),
            })?;
            files.push(FileEntry {
                path: relative,
                size_bytes: bytes.len() as u64,
                mode: permission_bits(&metadata),
                digest: ArtifactHash::of_bytes(&bytes),
            });
        } else {
            // Symlinks arrive here via `entry.metadata()` (which does not
            // follow), as do fifos, sockets and devices.
            return Err(ManifestError::IrregularFile { path: relative });
        }
    }
    Ok(())
}

#[cfg(unix)]
fn permission_bits(metadata: &std::fs::Metadata) -> u32 {
    use std::os::unix::fs::MetadataExt;
    metadata.mode() & 0o777
}

#[cfg(not(unix))]
fn permission_bits(_metadata: &std::fs::Metadata) -> u32 {
    0
}
