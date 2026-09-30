---
id: ND-11
title: 'Execute the DAG topologically, skipping cached steps'
status: To Do
assignee: []
created_date: '2026-09-30 19:25'
labels:
  - v0.1
  - executor
milestone: m-0
dependencies:
  - ND-1
  - ND-10
references:
  - .knowledge/architecture/pipeline-dag.md
  - .knowledge/architecture/artifact-model.md
priority: high
ordinal: 11000
---

## Description

<!-- SECTION:DESCRIPTION:BEGIN -->
The executor is what turns the graph and the store into a pipeline: walk the DAG in topological order, run independent branches concurrently, check the artifact store before each step, and stop in a way that a later resume can pick up.
<!-- SECTION:DESCRIPTION:END -->

## Acceptance Criteria
<!-- AC:BEGIN -->
- [ ] #1 Steps run in topological order, and independent branches run concurrently up to a configurable limit.
- [ ] #2 A step whose artifact is already in the store is skipped, and the skip is visible in the run output.
- [ ] #3 A failing step cancels its dependents, leaves completed artifacts intact, and returns which step failed and why.
- [ ] #4 Per-step timing and engine identity are recorded, which is what the CLI progress lines and V0.2 provenance read.
<!-- AC:END -->
