//! Repository-local automation for the Rust workspace.
//!
//! The public entrypoint is the Pixi task:
//!
//! ```text
//! pixi run xtask <command> [args]
//! ```
//!
//! Package scripts call the same CLI, so CI, hooks and local runs share one command surface
//! instead of growing separate shell snippets for Cargo, cargo-nextest, cargo-deny and the
//! native SDK copy step.

use std::ffi::OsString;
use std::path::{Path, PathBuf};
use std::process::{Command, ExitStatus};

use anyhow::{Context, Result, bail};
use clap::{Args, Parser, Subcommand, ValueEnum};

fn main() -> Result<()> {
    let cli = Cli::parse();
    let root = workspace_root();
    cli.command.run(&root)
}

#[derive(Debug, Parser)]
#[command(
    name = "xtask",
    version,
    about = "Repository-local automation for the nadir Rust workspace",
    long_about = "Repository-local automation for the nadir Rust workspace.\n\n\
                  Run through Pixi so the Cargo toolchain and cargo-* helpers come from \
                  pixi.lock:\n\n  pixi run xtask test --package nadir-core"
)]
struct Cli {
    #[command(subcommand)]
    command: Xtask,
}

#[derive(Debug, Subcommand)]
enum Xtask {
    /// Build a Cargo package or the whole workspace.
    Build(Build),
    /// Type-check a Cargo package or the whole workspace.
    Check(Check),
    /// Run cargo-nextest, optionally followed by doctests.
    Test(Test),
    /// Run cargo clippy with this workspace's warning policy.
    Clippy(Clippy),
    /// Run rustfmt, either checking or rewriting.
    Fmt(Fmt),
    /// Run cargo-deny checks.
    Deny(Deny),
    /// Generate Rust coverage with cargo-llvm-cov and cargo-nextest.
    Coverage(Coverage),
    /// Build Rust documentation.
    Doc(Doc),
    /// Native SDK adapter operations.
    Native(Native),
}

impl Xtask {
    fn run(self, root: &Path) -> Result<()> {
        match self {
            Self::Build(args) => args.run(root),
            Self::Check(args) => args.run(root),
            Self::Test(args) => args.run(root),
            Self::Clippy(args) => args.run(root),
            Self::Fmt(args) => args.run(root),
            Self::Deny(args) => args.run(root),
            Self::Coverage(args) => args.run(root),
            Self::Doc(args) => args.run(root),
            Self::Native(args) => args.run(root),
        }
    }
}

#[derive(Debug, Args)]
struct Scope {
    /// Cargo package to operate on. Defaults to --workspace when omitted.
    #[arg(
        short = 'p',
        long = "package",
        value_name = "PACKAGE",
        conflicts_with = "workspace"
    )]
    package: Option<String>,

    /// Operate on the whole Cargo workspace. This is the default when --package is omitted.
    #[arg(long, conflicts_with = "package")]
    workspace: bool,
}

impl Scope {
    fn cargo_args(&self) -> Vec<OsString> {
        match (&self.package, self.workspace) {
            (Some(package), _) => vec!["--package".into(), package.into()],
            (None, true | false) => vec!["--workspace".into()],
        }
    }
}

#[derive(Debug, Args)]
struct Build {
    #[command(flatten)]
    scope: Scope,

    /// Build optimized artifacts.
    #[arg(long)]
    release: bool,

    /// Additional arguments forwarded to `cargo build` after `--`.
    #[arg(last = true)]
    cargo_args: Vec<OsString>,
}

impl Build {
    fn run(self, root: &Path) -> Result<()> {
        let mut args = vec![OsString::from("build")];
        args.extend(self.scope.cargo_args());
        if self.release {
            args.push("--release".into());
        }
        args.extend(self.cargo_args);
        run(root, "cargo", args)
    }
}

#[derive(Debug, Args)]
struct Check {
    #[command(flatten)]
    scope: Scope,

    /// Check tests, benches and examples too.
    #[arg(long)]
    all_targets: bool,

    /// Additional arguments forwarded to `cargo check` after `--`.
    #[arg(last = true)]
    cargo_args: Vec<OsString>,
}

impl Check {
    fn run(self, root: &Path) -> Result<()> {
        let mut args = vec![OsString::from("check")];
        args.extend(self.scope.cargo_args());
        if self.all_targets {
            args.push("--all-targets".into());
        }
        args.extend(self.cargo_args);
        run(root, "cargo", args)
    }
}

#[derive(Debug, Args)]
struct Test {
    #[command(flatten)]
    scope: Scope,

    /// Skip doctests and run only cargo-nextest.
    #[arg(long)]
    no_doc: bool,

    /// Run only doctests and skip cargo-nextest.
    #[arg(long, conflicts_with = "no_doc")]
    doc_only: bool,

    /// Extra arguments forwarded to `cargo nextest run` after `--`.
    #[arg(last = true)]
    nextest_args: Vec<OsString>,
}

