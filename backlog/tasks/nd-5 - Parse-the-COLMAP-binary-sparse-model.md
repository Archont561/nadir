---
id: ND-5
title: Parse the COLMAP binary sparse model
status: To Do
assignee: []
created_date: '2026-09-30 19:25'
labels:
  - v0.1
  - reconstruction
milestone: m-0
dependencies:
  - ND-4
references:
  - .knowledge/photogrammetry/colmap-integration.md
priority: high
ordinal: 5000
---

## Description

<!-- SECTION:DESCRIPTION:BEGIN -->
Georeferencing, densification and every report need the sparse model as data, not as files on disk. Read cameras.bin, images.bin and points3D.bin into the workspace types: intrinsics per camera, pose and observations per image, and the sparse point cloud with track lengths.
<!-- SECTION:DESCRIPTION:END -->

## Acceptance Criteria
<!-- AC:BEGIN -->
- [ ] #1 cameras.bin, images.bin and points3D.bin round-trip into typed structures with the camera models COLMAP actually emits for drone imagery.
- [ ] #2 A truncated or version-mismatched file is a typed error naming the file, not a panic.
- [ ] #3 Registered-image count and reprojection RMSE are computed from the parsed model and reported by the stage.
- [ ] #4 Tests run against a small committed fixture model rather than a full reconstruction.
<!-- AC:END -->
