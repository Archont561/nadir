---
id: ND-19
title: Provide selectable native surface and cartography engines
status: To Do
assignee: []
created_date: '2026-09-30 20:38'
labels:
  - v2.0
  - surface
  - cartography
dependencies: []
references:
  - backlog/docs/roadmaps/v2-scale.md
  - .knowledge/photogrammetry/engine-assignment.md
priority: low
type: feature
ordinal: 19000
---

## Description

<!-- SECTION:DESCRIPTION:BEGIN -->
Replace selected PDAL and GDAL operations with interoperable Rust implementations without changing pipeline semantics or artifact contracts.
<!-- SECTION:DESCRIPTION:END -->

## Acceptance Criteria
<!-- AC:BEGIN -->
- [ ] #1 Built-in and external engine choices are configured per supported operation through the engine registry
- [ ] #2 Native filtering, ground classification, COG writing and orthorectification have correctness comparisons against the external baseline
- [ ] #3 Engine choice and version are captured in artifact provenance
<!-- AC:END -->
