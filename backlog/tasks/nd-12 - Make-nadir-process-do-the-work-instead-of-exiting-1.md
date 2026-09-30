---
id: ND-12
title: Make nadir process do the work instead of exiting 1
status: To Do
assignee: []
created_date: '2026-09-30 19:25'
labels:
  - v0.1
  - cli
milestone: m-0
dependencies:
  - ND-9
  - ND-11
references:
  - .knowledge/roadmap/v0-mvp.md
  - .knowledge/context.md
priority: high
ordinal: 12000
---

## Description

<!-- SECTION:DESCRIPTION:BEGIN -->
The CLI scaffold deliberately fails: process exits 1 rather than pretending. This task removes that, wiring ingest through to the written products, with the progress output the roadmap specifies — one line per stage with its duration, engine and headline statistic.
<!-- SECTION:DESCRIPTION:END -->

## Acceptance Criteria
<!-- AC:BEGIN -->
- [ ] #1 nadir process ./images runs the built-in pipeline end to end and writes the products under outputs/.
- [ ] #2 Progress is one line per completed stage with elapsed time, engine and a stage statistic, and cached stages say so.
- [ ] #3 --quality and --outputs select presets and the product set rather than being accepted and ignored.
- [ ] #4 Exit codes follow the contract: 0 success, 1 pipeline failure, 3 a required engine missing from PATH.
<!-- AC:END -->
