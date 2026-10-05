---
id: ND-30
title: Qualify and enforce the V0 runtime profile
status: To Do
assignee: []
created_date: '2026-10-05 12:12'
updated_date: '2026-10-05'
labels:
  - v0.1
  - engines
  - executor
milestone: m-0
dependencies:
  - ND-2
references:
  - .knowledge/architecture/v0-strict-sparse-profile.md
  - .knowledge/photogrammetry/colmap-integration.md
  - backlog/docs/decisions/012-v0-strict-sparse-reconstruction-profile.md
priority: high
type: task
ordinal: 2500
---

## Description

Resolve COLMAP only from a declared qualified Pixi or OCI runtime, fingerprint it completely, and run the fixed CPU-only recipe without ambient PATH discovery or user-supplied engine flags.

## Acceptance Criteria

- [ ] #1 Reject missing, changed, ambient PATH, GPU-enabled or otherwise unqualified COLMAP before any V0 stage writes promotable output.
- [ ] #2 Record runtime/package digest, executable digest and version, architecture, libc/OS profile, thread count, locale, timezone, environment allowlist, arguments and GPU-disabled state.
- [ ] #3 The developer profile is the locked Pixi environment; the end-user profile is an official OCI image by digest; system COLMAP is explicitly reported as unqualified.
- [ ] #4 The fixed V0 recipe exposes no raw engine overrides, no CUDA/GPU toggle, and no alternate matcher/mapper strategy.
- [ ] #5 Tests cover qualified, missing, changed, ambient, wrong-architecture and GPU-capable profiles, plus provenance stability for equivalent qualified launches.
