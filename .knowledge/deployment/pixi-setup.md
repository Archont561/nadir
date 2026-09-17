---
type: Deployment Guide
title: Pixi Setup
description: "Full pixi.toml, environments, developer onboarding, Docker integration, CI"
purpose: Full pixi.toml, environments, developer onboarding, Docker integration, CI
last_updated: 2025-02-23
status: stable
related:
  - docker-strategy.md
  - ../decisions/006-pixi-for-dependency-management.md
---

# Pixi Setup

## TL;DR

Pixi manages all system-level dependencies for Nadir: COLMAP, OpenMVS, GDAL,
PROJ, PDAL, Rust, Node.js, CMake. A single `pixi.toml` + `pixi.lock` replaces
apt-get, brew, rustup, nvm, and Dockerfile RUN lines. Developers run
`pixi install && pixi run build`. CI uses `prefix-dev/setup-pixi`. Docker
copies the `.pixi` environment.

## Installation

```bash
# Install Pixi (one-time, per developer)
curl -fsSL https://pixi.sh/install.sh | bash

# Verify
pixi --version
```

## The Complete `pixi.toml`

```toml
[project]
name = "nadir"
version = "0.1.0"
description = "Artifact-driven photogrammetry pipeline"
channels = ["conda-forge"]
platforms = ["linux-64", "osx-arm64", "osx-64"]

# ── Shared system dependencies ──
[dependencies]
# Rust toolchain
rust = ">=1.83"
cargo-edit = "*"

# Build tools
cmake = ">=3.28"
ninja = "*"
pkg-config = "*"
make = "*"

# Geospatial C libraries
gdal = ">=3.9"
proj = ">=9.4"
geos = ">=3.12"

# Photogrammetry engines
colmap = ">=3.9"
pdal = ">=2.7"

# Image processing
libopencv = ">=4.9"
libtiff = "*"
libjpeg-turbo = "*"
libpng = "*"

# Point cloud
laszip = "*"

# Utilities
ffmpeg = "*"
git = "*"
curl = "*"

# ── Feature: OpenMVS ──
[feature.openmvs.dependencies]
openmvs = ">=2.2"

# ── Feature: GPU support ──
[feature.gpu.dependencies]
cuda-toolkit = ">=12.0"
cudnn = ">=8.9"

# ── Feature: Web development ──
[feature.web.dependencies]
nodejs = "22.*"
pnpm = ">=9"

# ── Feature: Python adapter (future) ──
[feature.python.dependencies]
python = "3.12.*"
pydantic = ">=2.0"
rasterio = "*"

# ── Feature: WASM target ──
[feature.wasm.dependencies]
wasm-pack = "*"
binaryen = "*"

# ── Environments ──
[environments]
default = { features = [], solve-group = "default" }
full = { features = ["openmvs", "gpu"], solve-group = "full" }
web = { features = ["web"], solve-group = "web" }
python = { features = ["python"], solve-group = "python" }
wasm = { features = ["wasm"], solve-group = "wasm" }
ci = { features = [], solve-group = "default" }

# ── Tasks ──
[tasks]
# Build
build = "cargo build --release"
build-debug = "cargo build"
check = "cargo check"

# Test
test = "cargo test"
test-integration = "cargo test --test integration -- --nocapture"

# Lint
lint = "cargo clippy -- -D warnings"
fmt = "cargo fmt"
fmt-check = "cargo fmt --check"

# Run
run = "cargo run --release -- process"
serve = "cargo run --release -- serve --port 8080"
inspect = "cargo run --release -- inspect"
plan = "cargo run --release -- plan"

# Docker
docker-build = "docker build -t nadir-worker ."
docker-run = "docker run -p 8080:8080 -v $(pwd)/data:/var/nadir/workspace nadir-worker"

# Web (requires --environment web)
web-install = "cd apps/web && pnpm install"
web-dev = "cd apps/web && pnpm dev"
web-build = "cd apps/web && pnpm build"

# WASM (requires --environment wasm)
wasm-build = "cd crates/nadir-math && wasm-pack build --target web"
wasm-test = "cd crates/nadir-math && wasm-pack test --headless --chrome"

# CI
ci-check = { depends-on = ["fmt-check", "lint", "test"] }
ci-full = { depends-on = ["ci-check", "build"] }
```

## Developer Onboarding

