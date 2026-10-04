---
type: Architecture Decision Record
title: "ADR-006: Pixi for Dependency Management"
description: "Records the accepted architecture decision on Pixi for Dependency Management."
status: accepted
date: 2025-02-23
last_updated: 2026-09-30
deciders: project lead
related:
  - ../../../.knowledge/deployment/pixi-setup.md
  - ../../../.knowledge/deployment/docker-strategy.md
  - 009-one-manifest-per-root.md
  - 004-inverted-container-no-dind.md
---

# ADR-006: Pixi for Dependency Management

## TL;DR

We use Pixi (built on the Conda ecosystem) as the single source of truth for
all system-level dependencies: COLMAP, GDAL, PROJ, PDAL, OpenCV, Rust, Bun,
Python, CMake. A single `pixi.toml` + `pixi.lock` replaces apt-get, brew,
rustup, nvm, and Dockerfile RUN lines. Developers run `pixi install --locked
&& pixi run setup`. CI runs `pixi install --locked`. Docker copies the `.pixi`
environment.

> **Superseded in part, 2026-09-30.** The decision stands; three details in the original
> record are no longer accurate and are marked inline:
>
> 1. **Bun, not Node.js + pnpm.** The JavaScript runtime, package manager and test runner
>    is one binary from conda-forge. See [ADR-010](010-bun-not-node.md).
> 2. **`[workspace]`, not `[project]`.** The table was renamed; the example below has been
>    updated. The original `[project]` spelling no longer parses.
> 3. **linux-64 only.** macOS is not a target yet — see Consequences.
>
> The fictional `pixi.toml` below has been replaced with the shape that is actually in the
> repository. It was written as an illustration before there was a manifest; a reader who
> copied it would have got a file that does not solve.

## Context

Nadir depends on 6+ external C/C++ engines and libraries that are notoriously
difficult to install consistently across platforms:

| Dependency | Ubuntu | macOS | Docker | CI |
|---|---|---|---|---|
| COLMAP | `apt install colmap` | `brew install colmap` | Build from source | ??? |
| GDAL | `apt install libgdal-dev` | `brew install gdal` | `apt install` | `apt install` |
| PROJ | `apt install libproj-dev` | `brew install proj` | Bundled with GDAL | Bundled |
| PDAL | `apt install pdal` | `brew install pdal` | Build from source | ??? |
| OpenCV | `apt install libopencv-dev` | `brew install opencv` | `apt install` | `apt install` |
| Rust | `rustup` | `rustup` | `rust:1.83` image | `setup-rust` action |
| Bun | *(none — a single static binary)* | | | `pixi install` |

Before Pixi, the README would need 40+ lines of platform-specific install
instructions. CI would need separate setup steps for each tool. Docker would
scatter dependencies across `apt-get`, `pip install`, and source builds.

## Decision

**Pixi as the unified dependency manager.** All system tools, language
runtimes, and C libraries are declared in `pixi.toml` and locked in
`pixi.lock`. Pixi features and environments separate optional dependencies.

### The pixi.toml structure

The shape below is the one in the repository, reduced. The full manifest, with
the reasoning behind each table, is [`pixi.toml`](../../../pixi.toml); the
`[package]` section is
[ADR-009](009-one-manifest-per-root.md).

