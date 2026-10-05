---
id: ND-3
title: Snapshot one bounded calibrated ImageSet for V0
status: To Do
assignee: []
created_date: '2026-09-30 19:25'
updated_date: '2026-10-05'
labels:
  - v0.1
  - dataset
  - artifacts
milestone: m-0
dependencies:
  - ND-1
references:
  - .knowledge/architecture/v0-strict-sparse-profile.md
  - .knowledge/architecture/five-domains.md
  - .knowledge/photogrammetry/pipeline-stages.md
  - backlog/docs/decisions/012-v0-strict-sparse-reconstruction-profile.md
priority: high
type: task
ordinal: 3000
---

## Description

<!-- SECTION:DESCRIPTION:BEGIN -->
The first V0 stage admits exactly one bounded input root and converts it into an immutable calibrated ImageSet snapshot. V0 recursively discovers supported regular image files, sorts normalized logical paths, validates one resolved intrinsics profile, copies original bytes into the store, and rejects input shapes that would imply georeferencing, multiple rigs or mutable filesystem aliases.
<!-- SECTION:DESCRIPTION:END -->

## Acceptance Criteria
<!-- AC:BEGIN -->
- [ ] #1 Recursion admits supported regular JPEG files and explicitly calibrated PNG files, rejects symlinks, ordinary hard links, unsupported media and path escapes, and records normalized logical paths in deterministic order.
- [ ] #2 A run has one resolved intrinsics profile; mixed rigs, incompatible dimensions, missing calibration, focus/zoom changes or untracked orientation transforms fail before COLMAP starts.
- [ ] #3 Snapshot artifacts contain the original input bytes, per-image digest/size/path metadata, camera calibration metadata and V0 limit measurements for image count, bytes and pixels.
- [ ] #4 Cache identity uses the immutable snapshot digests and calibration contract, not source directory mtimes or mutable paths.
- [ ] #5 The stage reports admitted, rejected and duplicate candidates, and downstream V0 acceptance is based on every admitted image.
<!-- AC:END -->
