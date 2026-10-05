---
id: ND-31
title: Contain untrusted inputs and import candidate artifacts safely
status: To Do
assignee: []
created_date: '2026-10-05 12:12'
updated_date: '2026-10-05'
labels:
  - v0.1
  - artifacts
  - executor
  - engines
milestone: m-0
dependencies:
  - ND-1
  - ND-2
  - ND-30
references:
  - .knowledge/architecture/v0-strict-sparse-profile.md
  - backlog/docs/decisions/012-v0-strict-sparse-reconstruction-profile.md
priority: high
type: task
ordinal: 3500
---

## Description

Run untrusted inputs in qualified rootless OCI staging and let only the host verify and promote candidates. V0 trusted and untrusted cache namespaces do not cross-reuse.

## Acceptance Criteria

- [ ] #1 Refuse untrusted execution unless rootless OCI, no network, non-root user, read-only root/input mounts, dropped capabilities and CPU, memory, process, time, log and scratch limits are verified.
- [ ] #2 Give the container no writable cache or user-output mount; it writes only candidate staging trees under a bounded scratch directory.
- [ ] #3 Host-side import rejects unsafe paths, device files, FIFOs, sockets, symlinks, unexpected hard links, unsupported file types, missing files, extra files and contract/schema mismatches.
- [ ] #4 The host recomputes all digests, validates `SparseScene v1` structure and COLMAP workspace compatibility, then atomically promotes only verified manifests.
- [ ] #5 Trusted and untrusted invocation namespaces are separate in V0, and tests include malformed candidate trees, path traversal, digest tampering and resource-limit failures.
