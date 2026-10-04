# AGENTS.md

How to work in this repository, and why it is arranged this way. Read
`.knowledge/` for what the project is; this file is how to build it.

**Starting, resuming, or closing a sandbox session?** Follow
`.agents/skills/session/SKILL.md`. It sequences environment activation and provenance checks,
the baseline, dependency-aware backlog survey, stop-for-approval standup, task hand-off, and
post-merge report. Its four stable output shapes are in
`.agents/skills/session/standup-template.md`.

## The one command

```sh
pixi run gates
```

Everything CI runs, except the network-dependent advisory scan. If that passes, the change
is in good shape. Run it before claiming a task is done, not just the tests.

## Prerequisites

Pixi is the only tool you install yourself. It provisions Rust, Bun, Python, the geospatial
engines and every task runner from `pixi.lock`.

```sh
curl -fsSL https://pixi.sh/install.sh | bash
pixi install --locked
pixi run setup      # bun workspace, python sdk, git hooks
```

`pixi.toml` declares `requires-pixi = ">=0.81.0"`, and an older pixi refuses the manifest
before it installs anything. That floor and the `pixi-version:` pin in
`.github/workflows/ci.yml` are one decision written in two files — raise them together, or
every CI job fails at the *Install pixi* step with a version error that says nothing about
the change under test.

Node.js and pnpm are deliberately absent. Bun is the JavaScript runtime, the package
manager and the test runner.

## The parts, and what each one owns

| Path | Owns | Not |
| --- | --- | --- |
| `pixi.toml` | every system dependency, and the task graph | nothing per-language |
| `Cargo.toml` | the Rust workspace, and the `nadir-cli` package | JavaScript or Python config |
| `package.json` | the Bun workspace, the `all:*` turbo entry points, and the repo-wide agent tooling (`backlog.md`, `skills`) | tool versions |
| `.agents/skills/` | the agent skills, checked in | their provenance |
| `skills-lock.json` | where each skill came from, and its hash | the skill text |
| `backlog/` | the project's work, as Markdown tasks | anything the CLI can write |
| `turbo.json` | task ordering and caching | task commands |
| `crates/*/package.json` | the cargo commands, as a turbo facade | cargo configuration |
| `python/nadir/package.json` | the pytest/ruff commands, as a turbo facade | python dependencies |
| `deny.toml` | cargo-deny, at the workspace root | anything else |
| `lefthook.yml` | git hooks | CI |
| `.versionrc` | which files convco bumps the version in | the version itself |

**One source of truth per kind of thing.** If you are about to add a place a version, a
dependency or a task is recorded, check whether an existing one covers it. `pixi run
version-check` enforces the version case and will fail a commit that skips a file.

## The rules that will bite

**`Cargo.toml` is both `[workspace]` and `[package]`.** `pixi-build-rust` builds a package
by running `cargo install --locked --path <dir>`, and `cargo install` cannot install from a
virtual manifest. So the root is a package (`nadir-cli`), the binary's sources live in
`crates/cli/` behind a `[[bin]] path`, and `crates/cli` is excluded from the members glob.
Do not "fix" this by making the root virtual.

**`members = ["crates/*"]` is a glob, and the glob is the point.** Adding a crate is one
directory. The cost is that every non-crate directory under `crates/` must be in `exclude` —
currently `crates/cli` and `crates/.turbo`. The second one is not a typo: turbo writes a
log directory beside every workspace package, and `crates/package.json` makes `crates/` one.

**`cargo_vendor = true` in `.pixi-sandbox.toml` means the vendored crates are in the
transport.** Every crate in `Cargo.lock` is copied whenever pixi-sandbox packs the bundle.
Adding a dependency is a size decision, not just a build-time one. It is also what makes an
airlocked machine able to build the workspace rather than only run the binary.

**OpenMVS is not a dependency.** There is no conda-forge package for it and a manifest naming
it fails to solve. `pixi run build-openmvs` builds it from source; a binary installed that
way is only offline-restorable if pixi-sandbox packs that environment afterwards.

**COLMAP is pinned to `build = "cpu_*"`.** The CUDA build string drags `cuda-version` and a
GPU stack into every environment including CI runners with no GPU. Do not "fix" the version
bump that tries to move it to the default build.

**Python installs with `--no-deps`.** `pixi.toml` is the environment; a `pip install` that
resolves its own dependencies is a second, drifting source of truth.

**No per-crate lint overrides.** `[lints] workspace = true` in every crate, and the rules
live in `[workspace.lints]` in the root. A lint one crate can switch off is a lint that one
crate will switch off, at the moment it becomes inconvenient.

## Adding a crate

```sh
mkdir -p crates/<name>/src
# crates/<name>/Cargo.toml — name, version.workspace = true, [lints] workspace = true
# crates/<name>/src/lib.rs    — //! docs, the public API, #[cfg(test)] mod tests
pixi run typecheck             # the glob picks it up; no registration needed
```

Add it to `[workspace.dependencies]` and to the root `[dependencies]` if the CLI reports on
it. Nothing else. There is no workspace registration step; that is what the glob is for.

## Adding a JS or Python package

Create the directory, a `package.json` with the task scripts from `crates/package.json` or
`python/nadir/package.json`, and let the `packages/*` or `python/*` glob pick it up. Add
`turbo run <task>` in the root `all:*` script only if the task is new to the repository.

