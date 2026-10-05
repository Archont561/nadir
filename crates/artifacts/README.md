# nadir-artifacts

Content-addressed artifact storage and the invalidation rules it makes possible.

The V0 stage cache (ADR-012): [`InvocationKey`] names a requested stage
execution — stage, task and contract versions, named input digests, normalized
configuration, qualified runtime identity and every byte-affecting setting —
while [`OutputTreeManifest`] and its [`TreeDigest`] name the bytes a stage
actually produced. [`StageCache`] maps one to the other through unique staging
directories, structural validation, full byte digesting and atomic promotion;
a hit revalidates every byte before it is reported and hands the caller copies
([`StageCache::materialize`]), never a path inside the cache.

Integration tests live in `tests/` (`invocation_keys.rs`, `output_trees.rs`,
`stage_cache.rs`) and exercise the public seam: deterministic keys, manifest
validation as typed errors, promotion invariants, and the miss/hit/invalid
lookup trichotomy.

[`InvocationKey`]: https://docs.rs/nadir-artifacts/latest/nadir_artifacts/struct.InvocationKey.html
[`OutputTreeManifest`]: https://docs.rs/nadir-artifacts/latest/nadir_artifacts/struct.OutputTreeManifest.html
[`TreeDigest`]: https://docs.rs/nadir-artifacts/latest/nadir_artifacts/struct.TreeDigest.html
[`StageCache`]: https://docs.rs/nadir-artifacts/latest/nadir_artifacts/struct.StageCache.html
[`StageCache::materialize`]: https://docs.rs/nadir-artifacts/latest/nadir_artifacts/struct.StageCache.html#method.materialize
