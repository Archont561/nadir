# nadir

<p align="center">
  <a href="https://github.com/Archont561/nadir/actions/workflows/ci.yml"><img src="https://github.com/Archont561/nadir/actions/workflows/ci.yml/badge.svg" alt="CI"></a>
  <a href="https://www.rust-lang.org"><img src="https://img.shields.io/badge/Rust-1.88%2B-orange.svg?logo=rust" alt="Rust 1.88+"></a>
  <a href="https://pixi.sh"><img src="https://img.shields.io/badge/Pixi-0.81%2B-yellow.svg?logo=condaforge" alt="Pixi 0.81+"></a>
  <img src="https://img.shields.io/badge/Python-3.13-blue.svg?logo=python" alt="Python 3.13">
  <img src="https://img.shields.io/badge/Platform-linux--64-brightgreen.svg" alt="Platform: linux-64">
  <img src="https://img.shields.io/badge/Status-early%20scaffold-yellow.svg" alt="Status: early scaffold">
  <a href="https://github.com/Archont561/nadir/pulls"><img src="https://img.shields.io/badge/PRs-welcome-brightgreen.svg" alt="PRs welcome"></a>
</p>

<p align="center">
  <strong>Artifact-driven photogrammetry — one Rust engine, native Python and TypeScript bindings, reproducible external tools.</strong><br/>
  Turn overlapping aerial images into georeferenced point clouds, meshes, elevation models, and orthomosaics.
</p>

---

> [!WARNING]
> **Nadir is an early scaffold, not a working photogrammetry processor yet.** The workspace,
> CLI shell, engine discovery, versioned FFI transport, PyO3 extension, and N-API addon compile
> and have contract tests. `nadir process` deliberately exits with an error until the V0.1
> execution pipeline can produce real artifacts.

## 🛰️ Pipeline Visualization

```text
                                      native interfaces
                    ┌──────────────────────────────────────────┐
                    │                                          │
       CLI ─────────┤                                          │
       Python/PyO3 ─┤  versioned JSON transport → dispatcher  ├──► Rust core
       TypeScript ──┤                                          │       │
       N-API         │                                          │       ▼
                    └──────────────────────────────────────────┘  artifact DAG
                                                                       │
           ┌────────────┬────────────────┬──────────┬─────────┬─────────┴───────┐
           ▼            ▼                ▼          ▼         ▼                 ▼
        ingest       features/SfM   georeference   dense    surfaces         products
         Rust           COLMAP       PROJ + Rust   OpenMVS  PDAL + Rust      GDAL + Rust
           │            │                │          │         │                 │
           └────────────┴────────────────┴──────────┴─────────┴─────────────────┘
                                      immutable artifacts
```

Rust owns orchestration, artifact identity, and domain rules. Focused external engines own the
specialized reconstruction and geospatial work. Each stage is intended to be independently
cacheable, replaceable, retryable, and schedulable rather than hidden inside one monolithic
subprocess.

The native SDKs do not reimplement the pipeline. Both pass one versioned JSON request through
`nadir-protocol` and `nadir-engine`, so adding an operation changes one Rust dispatcher instead
of creating a separate domain implementation for every language.

---

## 🚀 Design Goals

| Icon | Principle | What it means |
|------|-----------|---------------|
| 📦 | **Artifacts, not jobs** | Tasks transform immutable artifacts; provenance is part of every result |
| ⚡ | **Content-addressed work** | Inputs, normalized parameters, and engine versions determine cache identity |
| 🧩 | **Step-scoped engines** | COLMAP, OpenMVS, GDAL, PROJ, and PDAL can be replaced or retried stage by stage |
| 🦀 | **One Rust implementation** | CLI, Python, and TypeScript reach the same domain model and dispatcher |
| 🕸️ | **Declarative DAG** | Dependency ordering, incremental rebuilds, and resume follow from the graph |
| 🔒 | **Reproducible and portable** | Pixi locks the complete toolchain; pixi-sandbox can transport it to an airlock |

The project follows a build-system metaphor rather than a job-queue metaphor: every node is an
artifact, every edge is a transformation, and changing one input invalidates only its downstream
artifacts.

---

## ⚡ Quick Start

