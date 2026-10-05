---
id: ND-17
title: Produce terrain and 3D-model products
status: To Do
assignee: []
created_date: '2026-09-30 20:38'
updated_date: '2026-10-05'
labels:
  - v1.0
  - surface
  - reconstruction
dependencies:
  - ND-7
  - ND-8
  - ND-9
references:
  - backlog/docs/roadmaps/v1-platform.md
  - .knowledge/photogrammetry/engine-assignment.md
  - .knowledge/architecture/v0-strict-sparse-profile.md
  - backlog/docs/decisions/012-v0-strict-sparse-reconstruction-profile.md
priority: medium
type: feature
ordinal: 17000
---

## Description

<!-- SECTION:DESCRIPTION:BEGIN -->
Introduce mapping-product coverage after the V0 strict sparse profile: terrain products, point-cloud export, mesh and texture outputs that consume explicit sparse, georeferenced and dense artifact contracts.
<!-- SECTION:DESCRIPTION:END -->

## Acceptance Criteria
<!-- AC:BEGIN -->
- [ ] #1 Terrain recipes run ground classification and produce a CRS-correct DTM
- [ ] #2 3D-model recipes create mesh and texture artifacts with documented OpenMVS adapters
- [ ] #3 LAS or LAZ point-cloud export and all new products participate in caching and provenance
<!-- AC:END -->
