---
id: ND-12
title: Make nadir process emit SparseScene v1 for the strict V0 profile
status: To Do
assignee: []
created_date: '2026-09-30 19:25'
updated_date: '2026-10-05'
labels:
  - v0.1
  - cli
milestone: m-0
dependencies:
  - ND-11
references:
  - backlog/docs/roadmaps/v0-mvp.md
  - .knowledge/context.md
  - .knowledge/architecture/v0-strict-sparse-profile.md
priority: high
type: task
ordinal: 12000
---

## Description

<!-- SECTION:DESCRIPTION:BEGIN -->
Replace the deliberate `nadir process` scaffold failure with the ADR-012 V0 contract. The command accepts one image directory, runs the fixed local sparse profile, materializes a user output copy of `SparseScene v1`, and refuses flags or modes that imply mapping products, dense reconstruction, georeferencing, GPU execution, custom recipes or a worker service.
<!-- SECTION:DESCRIPTION:END -->

## Acceptance Criteria
<!-- AC:BEGIN -->
- [ ] #1 `nadir process ./images` runs the fixed V0 profile and writes `outputs/sparse_scene_v1/manifest.json`, `cameras.json` and `points.ply` as user-owned copies or reflinks outside the cache.
- [ ] #2 Progress is one line per V0 stage with elapsed time, executed-vs-cached status, engine/runtime identity and a contract statistic such as admitted images, matched pairs, registered images or sparse points.
- [ ] #3 CLI help and errors state that V0 has no `--outputs` mapping products, georeferencing, dense reconstruction, GPU mode, worker submission or custom recipe execution.
- [ ] #4 Exit codes distinguish success, input/contract failure, missing/unqualified runtime, interrupted execution and internal error without pretending a partial SparseScene is complete.
- [ ] #5 The command can print machine-readable run evidence containing invocation keys, output-tree digests and acceptance facts for ND-14 qualification.
<!-- AC:END -->
