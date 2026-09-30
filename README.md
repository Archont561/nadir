# nadir

Artifact-driven photogrammetry pipeline engine.

> **Status: scaffold.** Every crate here is one constant, one function and two tests. The
> CLI parses arguments and reports the state of the machine; it does not reconstruct
> anything yet, and `nadir process` fails on purpose. What exists is the *build*: the
> lockfiles, the task graph, the gates and the offline transport that a real implementation
> will be written into.

Nadir turns overlapping drone photographs into georeferenced orthomosaics, DSMs, DTMs,
point clouds and 3D meshes by orchestrating external engines — COLMAP, OpenMVS, GDAL, PROJ,
PDAL — through a declarative DAG with content-addressed caching.

## What is here

| Layer | Path | What it is |
| --- | --- | --- |
| CLI | `crates/cli/` | the `nadir` binary; its manifest is the repository root |
| Rust crates | `crates/*/` | ten workspace members, one per pipeline stage |
| TypeScript | `packages/sdk`, `packages/client` | the API client and the browser-facing layer over it |
| Python | `python/nadir/` | the SDK for notebooks, CI jobs and web backends |
| Tooling | `pixi.toml`, `turbo.json`, `biome.json`, `deny.toml`, `lefthook.yml` | one source of truth per kind of thing |

The pipeline stages the crates cover, in order: `ingest`, `features`, `matching`, `sfm`,
`georef`, `dense`, `dsm`, `dtm`, `ortho`.

## Get set up

Pixi is the only prerequisite. It provisions Rust, Bun, Python, the geospatial engines and
every task runner from `pixi.lock`.

```sh
curl -fsSL https://pixi.sh/install.sh | bash
pixi install --locked
pixi run setup
```

There is no Node.js and no pnpm. Bun is the JavaScript runtime, package manager and test
runner; the reason is recorded in `pixi.toml`.

Or open the repository in a devcontainer and wait — `.devcontainer/devcontainer.json` runs
the two commands above and then reports what it found.

## Use it

```sh
pixi run gates          # everything CI runs
pixi run build          # the binary and every package
pixi run test           # 26 Rust tests, 2 pytest, 2 bun test
pixi run lint           # clippy + cargo-deny + ruff + biome
pixi run fmt            # rewrite with each formatter

cargo run -- engines    # which external engines are installed
cargo run -- plan       # the stage order (the DAG is not resolved yet)
cargo run -- crates     # every workspace crate and its stage
```

## No network? Use the sandbox

A packed transport lives on an orphan branch of this repository. On a machine with no
network at all:

```sh
bash scripts/restore.sh        # scripts/restore.ps1 on Windows
```

It verifies every blob against the manifest and unpacks both the environment and the
vendored crate sources, so the workspace builds offline and not merely runs. Locally:

```sh
pixi run sandbox-pack      # build a transport
pixi run sandbox-doctor    # verify it without writing anything
pixi run lint-sandbox-plan # check the publish plan (also in gates)
```

Never run `sandbox-publish` without reading the plan first. It writes a branch to `origin`.

## OpenMVS

There is no conda-forge package for OpenMVS, and a manifest naming one fails to solve. It
is therefore not a dependency of anything; `pixi run build-openmvs` compiles it from source
into the environment. A binary installed that way is only offline-restorable if a
`sandbox-pack` ran afterwards.

## Working on it

Read `AGENTS.md` before changing the layout — most of the arrangement is a consequence of
one constraint (`cargo install` cannot install from a virtual manifest) or one source of
truth per kind of thing, and both are easy to break by accident.

`.knowledge/` holds the architecture decisions, the dependency matrix and the roadmap.
