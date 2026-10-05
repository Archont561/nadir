---
id: ND-1
title: Separate V0 invocation keys from verified output-tree digests
status: To Do
assignee: []
created_date: '2026-09-30 19:25'
updated_date: '2026-10-05'
labels:
  - v0.1
  - artifacts
  - cache
milestone: m-0
dependencies: []
references:
  - .knowledge/architecture/artifact-model.md
  - .knowledge/architecture/v0-strict-sparse-profile.md
  - backlog/docs/decisions/012-v0-strict-sparse-reconstruction-profile.md
  - backlog/docs/roadmaps/v0-mvp.md
priority: high
type: task
ordinal: 1000
---

## Description

<!-- SECTION:DESCRIPTION:BEGIN -->
Build the artifact/cache foundation required by ADR-012. V0 does not treat a stage request hash as proof of output bytes: an invocation key names the requested work, while a verified manifest names the immutable output tree that was actually produced. Cache hits revalidate those bytes before reuse, and user-facing outputs are copies or reflinks rather than direct cache paths.
<!-- SECTION:DESCRIPTION:END -->

## Acceptance Criteria
<!-- AC:BEGIN -->
- [ ] #1 The store records V0 invocation keys from task/contract versions, named input digests, normalized config, qualified runtime identity and every byte-affecting setting.
- [ ] #2 Each completed stage entry maps to a manifest of output-tree file paths, sizes, modes and BLAKE3 digests; the manifest digest is distinct from the invocation key.
- [ ] #3 Stages write unique staging directories, validate structure, digest all expected bytes, and atomically promote only complete manifests.
- [ ] #4 A cache hit revalidates manifest bytes before returning, launches no engine subprocess, and never exposes the cache directory as the user output path.
- [ ] #5 Partially written, missing, extra, mutated or schema-incompatible stage trees are misses or typed validation errors, never successful hits.
<!-- AC:END -->
