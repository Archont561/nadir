---
id: ND-7
title: 'OpenMVS densification adapter, and a decision on how OpenMVS ships'
status: To Do
assignee: []
created_date: '2026-09-30 19:25'
labels:
  - v0.1
  - reconstruction
  - engines
  - blocked
milestone: m-0
dependencies:
  - ND-5
  - ND-6
references:
  - .knowledge/photogrammetry/openmvs-integration.md
  - .knowledge/deployment/pixi-setup.md
priority: medium
ordinal: 7000
---

## Description

<!-- SECTION:DESCRIPTION:BEGIN -->
OpenMVS has no conda-forge package for linux-64, so unlike COLMAP it is not simply a dependency: pixi run build-openmvs builds it from source, and a binary installed that way is only offline-restorable if a sandbox-pack ran afterwards. This task is the adapter (InterfaceCOLMAP then DensifyPointCloud) and the decision about how the engine reaches a user machine, written down as an ADR.
<!-- SECTION:DESCRIPTION:END -->

## Acceptance Criteria
<!-- AC:BEGIN -->
- [ ] #1 The adapter converts the COLMAP sparse model with InterfaceCOLMAP and densifies it with DensifyPointCloud, as two cached artifacts.
- [ ] #2 A missing OpenMVS binary is the same honest exit-code-3 path as any other absent engine, with a message naming pixi run build-openmvs.
- [ ] #3 An ADR records the shipping route chosen — source task, Docker stage, or conda-forge submission — and what it costs the offline sandbox.
- [ ] #4 The dense cloud is written as LAZ via PDAL and carries the CRS from georeferencing.
<!-- AC:END -->
