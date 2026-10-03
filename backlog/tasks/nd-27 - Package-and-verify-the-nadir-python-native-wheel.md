---
id: ND-27
title: Package and verify the nadir-python-native wheel
status: To Do
assignee: []
created_date: '2026-10-03 14:16'
labels:
  - v0.1
  - python-native
  - python
  - ffi
milestone: m-0
dependencies:
  - ND-26
references:
  - .knowledge/architecture/language-bindings.md
  - backlog/docs/decisions/011-shared-transport-native-bindings.md
modified_files:
  - crates/python-native
  - python/nadir
priority: medium
type: task
ordinal: 27000
---

## Description

<!-- SECTION:DESCRIPTION:BEGIN -->
Finish the PyO3 face as a distributable nadir-sdk wheel. Keep the Rust adapter to one invoke function, make the Python API typed and idiomatic, and prove a built wheel works outside the checkout.
<!-- SECTION:DESCRIPTION:END -->

## Acceptance Criteria
<!-- AC:BEGIN -->
- [ ] #1 Maturin builds an abi3 wheel from crates/python-native and python/nadir using only locked workspace dependencies
- [ ] #2 A clean-environment smoke test installs the wheel and exercises every protocol operation through the compiled extension
- [ ] #3 Python exceptions preserve the engine error code and detail while successful responses have precise public types
- [ ] #4 The wheel contains the Python namespace and native extension but excludes tests, Cargo build output and repository-only files
<!-- AC:END -->
