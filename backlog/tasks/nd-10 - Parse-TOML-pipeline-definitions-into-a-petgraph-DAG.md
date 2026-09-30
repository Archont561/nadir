---
id: ND-10
title: Parse TOML pipeline definitions into a petgraph DAG
status: To Do
assignee: []
created_date: '2026-09-30 19:25'
labels:
  - v0.1
  - pipeline
milestone: m-0
dependencies: []
references:
  - .knowledge/architecture/pipeline-dag.md
priority: high
ordinal: 10000
---

## Description

<!-- SECTION:DESCRIPTION:BEGIN -->
nadir plan currently prints a fixed ordering rather than a resolved graph. Replace that with a real parse: a declarative TOML pipeline whose steps name an engine, its parameters and its inputs, resolved into a petgraph DAG with the errors a human needs when the file is wrong.
<!-- SECTION:DESCRIPTION:END -->

## Acceptance Criteria
<!-- AC:BEGIN -->
- [ ] #1 A TOML pipeline parses into a typed graph whose nodes are steps and whose edges are artifact dependencies.
- [ ] #2 A cycle, an unknown step reference, or an unknown engine is a diagnostic naming the offending key, not a panic or a silent drop.
- [ ] #3 A built-in standard pipeline is embedded, so nadir process with no --pipeline has something to run.
- [ ] #4 nadir plan prints the resolved DAG, including the parallel branches, instead of a static list.
<!-- AC:END -->
