//! `nadir engines` and `nadir plan`: the two subcommands that answer questions about the
//! environment rather than about a dataset.
//!
//! Split out of `main.rs` because this is where the tier-3 boundary sits. Every external
//! engine is a subprocess on PATH, so "will nadir work on this machine" is answerable
//! before any dataset exists — and answering it up front is what keeps a missing COLMAP
//! from being discovered halfway through a forty-minute run.
//!
//! The PATH walk is hand-rolled rather than delegated to a `which` crate: it is fifteen
//! lines, it has no dependencies, and `which`'s behaviour on Windows (`PATHEXT`, `.exe`)
//! is more than this needs.

use std::path::PathBuf;
use std::process::Command;

/// The external engines the pipeline shells out to, and the flags that report a version.
///
/// COLMAP and PDAL come first: those two are the ones whose absence blocks V0.1 outright,
/// so a report that lists them first answers the question a user actually has.
///
/// The flags are not uniform, which is the reason each engine lists more than one. COLMAP
/// has no `--version` at all — `colmap --version` prints an error about an unrecognised
/// command and exits non-zero, so passing it would report a help message as a version. Its
/// version is on the first line of `colmap -h`. PDAL prints a rule of dashes around its
/// version, so the first non-empty line is punctuation.
///
/// A flag is kept in the list only because it was checked: an engine whose first flag yields
/// nothing is tried against the next, and a version-shaped line wins over whatever came
/// first.
const ENGINES: &[(&str, &[&str])] = &[
    ("colmap", &["-h", "--help"]),
    ("pdal", &["--version", "--help"]),
    ("gdal", &["--version"]),
    ("proj", &["--version"]),
    ("opencv_version", &["--help"]),
];

/// Report which engines are installed and what versions they report.
///
/// Always succeeds. A missing engine is the information this subcommand exists to produce,
/// so a non-zero exit would answer a different question than the one asked.
pub fn report() -> String {
    let mut out = String::from("Engines:\n");
    for (name, flags) in ENGINES {
        match which(name) {
            None => out.push_str(&format!("  {name:<16} not found on PATH\n")),
            Some(path) => match flags.iter().find_map(|flag| version(&path, flag)) {
                Some(v) => out.push_str(&format!("  {name:<16} {v}\n")),
                None => out.push_str(&format!("  {name:<16} found at {}\n", path.display())),
            },
        }
    }
    out.push_str("  openmvs          no conda-forge package; see `pixi run build-openmvs`\n");
    out
}

/// Find `name` on PATH, returning the first executable match.
fn which(name: &str) -> Option<PathBuf> {
    let path = std::env::var_os("PATH")?;
    std::env::split_paths(&path)
        .map(|dir| dir.join(name))
        .find(|candidate| is_executable(candidate))
}

/// Whether a path is a file with an execute bit set.
fn is_executable(path: &std::path::Path) -> bool {
    let Ok(metadata) = std::fs::metadata(path) else {
        return false;
    };
    if !metadata.is_file() {
        return false;
    }
    #[cfg(unix)]
    {
        use std::os::unix::fs::PermissionsExt;
        metadata.permissions().mode() & 0o111 != 0
    }
    #[cfg(not(unix))]
    {
        // Windows has no execute bit; existence of the file is the whole test.
        true
    }
}

/// Run `<engine> <flag>` and return the line that states its version.
///
/// Two rules, both learned from engines that do not follow one convention:
///
///   - stdout *and* stderr are searched, because `proj --version` writes to stderr and
///     `gdal --version` writes to stdout. A version string is a version string wherever it
///     arrives.
///   - the line chosen is the first one containing a dotted number, not the first non-empty
///     one. `pdal --version` leads with a rule of dashes and `colmap -h` can lead with a
///     banner; both are followed within a line or two by the actual version, and a report
///     that prints a line of punctuation is a report nobody reads.
///
/// Falls back to the first non-empty line when nothing looks like a version, because "found
/// but could not parse" is better than silence.
fn version(path: &std::path::Path, flag: &str) -> Option<String> {
    let output = Command::new(path).arg(flag).output().ok()?;
    let stdout = String::from_utf8_lossy(&output.stdout);
    let stderr = String::from_utf8_lossy(&output.stderr);

    let lines: Vec<&str> = stdout
        .lines()
        .chain(stderr.lines())
        .map(str::trim)
        .filter(|line| !line.is_empty())
        .collect();

    lines
        .iter()
        .find(|line| has_dotted_number(line))
        .or_else(|| lines.first())
        .map(|line| (*line).to_owned())
}

/// Whether a line contains something shaped like a dotted version number.
fn has_dotted_number(line: &str) -> bool {
    let bytes = line.as_bytes();
    bytes.iter().enumerate().any(|(index, byte)| {
        byte.is_ascii_digit()
            && bytes.get(index + 1) == Some(&b'.')
            && bytes.get(index + 2).is_some_and(u8::is_ascii_digit)
    })
}

/// The pipeline stages, in dependency order, as the architecture fixes them.
const STAGES: &[&str] = &[
    "ingest", "features", "matching", "sfm", "georef", "dense", "dsm", "dtm", "ortho",
];

/// Print the pipeline DAG without executing it.
///
/// Prints the stage sequence and the source it came from, and nothing else: the DAG is
/// declared in V0.1 (`pipeline-dag.md`). Reporting the intended order now is useful, and
/// pretending the graph resolves would be worse than saying it does not exist.
pub fn plan(pipeline: &Option<PathBuf>) -> String {
    let source = pipeline.as_ref().map_or_else(
        || "<built-in standard>".to_owned(),
        |p| p.display().to_string(),
    );

    let mut out = format!("Pipeline: {source}\nStages:\n");
    for (index, stage) in STAGES.iter().enumerate() {
        out.push_str(&format!("  {:>2}. {stage}\n", index + 1));
    }
    out.push_str(
        "Note: the DAG is not resolved yet. Dependencies between stages are V0.1 \
         (pipeline-dag.md), so this list is an ordering, not a graph.\n",
    );
    out
}
