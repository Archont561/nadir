---
id: ND-18
title: Partition and merge large photogrammetry datasets
status: To Do
assignee: []
created_date: '2026-09-30 20:38'
labels:
  - v2.0
  - dataset
  - executor
dependencies: []
references:
  - backlog/docs/roadmaps/v2-scale.md
priority: low
type: feature
ordinal: 18000
---

## Description

<!-- SECTION:DESCRIPTION:BEGIN -->
Process 10,000-plus-image datasets by partitioning them spatially with overlap, executing tiles in parallel, and merging the resulting models and mapping products.
<!-- SECTION:DESCRIPTION:END -->

## Acceptance Criteria
<!-- AC:BEGIN -->
- [ ] #1 A DatasetPartitioner creates content-addressed, overlap-aware spatial tile artifacts
- [ ] #2 The planner represents partition, parallel tile processing and merge as a resumable DAG
- [ ] #3 Merged point clouds and rasters are deduplicated and validated against a documented large-dataset fixture
<!-- AC:END -->