impl Test {
    fn run(self, root: &Path) -> Result<()> {
        if !self.doc_only {
            let mut nextest = vec![OsString::from("nextest"), OsString::from("run")];
            nextest.extend(self.scope.cargo_args());
            nextest.extend(self.nextest_args);
            run(root, "cargo", nextest)?;
        }

        if !self.no_doc {
            let mut doctest = vec![OsString::from("test"), OsString::from("--doc")];
            doctest.extend(self.scope.cargo_args());
            run(root, "cargo", doctest)?;
        }

        Ok(())
    }
}

#[derive(Debug, Args)]
struct Clippy {
    #[command(flatten)]
    scope: Scope,

    /// Lint tests, benches and examples too.
    #[arg(long, default_value_t = true, action = clap::ArgAction::Set)]
    all_targets: bool,

    /// Do not append `-D warnings` to clippy's lint arguments.
    #[arg(long)]
    allow_warnings: bool,
}

impl Clippy {
    fn run(self, root: &Path) -> Result<()> {
        let mut args = vec![OsString::from("clippy")];
        args.extend(self.scope.cargo_args());
        if self.all_targets {
            args.push("--all-targets".into());
        }
        if !self.allow_warnings {
            args.push("--".into());
            args.push("-D".into());
            args.push("warnings".into());
        }
        run(root, "cargo", args)
    }
}

#[derive(Debug, Args)]
struct Fmt {
    /// Check formatting without rewriting files.
    #[arg(long)]
    check: bool,
}

impl Fmt {
    fn run(self, root: &Path) -> Result<()> {
        let mut args = vec![OsString::from("fmt"), OsString::from("--all")];
        if self.check {
            args.push("--".into());
            args.push("--check".into());
        }
        run(root, "cargo", args)
    }
}

#[derive(Debug, Clone, ValueEnum)]
enum DenyCheck {
    Advisories,
    Bans,
    Licenses,
    Sources,
}

impl DenyCheck {
    fn as_str(&self) -> &'static str {
        match self {
            Self::Advisories => "advisories",
            Self::Bans => "bans",
            Self::Licenses => "licenses",
            Self::Sources => "sources",
        }
    }
}

#[derive(Debug, Args)]
struct Deny {
    /// cargo-deny checks to run. Defaults to the offline checks used by lint.
    #[arg(value_enum, value_name = "CHECK")]
    checks: Vec<DenyCheck>,
}

impl Deny {
    fn run(self, root: &Path) -> Result<()> {
        let checks = if self.checks.is_empty() {
            vec![DenyCheck::Bans, DenyCheck::Licenses, DenyCheck::Sources]
        } else {
            self.checks
        };

        let mut args = vec![
            OsString::from("deny"),
            OsString::from("--manifest-path"),
            root.join("Cargo.toml").into_os_string(),
            OsString::from("--all-features"),
            OsString::from("check"),
        ];
        args.extend(checks.iter().map(|check| OsString::from(check.as_str())));
        run(root, "cargo", args)
    }
}

#[derive(Debug, Args)]
struct Coverage {
    #[command(flatten)]
    scope: Scope,

    /// LCOV output path, relative to the workspace root unless absolute.
    #[arg(long, default_value = "lcov.info")]
    output_path: PathBuf,
}

impl Coverage {
    fn run(self, root: &Path) -> Result<()> {
        let output_path = absolutize(root, &self.output_path);
        let mut args = vec![OsString::from("llvm-cov"), OsString::from("nextest")];
        args.extend(self.scope.cargo_args());
        args.extend([
            OsString::from("--lcov"),
            OsString::from("--output-path"),
            output_path.into_os_string(),
        ]);
        run(root, "cargo", args)
    }
}

#[derive(Debug, Args)]
struct Doc {
    #[command(flatten)]
    scope: Scope,

    /// Do not build documentation for dependencies.
    #[arg(long, default_value_t = true, action = clap::ArgAction::Set)]
    no_deps: bool,
}

impl Doc {
    fn run(self, root: &Path) -> Result<()> {
        let mut args = vec![OsString::from("doc")];
        args.extend(self.scope.cargo_args());
        if self.no_deps {
            args.push("--no-deps".into());
        }
        run(root, "cargo", args)
    }
}

#[derive(Debug, Args)]
struct Native {
    #[command(subcommand)]
    command: NativeCommand,
}

impl Native {
    fn run(self, root: &Path) -> Result<()> {
        match self.command {
            NativeCommand::Node(args) => args.run(root),
            NativeCommand::Python(args) => args.run(root),
        }
    }
}

#[derive(Debug, Subcommand)]
enum NativeCommand {
    /// Build and copy the N-API shared library to packages/sdk/*.node.
    Node(NodeNative),
    /// Build or install the PyO3 Python extension with maturin.
    Python(PythonNative),
}

