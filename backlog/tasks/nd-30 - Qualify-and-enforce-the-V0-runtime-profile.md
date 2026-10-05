---
id: ND-30
title: Qualify and enforce the V0 runtime profile
status: To Do
assignee: []
created_date: '2026-10-05 12:12'
labels: [v0.1, engines, executor]
milestone: m-0
dependencies: [ND-2]
references:
  - .knowledge/architecture/v0-strict-sparse-profile.md
priority: high
type: task
ordinal: 2500
---
## Description
Resolve COLMAP only from a declared qualified Pixi or OCI runtime, fingerprint it completely, and run the fixed CPU-only recipe without ambient PATH discovery.

## Acceptance Criteria
- [ ] #1 Reject missing, changed, or unqualified COLMAP before execution.
- [ ] #2 Record runtime, executable, architecture, thread, environment, arguments, and GPU-disabled identity.
- [ ] #3 Expose no raw engine overrides.
- [ ] #4 Test qualified, changed, missing, and unqualified profiles.