```toml
[workspace]
name = "nadir"
version = "0.1.0"
license = "MIT"
channels = ["conda-forge"]
platforms = ["linux-64"]
requires-pixi = ">=0.81.0"
# Makes the `[package]` table legal. See ADR-009.
preview = ["pixi-build"]

# Feature layers rather than one flat dependency list, so a version bump to one
# toolchain is its own reviewable diff.
[workspace.dependencies]
rust = ">=1.98.1,<1.99"
bun = ">=1.3.11,<2"
python = ">=3.13,<3.14"
colmap = { version = ">=3.9", build = "cpu_*" }
gdal = ">=3.12"
proj = ">=9.7"
pdal = ">=2.9"
libopencv = ">=4.12"
cargo-nextest = ">=0.9.146,<0.10"
cargo-llvm-cov = ">=0.9.1,<0.10"
cargo-deny = ">=0.20.2,<0.21"
convco = ">=0.7.2,<0.8"
lefthook = ">=2.1.15,<3"
actionlint = ">=1.7.12,<2"
pytest = ">=8.4,<9"
pytest-cov = ">=6.2,<7"
ruff = ">=0.9,<0.15"
hatchling = ">=1.32,<2"
pip = ">=26,<27"

[feature.rust.dependencies]
# cargo build itself, from the pixi environment, so a contributor's rustup
# version cannot decide what compiles.
cargo = ">=1.98.1,<1.99"

[feature.js.dependencies]
# Bun only. No nodejs, no pnpm: see ADR-010.
# turbo and biome are Bun *workspace* dependencies, not conda ones — they
# change on the npm release cadence, not the conda one.

[feature.python.dependencies]
python-pdal = ">=3.5,<4"

[feature.engines.dependencies]
# The geospatial C/C++ stack the tier-2 bindings link against.

[feature.build.dependencies]
cmake = ">=4.2,<5"
ninja = ">=1.13,<2"
pkg-config = ">=0.29,<0.30"

[feature.utils.dependencies]
convco = ">=0.7.2,<0.8"
lefthook = ">=2.1.15,<3"
actionlint = ">=1.7.12,<2"

[environments]
# One environment. Every layer is on: the feature split exists so a bump is a
# reviewable diff and so a contributor can drop `engines` to save 4 GB, not so
# there are several environments to choose between wrongly.
default = { features = ["rust", "js", "python", "engines", "build", "utils"] }

# OpenMVS is deliberately absent from every layer. There is no conda-forge
# package for it, and a manifest naming one fails to solve outright:
# "No candidates were found for openmvs >=2.2". `pixi run build-openmvs`
# compiles it from source; a binary installed that way is only offline-
# restorable if the pixi-sandbox CLI packs that environment afterwards.
```

### Developer onboarding

```bash
git clone https://github.com/Archont561/nadir
cd nadir
pixi install --locked   # COLMAP, GDAL, PROJ, PDAL, Rust, Bun, Python
pixi run setup          # bun workspace, editable python sdk, git hooks
pixi run gates          # everything CI runs
```

Three commands. No platform-specific instructions.

### Docker integration

```dockerfile
FROM ghcr.io/prefix-dev/pixi:latest AS builder
WORKDIR /app
COPY pixi.toml pixi.lock ./
RUN pixi install --locked --frozen
COPY . .
RUN pixi run build

FROM ghcr.io/prefix-dev/pixi:latest
WORKDIR /app
COPY --from=builder /app/.pixi /app/.pixi
COPY --from=builder /app/target/release/nadir /usr/local/bin/nadir
ENV PATH="/app/.pixi/envs/default/bin:${PATH}"
ENTRYPOINT ["nadir", "serve", "--port", "8080"]
```

No `apt-get`. No `RUN install`. The Pixi environment IS the runtime.

For an airlocked machine there is a second path: `scripts/restore.sh` unpacks a
packed transport from an orphan branch of this repository, including the
vendored crate sources, so the workspace **builds** offline and not merely runs.
See [ADR-009](009-one-manifest-per-root.md) for why the vendored crates
matter.

### CI integration

```yaml
steps:
  - uses: prefix-dev/setup-pixi@v0.10.0
    with:
      pixi-version: v0.59.0
      cache: true
  - run: pixi install --locked --frozen-lockfile
  - run: pixi run gates
```

One setup step instead of six.

`--locked` is the point of the CI step. A CI run that re-solves produces a
different environment from the committed one, so a green run would say nothing
about the lockfile.

## Rationale

