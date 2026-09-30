---
id: ND-20
title: Establish the transport-independent Worker Protocol
status: To Do
assignee: []
created_date: '2026-09-30 20:38'
labels:
  - v2.0
  - protocol
  - executor
dependencies: []
references:
  - .knowledge/architecture/worker-protocol.md
  - .knowledge/roadmap/v2-scale.md
priority: low
type: feature
ordinal: 20000
---

## Description

<!-- SECTION:DESCRIPTION:BEGIN -->
Define the Worker API and protobuf contract so local and remote execution share task submission, status, cancellation, capabilities and event semantics.
<!-- SECTION:DESCRIPTION:END -->

## Acceptance Criteria
<!-- AC:BEGIN -->
- [ ] #1 Versioned protobuf schemas define tasks, artifacts, runs, capabilities and streamed events as the wire source of truth
- [ ] #2 A worker server executes work through the existing executor and isolates each task workspace
- [ ] #3 Local and remote transports expose equivalent submit, status, cancel and event behavior without SaaS concepts in the protocol
<!-- AC:END -->