Nadir currently targets `linux-64`. [Pixi](https://pixi.sh) is the only host prerequisite; it
provides Rust, Bun, Python, C/C++ build tools, repository utilities, and the available
photogrammetry engines from one lockfile.

```bash
git clone https://github.com/Archont561/nadir.git
cd nadir

pixi install --locked
pixi run setup
```

Probe the scaffold:

```bash
pixi run --locked cargo run -- engines
pixi run --locked cargo run -- plan
pixi run --locked cargo run -- crates
pixi run --locked cargo run -- inspect ./images
```

`setup` installs the Bun workspace, builds the native Python package in editable mode, and
installs the repository's Git hooks.

> [!TIP]
> Open the repository in its dev container to start from the same pinned Pixi image and have
> Git, GitHub CLI, the C compiler, project environment, SDKs, hooks, and opencode configured
> automatically.

---

## 🖥️ Platform and Engine Support

The supported platform list comes from `[workspace].platforms` in `pixi.toml`; the published
offline bundle is separately reviewed in `pixi-sandbox.toml`.

| Surface | Status | Notes |
|---------|--------|-------|
| `linux-64` | ✅ locked and tested | The default development environment and sandbox bundle target Linux x86-64 |
| macOS / Windows | ⏳ not declared | No lockfile environment or CI proof is currently published for these platforms |
| COLMAP | ✅ locked | CPU build from conda-forge; CUDA is intentionally excluded from the default environment |
| GDAL / PROJ / PDAL / OpenCV | ✅ locked | Available to adapters through the default Pixi environment |
| OpenMVS | ⚠️ source build only | No `linux-64` conda-forge package exists; use `pixi run build-openmvs` |

The CLI reports what is actually available on the current machine:

```bash
pixi run --locked cargo run -- engines
```

---

## 📖 CLI

```text
nadir process <IMAGES> [--pipeline <FILE>]   process a dataset (not implemented yet)
nadir inspect <IMAGES>                      count supported image files in a directory
nadir plan [--pipeline <FILE>]               print the planned stage ordering
nadir engines                                report external engines and versions
nadir crates                                 list workspace crates and their stage ownership
```

Examples:

```bash
# Count jpg/jpeg/tif/tiff/png inputs without starting reconstruction
pixi run --locked cargo run -- inspect ./images

# Show the built-in pipeline plan
pixi run --locked cargo run -- plan

# This currently exits 1 and explains that execution has not landed
pixi run --locked cargo run -- process ./images
```

The explicit `process` failure is part of the current contract: a scaffold that exits zero
without producing a point cloud or raster would be more dangerous than one that says it is not
implemented.

---

## 🔌 Native Bindings

Both SDKs currently expose `ping`, `version`, and `describe`. These are small probes, but they
cross the real native boundary and return data from the shared Rust engine.

### Python

```python
import nadir

assert nadir.ping("python") == "python"
print(nadir.version())
print(nadir.describe())
```

The package under `python/nadir` is built by Maturin and backed by the PyO3 adapter in
`crates/python-native`.

### TypeScript

```ts
import { describe, ping, version } from "@nadir/sdk";

console.log(ping("typescript"));
console.log(version());
console.log(describe());
```

`@nadir/sdk` loads the N-API addon from `crates/node-native`; `@nadir/client` is the higher-level
convenience layer. Pipeline operations will grow on the same transport contract.

Read the [language-binding architecture](.knowledge/architecture/language-bindings.md) for the
boundary rules.

---

## 🏗️ Repository Architecture

| Path | Responsibility |
|------|----------------|
| `crates/core` | Shared domain vocabulary and engine traits |
| `crates/protocol` | Versioned JSON request and response envelopes |
| `crates/engine` | Shared operation dispatcher behind every native binding |
| `crates/python-native` | Thin PyO3 adapter |
| `crates/node-native` | Thin N-API adapter |
| `crates/artifacts` | Content-addressed artifact storage |
| `crates/pipeline` | Declarative pipeline DAG |
| `crates/executor` | Stage execution, scheduling, and resume |
| `crates/dataset` | Image ingest, EXIF, GPS, and camera models |
| `crates/reconstruction` | COLMAP/OpenMVS reconstruction stages |
| `crates/geometry` | CRS transforms, georeferencing, and GCPs |
| `crates/surface` | Point-cloud filtering, DSMs, and DTMs |
| `crates/cartography` | Orthorectification, mosaicking, and tiling |
| `crates/math` | Native numerical primitives |
| `crates/cli` | `nadir` command sources; its manifest is the repository root |
| `python/nadir` | Native Python SDK built with Maturin |
| `packages/sdk` | Native TypeScript SDK |
| `packages/client` | Higher-level TypeScript convenience client |
| `pixi.toml` / `pixi.lock` | Toolchain, system dependencies, tasks, and exact resolution |
| `pixi-sandbox.toml` | Reviewed offline bundle and publish plan |
| `.devcontainer/` | Official Pixi image plus reproducible container setup |
| `backlog/` | Tasks, milestones, ADRs, roadmaps, and delivery plans |
| `.knowledge/` | Durable architecture, domain, deployment, and dependency reference |

The root `Cargo.toml` is both the Cargo workspace and the publishable `nadir-cli` package. The
root `pixi.toml` is likewise both the Pixi workspace and Conda package manifest. This is
intentional: `pixi-build-rust` invokes `cargo install --path`, which cannot select a member from
a virtual manifest.

See the [Rust workspace guide](crates/README.md) for crate dependency boundaries.

---

## 🛠️ Development and Quality Gates

Pixi is the supported entrypoint for repository tooling. The default environment carries all
six feature layers—Rust, JavaScript, Python, engines, build tools, and repository utilities—and
Turbo fans package work out across Cargo, Python, and TypeScript.

```bash
# Everything required before a change lands
pixi run gates

# Gates plus all build artifacts
pixi run ci

# Individual loops
pixi run build
pixi run test
pixi run typecheck
pixi run lint
pixi run cov
pixi run fmt

# Network-dependent Rust advisory and license scan
pixi run advisories
```

`pixi run gates` covers formatting/linting, type checks, tests, GitHub Actions linting, version
consistency, and a dry-run Conda publish plan. The advisory scan stays separate because it needs
the network; CI runs it as its own job so a registry outage is not confused with a source
failure.

Repository CLIs share Bun as their sole entrypoint instead of adding one Pixi task per
package:

```bash
pixi run bun x --bun backlog task list --plain
pixi run bun x --bun skills list
```

Git hooks use the same tasks as CI. The commit-message hook enforces Conventional Commits, the
pre-commit hook formats staged work, and pre-push runs the gates.

### Dev container

`.devcontainer/devcontainer.json` follows the pixi-sandbox setup: four keys, no Dockerfile, and
the official pinned Pixi image.

```jsonc
{
  "name": "nadir",
  "image": "ghcr.io/prefix-dev/pixi:0.81.0",
  "postCreateCommand": "bash .devcontainer/setup.sh"
}
```

The image contains Pixi but not the complete host toolchain. `.devcontainer/setup.sh` therefore:

1. installs `git` and `gh` as Pixi global tools;
2. exposes `cc`, `gcc`, and `ar` from conda-forge's C compiler package;
3. materializes every project environment from `pixi.lock`;
4. runs `pixi run setup` for Bun, Python, and Git hooks; and
5. installs opencode with Bun and refreshes its model catalog.

The C compiler and opencode are container-development tools rather than project dependencies, so
they stay out of the environment that is packed for offline use.

---

## 🔒 Offline Sandbox

The `developer` bundle contains the locked `default` environment, pixi-sandbox bootstrap tools,
and vendored Cargo sources. A push to `main` publishes the reviewed Linux bundle to
`sandbox/developer-linux-64`.

Transfer Git history containing both the project branch and that sandbox branch to the
disconnected machine, then run:

```bash
bash scripts/restore.sh
source .pixi/sandbox-env.sh

# These must not contact package registries after restore
pixi install --frozen --offline
pixi run --frozen -- cargo build --offline
```

The currently published transport uses pixi-sandbox 0.5.2, whose restore writes
`.pixi/sandbox-env.sh`; source it in each new shell to put the manifest-verified tools and
restored environment on `PATH`. The launcher selects the bundle for the host, verifies the
transport manifest, restores the Pixi environment, and wires Cargo to the vendored crates. If a
connected shallow clone does not contain the sandbox ref, it fetches only the selected branch.

Pixi-sandbox is a release tool rather than a project dependency, so transport operations call
its installed binary directly instead of going through wrapper tasks in `pixi.toml`:

```bash
pixi-sandbox plan --config pixi-sandbox.toml
pixi-sandbox doctor --branch-location .sandbox-out --verify
# Review the plan before running pixi-sandbox publish: it writes to origin.
```

> [!IMPORTANT]
> OpenMVS is not in the locked Conda environment because conda-forge has no compatible package.
> If it is built from source with `pixi run build-openmvs`, repack and republish the sandbox
> before expecting the binary to exist on an airlocked machine.

---

## 📍 Status and Roadmap

| Area | State |
|------|-------|
| Rust workspace and crate boundaries | ✅ scaffolded and link-checked |
| CLI parsing, engine report, inspection, and planning probes | ✅ implemented |
| Shared protocol and Rust dispatcher | ✅ implemented and contract-tested |
| PyO3 and N-API native boundaries | ✅ implemented and contract-tested |
| Content-addressed artifact store | 🚧 next implementation phase |
| Real DAG execution and resume | ⏳ planned for V0.1 |
| COLMAP → georeference → DSM → orthomosaic | ⏳ planned for V0.1 |
| HTTP worker and distributed execution | ⏳ later milestones |

The V0.1 exit demo is intentionally concrete: `nadir process ./images` must write a
georeferenced cloud-optimized GeoTIFF instead of exiting with status 1. Follow the
[V0.1 roadmap](backlog/docs/roadmaps/v0-mvp.md) or inspect live work with:

```bash
pixi run bun x --bun backlog task list --plain
```

Architecture decisions and plans live under `backlog/docs/`; durable technical reference lives
under `.knowledge/`. Start with the [project context](.knowledge/context.md) or the
[knowledge index](.knowledge/index.md).

---

## 📦 Publishing

Conda package production is split into a reviewable plan and an explicit build:

```bash
pixi run publish-plan   # resolve the publish set; build and upload nothing
pixi run publish-dist   # build packages under dist/; upload nothing
```

No repository task supplies a destination channel. Uploading is a separate release decision, so
a development command cannot accidentally publish a package.

---

## 📜 License

The Rust, Python, and JavaScript package manifests declare Nadir under the MIT license.
