---
id: ND-13
title: Serve execution over HTTP after the Worker Protocol
status: To Do
assignee: []
created_date: '2026-09-30 19:26'
updated_date: '2026-10-05'
labels:
  - v2.0
  - executor
  - deployment
  - protocol
milestone: m-2
dependencies:
  - ND-11
  - ND-20
references:
  - backlog/docs/decisions/005-worker-topology-per-step.md
  - .knowledge/architecture/worker-protocol.md
  - .knowledge/deployment/docker-strategy.md
  - backlog/docs/roadmaps/v2-scale.md
  - backlog/docs/decisions/012-v0-strict-sparse-reconstruction-profile.md
priority: low
type: feature
ordinal: 13000
---

## Description

<!-- SECTION:DESCRIPTION:BEGIN -->
HTTP serving and worker services are explicitly outside V0. After the local sparse/cache profile and Worker Protocol are qualified, expose execution through a server surface that reuses the executor and artifact contracts rather than inventing a separate orchestration path.
<!-- SECTION:DESCRIPTION:END -->

## Acceptance Criteria
<!-- AC:BEGIN -->
- [ ] #1 The HTTP service implements the accepted Worker Protocol submission, status, cancellation and event semantics.
- [ ] #2 The server runs the same executor and verified artifact store as the CLI, with no duplicated orchestration or unqualified runtime discovery.
- [ ] #3 Shutdown is graceful: an in-flight job is cancelled or drained deliberately and leaves stage lifecycle state consistent.
- [ ] #4 Container packaging declares qualified engine/runtime identity and resource limits for the post-V0 worker profile.
<!-- AC:END -->
