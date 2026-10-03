---
id: ND-9
title: Write cloud-optimized GeoTIFFs for the DSM and the orthomosaic
status: To Do
assignee: []
created_date: '2026-09-30 19:25'
labels:
  - v0.1
  - cartography
milestone: m-0
dependencies:
  - ND-8
references:
  - .knowledge/photogrammetry/geospatial-stack.md
  - backlog/docs/roadmaps/v0-mvp.md
priority: medium
ordinal: 9000
---

## Description

<!-- SECTION:DESCRIPTION:BEGIN -->
The cartography domain: take the height grid and the orthorectified colour mosaic and write them as tiled, overviewed, compressed COGs through the gdal crate, so the outputs open in QGIS and stream from object storage without a conversion step.
<!-- SECTION:DESCRIPTION:END -->

## Acceptance Criteria
<!-- AC:BEGIN -->
- [ ] #1 Both products are written as tiled GeoTIFFs with internal overviews and a compression the format validates as cloud-optimized.
- [ ] #2 The CRS, geotransform and nodata value written to the file match the grid metadata exactly.
- [ ] #3 A gdal driver or CRS failure is a typed error naming the product, not a partially written file left on disk.
- [ ] #4 Output paths follow the layout in the V0.1 CLI example: outputs/orthomosaic.cog.tif and outputs/dsm.cog.tif.
<!-- AC:END -->
