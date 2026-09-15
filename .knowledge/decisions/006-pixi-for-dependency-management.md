---
title: "ADR-006: Pixi for Dependency Management"
status: accepted
date: 2025-02-23
deciders: project lead
related:
  - ../deployment/pixi-setup.md
  - ../deployment/docker-strategy.md
  - 004-inverted-container-no-dind.md
---

# ADR-006: Pixi for Dependency Management

## TL;DR

We use Pixi (built on the Conda ecosystem) as the single source of truth for
all system-level dependencies: COLMAP, OpenMVS, GDAL, PROJ, PDAL, Rust,
Node.js, CMake. A single `pixi.toml` + `pixi.lock` replaces apt-get, brew,
rustup, nvm, and Dockerfile RUN lines. Developers run `pixi install && pixi
run build`. CI runs `pixi install --locked`. Docker copies the `.pixi`
environment.

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
| Node.js | `nvm install 22` | `nvm install 22` | `node:22` image | `setup-node` action |

Before Pixi, the README would need 40+ lines of platform-specific install
instructions. CI would need separate setup steps for each tool. Docker would
scatter dependencies across `apt-get`, `pip install`, and source builds.

## Decision

**Pixi as the unified dependency manager.** All system tools, language
runtimes, and C libraries are declared in `pixi.toml` and locked in
`pixi.lock`. Pixi features and environments separate optional dependencies
(GPU, WASM, Python, web).

### The pixi.toml structure

```toml
[project]
name = "nadir"
channels = ["conda-forge"]
platforms = ["linux-64", "osx-arm64"]

[dependencies]
rust = ">=1.83"
cmake = ">=3.28"
gdal = ">=3.9"
proj = ">=9.4"
colmap = ">=3.9"
pdal = ">=2.7"
libopencv = ">=4.9"
ffmpeg = "*"

[feature.web.dependencies]
nodejs = "22.*"
pnpm = ">=9"

[feature.gpu.dependencies]
cuda-toolkit = ">=12.0"

[feature.wasm.dependencies]
wasm-pack = "*"

[environments]
default = { features = [] }
web = { features = ["web"] }
gpu = { features = ["gpu"] }
wasm = { features = ["wasm"] }

[tasks]
build = "cargo build --release"
test = "cargo test"
serve = "cargo run --release -- serve --port 8080"
```

### Developer onboarding

```bash
git clone https://github.com/yourorg/nadir
cd nadir
pixi install          # installs COLMAP, GDAL, PROJ, PDAL, Rust, everything
pixi run build        # cargo build with all libraries in PATH
pixi run test         # cargo test
pixi run serve        # start the worker
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

### CI integration

```yaml
steps:
  - uses: prefix-dev/setup-pixi@v0.8
    with:
      locked: true
      cache: true
  - run: pixi run test
  - run: pixi run build
```

One setup step instead of six.

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
- Feature-based environments for GPU/WASM/Python
- Cross-platform (Linux + macOS from same `pixi.toml`)

### Negative
- Adds Pixi as a prerequisite (developers must install Pixi once)
- Conda-forge may lag behind latest releases for some packages
- OpenMVS may not be on conda-forge (may need source build task)
- Pixi environment adds ~2–4GB to disk usage

### Mitigated
- Pixi installs via a single curl command: `curl -fsSL https://pixi.sh/install.sh | bash`
- Conda-forge has excellent coverage for geospatial packages (GDAL, PROJ, PDAL, COLMAP)
- OpenMVS can be built via a Pixi task or a separate Docker stage
- Disk usage is acceptable for a photogrammetry project (datasets are 10–100GB)

## See Also

- [Pixi setup](../deployment/pixi-setup.md) — full `pixi.toml` and CI config
- [Docker strategy](../deployment/docker-strategy.md) — Pixi-based Dockerfile
- [ADR-004: Inverted container](./004-inverted-container-no-dind.md) — container model
