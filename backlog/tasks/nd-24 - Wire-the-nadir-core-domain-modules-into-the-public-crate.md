---
id: ND-24
title: Wire the nadir-core domain modules into the public crate
status: To Do
assignee: []
created_date: '2026-10-03 14:16'
labels:
  - v0.1
  - core
milestone: m-0
dependencies: []
references:
  - .knowledge/architecture/artifact-model.md
  - .knowledge/architecture/engine-registry.md
  - .knowledge/architecture/pipeline-dag.md
modified_files:
  - crates/core
priority: high
type: task
ordinal: 24000
---

## Description

<!-- SECTION:DESCRIPTION:BEGIN -->
Turn crates/core from a stage-reporting scaffold into the dependency-light domain kernel. Compile and expose the drafted artifact, engine and pipeline modules, reconcile their dependencies, and make their public types the vocabulary consumed by the rest of the workspace.
<!-- SECTION:DESCRIPTION:END -->

## Acceptance Criteria
<!-- AC:BEGIN -->
- [ ] #1 crates/core/src/lib.rs exposes the artifact, engine and pipeline modules and re-exports the intended stable types
- [ ] #2 The drafted modules compile under workspace lints with all required dependencies declared through workspace.dependencies
- [ ] #3 Integration tests cover artifact identity, engine capability metadata and valid and invalid pipeline construction through only the public API
- [ ] #4 nadir-core remains independent of CLI, FFI adapter and external-engine implementation crates
<!-- AC:END -->
