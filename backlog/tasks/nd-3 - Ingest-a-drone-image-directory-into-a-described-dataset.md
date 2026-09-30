---
id: ND-3
title: Ingest a drone image directory into a described dataset
status: To Do
assignee: []
created_date: '2026-09-30 19:25'
labels:
  - v0.1
  - dataset
milestone: m-0
dependencies:
  - ND-1
references:
  - .knowledge/architecture/five-domains.md
  - .knowledge/photogrammetry/pipeline-stages.md
priority: high
ordinal: 3000
---

## Description

<!-- SECTION:DESCRIPTION:BEGIN -->
The first real stage: read a directory of overlapping photographs and produce the dataset description everything downstream needs — per-image camera model, GPS position, RTK flag, timestamp and orientation — from EXIF, without loading pixel data.
<!-- SECTION:DESCRIPTION:END -->

## Acceptance Criteria
<!-- AC:BEGIN -->
- [ ] #1 kamadak-exif reads make, model, focal length, GPS position and altitude for each image; images missing GPS are recorded as such rather than dropped.
- [ ] #2 The dataset reports image count, the set of distinct camera models, GPS coverage and an estimated GSD.
- [ ] #3 A directory with mixed camera models is a warning that survives into the dataset, not a silent average.
- [ ] #4 Ingest is an artifact: the same directory ingests once and hits the cache on the second run.
<!-- AC:END -->
