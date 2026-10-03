---
id: ND-21
title: Publish TypeScript and Python worker SDKs
status: To Do
assignee: []
created_date: '2026-09-30 20:38'
labels:
  - v2.0
  - sdk
  - protocol
dependencies: []
references:
  - backlog/docs/roadmaps/v2-scale.md
  - backlog/docs/decisions/002-three-language-split.md
  - .knowledge/architecture/worker-protocol.md
priority: low
type: feature
ordinal: 21000
---

## Description

<!-- SECTION:DESCRIPTION:BEGIN -->
Generate and ship ergonomic TypeScript and Python clients over the Worker Protocol with shared validation and local-or-remote transports.
<!-- SECTION:DESCRIPTION:END -->

## Acceptance Criteria
<!-- AC:BEGIN -->
- [ ] #1 The TypeScript SDK validates requests, submits a mapping run and consumes run events
- [ ] #2 The Python SDK offers equivalent mapping, status, cancellation and event APIs
- [ ] #3 Generated protocol bindings are versioned with compatibility tests for both SDKs
<!-- AC:END -->
