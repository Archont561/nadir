---
type: Architecture Decision Record
title: "ADR-012: V0 Strict Sparse Reconstruction Profile"
description: "Narrow V0 to qualified, cached, local-coordinate sparse reconstruction before mapping products."
status: accepted
date: 2026-10-05
deciders: project lead
related:
  - ../../../.knowledge/architecture/v0-strict-sparse-profile.md
  - ../roadmaps/v0-mvp.md
---
# ADR-012: V0 Strict Sparse Reconstruction Profile

## Context
The former V0 combined sparse reconstruction, dense reconstruction,
georeferencing, cartography, serving, and caching, preventing meaningful
qualification of any boundary.

## Decision
V0 is one pinned CPU-only COLMAP path from an immutable calibrated ImageSet,
through feature extraction, exhaustive matching, and sparse reconstruction, to
a canonical local-coordinate `SparseScene v1`. It makes no CRS, scale,
georeferencing, dense-product, GPU, or multi-camera claim. The complete
contract is the [strict sparse profile](../../../.knowledge/architecture/v0-strict-sparse-profile.md).

## Consequences
The first release proves immutable stage artifacts, verified cache reuse,
correct pose semantics, containment, and deterministic fixture evidence.
Georeferencing, OpenMVS/PDAL/GDAL products, distributed execution, and language
workflow APIs are deferred and must later consume explicit artifact contracts.
