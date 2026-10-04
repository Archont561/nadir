---
type: Architecture Decision Record
title: "ADR-009: One Manifest per Root — the conda package and the Cargo package share the repository root"
description: "Records the accepted architecture decision on where the publishable pixi package is declared, and why there is no crates/cli/pixi.toml."
status: accepted
date: 2026-09-30
deciders: project lead
related:
  - ../../../.knowledge/deployment/pixi-setup.md
  - 006-pixi-for-dependency-management.md
  - ../../../.knowledge/architecture/overview.md
---

# ADR-009: One Manifest per Root

## TL;DR

`pixi.toml` is the `pixi` workspace root **and** the manifest of the `nadir-cli` conda
package, in the same file. There is deliberately no `crates/cli/pixi.toml` and no
`crates/cli/Cargo.toml`. `pixi-build-rust` runs `cargo install --locked --path <dir>`, and
`cargo install` cannot install from a virtual manifest — so the directory holding the conda
package's manifest must be a directory holding a Cargo package. The root is. Both manifests
live there.

`pixi publish --dry-run` is in `gates`; the real build (`pixi run publish-dist`) is not.

## Context

Two packaging routes were plausible.

**Option A — merge into the repository root** (what this ADR records).

```
Cargo.toml          [workspace] + [package nadir-cli] + [[bin]] path = "crates/cli/src/main.rs"
pixi.toml           [workspace]          + [package nadir-cli]
```

**Option B — a sub-manifest for the CLI.**

```
Cargo.toml          [workspace] + [package nadir-cli]
pixi.toml           [workspace]                          (environment only)
crates/cli/pixi.toml                              [package nadir-cli]
```

Option B is what most repositories reach for first, and it is tidier to look at: the CLI's
own concerns in the CLI's own directory. It also cannot work here.

`pixi-build-rust`'s build recipe is, in essence:

```sh
cargo install --locked --path <the directory holding the package's manifest> --root "$PREFIX"
```

Two consequences follow, and they are the whole ADR:

1. **`cargo install` needs a package.** It has no `-p` flag. Given a virtual manifest it
   fails, and given a workspace root that is *only* a workspace it has nothing to select. So
   the manifest-carrying directory must itself be `[package]`.

2. **The sources are wherever `[[bin]] path` says.** A package's manifest and a package's
   sources do not have to be in the same directory — cargo allows
   `path = "crates/cli/src/main.rs"` — but the *manifest* is pinned to a directory, and
   `pixi-build-rust` passes that directory to `cargo install`. So the directory must be the
   Cargo **root**, not the directory the sources happen to live in.

`crates/cli/` is not a Cargo package; its manifest is the root `Cargo.toml`, reached through
`[[bin]] path`. Pointing a `crates/cli/pixi.toml` at it would hand `cargo install` a
directory with no `Cargo.toml` in it.

## Decision

Merge into the root. Two manifests, two roles, one directory.

```toml
# pixi.toml
[workspace]
name = "nadir"
version = "0.1.0"
preview = ["pixi-build"]      # without this, [package] is rejected

[package]
name = "nadir-cli"
version = { workspace = true }
publish = true
license = "MIT"

[package.build]
backend = { name = "pixi-build-rust", version = ">=0.5,<0.6", channels = [
  "https://prefix.dev/conda-forge",
] }

[package.build.config]
compilers = ["c"]
binaries = ["nadir"]
extra-args = []

[package.build-dependencies]
rust = ">=1.98.1,<1.99"
```

### Three things that are load-bearing and easy to get wrong

**`preview = ["pixi-build"]` in `[workspace]`.** Without it, `pixi publish` fails with
*"section is only allowed when the `pixi-build` preview flag is enabled"*. The error points
at `[package]`, not at the flag that admits it, so it reads like a manifest bug rather than
a missing opt-in. It goes in `[workspace]` rather than per package: a workspace whose members
disagree about whether building is allowed would publish one and reject the next.

