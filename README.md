# nadir

Artifact-driven photogrammetry pipeline engine.

> **Status: scaffold.** The binary links, parses and reports: `nadir engines`, `plan`,
> `crates` and a counting `inspect` work; `process` fails on purpose, and `serve` does not
> exist yet. Every crate is one constant, one function and two tests — plus a drafted type
> vocabulary in `crates/core/src/` that `lib.rs` does not declare yet, so it is not
> compiled. What is finished is the *build*: the lockfiles, the task graph, the gates, the
> offline transport and the publish pipeline that the real implementation will be written
> into.

Nadir turns overlapping drone photographs into georeferenced orthomosaics, DSMs, DTMs,
point clouds and 3D meshes by orchestrating external engines — COLMAP, OpenMVS, GDAL, PROJ,
PDAL — through a declarative DAG with content-addressed caching.

## What is here

| Layer | Path | What it is |
| --- | --- | --- |
| CLI | `crates/cli/` | the `nadir` binary's sources; its manifest is the repository root |
| Rust crates | `crates/*/` | ten workspace members, one directory each |
| TypeScript | `packages/sdk`, `packages/client` | the wire-contract client and the browser layer over it |
| Python | `python/nadir/` | pure-Python SDK for notebooks, CI jobs and web backends |
| Backlog | `backlog/` | the V0.1 work as Markdown tasks (nd-1 … nd-14, milestone m-0) |
| Knowledge | `.knowledge/` | the ADRs, architecture notes, dependency matrix and roadmap |
| Tooling | `pixi.toml`, `turbo.json`, `biome.json`, `deny.toml`, `lefthook.yml` | one source of truth per kind of thing |

The repository root is both the Cargo workspace manifest and the `nadir-cli` package, and
`pixi.toml` is both the pixi workspace manifest and the `nadir-cli` conda package. Both
merges exist for the same reason: `pixi-build-rust` builds a package with
`cargo install --locked --path <dir>`, and `cargo install` cannot install from a virtual
manifest (ADR-009). `crates/cli/` therefore has no manifest of its own — it is reached
through `[[bin]] path` — and is excluded from the `crates/*` members glob.

What the crates will own:

| Crate | Owns |
| --- | --- |
| `nadir-core` | the shared vocabulary: engine traits, artifact identity, pipeline types |
| `nadir-artifacts` | content-addressed artifact storage and the invalidation rules it makes possible |
| `nadir-pipeline` | declarative TOML pipelines and the DAG they validate into |
| `nadir-executor` | topological execution with concurrent branches and resumable runs |
| `nadir-dataset` | image ingest, EXIF, GPS and camera models |
| `nadir-reconstruction` | COLMAP and OpenMVS adapters, one focused subprocess per stage |
| `nadir-geometry` | CRS transforms and the Umeyama georeferencing fit from ground control points |
| `nadir-surface` | point-cloud filtering, classification, DSM and DTM generation |
| `nadir-cartography` | orthorectification, mosaicking and Cloud-Optimized GeoTIFF output |
| `nadir-math` | pure-Rust linear algebra and interpolation (the wasm32 target of ADR-008) |

Three files in `crates/core/src/` are drafted ahead of the rest: `engine.rs` (the sixteen
engine traits of ADR-007, capabilities, progress, errors), `artifact.rs` (blake3 content
addressing and the artifact store) and `pipeline.rs` (task declarations, DAG validation,
execution order). They are written but not wired — `lib.rs` declares none of them, so they
are neither compiled nor tested, and the dependencies they need are not in `Cargo.lock`
yet. Wiring them in is the first stretch of the V0.1 backlog (nd-1, nd-2, nd-10, nd-11).

The pipeline stages those crates serve, in the order `nadir plan` prints them:
`ingest, features, matching, sfm, georef, dense, dsm, dtm, ortho`.

## Get set up

Pixi is the only prerequisite. It provisions Rust, Bun, Python, the geospatial engines and
every task runner from `pixi.lock`.

```sh
curl -fsSL https://pixi.sh/install.sh | bash
pixi install --locked
pixi run setup
```

`pixi.toml` requires pixi ≥ 0.81.0 and records one environment, `default`, for `linux-64`
only. There is no Node.js and no pnpm anywhere: Bun is the JavaScript runtime, package
manager and test runner, and the reason is a recorded decision (ADR-010), restated in
`pixi.toml`.

