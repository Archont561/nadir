---
id: ND-29
title: Add a stable C ABI embedding crate
status: To Do
assignee: []
created_date: '2026-10-03 14:16'
labels:
  - v0.2
  - c-api
  - ffi
dependencies:
  - ND-25
  - ND-26
references:
  - .knowledge/architecture/language-bindings.md
  - backlog/docs/decisions/011-shared-transport-native-bindings.md
modified_files:
  - crates/c-api
priority: low
type: feature
ordinal: 29000
---

## Description

<!-- SECTION:DESCRIPTION:BEGIN -->
Add a nadir-c-api crate as the lowest-common-denominator embedding face for languages without a dedicated Rust binding. Reuse the versioned nadir-engine transport and define explicit allocation and error ownership across the ABI.
<!-- SECTION:DESCRIPTION:END -->

## Acceptance Criteria
<!-- AC:BEGIN -->
- [ ] #1 The C ABI exposes transport version discovery, invoke and response deallocation with documented ownership for every pointer
- [ ] #2 The implementation delegates directly to nadir-engine and contains no domain or protocol interpretation
- [ ] #3 A C smoke program builds against the generated header, invokes ping and describe, and frees every returned allocation
- [ ] #4 ABI symbols, panic containment, thread-safety and compatibility policy are documented and tested
<!-- AC:END -->
