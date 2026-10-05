---
id: ND-4
title: Execute the fixed CPU-only COLMAP sparse path
status: To Do
assignee: []
created_date: '2026-09-30 19:25'
updated_date: '2026-10-05'
labels:
  - v0.1
  - reconstruction
  - engines
milestone: m-0
dependencies:
  - ND-1
  - ND-2
  - ND-3
  - ND-30
references:
  - .knowledge/photogrammetry/colmap-integration.md
  - .knowledge/architecture/v0-strict-sparse-profile.md
  - backlog/docs/decisions/007-engine-trait-api-surface.md
  - backlog/docs/decisions/012-v0-strict-sparse-reconstruction-profile.md
priority: high
type: task
ordinal: 4000
---

## Description

<!-- SECTION:DESCRIPTION:BEGIN -->
Implement the only qualified V0 engine path: COLMAP feature extraction, exhaustive matching and mapper execution against an immutable ImageSet snapshot, with GPU disabled and no user-selectable engine flags. Each invocation produces a verified stage artifact and records enough provenance to prove cache reuse and runtime qualification.
<!-- SECTION:DESCRIPTION:END -->

## Acceptance Criteria
<!-- AC:BEGIN -->
- [ ] #1 Feature extraction, exhaustive matching and sparse reconstruction are three separate stage invocations with distinct invocation keys and verified output-tree manifests.
- [ ] #2 The adapter uses one documented CPU-only flag set, explicitly disables COLMAP GPU use, fixes thread/locale/timezone-sensitive runtime inputs, and rejects raw flag overrides in V0.
- [ ] #3 COLMAP reads from snapshot materialization or read-only staging paths only; engine-private databases/workspaces are immutable after promotion and are not used as the public exchange format.
- [ ] #4 Structured progress and statistics are captured for features, pairs, matches, registrations, observations, tracks and mapper model count.
- [ ] #5 The invoked COLMAP identity comes from ND-30 and unsupported, changed or ambient PATH binaries fail before any stage writes promotable output.
<!-- AC:END -->
