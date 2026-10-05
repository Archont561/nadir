---
id: ND-5
title: Convert COLMAP output to canonical SparseScene v1
status: To Do
assignee: []
created_date: '2026-09-30 19:25'
updated_date: '2026-10-05'
labels:
  - v0.1
  - reconstruction
  - artifacts
milestone: m-0
dependencies:
  - ND-4
references:
  - .knowledge/photogrammetry/colmap-integration.md
  - .knowledge/architecture/v0-strict-sparse-profile.md
  - backlog/docs/decisions/012-v0-strict-sparse-reconstruction-profile.md
priority: high
type: task
ordinal: 5000
---

## Description

<!-- SECTION:DESCRIPTION:BEGIN -->
Parse COLMAP's `cameras.bin`, `images.bin` and `points3D.bin`, enforce the strict V0 acceptance policy, and publish the canonical local-coordinate `SparseScene v1` exchange artifact. V0 exports no CRS, scale, dense cloud, DSM, orthomosaic or mapping product.
<!-- SECTION:DESCRIPTION:END -->

## Acceptance Criteria
<!-- AC:BEGIN -->
- [ ] #1 Parsing handles the supported COLMAP camera models emitted by the V0 calibrated ImageSet path and returns typed errors for truncated, malformed or incompatible binary files.
- [ ] #2 Acceptance requires exactly one COLMAP model containing every admitted image, finite intrinsics/poses/points, a nonempty sparse cloud, and internally consistent tracks and observations.
- [ ] #3 COLMAP world-to-camera poses are converted to camera centres `-Rᵀ × t` and camera-to-world orientation `Rᵀ`; quaternion signs and ordering are canonicalized.
- [ ] #4 `SparseScene v1` contains canonical `manifest.json`, ordered `cameras.json` and little-endian `points.ply` in `local_sfm` coordinates with no CRS, GPS, scale or georeferencing claim.
- [ ] #5 Registration, connectivity, feature, match, point, observation, track and reprojection facts are recorded separately from a versioned quality-policy verdict.
<!-- AC:END -->
