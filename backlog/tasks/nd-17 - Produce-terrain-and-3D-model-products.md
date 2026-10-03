---
id: ND-17
title: Produce terrain and 3D-model products
status: To Do
assignee: []
created_date: '2026-09-30 20:38'
labels:
  - v1.0
  - surface
  - reconstruction
dependencies: []
references:
  - backlog/docs/roadmaps/v1-platform.md
  - .knowledge/photogrammetry/engine-assignment.md
priority: medium
type: feature
ordinal: 17000
---

## Description

<!-- SECTION:DESCRIPTION:BEGIN -->
Expand product coverage beyond the V0 DSM and orthomosaic with DTM, point-cloud export, mesh and texture outputs.
<!-- SECTION:DESCRIPTION:END -->

## Acceptance Criteria
<!-- AC:BEGIN -->
- [ ] #1 Terrain recipes run ground classification and produce a CRS-correct DTM
- [ ] #2 3D-model recipes create mesh and texture artifacts with documented OpenMVS adapters
- [ ] #3 LAS or LAZ point-cloud export and all new products participate in caching and provenance
<!-- AC:END -->