Or open the repository in a devcontainer and wait — `.devcontainer/devcontainer.json` runs
the two commands above and then reports what it found.

## Use it

```sh
pixi run gates          # every check that must pass before a commit lands
pixi run ci             # gates plus the build
pixi run build          # the binary and every package
pixi run test           # 26 Rust tests, 2 pytest, 2 bun test
pixi run lint           # clippy + cargo-deny + ruff + biome
pixi run fmt            # rewrite with each formatter
pixi run advisories     # RustSec + licence scan; needs the network, so not in gates
```

`gates` is `lint`, `typecheck`, `test`, `lint-actions` (actionlint), `version-check` and
`publish-plan`. The CI workflow adds to it the build, a smoke test of the release binary
installed the way `pixi publish` installs it, a validation of the sandbox publish plan, and
the network-dependent advisory scan as its own job.

The CLI itself:

```sh
cargo run -- engines    # which external engines are on PATH, at which versions
cargo run -- plan       # the nine stages in order; the graph is V0.1, this is an ordering
cargo run -- inspect D  # count the images in a dataset directory
cargo run -- crates     # every workspace crate and the stage it claims
cargo run -- process D  # exit 1 on purpose, until the V0.1 pipeline lands
```

`engines` in a working environment, real output:

```
Engines:
  colmap           COLMAP 3.13.0 -- Structure-from-Motion and Multi-View Stereo
  pdal             pdal 2.9.3 (git-version: fa4ad9)
  gdal             GDAL 3.12.2 "Chicoutimi", released 2026/02/03
  proj             Rel. 9.7.1, December 1st, 2025
  opencv_version   Usage: opencv_version [params]
  openmvs          no conda-forge package; see `pixi run build-openmvs`
```

## No network? Use the sandbox

A packed transport lives on the `sandbox/developer-linux-64` orphan branch of this
repository, repacked and pushed by CI on every push to `main`. On a machine with no
network at all:

```sh
bash scripts/restore.sh        # scripts/restore.ps1 on Windows
```

It verifies every blob against the manifest and unpacks both the environment and the
vendored crate sources (`cargo_vendor = true`), so the workspace builds offline and not
merely runs. A restore today materialises about 1.2 GiB — the `default` environment plus
54 vendored crates — writes `.cargo/config.toml` pointing cargo at them, and after it,
`pixi install --frozen --offline` is a no-op and `cargo build --offline` works. Locally:

```sh
pixi run sandbox-pack      # build a transport
pixi run sandbox-doctor    # verify it without writing anything
pixi run lint-sandbox-plan # check the publish plan (a CI job, not a gate)
```

Never run `sandbox-publish` without reading the plan first. It writes a branch to `origin`.

## Publishing

`pixi.toml` doubles as the `nadir-cli` conda package manifest, so `pixi publish` builds
the binary with the same pinned toolchain the lockfile gives a developer. Two tasks, and
the split is the point: `publish-plan` resolves and validates the publish set without
building (it is a gate), and `publish-dist` builds the `.conda` archives into `dist/`.
Neither uploads — nothing reaches a channel without an explicit `--channel`, which no task
passes. Publishing is a deliberate, separate act.

## OpenMVS

There is no conda-forge package for OpenMVS, and a manifest naming one fails to solve. It
is therefore not a dependency of anything; `pixi run build-openmvs` compiles it from source
into the environment. A binary installed that way is only offline-restorable if a
`sandbox-pack` ran afterwards.

## Working on it

Read `AGENTS.md` before changing the layout — most of the arrangement is a consequence of
one constraint (`cargo install` cannot install from a virtual manifest) or one source of
truth per kind of thing, and both are easy to break by accident.

The work is tracked in `backlog/` — `pixi run backlog task list --plain`. Milestone m-0 is
V0.1, the minimum viable pipeline: ingest, COLMAP, georeferencing, a DSM and a
cloud-optimized orthomosaic, driven by a cached DAG, ending with `nadir process` doing the
work instead of exiting 1. `.knowledge/` holds the architecture decisions (ADR-001 …
ADR-010), the dependency matrix and the roadmap.
