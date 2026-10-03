---
id: ND-25
title: Stabilize the nadir-protocol transport contract
status: To Do
assignee: []
created_date: '2026-10-03 14:16'
labels:
  - v0.1
  - protocol
  - ffi
milestone: m-0
dependencies: []
references:
  - .knowledge/architecture/language-bindings.md
  - backlog/docs/decisions/011-shared-transport-native-bindings.md
modified_files:
  - crates/protocol
priority: high
type: task
ordinal: 25000
---

## Description

<!-- SECTION:DESCRIPTION:BEGIN -->
Evolve crates/protocol from the initial ping/version/describe proof into a compatibility-tested contract suitable for every in-process binding. Specify envelope versioning, operation evolution, structured failures and canonical JSON fixtures before pipeline operations are exposed.
<!-- SECTION:DESCRIPTION:END -->

## Acceptance Criteria
<!-- AC:BEGIN -->
- [ ] #1 Request and response envelopes document required fields, defaults, error semantics and the exact rules for incrementing TRANSPORT_VERSION
- [ ] #2 Malformed JSON, unknown operations and unsupported transport versions return deterministic structured failures without panics
- [ ] #3 Canonical request and response fixtures are exercised by Rust and are reusable by Python and TypeScript conformance tests
- [ ] #4 Property tests prove arbitrary JSON payloads round-trip without truncation or reinterpretation
<!-- AC:END -->
