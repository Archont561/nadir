---
id: ND-6
title: Add post-V0 local-to-CRS georeferencing from SparseScene v1
status: To Do
assignee: []
created_date: '2026-09-30 19:25'
updated_date: '2026-10-05'
labels:
  - v1.0
  - geometry
  - math
milestone: m-1
dependencies:
  - ND-5
references:
  - .knowledge/photogrammetry/geospatial-stack.md
  - .knowledge/architecture/v0-strict-sparse-profile.md
  - .knowledge/architecture/five-domains.md
  - backlog/docs/decisions/012-v0-strict-sparse-reconstruction-profile.md
  - backlog/docs/roadmaps/v1-platform.md
priority: medium
type: feature
ordinal: 6000
---

## Description

<!-- SECTION:DESCRIPTION:BEGIN -->
Georeferencing is explicitly outside V0. This follow-on task consumes an accepted local-coordinate `SparseScene v1`, fits a similarity transform against GPS/RTK/GCP evidence, and publishes a separate georeferenced scene artifact with an explicit CRS, scale and accuracy statement.
<!-- SECTION:DESCRIPTION:END -->

## Acceptance Criteria
<!-- AC:BEGIN -->
- [ ] #1 The input contract requires a validated `SparseScene v1`; the task never mutates or reinterprets the V0 local scene in place.
- [ ] #2 The Umeyama fit returns rotation, translation and scale plus per-image residuals, outlier reporting and an overall RMSE.
- [ ] #3 Target CRS selection uses documented defaults and explicit overrides, with PROJ failures reported as typed errors.
- [ ] #4 The output carries CRS, scale, transform provenance and quality metrics separately from the original local sparse scene.
<!-- AC:END -->
