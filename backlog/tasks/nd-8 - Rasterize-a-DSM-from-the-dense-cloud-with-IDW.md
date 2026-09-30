---
id: ND-8
title: Rasterize a DSM from the dense cloud with IDW
status: To Do
assignee: []
created_date: '2026-09-30 19:25'
labels:
  - v0.1
  - surface
milestone: m-0
dependencies:
  - ND-6
  - ND-7
references:
  - .knowledge/architecture/five-domains.md
  - .knowledge/photogrammetry/pipeline-stages.md
priority: medium
ordinal: 8000
---

## Description

<!-- SECTION:DESCRIPTION:BEGIN -->
The first product computed by nadir itself rather than by an external engine: an inverse-distance-weighted surface over the dense point cloud, parallelised with rayon, at a resolution the caller asks for in ground units.
<!-- SECTION:DESCRIPTION:END -->

## Acceptance Criteria
<!-- AC:BEGIN -->
- [ ] #1 IDW interpolation produces a float32 height grid at a requested GSD, with nodata where the search radius finds no points.
- [ ] #2 The rasterizer is tiled and parallel, and produces identical output regardless of thread count.
- [ ] #3 The grid carries its CRS, origin and pixel size, so the writer needs no out-of-band geotransform.
- [ ] #4 The nodata ratio is reported as a statistic of the stage, because V0.2 gates on it.
<!-- AC:END -->
