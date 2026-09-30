//! nadir-artifacts — Content-addressed artifact storage and the invalidation rules it makes possible.
//!
//! **Scaffold.** One constant and one function, so the crate compiles, links into the
//! workspace, and has a test that fails if the wiring is broken. The real content arrives
//! with the stage named by [`STAGE`].
//!
//! Declared as a workspace member with its own `Cargo.toml` rather than as a directory of
//! `.rs` files, because a crate that is not a member is not compiled by `cargo build
//! --workspace`, is not in `Cargo.lock`, and is not in the licence report `pixi run lint`
//! produces. Adding a crate is one file in its own directory; the workspace notices
//! (`members = ["crates/*"]` in the repository-root `Cargo.toml`).

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
