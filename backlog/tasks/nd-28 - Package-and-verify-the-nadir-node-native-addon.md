---
id: ND-28
title: Package and verify the nadir-node-native addon
status: To Do
assignee: []
created_date: '2026-10-03 14:16'
labels:
  - v0.1
  - node-native
  - typescript
  - ffi
milestone: m-0
dependencies:
  - ND-26
references:
  - .knowledge/architecture/language-bindings.md
  - backlog/docs/decisions/011-shared-transport-native-bindings.md
modified_files:
  - crates/node-native
  - packages/sdk
priority: medium
type: task
ordinal: 28000
---

## Description

<!-- SECTION:DESCRIPTION:BEGIN -->
Finish the N-API face as an installable TypeScript SDK artifact. Replace checkout-specific loading assumptions with explicit supported-target resolution while preserving the single invoke export.
<!-- SECTION:DESCRIPTION:END -->

## Acceptance Criteria
<!-- AC:BEGIN -->
- [ ] #1 The release build produces the declared addon for every supported target and the package loader selects it deterministically
- [ ] #2 An unsupported platform or missing addon fails with an actionable message naming the platform and expected artifact
- [ ] #3 A packed-package smoke test installs @nadir/sdk outside the workspace and exercises every operation through N-API
- [ ] #4 The published package contains dist, declarations and native artifacts without Cargo target output or test files
<!-- AC:END -->
