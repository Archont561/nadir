# nadir

Rust-native, artifact-driven photogrammetry with first-class Python and TypeScript bindings.

> **Status: early scaffold.** The workspace, CLI shell, versioned FFI transport, PyO3
> extension, and N-API addon compile and are covered by contract tests. Pipeline execution
> is not implemented yet: `nadir process` fails explicitly rather than pretending to run.

Nadir is designed to turn overlapping aerial images into georeferenced point clouds,
meshes, DSMs, DTMs, and orthomosaics. Rust owns the domain model and orchestration;
step-scoped engines such as COLMAP, OpenMVS, GDAL, PROJ, and PDAL perform specialized
reconstruction and geospatial work.

## Why Nadir

- **Build system, not a job queue.** Tasks transform immutable artifacts into artifacts.
- **Content-addressed results.** Inputs, normalized parameters, and engine versions determine
  cache identity, making incremental rebuilds and resume natural properties.
- **Step-scoped engines.** Each stage can be cached, replaced, retried, or scheduled
  independently instead of hiding the pipeline behind one monolithic subprocess.
- **One Rust implementation.** Python and TypeScript are thin native faces over the same
  versioned transport and dispatcher; domain behavior is not reimplemented per language.
- **Reproducible tooling.** Pixi owns Rust, Bun, Python, native engines, and repository tools
  in one locked environment, with an offline sandbox transport for air-gapped machines.

## Architecture

```text
                         ┌──────────────────────────┐
Python ─ PyO3 adapter ───┤                          │
                         │  versioned JSON protocol ├─ dispatcher ─ Rust core
TypeScript ─ N-API addon ┤                          │                 │
                         └──────────────────────────┘                 ▼
                                                              pipeline crates
                                                                    │
                                      COLMAP · OpenMVS · GDAL · PROJ · PDAL
```

Each native adapter exports one operation: JSON text in, JSON text out. The transport is
versioned in `nadir-protocol`; `nadir-engine` validates and dispatches it; domain rules stay
in `nadir-core` and the stage crates. Adding an operation changes the protocol and one Rust
dispatcher, not every native ABI.

See [the binding architecture](.knowledge/architecture/language-bindings.md) and
[the Rust workspace guide](crates/README.md).

## Repository layout

| Path | Responsibility |
| --- | --- |
| `crates/core` | shared domain vocabulary and engine traits |
| `crates/protocol` | versioned request and response envelopes |
| `crates/engine` | shared operation dispatcher |
| `crates/python-native` | thin PyO3 adapter |
| `crates/node-native` | thin N-API adapter |
| `crates/{artifacts,pipeline,executor}` | caching, declarative DAGs, and execution |
| `crates/{dataset,reconstruction,geometry,surface,cartography,math}` | photogrammetry domains |
| `crates/cli` | `nadir` command sources; the manifest is the repository root |
| `python/nadir` | native Python SDK built with Maturin |
| `packages/sdk` | native TypeScript SDK |
| `packages/client` | higher-level TypeScript convenience client |
| `backlog` | tasks, milestones, decisions, roadmaps, and delivery plans |
| `.knowledge` | durable architecture, domain, deployment, and dependency reference |

The root `Cargo.toml` is both the Cargo workspace and the publishable `nadir-cli` package.
The root `pixi.toml` is likewise both the Pixi workspace and Conda package manifest. This
layout is required because `pixi-build-rust` invokes `cargo install --path`, which cannot
select a member from a virtual manifest.

## Set up

Pixi is the only prerequisite:

```sh
curl -fsSL https://pixi.sh/install.sh | bash
pixi install --locked
pixi run setup
```

`setup` installs the Bun workspace, builds the native Python package in editable mode, and
installs Git hooks. The locked default environment targets `linux-64` and includes the
Rust, Bun, Python, C/C++, and photogrammetry toolchains.

### Offline restore

The published sandbox contains the environment and vendored Cargo sources:

```sh
bash scripts/restore.sh       # PowerShell: scripts/restore.ps1
source .pixi/sandbox-env.sh
pixi install --frozen --offline
cargo build --offline
```

The restore script fetches the selected `sandbox/<bundle>-<platform>` branch when a shallow
clone does not already contain it, verifies every blob, and materializes the environment.

## Develop

```sh
pixi run gates          # lint, type-check, test, workflow lint, versions, publish plan
pixi run ci             # gates plus builds
pixi run build          # Rust and language packages
pixi run test           # Rust, Python, and Bun contract tests
pixi run fmt            # rustfmt, Ruff, and Biome
pixi run advisories     # network-dependent dependency advisories
```

Useful CLI probes while the processing pipeline is being implemented:

```sh
cargo run -- engines    # report available external engines
cargo run -- plan       # print the planned stage ordering
cargo run -- inspect D  # count images in a dataset directory
cargo run -- crates     # report workspace stage ownership
cargo run -- process D  # intentionally fails until execution lands
```

## Bindings

Both SDKs currently expose transport-level probes—`ping`, `version`, and `describe`—that
cross a real native boundary and return results from the shared Rust engine. Pipeline
operations will grow on this same contract.

```python
import nadir

assert nadir.ping("python") == "python"
print(nadir.version())
print(nadir.describe())
```

```ts
import { describe, ping, version } from "@nadir/sdk";

console.log(ping("typescript"));
console.log(version());
console.log(describe());
```

## Project records

Use `pixi run backlog task list --plain` to inspect current work. The `backlog/` directory
owns change over time:

- `tasks/` and `milestones/` are Backlog.md records;
- `docs/decisions/` contains ADR-001 through ADR-010;
- `docs/roadmaps/` contains version plans;
- `docs/plans/` contains time-bound delivery plans.

The `.knowledge/` bundle is intentionally limited to durable technical reference. Start
with [project context](.knowledge/context.md) or [the knowledge index](.knowledge/index.md).

## Publishing

`pixi run publish-plan` validates the Conda publish set without uploading. `pixi run
publish-dist` writes packages to `dist/`. Publishing is deliberately separate from building
and requires an explicit destination channel.

OpenMVS is the exception to the locked Conda stack because no conda-forge package exists.
`pixi run build-openmvs` builds it from source; repack the sandbox afterward if that binary
must be available offline.