## Agent tooling: skills and the backlog

Bun is the sole entrypoint for JavaScript CLIs. `backlog.md`, `skills`, and future CLI
packages are root devDependencies reached through the one generic `bun` Pixi task:

```sh
pixi run bun x --bun skills list
pixi run bun x --bun backlog task list --plain
```

`bun x`, not `bunx`: the conda-forge package ships no `bunx` shim. `--bun` overrides the
packages' `#!/usr/bin/env node` shebangs because no environment here provides Node.js
(ADR-010). A direct `node_modules/.bin/backlog` therefore fails with `env: node: No such
file or directory`; that is the shebang, not a broken install. The generic task depends on
`bun-install`, so the committed `bun.lock` still decides what runs.

**Skills** live in `.agents/skills/`, checked in, with their origin and content hash in
`skills-lock.json`. They are vendored on purpose: a skill that resolves over the network
is a different skill on a machine with no network, and this repository ships an offline
sandbox. `pixi run bun x --bun skills list` shows what is installed; replace `list` with
`update` to refresh a vendored copy and rewrite the lock as a reviewable diff.

**Work is Markdown**, under `backlog/`, versioned with the code — task state is a git
commit, not a row in someone else's database. The conventions:

| Thing | Value |
| --- | --- |
| Task prefix | `nd` (`backlog/config.yml`) |
| Milestones | `backlog/milestones/`, `m-0` is V0.1 |
| Statuses | `To Do`, `In Progress`, `Done` |
| Labels | the crate or domain (`artifacts`, `reconstruction`, `cli`), plus the milestone (`v0.1`) |

Drive it with the CLI, not an editor: `task create`, `task edit --check-ac`, `task
complete`. Hand-editing the front matter is how IDs, dependency links and AC numbering
drift. Every read needs `--plain` (or `--json` to parse) — the bare commands open an
interactive TUI that never returns in a non-interactive shell, and `backlog board` and
`backlog browser` block outright. The full workflow is the `backlog` skill in
`.agents/skills/backlog/`.

## Commit messages

Conventional Commits, checked by `pixi run lint-commit`. `pixi run changelog` regenerates
`CHANGELOG.md` and bumps every file in `.versionrc`. Type prefixes: `feat`, `fix`, `docs`,
`refactor`, `test`, `build`, `ci`, `chore`, `perf`, `style`. Scopes are the crate names.

## Git hooks

`pixi run setup` installs them via `lefthook install`. The pre-commit hook formats what is
staged; pre-push runs `pixi run gates`. `pixi run hooks-install` re-runs it if `.git/hooks`
is ever clobbered.

## The offline sandbox

For a machine with no network:

```sh
bash scripts/restore.sh        # or scripts/restore.ps1 on Windows
```

It clones an orphan branch, verifies every blob against the manifest, and unpacks the
environment plus the vendored crate sources. The branch has to exist first: it is written
by the `publish sandbox` workflow on every push to `main`, so a fresh clone whose last
publish has not finished yet gets `not a valid object name: origin/sandbox/...`.

Pixi-sandbox is a release binary rather than a project dependency, so it has no wrapper
tasks in `pixi.toml`. CI installs the SHA-pinned binary and runs `pixi-sandbox plan
--config .pixi-sandbox.toml` directly; use `pixi-sandbox doctor --branch-location
.sandbox-out --verify` to verify a local pack without writing anything.

Never run `pixi-sandbox publish` without reviewing `pixi-sandbox plan` first. Publish writes
to an orphan branch on `origin`.

## Working in a devcontainer

`.devcontainer/devcontainer.json` uses the Pixi 0.81.0 image and delegates creation to
`.devcontainer/setup.sh`. The script installs the baseline Git/GitHub and C compiler tools,
materialises every locked Pixi environment, runs `pixi run setup`, and installs opencode in
Bun's global tool directory.

## What is not built yet

Every crate is a scaffold: one constant, one function, two tests. The CLI's `process`
subcommand deliberately fails, and `plan` prints an ordering rather than a resolved DAG. A
scaffold that exits 0 without doing the work is worse than one that says it is not built.

<!-- BEGIN:turborepo-agent-rules -->

# This is NOT the Turborepo you know

Turborepo configuration, task behavior, and CLI commands can vary between installed versions and may differ from your training data. Resolve the `turbo` package from this file's directory or relevant workspace; in monorepos, it may not be visible from the repository root. For example, run `node -p "require.resolve('turbo/package.json')"` from a workspace that depends on `turbo`.

Read `docs/README.md` inside that installed package first, then read the relevant pages from its `docs/` directory before changing Turborepo configuration or commands. Heed deprecation notices. These bundled docs match the installed package version and are available without network access.

This block is written and re-added by `turbo` before repository-scoped commands when an AI agent is detected. In the Turborepo source repository, its template is defined in `crates/turborepo-cli/src/cli/agent_guidance.rs`. Removing the managed block while updates are enabled means a later qualifying invocation will add it again. Set `"agentGuidance": false` in the root `turbo.json` or `turbo.jsonc` to opt out; this does not remove an existing block. Keep the block committed with your work to avoid an uncommitted change on the next agent invocation.
<!-- END:turborepo-agent-rules -->
