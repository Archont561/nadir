---
id: ND-16
title: 'Add quality gates, GCP georeferencing and resource-aware scheduling'
status: To Do
assignee: []
created_date: '2026-09-30 20:38'
labels:
  - v1.0
  - geometry
  - executor
dependencies: []
references:
  - backlog/docs/roadmaps/v1-platform.md
  - .knowledge/architecture/overview.md
priority: medium
type: feature
ordinal: 16000
---

## Description

<!-- SECTION:DESCRIPTION:BEGIN -->
Make production runs resource-aware and quality-gated, including GCP-constrained georeferencing and explicit preflight failures.
<!-- SECTION:DESCRIPTION:END -->

## Acceptance Criteria
<!-- AC:BEGIN -->
- [ ] #1 Each task declares CPU, memory, GPU and disk requirements and the scheduler enforces configured limits
- [ ] #2 CSV GCP input, target CRS selection and GCP RMSE are supported and recorded in provenance
- [ ] #3 Quality gates expose their measurements and stop or warn according to a documented policy
<!-- AC:END -->