#[derive(Debug, Args)]
struct NodeNative {
    /// Build the native addon with Cargo's release profile.
    #[arg(long)]
    release: bool,

    /// Output .node path, relative to the workspace root unless absolute.
    #[arg(
        long,
        default_value = "packages/sdk/nadir-node-native.linux-x64-gnu.node"
    )]
    output: PathBuf,
}

impl NodeNative {
    fn run(self, root: &Path) -> Result<()> {
        let mut build = vec![
            OsString::from("build"),
            OsString::from("--package"),
            OsString::from("nadir-node-native"),
        ];
        if self.release {
            build.push("--release".into());
        }
        run(root, "cargo", build)?;

        let profile = if self.release { "release" } else { "debug" };
        let source = root
            .join("target")
            .join(profile)
            .join(dynamic_library_name("nadir_node_native"));
        let output = absolutize(root, &self.output);
        if let Some(parent) = output.parent() {
            std::fs::create_dir_all(parent)
                .with_context(|| format!("create {}", parent.display()))?;
        }
        std::fs::copy(&source, &output).with_context(|| {
            format!(
                "copy native addon from {} to {}",
                source.display(),
                output.display()
            )
        })?;
        println!("{} -> {}", source.display(), output.display());
        Ok(())
    }
}

#[derive(Debug, Args)]
struct PythonNative {
    #[command(subcommand)]
    command: PythonNativeCommand,
}

impl PythonNative {
    fn run(self, root: &Path) -> Result<()> {
        match self.command {
            PythonNativeCommand::Build(args) => args.run(root),
            PythonNativeCommand::Develop(args) => args.run(root),
        }
    }
}

#[derive(Debug, Subcommand)]
enum PythonNativeCommand {
    /// Build the distributable Python wheel into dist/pypi.
    Build(PythonBuild),
    /// Install the Python extension editable into Pixi's interpreter.
    Develop(PythonDevelop),
}

#[derive(Debug, Args)]
struct PythonBuild {
    /// Build without Cargo's release profile.
    #[arg(long)]
    debug: bool,

    /// Wheel output directory, relative to the workspace root unless absolute.
    #[arg(long, default_value = "dist/pypi")]
    out: PathBuf,
}

impl PythonBuild {
    fn run(self, root: &Path) -> Result<()> {
        let out = absolutize(root, &self.out);
        let mut args = vec![
            OsString::from("build"),
            OsString::from("--features"),
            OsString::from("extension-module"),
            OsString::from("--out"),
            out.into_os_string(),
        ];
        if !self.debug {
            args.insert(1, OsString::from("--release"));
        }
        run_in(&root.join("python/nadir"), "maturin", args)
    }
}

#[derive(Debug, Args)]
struct PythonDevelop {
    /// Install the editable extension with Cargo's release profile.
    #[arg(long)]
    release: bool,
}

impl PythonDevelop {
    fn run(self, root: &Path) -> Result<()> {
        let mut args = vec![
            OsString::from("develop"),
            OsString::from("--features"),
            OsString::from("extension-module"),
        ];
        if self.release {
            args.push("--release".into());
        }
        run_in(&root.join("python/nadir"), "maturin", args)
    }
}

fn workspace_root() -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR"))
        .ancestors()
        .nth(2)
        .expect("xtask lives at crates/xtask")
        .to_path_buf()
}

fn absolutize(root: &Path, path: &Path) -> PathBuf {
    if path.is_absolute() {
        path.to_path_buf()
    } else {
        root.join(path)
    }
}

fn dynamic_library_name(stem: &str) -> String {
    let prefix = std::env::consts::DLL_PREFIX;
    let suffix = std::env::consts::DLL_SUFFIX;
    format!("{prefix}{stem}{suffix}")
}

fn run<I, S>(root: &Path, program: &str, args: I) -> Result<()>
where
    I: IntoIterator<Item = S>,
    S: Into<OsString>,
{
    run_in(root, program, args)
}

fn run_in<I, S>(cwd: &Path, program: &str, args: I) -> Result<()>
where
    I: IntoIterator<Item = S>,
    S: Into<OsString>,
{
    let args = args.into_iter().map(Into::into).collect::<Vec<_>>();
    print_command(program, &args);
    let status = Command::new(program)
        .args(&args)
        .current_dir(cwd)
        .status()
        .with_context(|| format!("spawn {program}"))?;
    ensure_success(program, status)
}

fn print_command(program: &str, args: &[OsString]) {
    eprintln!("$ {}{}", program, shellish_args(args));
}

fn shellish_args(args: &[OsString]) -> String {
    args.iter()
        .map(|arg| format!(" {}", arg.to_string_lossy()))
        .collect::<String>()
}

fn ensure_success(program: &str, status: ExitStatus) -> Result<()> {
    if status.success() {
        Ok(())
    } else {
        match status.code() {
            Some(code) => bail!("{program} exited with status {code}"),
            None => bail!("{program} was terminated by a signal"),
        }
    }
}
