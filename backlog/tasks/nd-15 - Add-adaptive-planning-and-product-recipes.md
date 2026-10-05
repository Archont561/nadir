---
id: ND-15
title: Add adaptive planning and product recipes
status: To Do
assignee: []
created_date: '2026-09-30 20:38'
updated_date: '2026-10-05'
labels:
  - v1.0
  - pipeline
  - reconstruction
dependencies:
  - ND-10
references:
  - backlog/docs/roadmaps/v1-platform.md
  - .knowledge/photogrammetry/engine-assignment.md
  - .knowledge/architecture/v0-strict-sparse-profile.md
  - backlog/docs/decisions/012-v0-strict-sparse-reconstruction-profile.md
priority: medium
type: feature
ordinal: 15000
---

## Description

<!-- SECTION:DESCRIPTION:BEGIN -->
Select matching and SfM strategies from dataset characteristics, then prune the DAG to the requested product and pipeline variant.
<!-- SECTION:DESCRIPTION:END -->

## Acceptance Criteria
<!-- AC:BEGIN -->
- [ ] #1 Preflight selects documented matching and SfM strategies from image count, GPS and RTK availability
- [ ] #2 Product targets prune unnecessary DAG branches while preserving required dependencies
- [ ] #3 Named pipeline variants and the selected strategy are visible in nadir plan and process output
<!-- AC:END -->
