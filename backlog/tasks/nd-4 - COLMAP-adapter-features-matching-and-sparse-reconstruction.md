---
id: ND-4
title: 'COLMAP adapter: features, matching and sparse reconstruction'
status: To Do
assignee: []
created_date: '2026-09-30 19:25'
labels:
  - v0.1
  - reconstruction
  - engines
milestone: m-0
dependencies:
  - ND-2
  - ND-3
references:
  - .knowledge/photogrammetry/colmap-integration.md
  - backlog/docs/decisions/007-engine-trait-api-surface.md
priority: high
ordinal: 4000
---

## Description

<!-- SECTION:DESCRIPTION:BEGIN -->
The reconstruction domain against the real tool. Three COLMAP invocations — feature_extractor, exhaustive_matcher, mapper — behind the engine trait, with the flag surfaces hidden inside the adapter rather than leaking into the pipeline (ADR-007). COLMAP is pinned to the CPU build in pixi.toml, so this must work without a GPU.
<!-- SECTION:DESCRIPTION:END -->

## Acceptance Criteria
<!-- AC:BEGIN -->
- [ ] #1 Each of the three stages is a separate cached artifact, so a rerun that changes only mapper parameters reuses the matches.
- [ ] #2 Quality presets map to concrete COLMAP flags in one place in the adapter, not at the call sites.
- [ ] #3 COLMAP progress output is parsed into structured progress events rather than echoed verbatim.
- [ ] #4 The adapter reports the COLMAP version it invoked, and refuses a version it does not support instead of guessing.
<!-- AC:END -->
