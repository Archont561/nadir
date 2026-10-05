//! nadir-artifacts — Content-addressed artifact storage and the invalidation rules it makes possible.
//!
//! The V0 stage cache (ADR-012): invocation keys name requested work,
//! output-tree manifests name the bytes a stage actually produced, and the
//! store maps one to the other only after revalidating every byte. A hit
//! launches no engine subprocess and never hands out a cache path as a user
//! output — callers receive copies through [`StageCache::materialize`].
//!
//! Declared as a workspace member with its own `Cargo.toml` rather than as a directory of
//! `.rs` files, because a crate that is not a member is not compiled by `cargo build
//! --workspace`, is not in `Cargo.lock`, and is not in the licence report `pixi run lint`
//! produces. Adding a crate is one file in its own directory; the workspace notices
//! (`members = ["crates/*"]` in the repository-root `Cargo.toml`).

/// The labeled-hash encoding shared by this crate's identity types.
mod hashing;

/// Invocation keys: request identity for a stage execution.
pub mod invocation;

/// Output-tree manifests: produced-bytes identity and its validation.
pub mod manifest;

/// The stage cache: staging, atomic promotion, revalidating lookups.
pub mod store;

/// Re-exported so callers of the stage cache need no second digest vocabulary:
/// every digest in this crate is an [`nadir_core::ArtifactHash`].
pub use nadir_core::ArtifactHash;

pub use invocation::{InvocationKey, InvocationSpec, NamedDigest, RuntimeIdentity, Setting};
pub use manifest::{FileEntry, MANIFEST_FORMAT, ManifestError, OutputTreeManifest, TreeDigest};
pub use store::{
    EntryInvalid, Lookup, Promoted, STAGE_ENTRY_FORMAT, StageCache, StageCacheError, StageEntry,
    StageHit,
};

/// The pipeline stage this crate exists to implement, as it appears in a pipeline file.
///
/// Named here rather than only in the docs because `nadir plan` reads it: a pipeline that
/// names `georef` and a crate that claims `ortho` is a mismatch `nadir plan` can report.
pub const STAGE: &str = "artifacts";

/// A one-line description of what this crate will do.
///
/// Returns a `String` rather than a `&'static str` on purpose, and the reason is the test:
/// it is the smallest function whose output could be wrong in a way the type system cannot
/// catch, which is exactly the shape of the code this crate will grow into.
pub fn describe() -> String {
    format!("nadir/{}: {}", crate_name(), STAGE)
}

/// The crate's name, from `Cargo.toml`.
///
/// One source of truth for the name: a hardcoded `"nadir-artifacts"` would survive a rename and
/// then be wrong in every report that prints it.
pub fn crate_name() -> &'static str {
    env!("CARGO_PKG_NAME")
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn describe_names_the_crate_and_its_stage() {
        let description = describe();
        assert!(
            description.contains(crate_name()),
            "expected the crate name in {description:?}"
        );
        assert!(
            description.contains(STAGE),
            "expected the stage name in {description:?}"
        );
    }

    #[test]
    fn the_stage_is_not_empty() {
        // A crate whose STAGE was emptied by a bad merge still compiles; this is the test
        // that notices.
        assert!(!STAGE.is_empty(), "STAGE must name a pipeline stage");
    }
}
