---
id: ND-26
title: Dispatch real core operations through nadir-engine
status: To Do
assignee: []
created_date: '2026-10-03 14:16'
labels:
  - v0.1
  - engine
  - ffi
milestone: m-0
dependencies:
  - ND-24
  - ND-25
references:
  - .knowledge/architecture/language-bindings.md
  - backlog/docs/decisions/011-shared-transport-native-bindings.md
modified_files:
  - crates/engine
priority: high
type: task
ordinal: 26000
---

## Description

<!-- SECTION:DESCRIPTION:BEGIN -->
Replace the proof-only dispatcher in crates/engine with application operations that call the same Rust services used by the CLI. Keep transport translation in this crate and keep photogrammetry rules in core and stage crates.
<!-- SECTION:DESCRIPTION:END -->

## Acceptance Criteria
<!-- AC:BEGIN -->
- [ ] #1 Planning and dataset inspection are available through versioned protocol operations and return structured results
- [ ] #2 Every handler delegates domain behavior to core or a stage crate rather than restating it in the dispatcher
- [ ] #3 Core and validation failures map to stable transport errors with machine-readable codes and human-readable messages
- [ ] #4 Contract tests prove equivalent CLI and engine requests produce equivalent domain results
<!-- AC:END -->
