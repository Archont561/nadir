---
id: ND-23
title: Design a decision-aligned cloud demonstration
status: To Do
assignee: []
created_date: '2026-09-30 20:38'
labels:
  - platform
  - deployment
  - spike
dependencies: []
references:
  - backlog/docs/plans/demo-plan.md
  - .knowledge/deployment/hosting-comparison.md
  - backlog/docs/decisions/004-inverted-container-no-dind.md
  - backlog/docs/decisions/010-bun-not-node.md
priority: low
type: feature
ordinal: 23000
---

## Description

<!-- SECTION:DESCRIPTION:BEGIN -->
Turn the cloud demo proposal into an implementation-ready plan that retains the inverted worker container and reconciles its obsolete Node or pnpm assumptions with the Bun decision.
<!-- SECTION:DESCRIPTION:END -->

## Acceptance Criteria
<!-- AC:BEGIN -->
- [ ] #1 The plan defines the UI, API or BFF, durable job orchestration, artifact storage and worker responsibilities
- [ ] #2 The worker deployment uses the inverted container model and declares resource and storage assumptions for a supported host
- [ ] #3 Every proposed JavaScript tool is compatible with the repository Bun-only policy or has an accepted replacement decision
<!-- AC:END -->
