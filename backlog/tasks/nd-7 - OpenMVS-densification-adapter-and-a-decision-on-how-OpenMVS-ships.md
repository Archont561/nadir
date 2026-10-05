---
id: ND-7
title: Add post-V0 OpenMVS densification and shipping decision
status: To Do
assignee: []
created_date: '2026-09-30 19:25'
updated_date: '2026-10-05'
labels:
  - v1.0
  - reconstruction
  - engines
  - blocked
milestone: m-1
dependencies:
  - ND-5
  - ND-6
references:
  - .knowledge/photogrammetry/openmvs-integration.md
  - .knowledge/deployment/pixi-setup.md
  - backlog/docs/roadmaps/v1-platform.md
  - backlog/docs/decisions/012-v0-strict-sparse-reconstruction-profile.md
priority: medium
type: feature
ordinal: 7000
---

## Description

<!-- SECTION:DESCRIPTION:BEGIN -->
OpenMVS and dense reconstruction are not V0 acceptance work. After `SparseScene v1` and the georeferencing contract are qualified, decide how OpenMVS ships and implement InterfaceCOLMAP/DensifyPointCloud as post-V0 cached stages.
<!-- SECTION:DESCRIPTION:END -->

## Acceptance Criteria
<!-- AC:BEGIN -->
- [ ] #1 An ADR records the OpenMVS shipping route — source task, Docker/OCI stage, conda-forge submission or another qualified profile — and the offline-sandbox cost.
- [ ] #2 The adapter consumes accepted sparse/georeferenced scene artifacts and publishes dense artifacts without changing V0 `SparseScene v1` semantics.
- [ ] #3 Missing or unqualified OpenMVS uses the same honest runtime-failure taxonomy as other engines.
- [ ] #4 Dense outputs are cached, validated and recorded with engine/runtime identity before any surface or product task can consume them.
<!-- AC:END -->
