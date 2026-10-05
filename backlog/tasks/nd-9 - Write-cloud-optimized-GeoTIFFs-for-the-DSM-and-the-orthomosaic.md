---
id: ND-9
title: Write post-V0 GeoTIFF mapping products
status: To Do
assignee: []
created_date: '2026-09-30 19:25'
updated_date: '2026-10-05'
labels:
  - v1.0
  - cartography
milestone: m-1
dependencies:
  - ND-8
references:
  - .knowledge/photogrammetry/geospatial-stack.md
  - backlog/docs/roadmaps/v1-platform.md
  - backlog/docs/decisions/012-v0-strict-sparse-reconstruction-profile.md
priority: medium
type: feature
ordinal: 9000
---

## Description

<!-- SECTION:DESCRIPTION:BEGIN -->
GeoTIFFs, COGs, DSMs and orthomosaics are no longer V0. This post-V0 cartography task writes qualified mapping products from georeferenced surface and imagery artifacts after those upstream contracts exist.
<!-- SECTION:DESCRIPTION:END -->

## Acceptance Criteria
<!-- AC:BEGIN -->
- [ ] #1 Mapping products are written as tiled GeoTIFF/COG files with internal overviews and compression validated by GDAL tooling.
- [ ] #2 CRS, geotransform, nodata and resolution in each file match the source artifact metadata exactly.
- [ ] #3 GDAL driver, CRS or write failures are typed errors naming the product and leave no partially promoted output.
- [ ] #4 Product paths and naming are documented in the post-V0 roadmap, not in the V0 strict sparse profile.
<!-- AC:END -->
