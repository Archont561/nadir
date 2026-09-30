---
id: ND-1
title: Content-addressed artifact store with blake3 hashing
status: To Do
assignee: []
created_date: '2026-09-30 19:25'
labels:
  - v0.1
  - artifacts
milestone: m-0
dependencies: []
references:
  - .knowledge/architecture/artifact-model.md
  - .knowledge/roadmap/v0-mvp.md
priority: high
ordinal: 1000
---

## Description

<!-- SECTION:DESCRIPTION:BEGIN -->
crates/artifacts is a scaffold with one constant. Every other V0.1 task caches through it, so it lands first. An artifact is identified by the blake3 hash of its inputs — the engine identity, its parameters and the hashes of its own inputs — so a rerun with an unchanged prefix skips work instead of recomputing it, and a changed parameter invalidates exactly the tail that depends on it.
<!-- SECTION:DESCRIPTION:END -->

## Acceptance Criteria
<!-- AC:BEGIN -->
- [ ] #1 An Artifact carries a content hash, a type, a path inside the store and the provenance of the step that produced it.
- [ ] #2 The store resolves a hit or miss for a given (engine, params, input hashes) triple without reading the artifact payload.
- [ ] #3 A partially written artifact is never visible as a hit: writes land through a temporary path and an atomic rename.
- [ ] #4 Changing one input parameter invalidates that artifact and everything downstream of it, and nothing upstream.
<!-- AC:END -->
