---
type: Roadmap
title: V0 MVP Details
description: "V0.1 and V0.2 milestones, deliverables, CLI output examples"
purpose: V0.1 and V0.2 milestones, deliverables, CLI output examples
last_updated: 2025-02-23
status: stable
related:
  - full-roadmap.md
  - ../plans/demo-plan.md
  - ../../../.knowledge/architecture/pipeline-dag.md
---

# V0 MVP Details

## V0.1 — Minimum Viable Pipeline

**Goal:** `nadir process ./images` produces a georeferenced orthomosaic,
DSM, and point cloud from drone imagery on a single workstation.

**Timeline:** 3–4 weeks solo, full-time.

### Milestones

**Week 1–2: Infrastructure + Dataset + COLMAP**
- [ ] Cargo workspace scaffolding
- [ ] CLI skeleton (`clap`)
- [ ] Config parsing (`serde` + `figment`)
- [ ] Process runner (`tokio::process`)
- [ ] Artifact store (`blake3` hashing)
- [ ] Image ingest (`kamadak-exif`)
- [ ] COLMAP adapter (feature_extractor, exhaustive_matcher, mapper)
- [ ] COLMAP binary model parser

**Week 3: OpenMVS + Geometry**
- [ ] OpenMVS adapter (InterfaceCOLMAP, DensifyPointCloud)
- [ ] Umeyama georeferencing transform (`nalgebra`)
- [ ] PROJ CRS conversion

**Week 4: Surface + Cartography + Pipeline**
- [ ] IDW DSM rasterizer (`rayon`)
- [ ] GeoTIFF/COG writer (`gdal` crate)
- [ ] TOML pipeline parser (`petgraph`)
- [ ] Topological executor with cache check
- [ ] `nadir serve --port 8080`
- [ ] Docker deployment

### Expected CLI Output

```bash
$ nadir inspect ./flight_042
Images:       1,284
Cameras:      DJI FC6310 (1,284)
GPS:          1,284/1,284 (100%)
RTK:          yes
GSD:          ~2.4 cm
Overlap:      78% forward, 65% side
Issues:       none

$ nadir process ./flight_042 --quality survey --outputs orthomosaic,dsm
✓ ingest          2.1s    1,284 images
✓ features       18m 42s  colmap (sift, 8192)
✓ matching        7m 13s  colmap (exhaustive)
✓ sfm            11m 04s  colmap (98.2% registered, RMSE 0.42px)
✓ georef          0.8s    EPSG:32632
✓ dense          42m 17s  openmvs (182M points)
✓ dsm             3m 22s  builtin (IDW, 5cm)
✓ ortho           5m 11s  builtin (COG, 3cm)

Outputs:
  ./flight_042/outputs/orthomosaic.cog.tif  (412 MB)
  ./flight_042/outputs/dsm.cog.tif          (89 MB)
  ./flight_042/outputs/dense_cloud.laz      (2.1 GB)
```

---

## V0.2 — Reliability & Inspection

**Goal:** Users can inspect datasets before processing, resume interrupted
runs, understand why outputs look the way they do, and trust the results.

**Timeline:** 2–3 weeks after V0.1.

### Milestones

**Preflight**
- [ ] Image count validation
- [ ] Camera model consistency check
- [ ] GPS coverage analysis
- [ ] GSD estimation
- [ ] Overlap estimation
- [ ] `nadir inspect` command with `--json` flag

**Quality Control**
- [ ] `QualityGate` trait
- [ ] SfM gate (registered %, RMSE, components)
- [ ] Dense gate (point density, coverage)
- [ ] DSM gate (nodata ratio)
- [ ] QC status: Pass / Warning / Fail
- [ ] Integrate gates into executor

**Resume & Recovery**
- [ ] Persist task state to disk
- [ ] Detect incomplete artifacts
- [ ] `nadir resume` command
- [ ] Retry policy (retryable vs fatal failures)

**Provenance**
- [ ] `LineageRecord` per artifact
- [ ] Recursive lineage traversal
- [ ] `nadir explain <artifact>` command

**Reporting**
- [ ] Dataset statistics
- [ ] Reconstruction metrics
- [ ] Georeferencing accuracy
- [ ] Product inventory
- [ ] `nadir report` command

### Expected CLI Output

```bash
$ nadir explain ./flight_042/outputs/dsm.tif
DSM: dsm.tif
  ├─ Hash:    blake3:p6q7r8...
  ├─ Engine:  nadir-surface v0.1.0 (IDW)
  ├─ Params:  { resolution: 0.05 }
  └─ Dense Cloud: cache/m3n4o5...
       ├─ Engine: openmvs v2.2.0
       └─ SfM: cache/j0k1l2...
            ├─ Engine: colmap v3.9.1
            └─ 1,284 images (98.2% registered)

$ nadir report ./flight_042
Dataset:      1,284 images, DJI FC6310, RTK
Registration: 98.2% (1,261/1,284)
Sparse:       4.2M points, RMSE 0.42px
Dense:        182M points
CRS:          EPSG:32632
Products:     orthomosaic ✓  dsm ✓  point cloud ✓
```

## See Also

- [Full roadmap](full-roadmap.md)
- [Demo plan](../plans/demo-plan.md)
- [V1 Platform](v1-platform.md)
