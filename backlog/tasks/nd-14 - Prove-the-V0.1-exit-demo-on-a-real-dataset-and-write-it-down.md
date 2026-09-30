---
id: ND-14
title: Prove the V0.1 exit demo on a real dataset and write it down
status: To Do
assignee: []
created_date: '2026-09-30 19:26'
labels:
  - v0.1
  - docs
milestone: m-0
dependencies:
  - ND-12
references:
  - .knowledge/roadmap/v0-mvp.md
  - .knowledge/log.md
priority: medium
ordinal: 14000
---

## Description

<!-- SECTION:DESCRIPTION:BEGIN -->
The milestone is not done because the code compiles; it is done when a real drone dataset goes in and a georeferenced orthomosaic, DSM and point cloud come out, with the numbers recorded so the next change can be compared against them.
<!-- SECTION:DESCRIPTION:END -->

## Acceptance Criteria
<!-- AC:BEGIN -->
- [ ] #1 A public drone dataset is processed end to end on one workstation, and the outputs open correctly in QGIS.
- [ ] #2 Registration percentage, reprojection RMSE, georeferencing RMSE, product GSDs and wall-clock time per stage are recorded.
- [ ] #3 A second run of the same dataset is served from the artifact cache and is visibly faster.
- [ ] #4 The run is written up in .knowledge, and the V0.1 checklist in roadmap/v0-mvp.md is updated to match what shipped.
<!-- AC:END -->
