---
type: Architecture Decision Record
title: "ADR-010: Bun, Not Node — one JavaScript runtime, and no second package manager"
description: "Records the accepted architecture decision to use Bun as the only JavaScript runtime, package manager and test runner, with no nodejs and no pnpm in any pixi feature."
status: accepted
date: 2026-09-30
deciders: project lead
related:
  - 006-pixi-for-dependency-management.md
  - 002-three-language-split.md
  - ../../../.knowledge/deployment/pixi-setup.md
---

# ADR-010: Bun, Not Node

## TL;DR

Bun is the JavaScript runtime, the package manager and the test runner. `nodejs` and
`pnpm` appear in no `[feature.*]` table and in no environment. `bun.lock` at the
repository root is the single JavaScript resolution record, and every JavaScript tool is
reached through `bun x` / `bun run`, never through a `node_modules/.bin` path.

This ADR exists because three places in the repository referred to it before it was
written — `pixi.toml`, `AGENTS.md`, and ADR-006's `[feature.js.dependencies]` comment
("Bun only. No nodejs, no pnpm: see ADR-010").

## Context

The design-era environment plan in [`pixi-setup.md`](../../../.knowledge/deployment/pixi-setup.md)
declared a `web` feature carrying `nodejs = "22.*"` and `pnpm = ">=9"`, on the assumption
that the TypeScript SDK and the demo UI would be an ordinary Node project alongside the
Rust workspace. Three facts made that arrangement worse than it looked once the workspace
was actually scaffolded.

**1. conda-forge has no `turbo`.** Turbo is the task orchestrator for the whole
repository — Rust, Python and TypeScript packages alike reach it through the root
`package.json`. It ships on npm and follows the npm release cadence, not the conda one.
So a JavaScript package manager is required *regardless* of whether a `web` app exists;
it is not a web-feature concern that can be deferred.

**2. A second package manager is a second resolution record.** `pnpm` plus Bun means
`pnpm-lock.yaml` and `bun.lock`, two files claiming to describe the same dependency
graph, each authoritative for whichever command a contributor happened to run. ADR-006's
rule — one source of truth per kind of thing — rules this out directly, and the failure
mode is the expensive kind: a CI run that is green against one lockfile and a local run
that is broken against the other.

**3. `node_modules/.bin` shims carry a `node` shebang.** With no `nodejs` in the
environment, those entries are unrunnable, and a task that calls one fails with
`bad interpreter` rather than anything informative. This is not a reason to add Node; it
is the reason tasks must call `bun x <tool>` instead. Adding `nodejs` to make the shims
work would mean two runtimes present and neither one obviously the one in use.

Bun covers all three roles in one conda-forge package: it runs TypeScript directly
(no build step for the SDK's tests), it installs workspaces from `bun.lock`, and
`bun test` is the test runner. The cost of the choice is concentrated in one place —
Bun's own compatibility surface — rather than spread across a runtime, a package manager
and a test runner that version independently.

## Decision

`bun` is the only JavaScript dependency in `pixi.toml`, in `[feature.js.dependencies]`.

```toml
[feature.js.dependencies]
bun = { workspace = true }
# turbo and biome are Bun *workspace* dependencies, not conda ones — they change on the
# npm release cadence, not the conda one.
```

Consequences that are rules rather than preferences:

- **No `nodejs`, no `pnpm`, no `npm`, in any feature or environment.** A dependency that
  needs Node to build is a finding to record, not a line to add.
- **`bun.lock` at the root is the only JavaScript lockfile**, and `bun install
  --frozen-lockfile` is how CI and the `bun-install` pixi task materialise it.
- **Tasks call `bun x <tool>` or `bun run <script>`**, never `node_modules/.bin/<tool>`.
- **`turbo` and `biome` stay npm dependencies** of the Bun workspace. They are not conda
  packages: pinning them in `pixi.toml` would put a JavaScript tool's version in the
  manifest that owns the *system* dependencies, and conda-forge does not carry `turbo` at
  all.
- **The repository-global utilities that hooks need — `convco`, `lefthook`, `actionlint` —
  are conda dependencies, not npm ones.** A machine restored from the offline sandbox
  branch gets a `default` environment's `bin/` on PATH and no `node_modules`; git hooks
  have to work there.

## Consequences

### Positive
- One runtime, one lockfile, one install command for the whole JavaScript side.
- The offline sandbox transport carries a working JavaScript toolchain, because the
  runtime is a conda package inside the packed environment.
- No `nvm`, no Node version file, no divergence between a contributor's global Node and
  the pinned one.

### Negative
- Any tool that assumes Node — a bin shim, a postinstall script, a native addon built
  against Node's ABI — is a compatibility risk carried by Bun rather than avoided. The
  mitigation is the `bun x` rule, not a fallback runtime.
- `turbo` and `biome` versions live in `package.json` while every other tool version lives
  in `pixi.toml`, so "where is this tool pinned?" has two answers. The split is by release
  cadence and is recorded here so it is a decision rather than an inconsistency.

### Neutral
- The `web` feature from `pixi-setup.md` is not declared at all. When a UI lands it is a
  `[feature.web]` plus an `[environments]` entry plus a `[[bundle]]` in
  `.pixi-sandbox.toml` — and it will carry Bun, not Node.

## See Also

- [`pixi.toml`](../../../pixi.toml) — `[feature.js.dependencies]`, and the header note on why there is no Node
- [ADR-006: Pixi for Dependency Management](006-pixi-for-dependency-management.md) — the one-source-of-truth rule this follows from
- [ADR-002: Three-Language Split](002-three-language-split.md) — why there is a TypeScript tier at all
- [Pixi setup](../../../.knowledge/deployment/pixi-setup.md) — design-era plan; its `web` feature predates this ADR
