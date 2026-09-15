---
title: "ADR-003: Drop Python Adapter for MVP"
status: accepted
date: 2025-02-23
deciders: project lead
related:
  - 001-step-scoped-vs-odm-monolith.md
  - 002-three-language-split.md
  - ../architecture/engine-registry.md
  - ../implementation/process-runner.md
---

# ADR-003: Drop Python Adapter for MVP

## TL;DR

The original architecture included a Python adapter layer between the Rust
worker and ODM. We dropped it for the MVP because every responsibility of the
adapter (config compilation, subprocess spawning, output collection, metadata
extraction) can be done directly in Rust with ~50 lines of `match` statements
and `tokio::process`. Python returns in V2.0+ as an optional engine plugin
language for ML and advanced geospatial processing.

## Context

The original three-language architecture assigned Python the role of "engine
adapter": the Rust worker would invoke a Python script that compiled semantic
config into ODM CLI flags, spawned the ODM subprocess, and collected outputs.

```
Original design:
  Rust Worker → Python Adapter → ODM subprocess
```

On closer inspection, the Python adapter's responsibilities were:

| Responsibility | Can Rust do it? | Complexity |
|---|---|---|
| Compile semantic config → CLI flags | Yes | ~50 lines of `match` |
| Know ODM output file paths | Yes | Static strings |
| Spawn subprocess | Yes | `tokio::process::Command` |
| Parse stdout progress | Yes | Regex on stdout lines |
| Check which output files exist | Yes | `tokio::fs::metadata` |
| Extract CRS/bounds from GeoTIFF | Yes | Shell out to `gdalinfo` |
| Extract point count from LAZ | Yes | Shell out to `pdal info` |

Every responsibility is trivially achievable in Rust. The Python adapter added:
- A language boundary (IPC serialization)
- A deployment dependency (Python runtime + pip packages)
- A debugging layer (errors cross two language boundaries)
- Startup overhead (Python interpreter initialization)

## Decision

**No Python adapter for MVP (V0.1–V1.0).** The Rust worker compiles config,
spawns engines, and collects outputs directly. Python is reintroduced in V2.0+
as an optional engine plugin language when we need ML frameworks (PyTorch),
advanced geospatial libraries (rasterio, scipy), or stage-level ODM control.

### MVP architecture (2 languages)

```
TS/Python/Rust SDK → protobuf → Rust Worker → External Engines
                                              ├── COLMAP (subprocess)
                                              ├── OpenMVS (subprocess)
                                              ├── GDAL (FFI via crate)
                                              └── PROJ (FFI via crate)
```

### Future architecture (3 languages, V2.0+)

```
Rust Worker
    ├── engines/
    │   ├── colmap.rs          # Rust native adapter (subprocess)
    │   ├── openmvs.rs         # Rust native adapter (subprocess)
    │   ├── gdal.rs            # Rust FFI adapter (crate)
    │   ├── odm_advanced.py    # Python plugin (stage-level ODM control)
    │   ├── segment.py         # Python plugin (PyTorch SAM)
    │   └── classify.py        # Python plugin (PointNet)
```

## Rationale

### The config compiler is trivially small

The entire "ODM config compiler" in Rust:

```rust
pub fn compile_args(&self, config: &MappingConfig) -> Vec<String> {
    let mut args = vec!["--project-path".into(), self.project_path.clone()];

    match Quality::try_from(config.quality).unwrap() {
        Quality::Fast => {
            args.extend(["--feature-quality".into(), "low".into()]);
        }
        Quality::Survey => {
            args.extend([
                "--feature-quality".into(), "high".into(),
                "--pc-quality".into(), "high".into(),
                "--dem-resolution".into(), "0.05".into(),
            ]);
        }
        _ => {}
    }

    if let Some(engine) = &config.engine {
        for (key, value) in &engine.odm {
            args.push(format!("--{}", key.replace('_', "-")));
            args.push(value.clone());
        }
    }

    args  // That's it. ~30 lines.
}
```

This does not justify a Python runtime dependency.

### Deployment simplicity

MVP deployment without Python:

```
System requirements:
  ├── nadir-worker (Rust binary)
  ├── colmap (apt/brew/pixi)
  ├── openmvs (pre-built binaries)
  ├── libgdal (apt/brew/pixi)
  └── libproj (apt/brew/pixi)

No Python environment.
No pip dependencies.
No virtualenv.
No IPC serialization.
```

### When Python returns (V2.0+)

Python becomes valuable when we need:

1. **ML-based engines:** Segmentation (SAM, Detectron2), classification
   (PointNet), change detection. These are PyTorch. The "engine" is
   inherently Python.

2. **Complex post-processing:** NDVI from multispectral, custom DEM
   interpolation, advanced filtering with scipy/numpy.

3. **Stage-level ODM control:** Hooking into individual ODM stages (run
   OpenSfM separately, inspect reconstruction, decide whether to continue).
   ODM's internal stage system is Python.

4. **Direct geospatial algorithms:** Rasterio, PDAL Python bindings,
   geopandas for vector operations.

At that point, Python adapters are **engine plugins**, not a mandatory
infrastructure layer. The Rust worker discovers them via manifest files:

```yaml
# engines/segment/manifest.yaml
name: segment-anything
version: 1.0.0
runtime: python
tasks:
  - image.segmentation
dependencies:
  - pytorch
  - segment-anything
```

## Consequences

### Positive
- Simpler MVP deployment (no Python runtime)
- Faster startup (no Python interpreter initialization)
- Single-language debugging (Rust → subprocess, not Rust → Python → subprocess)
- No IPC serialization overhead
- Smaller Docker image
- Config compilation is ~30 lines of Rust, easy to understand and modify

### Negative
- When we do need Python (V2.0+), we'll need to add the plugin system
- Some geospatial operations are more verbose in Rust than Python (e.g.,
  rasterio one-liners become GDAL FFI calls)

### Mitigated
- The plugin system is a V2.0 concern. By then the core architecture is
  stable and adding a Python engine adapter is a localized change.
- GDAL and PROJ Rust crates handle most geospatial operations without Python.

## See Also

- [ADR-001: Step-scoped tools](./001-step-scoped-vs-odm-monolith.md) — the engines we call directly
- [ADR-002: Three-language split](./002-three-language-split.md) — Python's future role
- [Process runner](../implementation/process-runner.md) — how Rust spawns engines
- [Engine registry](../architecture/engine-registry.md) — the trait-based adapter pattern
