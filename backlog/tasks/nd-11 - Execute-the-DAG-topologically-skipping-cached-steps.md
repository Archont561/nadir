---
id: ND-11
title: Execute the fixed V0 sparse profile with verified cache reuse
status: To Do
assignee: []
created_date: '2026-09-30 19:25'
updated_date: '2026-10-05'
labels:
  - v0.1
  - executor
  - cache
milestone: m-0
dependencies:
  - ND-1
  - ND-4
  - ND-5
  - ND-30
  - ND-31
references:
  - .knowledge/architecture/v0-strict-sparse-profile.md
  - .knowledge/architecture/pipeline-dag.md
  - .knowledge/architecture/artifact-model.md
  - backlog/docs/decisions/012-v0-strict-sparse-reconstruction-profile.md
priority: high
type: task
ordinal: 11000
---

## Description

<!-- SECTION:DESCRIPTION:BEGIN -->
The V0 executor runs one fixed linear profile rather than arbitrary TOML recipes: ImageSet snapshot → features → exhaustive matches → sparse reconstruction → SparseScene v1 validation. It coordinates per-invocation locks, stage lifecycle state, verified cache hits and failure cleanup so the qualification evidence can prove both fresh-store determinism and same-store zero-COLMAP reuse.
<!-- SECTION:DESCRIPTION:END -->

## Acceptance Criteria
<!-- AC:BEGIN -->
- [ ] #1 The executor runs only the accepted V0 stage sequence and rejects requests for georeferencing, dense reconstruction, products, GPU execution, worker dispatch or custom recipe selection.
- [ ] #2 Each stage transitions through planned, waiting_for_lock, staging, executing, validating, atomically_promoting and completed or a documented terminal failure state.
- [ ] #3 Cache hits acquire the invocation lock, revalidate the referenced manifest bytes and report a skip without launching COLMAP or mutating the promoted tree.
- [ ] #4 A failing, cancelled or abandoned stage leaves completed inputs intact, removes or tombstones staging candidates, and reports which contract condition failed.
- [ ] #5 Run evidence records stage timings, qualified runtime identity, invocation keys, output-tree digests and whether each stage executed or reused cache.
<!-- AC:END -->
