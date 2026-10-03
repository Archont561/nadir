---
id: ND-2
title: 'Run external engines as subprocesses, and report a missing one honestly'
status: To Do
assignee: []
created_date: '2026-09-30 19:25'
labels:
  - v0.1
  - core
  - engines
milestone: m-0
dependencies: []
references:
  - .knowledge/architecture/engine-registry.md
  - backlog/docs/decisions/001-step-scoped-vs-odm-monolith.md
priority: high
ordinal: 2000
---

## Description

<!-- SECTION:DESCRIPTION:BEGIN -->
ADR-001 scopes the pipeline to step-scoped external engines, which means almost every stage is a subprocess. One runner owns that: argument construction, working directory, streamed stdout/stderr, cancellation, and the difference between "the engine is not installed here" and "the engine ran and failed". The CLI already reserves exit code 3 for the first case and has no call site for it yet.
<!-- SECTION:DESCRIPTION:END -->

## Acceptance Criteria
<!-- AC:BEGIN -->
- [ ] #1 A runner in nadir-core spawns an engine with tokio::process, streams both output pipes to tracing, and returns the exit status with the captured tail on failure.
- [ ] #2 An engine that is absent from PATH is a distinct error from an engine that exited non-zero, and the CLI maps it to exit code 3.
- [ ] #3 Cancelling the future kills the child process group rather than leaving an orphan.
- [ ] #4 Tests cover the success, non-zero-exit and missing-binary paths without requiring COLMAP to be installed.
<!-- AC:END -->