### Before Pixi (the old way)

```bash
# README.md — 40 lines of platform-specific instructions
# Ubuntu:
sudo apt-get update && sudo apt-get install -y colmap libgdal-dev \
    libproj-dev libopencv-dev pdal cmake ninja-build ffmpeg
# macOS:
brew install colmap gdal proj opencv pdal cmake ninja ffmpeg
# Then: curl --proto '=https' --tlsv1.2 -sSf https://sh.rustup.rs | sh
# Then: curl -fsSL https://fnm.vercel.app/install | bash && fnm install 22
# Then: cargo build --release
# "If you get a linker error about libgdal, try setting PKG_CONFIG_PATH..."
```

### After Pixi

```bash
git clone https://github.com/yourorg/nadir
cd nadir
pixi install
pixi run build
pixi run test
pixi run inspect ./test-images/
```

Three commands. Same on Ubuntu, macOS, and CI.

## Environment Usage

```bash
# Core development (Rust + engines)
pixi install
pixi run build
pixi run test

# Full stack (adds Node.js for web)
pixi run --environment web web-dev

# Python work (future)
pixi run --environment python python scripts/analyze.py

# WASM development
pixi run --environment wasm wasm-build

# GPU builds (on a GPU machine)
pixi run --environment full build
```

## Docker Integration

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
ENV LD_LIBRARY_PATH="/app/.pixi/envs/default/lib:${LD_LIBRARY_PATH}"
EXPOSE 8080
ENTRYPOINT ["nadir", "serve", "--port", "8080"]
```

No `apt-get`. No `RUN install`. The Pixi environment IS the runtime.

## CI Integration (GitHub Actions)

```yaml
# .github/workflows/ci.yml
name: CI
on: [push, pull_request]

jobs:
  test:
    runs-on: ubuntu-latest
    steps:
      - uses: actions/checkout@v4

      - uses: prefix-dev/setup-pixi@v0.8
        with:
          pixi-version: latest
          locked: true
          cache: true

      - name: Check formatting
        run: pixi run fmt-check

      - name: Lint
        run: pixi run lint

      - name: Test
        run: pixi run test

      - name: Build
        run: pixi run build

  integration:
    runs-on: ubuntu-latest
    needs: test
    steps:
      - uses: actions/checkout@v4

      - uses: prefix-dev/setup-pixi@v0.8
        with:
          locked: true
          cache: true

      - name: Integration test
        run: pixi run test-integration
```

One setup step instead of six. No `setup-rust`, `setup-node`, or `apt-get`.

## What Pixi Replaces

| Before | After |
|---|---|
| `apt-get install colmap gdal proj pdal` | `pixi.toml: colmap, gdal, proj, pdal` |
| `brew install colmap gdal proj` | Same `pixi.toml` |
| `rustup install stable` | `pixi.toml: rust = ">=1.83"` |
| `nvm install 22` | `pixi.toml: nodejs = "22.*"` (web feature) |
| `pip install rasterio` | `pixi.toml: rasterio` (python feature) |
| `Dockerfile: RUN apt-get ...` | `Dockerfile: COPY .pixi` |
| `CI: apt-get + setup-rust + setup-node` | `CI: setup-pixi` |
| `README: 40 lines of install instructions` | `README: pixi install && pixi run build` |

## The OpenMVS Caveat

OpenMVS may not be available on conda-forge for all platforms. Options:

1. **Conda-forge** (if available): `openmvs = ">=2.2"` in `[feature.openmvs]`
2. **Source build task**:
   ```toml
   [feature.openmvs.tasks]
   build-openmvs = """
       git clone https://github.com/cdcseacave/openMVS.git /tmp/openmvs && \
       cmake -B /tmp/openmvs/build /tmp/openmvs && \
       cmake --build /tmp/openmvs/build -j$(nproc) && \
       cp /tmp/openmvs/build/bin/* $CONDA_PREFIX/bin/
   """
   ```
3. **Docker stage**: Build OpenMVS in a separate Docker stage and copy
   binaries into the Pixi environment.

## See Also

- [ADR-006: Pixi for dependencies](../decisions/006-pixi-for-dependency-management.md)
- [Docker strategy](./docker-strategy.md) — Pixi-based Dockerfile
- [Dependency matrix](../references/dependency-matrix.md) — versions
