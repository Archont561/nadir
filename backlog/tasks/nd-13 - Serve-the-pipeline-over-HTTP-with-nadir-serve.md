---
id: ND-13
title: Serve the pipeline over HTTP with nadir serve
status: To Do
assignee: []
created_date: '2026-09-30 19:26'
labels:
  - v0.1
  - executor
  - deployment
milestone: m-0
dependencies:
  - ND-11
references:
  - .knowledge/decisions/005-worker-topology-per-step.md
  - .knowledge/deployment/docker-strategy.md
priority: medium
ordinal: 13000
---

## Description

<!-- SECTION:DESCRIPTION:BEGIN -->
The V0.1 worker surface: one binary, one process, an HTTP endpoint that accepts a job and reports its state. ADR-005 keeps the topology fat for now — the same binary that runs the CLI serves the endpoint — and ADR-004 means the container is the engine environment rather than a Docker-in-Docker host.
<!-- SECTION:DESCRIPTION:END -->

## Acceptance Criteria
<!-- AC:BEGIN -->
- [ ] #1 nadir serve --port 8080 accepts a job submission, returns a job id, and reports status and per-step progress for it.
- [ ] #2 The server runs the same executor the CLI does, against the same artifact store, with no duplicated orchestration.
- [ ] #3 Shutdown is graceful: an in-flight job is cancelled or drained deliberately, not by killing the process group at random.
- [ ] #4 A Dockerfile builds an image that runs the server with the engines present, following the inverted container model.
<!-- AC:END -->
