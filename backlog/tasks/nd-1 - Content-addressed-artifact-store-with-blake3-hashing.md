---
id: ND-1
title: Separate V0 invocation keys from verified output-tree digests
status: Done
assignee: []
created_date: '2026-09-30 19:25'
updated_date: '2026-10-05 14:09'
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
- [x] #1 The store records V0 invocation keys from task/contract versions, named input digests, normalized config, qualified runtime identity and every byte-affecting setting.
- [x] #2 Each completed stage entry maps to a manifest of output-tree file paths, sizes, modes and BLAKE3 digests; the manifest digest is distinct from the invocation key.
- [x] #3 Stages write unique staging directories, validate structure, digest all expected bytes, and atomically promote only complete manifests.
- [x] #4 A cache hit revalidates manifest bytes before returning, launches no engine subprocess, and never exposes the cache directory as the user output path.
- [x] #5 Partially written, missing, extra, mutated or schema-incompatible stage trees are misses or typed validation errors, never successful hits.
<!-- AC:END -->

## Implementation Plan

<!-- SECTION:PLAN:BEGIN -->
TDD at the public nadir_artifacts seam: add integration tests for invocation keys (task/contract versions, named input digests, normalized config, qualified runtime identity, byte-affecting settings), output-tree manifests with paths/sizes/modes/blake3 digests, tree digests distinct from invocation keys, staging + atomic promotion, byte-revalidating lookups, typed validation errors for missing/extra/mutated/schema-incompatible trees, and materialize-copies-outputs. Implement in nadir-artifacts on top of nadir-core ArtifactHash; no COLMAP execution, no locks (ND-11 scope).
<!-- SECTION:PLAN:END -->

## Implementation Notes

<!-- SECTION:NOTES:BEGIN -->
Evidence (nadir-artifacts, TDD at the public crate seam):

- Invocation keys: `InvocationSpec` hashes stage, task version, contract version, named input digests (name + digest), normalized resolved config, and qualified `RuntimeIdentity` (engine, profile, version, executable digest, byte-affecting runtime settings). Tests: `invocation_keys.rs` — determinism, order-independence, whitespace normalization, and a table proving every identity input changes the key.
- Output-tree manifests: `OutputTreeManifest` records canonical relative path, size, permission mode, and BLAKE3 digest per file; `TreeDigest` digests the manifest under `nadir.output-tree.v1`. Distinct type and hash domain from `InvocationKey`. Tests: `output_trees.rs`.
- Stage entries: `StageEntry` maps an invocation key to an embedded manifest plus its tree digest (JSON, format-tagged `nadir.stage-entry.v1`). Tests: `stage_cache.rs` — two different requests producing identical bytes map to one tree digest and one shared object with two entries.
- Staging/promotion: unique staging dirs (`staging_dir`), structural validation against declared outputs (missing → `MissingExpected`, extra → `UndeclaredFile`, symlink → `IrregularFile`, unsafe declared paths refused), full byte digesting, then atomic rename into `objects/` and a temp+rename index write. Un-promoted work stays a Miss; identical repromotion is idempotent; same key with different bytes is refused (`ConflictingEntry`).
- Hits revalidate: `lookup` re-digests every manifest byte before returning Hit; `StageHit` carries no store path and `materialize` copies bytes (with recorded modes) to user paths. Mutating the user copy cannot affect the store.
- Invalid entries are typed, never hits: mutated bytes, missing/extra files, vanished trees, schema-incompatible or tampered entries (`EntryInvalid::{Manifest, TreeMissing, Schema, TreeDigestMismatch, KeyMismatch}`).

Scoping notes: the store has no engine API at all, so a hit launches no engine subprocess by construction; same-run zero-COLMAP-launch run evidence belongs to ND-11/ND-14. Per-invocation advisory locks are ND-11 scope. File digesting reads whole files into memory (mirrors the vendored blake3 hasher); streaming is a future concern when real COLMAP databases arrive with ND-4.
<!-- SECTION:NOTES:END -->

## Final Summary

<!-- SECTION:FINAL_SUMMARY:BEGIN -->
Implemented the ADR-012 V0 stage-cache foundation in nadir-artifacts: InvocationKey (request identity: stage/task/contract versions, named input digests, normalized config, qualified runtime identity, byte-affecting settings), OutputTreeManifest + TreeDigest (produced-bytes identity with path/size/mode/blake3 records), and StageCache (unique staging dirs, structural validation, full byte digesting, atomic rename-based promotion, byte-revalidating lookups, typed miss/hit/invalid trichotomy, and materialize-copies for user outputs). 33 integration tests + 1 doctest, all workspace gates green (pixi run gates: 21/21 tasks). Zero-engine-subprocess run evidence and per-invocation locks remain ND-11/ND-14 scope.
<!-- SECTION:FINAL_SUMMARY:END -->
