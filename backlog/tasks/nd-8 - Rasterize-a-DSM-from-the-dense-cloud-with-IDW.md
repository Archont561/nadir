---
id: ND-8
title: Rasterize post-V0 DSM products from qualified dense artifacts
status: To Do
assignee: []
created_date: '2026-09-30 19:25'
updated_date: '2026-10-05'
labels:
  - v1.0
  - surface
milestone: m-1
dependencies:
  - ND-6
  - ND-7
references:
  - .knowledge/architecture/five-domains.md
  - .knowledge/photogrammetry/pipeline-stages.md
  - backlog/docs/roadmaps/v1-platform.md
  - backlog/docs/decisions/012-v0-strict-sparse-reconstruction-profile.md
priority: medium
type: feature
ordinal: 8000
---

## Description

<!-- SECTION:DESCRIPTION:BEGIN -->
DSM rasterization is a mapping-product stage deferred until after V0. It consumes qualified georeferenced dense artifacts and produces a validated surface grid with CRS, resolution and nodata semantics suitable for downstream product writers.
<!-- SECTION:DESCRIPTION:END -->

## Acceptance Criteria
<!-- AC:BEGIN -->
- [ ] #1 IDW interpolation produces a float32 height grid at a requested ground resolution with documented nodata behavior.
- [ ] #2 The rasterizer is tiled and parallel, and produces identical output regardless of thread count under the qualified profile.
- [ ] #3 The grid carries CRS, origin, pixel size and nodata metadata so writers need no out-of-band geotransform.
- [ ] #4 Nodata ratio, coverage and source dense-artifact identity are reported for post-V0 quality gates.
<!-- AC:END -->
