# Nadir Knowledge Bundle Update Log

## 2026-10-05
* **Decision**: Added [ADR-012](../backlog/docs/decisions/012-v0-strict-sparse-reconstruction-profile.md) and the [V0 strict sparse reconstruction profile](architecture/v0-strict-sparse-profile.md). V0.1 now proves immutable local sparse SfM with a qualified CPU-only COLMAP profile instead of promising a georeferenced mapping stack.
* **Scope**: Georeferencing, dense reconstruction, PDAL/GDAL products, GPU execution, multi-camera input, distributed workers, and general reconstruction SDKs are explicit follow-on work rather than implicit V0 obligations.
* **Safety**: Recorded the accepted invocation/output identity split, immutable input snapshots and workspace artifacts, atomic verified publication, OS-user cache locks, policy-gated product acceptance, trusted/untrusted runtime separation, host-side candidate import, and independent user-output materialization.
* **Evidence**: Recorded property-test and fixture requirements, including committed proptest regressions, clean-store byte equality, zero-subprocess same-store cache evidence, and candidate CC BY fixture qualification.

## 2026-09-30
* **New**: [ADR-010: Bun, Not Node](../backlog/docs/decisions/010-bun-not-node.md) — written because
  `pixi.toml`, `AGENTS.md` and ADR-006 all cited it before it existed; ADR-006's link to
  `010-bun-not-node.md` was dangling.
* **Index**: Added ADR-009 and ADR-010 to the [bundle index](index.md) and to the ADR table
  in [legacy-navigation.md](legacy-navigation.md); the "why did we decide X" range now reads
  `001` through `010` rather than `001` through `008`.
* **Superseded**: [deployment/pixi-setup.md](deployment/pixi-setup.md) marked
  `superseded-in-part`, with a divergence table pointing at `/pixi.toml` as authoritative —
  the `web` feature (`nodejs`/`pnpm`), the six-environment layout, the `openmvs` dependency,
  the unpinned COLMAP build string, the per-language `[tasks]`, and the CI snippet are all
  design-era rather than current.
* **Corrected**: [references/dependency-matrix.md](references/dependency-matrix.md) dev-tool
  table rewritten against the shipped pins — Node.js and pnpm removed, Bun added, and the
  `wasm`/`gpu` rows marked as not-yet-declared environments.
* **Corrected**: `AGENTS.md` no longer claims `lint-sandbox-plan` is part of `gates` (it is
  a separate CI job, because `pixi-sandbox` is not a conda dependency), and now records the
  `requires-pixi` / CI pin coupling that broke the `ci` workflow on 2026-09-30.

## 2026-09-17
* **Migration**: Updated the bundle to conform to Open Knowledge Format (OKF) v0.2.
* **Metadata**: Added concept types and descriptions, and normalized lifecycle status values.
* **Navigation**: Added the reserved [bundle index](/index.md) and converted the project context to a concept.
