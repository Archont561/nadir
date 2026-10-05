---
id: ND-10
title: Add declarative DAG recipes after the fixed V0 profile
status: To Do
assignee: []
created_date: '2026-09-30 19:25'
updated_date: '2026-10-05'
labels:
  - v1.0
  - pipeline
milestone: m-1
dependencies:
  - ND-11
references:
  - .knowledge/architecture/pipeline-dag.md
  - .knowledge/architecture/v0-strict-sparse-profile.md
  - backlog/docs/roadmaps/v1-platform.md
  - backlog/docs/decisions/012-v0-strict-sparse-reconstruction-profile.md
priority: medium
type: feature
ordinal: 10000
---

## Description

<!-- SECTION:DESCRIPTION:BEGIN -->
V0 uses one fixed strict sparse profile. Declarative TOML recipes, user-selected pipeline variants and adaptive product DAGs are follow-on work that must preserve the V0 artifact contracts while adding configurable local processing.
<!-- SECTION:DESCRIPTION:END -->

## Acceptance Criteria
<!-- AC:BEGIN -->
- [ ] #1 A TOML recipe parses into typed steps and artifact dependencies without changing the canonical V0 stage contracts.
- [ ] #2 Cycles, unknown step references, unknown engines and attempts to bypass qualification are diagnostics naming the offending key.
- [ ] #3 Built-in recipes are explicitly post-V0 profiles; `nadir plan` distinguishes the fixed V0 sparse profile from configurable later recipes.
- [ ] #4 Product targets can prune later mapping-product branches while preserving required dependencies and cache identity.
<!-- AC:END -->