| Factor | Before Pixi | With Pixi |
|---|---|---|
| README install instructions | 40+ lines, platform-specific | 3 lines |
| CI setup steps | 6+ (rust, node, gdal, proj, colmap, pdal) | 1 (`setup-pixi`) |
| Dockerfile RUN lines | 10+ `apt-get install` | 1 (`pixi install`) |
| Version reproducibility | ⚠️ "latest" drift | ✅ `pixi.lock` |
| Cross-platform | ❌ Different commands per OS | ✅ Same `pixi.toml` |
| Optional deps (GPU, WASM) | ❌ Manual | ✅ `pixi run --environment gpu` |
| New developer onboarding | ~30 minutes | ~2 minutes |

### Why Pixi over alternatives

| Tool | Why not |
|---|---|
| **Nix** | Steeper learning curve, smaller community for geospatial |
| **devcontainers** | Requires VS Code, doesn't solve CI/Docker |
| **asdf/.tool-versions** | Only handles language runtimes, not C libraries |
| **Docker only** | Slow dev loop, poor IDE integration |
| **apt/brew scripts** | No lockfile, no cross-platform, no CI integration |

Pixi uniquely bridges "language package manager" and "OS package manager"
with a lockfile, which is exactly what Nadir needs (Rust + Node + COLMAP +
GDAL + PROJ + PDAL in one reproducible environment).

## Consequences

### Positive
- Single source of truth for all dependencies
- Reproducible builds via `pixi.lock`
- Trivial CI setup (one GitHub Action)
- Clean Dockerfiles (no `apt-get`)
- Feature layers, so a version bump to one toolchain is its own reviewable diff
- The CLI ships as a conda package (`nadir-cli`), so a user needs no Rust toolchain
- An offline path exists that restores both the environment and the crate sources

### Negative
- Adds Pixi as a prerequisite (developers must install Pixi once)
- Conda-forge may lag behind latest releases for some packages
- Pixi environment adds ~1–4 GB to disk usage depending on which feature layers are enabled
- **linux-64 only.** The original record claimed Linux + macOS from one `pixi.toml`.
  `platforms = ["linux-64"]` because that is the only platform the geospatial
  stack was verified on. Widening it is a decision to re-make when the lock
  actually solves for `osx-arm64`, not a line to flip optimistically.
- `colmap` must be pinned to `build = "cpu_*"`. The default build string drags
  `cuda-version` and a GPU stack into every environment, including CI runners
  with no GPU. A version bump that tries to move it will pull a CUDA stack into
  the lockfile, so this is a constraint the release process has to know about.
- `preview = ["pixi-build"]` is required for the `[package]` table, so the
  manifest depends on a pixi preview flag. Small, but a real coupling.

### Mitigated
- Pixi installs via a single curl command: `curl -fsSL https://pixi.sh/install.sh | bash`
- Conda-forge has excellent coverage for geospatial packages (GDAL, PROJ, PDAL, COLMAP)
- **OpenMVS: confirmed absent, not hypothetical.** `pixi search openmvs` returns
  nothing and a manifest naming it fails to solve — "No candidates were found for
  openmvs >=2.2". So it is not a dependency at all; `pixi run build-openmvs`
  compiles it from source. The `nadir engines` subcommand reports it as
  *expected to be missing* rather than omitting it, so its absence does not read
  as a broken environment.
- Disk usage is acceptable for a photogrammetry project (datasets are 10–100GB),
  and the layers can be dropped: `pixi run --no-default-feature engines`
  saves roughly 4 GB for someone who is only writing TypeScript.

## See Also

- [ADR-009: One Manifest per Root](009-one-manifest-per-root.md) — where the conda package is declared
- [ADR-010: Bun, Not Node](010-bun-not-node.md) — the JavaScript runtime
- [Pixi setup](../../../.knowledge/deployment/pixi-setup.md) — full `pixi.toml` and CI config
- [Docker strategy](../../../.knowledge/deployment/docker-strategy.md) — Pixi-based Dockerfile
- [ADR-004: Inverted container](004-inverted-container-no-dind.md) — container model
