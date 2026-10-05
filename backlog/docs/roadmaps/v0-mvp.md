---
type: Roadmap
title: V0 MVP Details
description: "ADR-012 strict sparse V0.1 and sparse-profile V0.2 details"
purpose: ADR-012 strict sparse V0.1 and sparse-profile V0.2 details
last_updated: 2026-10-05
status: stable
related:
  - full-roadmap.md
  - ../plans/demo-plan.md
  - ../../../.knowledge/architecture/v0-strict-sparse-profile.md
  - ../../../.knowledge/architecture/pipeline-dag.md
  - ../decisions/012-v0-strict-sparse-reconstruction-profile.md
---

# V0 MVP Details

## V0.1 — Strict Sparse Reconstruction Profile

**Goal:** `nadir process ./images` accepts one bounded calibrated ImageSet,
creates an immutable input snapshot, runs the qualified CPU-only COLMAP sparse
path, verifies stage-level cache artifacts, and writes one canonical
local-coordinate `SparseScene v1`.

**Non-goals:** V0.1 has no georeferencing, CRS, scale, dense reconstruction,
OpenMVS, PDAL, GDAL, DSM/DTM, orthomosaic, mesh, GPU execution, HTTP worker
service, custom DAG recipes or mapping products.

**Timeline:** evidence-driven; completion is based on fixture qualification,
not a calendar promise.

### Contract Checklist

- [ ] One input root resolves to one bounded calibrated ImageSet.
- [ ] Input bytes are snapshotted immutably; symlinks, unsafe hard links and
      path escapes are rejected.
- [ ] Runtime qualification selects a locked Pixi developer profile or an
      official OCI image by digest; ambient COLMAP is unqualified.
- [ ] COLMAP runs CPU-only feature extraction, exhaustive matching and mapper
      stages with fixed adapter-owned flags.
- [ ] Stage cache entries map invocation keys to verified output-tree
      manifests; invocation keys are not treated as output digests.
- [ ] Cache hits revalidate bytes and launch no COLMAP subprocesses.
- [ ] `SparseScene v1` is the only exchange artifact: `manifest.json`,
      canonical `cameras.json`, and little-endian `points.ply` in `local_sfm`
      coordinates.
- [ ] Acceptance requires exactly one COLMAP model containing every admitted
      image, finite calibration/poses/geometry and a nonempty sparse cloud.
- [ ] Structural validity is separated from a versioned quality-policy verdict.

### Implementation Milestones

**Foundation: artifacts, runtime and input safety**
- [x] ND-1 — separate invocation keys from verified output-tree digests
- [ ] ND-2 — run qualified subprocesses with honest failure taxonomy
- [ ] ND-30 — qualify and enforce the V0 runtime profile
- [ ] ND-3 — snapshot one bounded calibrated ImageSet
- [ ] ND-31 — contain untrusted inputs and import candidates safely

**Sparse reconstruction path**
- [ ] ND-4 — execute the fixed CPU-only COLMAP sparse path
- [ ] ND-5 — convert COLMAP output to canonical `SparseScene v1`

**Execution, CLI and evidence**
- [ ] ND-11 — run the fixed V0 profile with verified cache reuse
- [ ] ND-12 — make `nadir process` emit `SparseScene v1`
- [ ] ND-14 — prove the strict sparse exit demo and document evidence

### Expected CLI Output

```bash
$ nadir inspect ./flight_042
Images:        96 candidates
Admitted:      96 JPEG images
Calibration:   DJI FC6310 / OPENCV profile (explicit)
V0 profile:    strict-sparse-local
Issues:        none

$ nadir process ./flight_042
✓ snapshot       1.8s   96 admitted images, immutable ImageSet blake3:...
✓ features       4m12s  colmap cpu (8192 sift), tree blake3:...
✓ matching       1m06s  colmap exhaustive, 4560 pairs, tree blake3:...
✓ sfm            2m41s  colmap mapper, 96/96 registered, 128k sparse points
✓ sparse_scene   0.4s   SparseScene v1 local_sfm, manifest blake3:...

Outputs:
  ./flight_042/outputs/sparse_scene_v1/manifest.json
  ./flight_042/outputs/sparse_scene_v1/cameras.json
  ./flight_042/outputs/sparse_scene_v1/points.ply

Acceptance:
  one model containing every admitted image: yes
  georeferenced / dense / mapping product: no (outside V0)
```

A same-store rerun should print each COLMAP stage as a verified cache hit and
record zero COLMAP launches.

---

## V0.2 — Sparse Reliability & Inspection

**Goal:** Users and developers can inspect, explain and tune the accepted sparse
profile without expanding its product scope.

### Milestones

**Preflight and limits**
- [ ] Image count, byte, pixel and pair limit reporting
- [ ] Calibration diagnostics with exact rejection reasons
- [ ] `nadir inspect --json` for V0 ImageSet facts

**Quality Policy**
- [ ] Versioned sparse quality policy separate from structural validation
- [ ] Registration, connectivity, reprojection, track and observation gates
- [ ] Policy verdicts: pass / warning / fail with recorded measurements

**Cache and Recovery**
- [ ] `nadir cache verify` for manifests and output-tree digests
- [ ] `nadir explain <run>` for invocation keys and provenance
- [ ] Abandoned staging detection and cleanup guidance
- [ ] Retry classification for runtime, input, validation and policy failures

**Reporting**
- [ ] Fixture qualification report template
- [ ] Sparse scene inventory and provenance summary
- [ ] Regression comparison across COLMAP/runtime profile changes

### Expected CLI Output

```bash
$ nadir explain ./flight_042/outputs/sparse_scene_v1/manifest.json
SparseScene v1
  coordinate_frame: local_sfm (no CRS, no scale)
  input:            ImageSet blake3:a1b2c3... (96 admitted images)
  runtime:          pixi linux-64 colmap 3.x cpu-only digest:...
  stages:
    snapshot        executed  tree blake3:...
    features        cache hit tree blake3:...
    matching        cache hit tree blake3:...
    sfm             cache hit tree blake3:...
  acceptance:       one model, 96/96 registered, 128k points
  policy:           pass under sparse-policy/v1
```

## Deferred from V0.1

The following are intentionally moved to later roadmap labels and milestones:

- **V1.0:** georeferencing, PROJ/CRS, OpenMVS, dense reconstruction, DSM/DTM,
  orthomosaics, GeoTIFF/COG output and declarative/adaptive DAG recipes.
- **V2.0:** HTTP serving, remote workers, worker SDK workflows and distributed
  execution.
- **V3.0:** GPU execution profiles and native/browser reconstruction engines.

## See Also

- [Full roadmap](full-roadmap.md)
- [Demo plan](../plans/demo-plan.md)
- [V1 Platform](v1-platform.md)
- [ADR-012](../decisions/012-v0-strict-sparse-reconstruction-profile.md)
