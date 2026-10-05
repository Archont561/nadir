---
id: ND-31
title: Contain untrusted inputs and import candidate artifacts safely
status: To Do
assignee: []
created_date: '2026-10-05 12:12'
labels: [v0.1, artifacts, executor, engines]
milestone: m-0
dependencies: [ND-1, ND-2, ND-30]
references:
  - .knowledge/architecture/v0-strict-sparse-profile.md
priority: high
type: task
ordinal: 11500
---
## Description
Run untrusted inputs in qualified rootless OCI staging and let only the host verify and promote candidates.

## Acceptance Criteria
- [ ] #1 Refuse execution unless all containment and resource controls are verified.
- [ ] #2 Give the container no writable cache or output mount.
- [ ] #3 Host-check paths, file types, digests, structure, and compatibility before atomic promotion.
- [ ] #4 Keep trusted/untrusted namespaces separate and test malformed candidates.