**`compilers = ["c"]`, not `["rust", "c"]`.** The backend defaults to both. The `rust` entry
asks conda-forge's compiler infrastructure for `rust_linux-64`, which requires the `rust`
metapackage — and on conda-forge `rust` *is* the platform package, the metapackage no longer
exists, and the resulting solve fails as a wall of "rust 1.xx.* for which no candidates were
found" with nothing mentioning the compiler. The `c` entry supplies the linker; the
toolchain comes from `[package.build-dependencies] rust`.

**`binaries = ["nadir"]`, named explicitly.** An empty list installs whatever cargo decides
the package produces — a decision made implicitly by a file that a later `src/bin/` would
change without anyone editing the build config.

### The `Cargo.lock` is shared, and load-bearing

`cargo install --locked` does not resolve dependencies; it asserts the lockfile is current.
The root `Cargo.lock` therefore serves three consumers that must not diverge:

| Consumer | Reads it for |
| --- | --- |
| `pixi publish` | `--locked`; a stale lock fails the build |
| `pixi-sandbox.toml` `cargo_vendor` | vendors every crate into the offline transport |
| a contributor | the graph `cargo build` reproduces |

One lock, three consequences. Editing a manifest without the lock breaks the offline
sandbox and the conda build simultaneously.

### `[package.run-dependencies]` is empty, and that is a fact

Today `nadir` links the C runtime and nothing else — `crates/cli` depends on `clap`,
`anyhow`, `thiserror`, `tracing` and the workspace crates. The tier-2 bindings (`gdal`,
`proj`, `opencv`) are **commented out** in `[workspace.dependencies]`, with the reason, and
are not compiled in.

Verified from the built package:

```
bin/nadir links against:
  ├─ libc.so.6 (system)
  ├─ ld-linux-x86-64.so.2 (system)
  ├─ libgcc_s.so.1 (libgcc)
  └─ libpthread.so.0 (system)
```

When the tier-2 bindings land they belong in `[package.run-dependencies]`, not in the
feature layers. Run dependencies are what the *consuming* environment receives; feature
layers affect only this repository's own environments. Getting that backwards produces a
package that installs cleanly and then fails on `libgdal.so.34` — a package that is broken
for every user and not for its author.

## Rationale

| Factor | Sub-manifest (B) | Merged root (A) |
| --- | --- | --- |
| `pixi-build-rust` can run `cargo install` | ✗ no `Cargo.toml` there | ✓ root is the package |
| Where to look for the CLI's packaging | neat | root, next to `Cargo.toml` |
| Version in one place | two files | one authority, `version-check`ed |
| Where `Cargo.lock` must live | ambiguous | root, shared by 3 consumers |
| Cost of the tidiness | broken build | a comment |

## Consequences

### Positive
- `pixi publish` works, and `pixi publish --dry-run` is a cheap gate.
- The binary ships on conda-forge as `nadir-cli` without a Dockerfile.
- An airlocked machine gets both the toolchain *and* the crate sources from one transport.
- The same root `Cargo.lock` guarantees the published binary, the offline build and the local
  build came from one dependency graph.

### Negative
- `pixi.toml` is two things at once, so a reader looking for "the environment" lands in a
  file that is also a package manifest. Mitigated by the section header comments, and by the
  fact that both roles read the same version.
- `[package]` requires an opt-in preview flag, so the manifest is not portable to a pixi
  release where that flag changes. A preview flag is a small, visible dependency on upstream.
- `pixi-build-rust` builds with `--locked`, which makes the committed `Cargo.lock`
  load-bearing for releases. A forgotten lock update fails the build rather than silently
  shipping different dependencies — which is the behaviour we want, at the cost of a
  confusing error message for the first person who hits it.

### Neutral
- `publish-plan` is in `gates`, `publish-dist` is not. The dry run validates the manifest
  shape for free; the real build takes ~60s compiling the workspace under a pinned
  toolchain, which is too slow to gate every push.
- Neither task ever uploads. `pixi publish` only reaches a channel with `--channel`, and
  nothing in `pixi.toml` passes it.

## See Also

- [`pixi.toml`](../../../pixi.toml) — the merged manifest, `[package]` section
- [ADR-006: Pixi for Dependency Management](006-pixi-for-dependency-management.md) — why Pixi owns system dependencies
- [Pixi setup](../../../.knowledge/deployment/pixi-setup.md) — the full environment and task graph
