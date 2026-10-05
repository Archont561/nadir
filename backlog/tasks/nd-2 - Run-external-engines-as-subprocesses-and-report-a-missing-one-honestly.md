---
id: ND-2
title: Run qualified stage subprocesses with honest engine failures
status: To Do
assignee: []
created_date: '2026-09-30 19:25'
updated_date: '2026-10-05'
labels:
  - v0.1
  - core
  - engines
milestone: m-0
dependencies: []
references:
  - .knowledge/architecture/engine-registry.md
  - .knowledge/architecture/v0-strict-sparse-profile.md
  - backlog/docs/decisions/001-step-scoped-vs-odm-monolith.md
  - backlog/docs/decisions/012-v0-strict-sparse-reconstruction-profile.md
priority: high
type: task
ordinal: 2000
---

## Description

<!-- SECTION:DESCRIPTION:BEGIN -->
V0 still invokes external engines as subprocesses, but only through a runner that preserves the strict sparse profile: explicit executable, arguments, working directory, environment, cancellation, log capture and failure taxonomy. Runtime qualification belongs in ND-30; this task supplies the safe subprocess boundary that ND-30 can drive.
<!-- SECTION:DESCRIPTION:END -->

## Acceptance Criteria
<!-- AC:BEGIN -->
- [ ] #1 A runner spawns a declared executable with explicit argv, current directory and environment, streams stdout/stderr to tracing, and records a bounded log tail on failure.
- [ ] #2 Missing executable, unqualified executable, non-zero exit, timeout, cancellation and validation failure are distinct typed errors; the CLI can map missing/unqualified runtime separately from a failed stage.
- [ ] #3 Cancelling the future terminates the child process group and marks the staging directory abandoned rather than promotable.
- [ ] #4 The runner exposes the final argv, executable digest/version and environment fingerprint to provenance without allowing raw user-provided COLMAP flag overrides.
- [ ] #5 Tests cover success, non-zero exit, missing executable, timeout and cancellation without requiring COLMAP to be installed.
<!-- AC:END -->
