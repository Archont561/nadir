//! The `nadir` binary: argument parsing, messages, exit codes.
//!
//! This is the binary's crate, and it has no `Cargo.toml` of its own — the repository root
//! is its manifest, because that file is also the workspace root. The header of
//! `../../Cargo.toml` explains why; the short version is that `pixi-build-rust` runs
//! `cargo install --path <root>`, which cannot install a virtual manifest.
//!
//! **Scaffold.** The subcommands exist so the binary links, parses arguments and reports
//! something true about the machine. `process` deliberately fails rather than pretending:
//! a scaffold that exits 0 without producing a point cloud is worse than one that says it
//! is not built yet.

use std::path::PathBuf;
use std::process::ExitCode;

use clap::{Parser, Subcommand};

mod engine_status;

/// The exit codes the CLI contract fixes (`.knowledge/context.md`).
///
/// These numbers are an interface. A wrapper script or a worker that shells out to `nadir`
/// branches on them, so changing one is a breaking change — which is why they are named
/// constants in their own module rather than bare `1`s at the return sites.
///
/// A third code is reserved for "a required external engine is not on PATH", separate from
/// [`FAILURE`] so an operator can tell "COLMAP is not installed here" from "the pipeline
/// failed" without pattern-matching stderr. It has no call site yet — the first subcommand
/// that shells out to COLMAP is the first to need it, and a constant with no caller is a
/// lie about what the CLI does.
mod exit {
    /// The command succeeded.
    pub const OK: u8 = 0;
    /// The command failed for an expected reason: bad input, a missing file, not built yet.
    pub const FAILURE: u8 = 1;
}

#[derive(Debug, Parser)]
#[command(
    name = "nadir",
    version,
    about = "Artifact-driven photogrammetry pipeline engine",
    long_about = "Nadir turns overlapping drone photographs into georeferenced orthomosaics, \
                  DSMs, DTMs, point clouds and 3D meshes by orchestrating external engines \
                  through a declarative DAG with content-addressed caching."
)]
struct Cli {
    #[command(subcommand)]
    command: Command,
}

#[derive(Debug, Subcommand)]
enum Command {
    /// Process a dataset through a pipeline.
    Process {
        /// Directory of overlapping input images.
        images: PathBuf,
        /// Pipeline definition; defaults to the built-in standard pipeline.
        #[arg(short, long)]
        pipeline: Option<PathBuf>,
    },
    /// Inspect a dataset and report its geometry before processing it.
    Inspect {
        /// Directory of input images to inspect.
        images: PathBuf,
    },
    /// Print the pipeline stages without executing them.
    Plan {
        /// Pipeline definition; defaults to the built-in standard pipeline.
        #[arg(short, long)]
        pipeline: Option<PathBuf>,
    },
    /// Report which external engines are installed, and at which versions.
    Engines,
    /// List the workspace crates and the pipeline stage each one implements.
    Crates,
}

fn main() -> ExitCode {
    // `RUST_LOG` wins if set; the default keeps `tracing::error!` visible for the failure
    // paths and quiet for everything else, so a working run prints nothing but its result.
    tracing_subscriber::fmt()
        .with_env_filter(
            tracing_subscriber::EnvFilter::try_from_default_env()
                .unwrap_or_else(|_| "nadir=info,warn".into()),
        )
        .init();

    match run(Cli::parse().command) {
        Ok(()) => ExitCode::from(exit::OK),
        Err(err) => {
            tracing::error!("{err}");
            ExitCode::from(err.code)
        }
    }
}

/// The failure every subcommand returns, carrying the exit code with it.
///
/// The code travels on the error rather than beside it because a subcommand knows whether
/// its failure is operational, and only `main` gets to decide what that means to a caller.
#[derive(Debug, thiserror::Error)]
#[error("{message}")]
struct CliError {
    message: String,
    code: u8,
}

impl CliError {
    /// An ordinary failure: bad input, a missing file, a stage that is not built.
    fn new(message: impl Into<String>) -> Self {
        Self {
            message: message.into(),
            code: exit::FAILURE,
        }
    }
}

fn run(command: Command) -> Result<(), CliError> {
    match command {
        Command::Process { images, pipeline } => {
            let source = pipeline.as_ref().map_or_else(
                || "<built-in standard>".to_owned(),
                |p| p.display().to_string(),
            );
            Err(CliError::new(format!(
                "`nadir process` is not implemented. Input: {}, pipeline: {source}\n\
                 The V0.1 ingest pipeline (`.knowledge/roadmap/v0-mvp.md`) is the first thing \
                 that makes this command do work.",
                images.display()
            )))
        }
        Command::Inspect { images } => {
            println!("{}", inspect(&images).map_err(CliError::new)?);
            Ok(())
        }
        Command::Plan { pipeline } => {
            print!("{}", engine_status::plan(&pipeline));
            Ok(())
        }
        Command::Engines => {
            print!("{}", engine_status::report());
            Ok(())
        }
        Command::Crates => {
            print!("{}", crates());
            Ok(())
        }
    }
}

/// List the workspace crates and the stage each claims.
///
/// Reads the stage from each crate rather than keeping a second list here, so a crate whose
/// `STAGE` changed cannot be reported here under the old name.
fn crates() -> String {
    let rows: &[(&str, &str)] = &[
        ("nadir-core", nadir_core::STAGE),
        ("nadir-artifacts", nadir_artifacts::STAGE),
        ("nadir-pipeline", nadir_pipeline::STAGE),
        ("nadir-executor", nadir_executor::STAGE),
        ("nadir-dataset", nadir_dataset::STAGE),
        ("nadir-reconstruction", nadir_reconstruction::STAGE),
        ("nadir-geometry", nadir_geometry::STAGE),
        ("nadir-surface", nadir_surface::STAGE),
        ("nadir-cartography", nadir_cartography::STAGE),
        ("nadir-math", nadir_math::STAGE),
    ];

    let mut out = String::from("Crates:\n");
    for (name, stage) in rows {
        out.push_str(&format!("  {name:<22} {stage}\n"));
    }
    out
}

/// Count the images in a dataset directory.
///
/// Deliberately counts and stops there. Reporting a GSD or a GPS coverage rate means reading
/// EXIF and a projection, and a number computed from EXIF that is wrong is worse than a
/// number that is absent — so that is what the V0.1 dataset pass is for.
fn inspect(images: &std::path::Path) -> Result<String, String> {
    let entries = std::fs::read_dir(images)
        .map_err(|err| format!("cannot read {}: {err}", images.display()))?;

    let mut count = 0usize;
    for entry in entries {
        let entry = entry.map_err(|err| format!("cannot read {}: {err}", images.display()))?;
        let path = entry.path();
        let is_image = path
            .extension()
            .and_then(std::ffi::OsStr::to_str)
            .is_some_and(|ext| {
                matches!(
                    ext.to_ascii_lowercase().as_str(),
                    "jpg" | "jpeg" | "tif" | "tiff" | "png"
                )
            });
        if is_image && path.is_file() {
            count += 1;
        }
    }

    Ok(format!(
        "Images:   {count}\nDirectory: {}\nIssues:   GPS coverage, camera model and GSD are \
         not reported yet",
        images.display()
    ))
}
