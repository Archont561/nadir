---
id: ND-22
title: Compile the pure Rust math core for WASM
status: To Do
assignee: []
created_date: '2026-09-30 20:38'
labels:
  - v2.5
  - wasm
  - math
dependencies: []
references:
  - backlog/docs/roadmaps/wasm-vision.md
  - backlog/docs/decisions/008-wasm-edge-browser-strategy.md
priority: low
type: feature
ordinal: 22000
---

## Description

<!-- SECTION:DESCRIPTION:BEGIN -->
Make the portable Nadir math and raster core build for wasm32 while keeping native and browser results consistent.
<!-- SECTION:DESCRIPTION:END -->

## Acceptance Criteria
<!-- AC:BEGIN -->
- [ ] #1 The WASM-compatible math crate exposes documented transforms, rasterization and safe typed inputs
- [ ] #2 Native and wasm32 builds pass shared deterministic test vectors
- [ ] #3 The build clearly rejects or isolates GDAL and PROJ paths that cannot run in the browser
<!-- AC:END -->
