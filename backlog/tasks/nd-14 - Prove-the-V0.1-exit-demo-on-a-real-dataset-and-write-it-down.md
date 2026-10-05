---
id: ND-14
title: Prove the V0.1 strict sparse exit demo and document the evidence
status: To Do
assignee: []
created_date: '2026-09-30 19:26'
updated_date: '2026-10-05'
labels:
  - v0.1
  - docs
  - qualification
milestone: m-0
dependencies:
  - ND-12
references:
  - backlog/docs/roadmaps/v0-mvp.md
  - .knowledge/log.md
  - .knowledge/architecture/v0-strict-sparse-profile.md
  - backlog/docs/decisions/012-v0-strict-sparse-reconstruction-profile.md
priority: medium
type: task
ordinal: 14000
---

## Description

<!-- SECTION:DESCRIPTION:BEGIN -->
The V0.1 milestone is complete only when the strict sparse profile is demonstrated with real fixture evidence: one bounded calibrated ImageSet in, one accepted local-coordinate `SparseScene v1` out, deterministic fresh-store stage trees, and a same-store rerun that proves verified cache reuse by launching no COLMAP subprocesses.
<!-- SECTION:DESCRIPTION:END -->

## Acceptance Criteria
<!-- AC:BEGIN -->
- [ ] #1 An owned or license-compatible fixture ImageSet is processed on the qualified Linux x86-64 CPU profile, and the output validates as canonical `SparseScene v1` with exactly one model containing every admitted image.
- [ ] #2 Two fresh stores produce equal stage output-tree digests for the accepted fixture, or any nondeterminism is explained and blocked from V0 acceptance.
- [ ] #3 A same-store rerun records cache hits for every completed COLMAP stage and proves zero COLMAP launches while revalidating the referenced manifests.
- [ ] #4 Registration, reprojection, feature, match, point, track, observation, wall-clock, RAM, disk and timeout evidence is recorded with the image/byte/pixel/pair limits it justifies.
- [ ] #5 The run is written up in `.knowledge`, the V0 roadmap checklist is updated, and the write-up explicitly states that no georeferencing, dense reconstruction, GPU execution, worker service or mapping product was claimed.
<!-- AC:END -->
