---
type: Roadmap
title: V1 Platform Details
description: "V1.0 adaptive planning, QC, recipes, resource scheduling"
purpose: V1.0 adaptive planning, QC, recipes, resource scheduling
last_updated: 2025-02-23
status: stable
related:
  - full-roadmap.md
  - v0-mvp.md
  - ../implementation/qc-and-adaptive.md
---

# V1 Platform Details

## V1.0 — Production Processing Platform

**Goal:** A flexible, adaptive, resource-aware photogrammetry platform that
handles diverse mission types on a single machine.

**Timeline:** ~6 months after V0.2.

### Adaptive Pipeline Planning

- [ ] Matching strategy by image count:
  - < 300 → exhaustive
  - 300–3,000 with GPS → spatial (k=50)
  - > 3,000 → vocabulary tree
- [ ] SfM strategy:
  - RTK available → pose prior mapper
  - Large dataset → hierarchical mapper
  - Default → incremental mapper
- [ ] COLMAP spatial_matcher and vocab_tree_matcher adapters
- [ ] COLMAP hierarchical_mapper and pose_prior_mapper adapters

### Product Recipes

- [ ] `--product orthomosaic`: prune mesh, DTM, texture
- [ ] `--product terrain`: prune ortho, mesh; add ground classification + DTM
- [ ] `--product 3d-model`: prune DSM, DTM, ortho; keep mesh + texture
- [ ] `--product full`: all products
- [ ] DAG pruning algorithm (walk backwards from targets)

### Pipeline Variants

- [ ] `pipelines/fast-ortho.toml`
- [ ] `pipelines/dem-only.toml`
- [ ] `pipelines/model-only.toml`
- [ ] `pipelines/odm-compatible.toml`
- [ ] `--pipeline <name>` CLI flag

### Additional Engine Adapters

- [ ] DTM generation (PDAL SMRF + Rust IDW)
- [ ] Mesh + texture (OpenMVS ReconstructMesh + TextureMesh)
- [ ] Point cloud export (LAS/LAZ via `las` crate)
- [ ] COLMAP dense MVS alternative (patch_match_stereo + stereo_fusion)

### Resource-Aware Scheduling

- [ ] `ResourceRequirements` per task (CPU, RAM, GPU, disk)
- [ ] `ResourceScheduler` with Tokio semaphores
- [ ] Concurrent DAG branch execution (DSM + Mesh in parallel)
- [ ] Disk space checking before task launch
- [ ] Memory-aware task queuing
- [ ] Config: `[processing] max_cpu = 16, max_gpu = 1`

### Advanced Georeferencing

- [ ] GCP file parser (CSV format)
- [ ] GCP-constrained bundle adjustment (COLMAP model_aligner)
- [ ] GCP RMSE computation and QC gate
- [ ] User-specified output CRS
- [ ] PROJ reprojection of all output artifacts

### Expected CLI Output

```bash
$ nadir process ./flight_042 --product full --threads 16
Preflight: 2,431 images, RTK, GSD 2.1cm, overlap 80%/68%  [OK]
Strategy:  spatial matching (k=50), pose-prior SfM, OpenMVS dense

✓ ingest          3.2s
✓ features       24m 11s  colmap (spatial)
✓ matching        8m 44s  colmap (spatial, k=50)
✓ sfm            14m 02s  colmap (pose-prior)  →  QC: 99.1%, RMSE 0.31px  [PASS]
✓ georef          1.1s    EPSG:32632, GCP RMSE 0.012m
✓ dense          51m 33s  openmvs (182M points)
✓ filter          4m 12s  pdal (SMRF)
│
├─✓ dsm           3m 44s  (parallel with mesh)
├─✓ dtm           2m 58s  (parallel with mesh)
├─✓ mesh          8m 21s  (parallel with DSM/DTM)
└─✓ texture       6m 17s

✓ ortho           7m 02s  (COG)
✓ report          0.4s
```

## See Also

- [Full roadmap](./full-roadmap.md)
- [QC & adaptive](../implementation/qc-and-adaptive.md)
- [V2 Scale](./v2-scale.md)
