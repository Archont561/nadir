---
id: ND-6
title: Georeference the sparse model with a Umeyama transform and PROJ
status: To Do
assignee: []
created_date: '2026-09-30 19:25'
labels:
  - v0.1
  - geometry
  - math
milestone: m-0
dependencies:
  - ND-5
references:
  - .knowledge/photogrammetry/geospatial-stack.md
  - .knowledge/architecture/five-domains.md
priority: high
ordinal: 6000
---

## Description

<!-- SECTION:DESCRIPTION:BEGIN -->
COLMAP reconstructs in an arbitrary local frame. Fit the similarity transform from that frame to the GPS positions of the ingested images with Umeyama (nalgebra), then convert to a projected CRS with PROJ so every later product carries real coordinates.
<!-- SECTION:DESCRIPTION:END -->

## Acceptance Criteria
<!-- AC:BEGIN -->
- [ ] #1 The Umeyama fit returns rotation, translation and scale, plus per-image residuals and an overall RMSE.
- [ ] #2 Images whose GPS residual is a gross outlier are reported, and the fit can exclude them.
- [ ] #3 The target CRS is derived from the dataset centroid UTM zone by default and can be overridden explicitly.
- [ ] #4 The transform is applied to the sparse cloud and camera centres, and the result carries its EPSG code.
<!-- AC:END -->
