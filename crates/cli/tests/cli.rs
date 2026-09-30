//! Integration tests for the `nadir` binary.
//!
//! These run the built executable through `CARGO_BIN_EXE_nadir` rather than calling into
//! `main.rs`. That path is the only one that exercises argument parsing, the exit-code
//! contract and the log setup together — the parts a wrapper script actually depends on, and
//! the parts an in-process call cannot see.

use std::path::Path;
use std::process::Command;

/// Run the binary and return its exit code and stdout.
fn nadir(args: &[&str]) -> (i32, String) {
    let output = Command::new(env!("CARGO_BIN_EXE_nadir"))
        .args(args)
        .output()
        .expect("cargo builds the binary before running its integration tests");

    let stdout = String::from_utf8_lossy(&output.stdout).into_owned();
    assert!(
        output.status.success() || output.status.code().is_some(),
        "the CLI should exit with a code, not be killed by a signal"
    );

    (output.status.code().unwrap_or(-1), stdout)
}

#[test]
fn engines_reports_without_failing() {
    // A missing engine is the answer this subcommand exists to give, so it must not exit
    // non-zero even on a machine with nothing installed.
    let (code, stdout) = nadir(&["engines"]);

    assert_eq!(code, 0, "`nadir engines` should always succeed");
    assert!(stdout.contains("colmap"), "COLMAP should be listed");
    assert!(
        stdout.contains("openmvs"),
        "OpenMVS should be noted as unpackageable, not omitted silently"
    );
}

#[test]
fn plan_lists_the_stages_in_order() {
    let (code, stdout) = nadir(&["plan"]);

    assert_eq!(code, 0, "`nadir plan` should always succeed");

    // The order is the contract, not the individual names: a reader checking this is asking
    // "does a point cloud come before the DSM built from it".
    let ingest = stdout.find("ingest").expect("ingest stage listed");
    let sfm = stdout.find("sfm").expect("sfm stage listed");
    let dsm = stdout.find("dsm").expect("dsm stage listed");
    let ortho = stdout.find("ortho").expect("ortho stage listed");

    assert!(ingest < sfm, "ingest must precede sfm");
    assert!(sfm < dsm, "sfm must precede dsm");
    assert!(dsm < ortho, "dsm must precede ortho");
}

#[test]
fn crates_lists_every_workspace_crate() {
    let (code, stdout) = nadir(&["crates"]);

    assert_eq!(code, 0, "`nadir crates` should always succeed");
    for expected in ["nadir-core", "nadir-cartography", "nadir-math"] {
        assert!(
            stdout.contains(expected),
            "{expected} missing from:\n{stdout}"
        );
    }
}

#[test]
fn inspect_counts_the_images_in_a_directory() {
    // A real directory with real files, so the extension filter is exercised rather than
    // assumed. `tempfile` is a dev-dependency: it removes the directory on drop, including
    // when an assertion panics, which a hand-rolled temp dir would leak.
    let dir = tempfile::tempdir().expect("a temporary directory");
    for name in ["a.jpg", "b.JPG", "c.tiff", "notes.txt"] {
        std::fs::write(dir.path().join(name), b"not a real image").expect("write a fixture file");
    }

    let (code, stdout) = nadir(&["inspect", dir.path().to_str().expect("utf-8 path")]);

    assert_eq!(
        code, 0,
        "`nadir inspect` should succeed on a readable directory"
    );
    assert!(
        stdout.contains("Images:   3"),
        "expected 3 images (case-insensitive extension), got:\n{stdout}"
    );
}

#[test]
fn inspect_fails_on_a_missing_directory() {
    let missing = Path::new("/nonexistent-nadir-scaffold-dataset");
    assert!(!missing.exists(), "the fixture path must not exist");

    let (code, _) = nadir(&["inspect", missing.to_str().expect("utf-8 path")]);

    assert_eq!(
        code, 1,
        "a missing directory is an ordinary failure, exit code 1"
    );
}

#[test]
fn process_fails_loudly_rather_than_succeeding_with_nothing() {
    // The one behaviour worth pinning down: a scaffold that exits 0 without producing a
    // point cloud would let a CI job or a script believe the work happened.
    let (code, _) = nadir(&["process", "/tmp"]);

    assert_eq!(
        code, 1,
        "`nadir process` is not built yet and must report failure"
    );
}
