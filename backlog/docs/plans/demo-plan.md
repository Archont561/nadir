---
type: Deployment Guide
title: Post-V0 Hosted Demo Plan
description: "Hosted product demo after ADR-012 strict sparse V0 is qualified"
purpose: Sketches a later cloud/UI demonstration without moving worker service or mapping products into V0.
last_updated: 2026-10-05
status: superseded-in-part
related:
  - ../../../.knowledge/deployment/hosting-comparison.md
  - ../../../.knowledge/deployment/docker-strategy.md
  - ../roadmaps/v0-mvp.md
  - ../roadmaps/v1-platform.md
  - ../roadmaps/v2-scale.md
  - ../decisions/004-inverted-container-no-dind.md
  - ../decisions/012-v0-strict-sparse-reconstruction-profile.md
---

# Post-V0 Hosted Demo Plan

> **ADR-012 scope note:** this document is not a V0.1 plan. V0.1 is a local
> strict sparse reconstruction profile only: one bounded calibrated ImageSet,
> immutable snapshot, qualified CPU-only COLMAP execution, verified cache reuse,
> and canonical local `SparseScene v1`. V0.1 has no georeferencing, dense
> reconstruction, mapping products, GPU execution, HTTP worker service, UI or
> cloud control plane.

## TL;DR

The hosted demo remains useful as a later product exploration, but it must be
sequenced after local contracts are proven:

1. **V0.1 prerequisite:** local CLI produces accepted `SparseScene v1` and cache
   evidence.
2. **V1 prerequisite:** georeferencing, dense reconstruction and mapping
   products consume explicit artifacts and have their own qualification evidence.
3. **V2 prerequisite:** Worker Protocol/HTTP serving defines submission,
   status, cancellation, events and runtime isolation.
4. **Hosted demo:** a web app orchestrates those already-qualified capabilities.

The original Astro + Clerk + Postgres + Drizzle + Inngest + object-storage idea
is a plausible later shape, not an input to V0 acceptance.

## What a Later Demo Should Prove

A post-V0 hosted demo should prove product workflow, not redefine the V0 engine
contract:

1. Authenticated project creation and image upload.
2. Durable job orchestration using the accepted Worker Protocol.
3. Progress and provenance surfaced from the same local executor/cache model.
4. Download or preview of already-qualified artifacts.
5. Clear separation between local sparse output, georeferenced products and
   hosted job state.

It must not claim that V0 includes worker service, remote execution,
georeferencing, dense reconstruction, DSMs, orthomosaics, GeoTIFFs or maps.

## Scope by Release

| Capability | V0.1 | V1.0 | V2.0 | Hosted demo |
|---|---:|---:|---:|---:|
| Immutable calibrated ImageSet snapshot | ✅ | ✅ | ✅ | Uses |
| CPU-only COLMAP sparse path | ✅ | ✅ | ✅ | Uses |
| `SparseScene v1` local output | ✅ | ✅ | ✅ | Uses/downloads |
| Verified stage cache | ✅ | ✅ | ✅ | Surfaces |
| Georeferencing / CRS | ❌ | ✅ | ✅ | Optional after V1 |
| Dense reconstruction / OpenMVS | ❌ | ✅ | ✅ | Optional after V1 |
| DSM/orthomosaic/GeoTIFF products | ❌ | ✅ | ✅ | Optional after V1 |
| HTTP worker service | ❌ | ❌ | ✅ | Required |
| Browser map preview | ❌ | product-dependent | product-dependent | Optional |

## Architecture Sketch After V2

```text
Browser
  ↓
Web app / BFF
  ↓
Database-backed job state ── object storage
  ↓
Worker Protocol client
  ↓
Nadir worker service
  ↓
Same executor + artifact store + qualified runtime profiles as local CLI
```

The worker image may follow the inverted-container model from ADR-004, but the
runtime identity must still be explicit: image digest, engine package digests,
COLMAP/OpenMVS/GDAL/PROJ/PDAL versions as applicable, resource limits and
network/storage permissions.

## Guardrails for Reviving This Plan

- Do not bypass `SparseScene v1` by scraping COLMAP workspaces.
- Do not introduce ODM wrapping as a shortcut around step-scoped artifacts.
- Do not make the web app responsible for photogrammetry domain rules.
- Do not store mutable cache paths as user-visible outputs.
- Do not expose database URLs, R2 credentials or worker secrets to browser code.
- Do not add a worker service to V0.1 tasks or acceptance criteria.
- Do not show map previews unless the georeferenced product contract exists.

## Later Implementation Outline

### Phase A — Local contracts exist

- V0.1 strict sparse profile is accepted and documented.
- V1 georeferencing/dense/product tasks are accepted if the demo needs maps.
- CLI and Worker Protocol return the same artifact/provenance vocabulary.

### Phase B — Worker service exists

- Worker exposes submission, status, cancellation and event streaming through
  the accepted protocol.
- Jobs execute through the same verified artifact store as local CLI runs.
- Runtime profiles and resource limits are visible in job evidence.

### Phase C — Web product shell

- Build a web app with authenticated projects and uploads.
- Store job metadata in a database and image/artifact bytes in object storage.
- Submit jobs through the Worker Protocol rather than shelling directly from the
  web process.
- Render artifact inventory, provenance and optional product previews.

## Demo Script After Prerequisites

```text
1. Sign in and create a project.
2. Upload a bounded calibrated ImageSet.
3. Submit a job to the worker service.
4. Watch progress from snapshot → features → matching → SfM → SparseScene v1
   and, if V1 products are enabled, later georeferencing/product stages.
5. Open the provenance panel to see invocation keys, output-tree digests,
   runtime identities and cache-hit status.
6. Download the accepted sparse scene and any separately qualified products.
```

## See Also

- [V0 MVP roadmap](../roadmaps/v0-mvp.md) — local strict sparse prerequisite
- [V1 Platform roadmap](../roadmaps/v1-platform.md) — mapping products after V0
- [V2 Scale roadmap](../roadmaps/v2-scale.md) — Worker Protocol and remote execution
- [ADR-012](../decisions/012-v0-strict-sparse-reconstruction-profile.md)
- [Docker strategy](../../../.knowledge/deployment/docker-strategy.md)
