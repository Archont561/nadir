---
title: "ADR-007: Engine Trait API Surface"
status: accepted
date: 2025-02-23
deciders: project lead
related:
  - ../architecture/engine-registry.md
  - ../photogrammetry/engine-assignment.md
  - 001-step-scoped-vs-odm-monolith.md
---

# ADR-007: Engine Trait API Surface

## TL;DR

The photogrammetry API surface consists of exactly **16 stable Rust traits**.
Each trait has a small method signature using domain types (not engine types).
Configuration structs are small (5–10 fields). Engine-specific CLI flags live
inside adapters and are never exposed to the pipeline. This gives us a stable
API that survives engine swaps and version upgrades.

## Context

External photogrammetry engines have enormous configuration surfaces:

| Engine | CLI flags | Config complexity |
|---|---|---|
| COLMAP `feature_extractor` | ~40 flags | SIFT params, GPU, camera model, tiling |
| COLMAP `mapper` | ~60 flags | BA iterations, filtering, triangulation |
| OpenMVS `DensifyPointCloud` | ~25 flags | Resolution, views, filtering |
| GDAL `gdalwarp` | ~50 flags | Resampling, CRS, nodata, compression |
| PDAL pipeline | ~30 filter options | SMRF params, outlier thresholds |

Exposing these directly to the pipeline would create a config surface of 200+
parameters that changes with every engine version. The pipeline would be
tightly coupled to specific engine implementations.

## Decision

**16 stable traits with small configs.** The pipeline knows only trait
methods and domain-level configuration. Engine-specific flags are hidden
inside adapters.

### The 16 traits

```
Domain 1: Dataset (1 trait)
  1. ImageProvider          — ingest(path) → ImageSet

Domain 2: Reconstruction (6 traits)
  2. FeatureExtractor       — extract(images, cfg) → Features
  3. FeatureMatcher         — match_features(features, cfg) → Matches
  4. SparseReconstructor    — reconstruct(matches, cfg) → SparseReconstruction
  5. DenseReconstructor     — densify(scene, cfg) → DensePointCloud
  6. MeshGenerator          — mesh(cloud, cfg) → Mesh
  7. TextureGenerator       — texture(mesh, images, cfg) → TexturedMesh

Domain 3: Geometry (2 traits)
  8. Georeferencer          — georeference(scene, gps, cfg) → GeoreferencedScene
  9. CoordinateTransformer  — transform(point) → GeoPoint

Domain 4: Surface (3 traits)
  10. PointCloudFilter      — filter(cloud, cfg) → DensePointCloud
  11. SurfaceGenerator      — generate_dsm(cloud, cfg) → Raster
                            — generate_dtm(cloud, cfg) → Raster
  12. PointCloudClassifier  — classify(cloud, cfg) → ClassifiedPointCloud

Domain 5: Cartography (2 traits)
  13. Orthorectifier        — orthorectify(images, scene, surface, cfg) → Raster
  14. Mosaicker             — mosaic(orthos, cfg) → Raster

I/O (2 traits)
  15. RasterStore           — read(path) → Raster / write(raster, path)
  16. PointCloudStore       — read(path) → PointCloud / write(cloud, path)
```

### Config structs are small

```rust
pub struct FeatureConfig {
    pub detector: Detector,       // Sift | Orb | Akaze
    pub max_features: u32,        // 8192
}

pub struct SfmConfig {
    pub max_iterations: u32,      // 100
    pub min_inliers: u32,         // 30
}

pub struct DemConfig {
    pub resolution: f64,          // 0.05
    pub gap_filling: GapFilling,  // None | Standard | Aggressive
}
```

No COLMAP flags. No OpenMVS flags. No GDAL options. The adapter translates.

### The adapter hides the ugly parts

```rust
impl SparseReconstructor for ColmapEngine {
    fn reconstruct(&self, matches: &Matches, cfg: &SfmConfig) -> Result<SparseReconstruction> {
        // The 20+ COLMAP-specific flags live HERE, not in the pipeline
        self.run(&[
            "mapper",
            "--database_path", &ws.db(),
            "--Mapper.ba_max_num_iterations", &cfg.max_iterations.to_string(),
            "--Mapper.filter_min_tri_angle", "1.5",
            "--Mapper.ba_global_max_refinements", "5",
            // ... 15 more COLMAP-specific flags
        ])?;
        parse_colmap_model(&ws.sparse())
    }
}
```

### Engine swap is a one-line config change

```toml
# pipelines/standard.toml
[[steps]]
id = "sfm"
task = "sparse_reconstruction"
engine = "colmap"     # ← change to "opensfm" or "rust"
```

The pipeline, config, and artifact types don't change. Only the adapter
implementation behind the trait changes.

## Rationale

| Factor | Expose engine flags | 16 stable traits |
|---|---|---|
| API stability | ❌ Changes with engine versions | ✅ Stable across engine swaps |
| Config complexity | 200+ parameters | 5–10 per config struct |
| Engine swapping | ❌ Requires pipeline changes | ✅ One-line TOML change |
| Discoverability | ❌ Users must read engine docs | ✅ IDE autocomplete on traits |
| Type safety | ❌ String flags | ✅ Rust enums and structs |
| Testing | ❌ Mock entire engine CLI | ✅ Mock trait implementations |
| Documentation | ❌ 200+ flags to document | ✅ 16 traits to document |

## Consequences

### Positive
- Stable API that survives engine version upgrades
- Small, discoverable config surface
- Engine swapping without pipeline changes
- Type-safe configuration (compile-time errors, not runtime)
- Easy to mock for testing
- Clear boundary between "what to compute" and "how to compute it"

### Negative
- Advanced users may need an escape hatch for engine-specific flags
- Adapter code must be updated when engines add new capabilities
- Some engine features may not be expressible through the small config

### Mitigated
- Escape hatch via `engine.overrides` map in config:
  ```toml
  [steps.params]
  detector = "sift"
  [steps.params.engine_overrides]
  use_gpu = "true"
  gpu_index = "0"
  ```
- New engine capabilities are added as new config fields, not new traits
- The trait API is versioned and follows semver

## See Also

- [Engine registry](../architecture/engine-registry.md) — full trait signatures and resolution
- [Engine assignment](../photogrammetry/engine-assignment.md) — which engine implements which trait
- [ADR-001: Step-scoped tools](./001-step-scoped-vs-odm-monolith.md) — why per-engine adapters
