---
name: session
description: >-
  Run a Nadir development session inside a restored Pixi sandbox from start to finish: activate
  or restore the offline toolchain, verify its provenance, baseline what this machine can prove,
  survey the Markdown backlog, propose a scoped session and stop for approval, then close landed
  work with evidence and a paste-ready opening prompt for the next session. Use whenever starting,
  resuming, handing off, or closing work in this repository; when the user asks to bootstrap the
  sandbox, choose the next task, report session status, or prepare the next session—even if they
  do not explicitly name this skill.
compatibility: >-
  Nadir checkout with Git and either the published sandbox/developer-linux-64 branch or an already
  restored .pixi environment. GitHub and npm access are optional; strict-airlock fallbacks are
  documented below.
---

# Session lifecycle for Nadir

Run every session as one loop: **(1)** enter the sandbox, **(2)** survey the repository and
backlog, **(3)** propose a bounded session and stop, **(4)** execute approved work, and **(5)**
close landed work with evidence and the next opening prompt. Use the four literal output shapes
in [`standup-template.md`](standup-template.md).

Two rules span the loop:

- **Proof boundary:** report what this machine proved. A missing network, native runner, published
  bundle, or merged commit is an open item rather than a checked criterion.
- **State preservation:** inspect the branch and working tree before changing anything. Preserve
  user changes, stay on the session's assigned branch, and do not rewrite history or switch
  branches to make startup look clean.

## 1. Enter the sandbox

### Inspect first

From the repository root:

```bash
git status --short --branch
git log -1 --oneline
command -v pixi >/dev/null 2>&1 && pixi --version || true
```

A dirty tree may be the previous session's hand-off. Account for every changed path before
editing; never reset, clean, stash, or overwrite it merely to obtain a baseline.

### Activate an existing restore or restore once

The published `developer` bundle contains the `default` Pixi environment, verified bootstrap
tools, and vendored Cargo crates. Its current launcher may come from either generation of
pixi-sandbox, so follow the files the restore produced instead of assuming one PATH model:

```bash
if command -v pixi >/dev/null 2>&1; then
  : # host/devcontainer pixi is already available
elif test -r .pixi/sandbox-env.sh; then
  # The transport currently published by this repository uses this activation hook.
  source .pixi/sandbox-env.sh
elif test -x "$HOME/.local/bin/pixi"; then
  # Newer transports register verified user launchers instead.
  export PATH="$HOME/.local/bin:$PATH"
else
  bash scripts/restore.sh
  if test -r .pixi/sandbox-env.sh; then
    source .pixi/sandbox-env.sh
  elif test -x "$HOME/.local/bin/pixi"; then
    export PATH="$HOME/.local/bin:$PATH"
  else
    PIXI_BIN=$(find .pixi/tools -mindepth 2 -maxdepth 2 -type f -name pixi -print -quit)
    test -n "$PIXI_BIN"
    export PATH="$(dirname "$PIXI_BIN"):$PATH"
  fi
fi

pixi --version
pixi install --frozen --offline
```

`pixi install --frozen --offline` must be a no-op after restore. If it wants a channel or changes
the solve, stop: the transport and checkout disagree.

When the sandbox branch is not already in local Git history, `scripts/restore.sh` fetches only
`sandbox/developer-<platform>`. A truly disconnected machine therefore needs that branch
transferred with the checkout before startup; inability to fetch is missing input, not a reason
to install an unverified tool by another route.

### Verify transport provenance

Read `.pixi/.restore-transport/.pixi-sandbox/manifest.json` when present, or the manifest on
`sandbox/developer-linux-64`. Record:

- `source.commit` and whether it names the expected `main` revision;
- `source.lock_sha256` and whether it matches this checkout's `pixi.lock`;
- the bundled Pixi and pixi-sandbox versions and their `linkage`;
- the restored environment name/platform and Cargo vendor metadata.

A feature branch may be newer than the bundle while still using an unchanged lockfile. That is
usable, but say so. A changed `pixi.lock` or `Cargo.lock` is not available offline until a new
transport has been packed and published.

### Pixi is the project-tool entrypoint

Use host `git` and `gh` for repository control. Run compilers, package tools, tests, and repo
utilities through the restored environment:

```bash
pixi run --frozen gates
pixi run --frozen test
pixi run --frozen lint
pixi run --frozen -- cargo check --offline --workspace
pixi run --frozen -- cargo nextest run --offline --workspace
pixi run --frozen bun x --bun backlog task list -s "To Do" --plain
pixi run --frozen bun x --bun skills list
```

Prefer named Pixi tasks because `pixi.toml` is the repository's task graph. Bun-hosted CLIs do
not get one task each: invoke all of them through `pixi run --frozen bun x --bun <package>` so
Bun remains their sole runtime and entrypoint. For a Cargo command that has no task, use
`pixi run --frozen -- cargo ... --offline`; the restored `.cargo/config.toml` points Cargo at
`.pixi-sandbox/vendor`. Bare `cargo`, `rustc`, `bun`, `python`, `maturin`, `ruff`, `lefthook`,
and engine commands bypass the locked environment and do not prove the repository configuration.

Pixi-sandbox itself is the exception: it is a separately installed release binary, not a
project dependency and not a task in `pixi.toml`. Normal sessions use `scripts/restore.sh` and
the publish workflow. When diagnosing transport, call `pixi-sandbox plan`, `pack`, or `doctor`
directly only after verifying which release binary is on `PATH`.

### Establish an honest baseline

The conda environment and Cargo vendor are in the transport; npm's `node_modules` is not. Choose
the strongest baseline the session can actually run:

1. If dependencies are already materialized, run `pixi run --frozen gates` and record each
   suite's counts/result.
2. On a connected sandbox, run `pixi run --frozen setup` once, then the full gates.
3. On a strict airlock without `node_modules`, do not pretend the Turbo/Bun gates ran. Run the
   Rust-only offline proof (`cargo fmt --check`, workspace check, nextest, doctests through
   `pixi run --frozen -- cargo ... --offline`) and record the missing JS/Python setup as an
   environment limitation.

Never copy a historical test count into the standup. The count observed now is the baseline the
closing report compares against.

## 2. Survey repository state and backlog

1. **Synchronize knowledge, not branches.** If GitHub is reachable, run `git fetch origin` and
   compare the current branch with `origin/main`. Keep the assigned branch. If it is behind or
   diverged, report that before proposing edits; rebasing or merging needs user approval when it
   could disturb session work.
2. **Load standing context.** Read `AGENTS.md`, `.knowledge/context.md`, `.knowledge/index.md`,
   and only the ADRs/references linked by candidate tasks. `pixi.toml`, manifests, and current
   code outrank design-era prose when they disagree.
3. **List open work.** Prefer
   `pixi run --frozen bun x --bun backlog task list -s "To Do" --plain`. If the Bun workspace cannot be
   materialized on a strict airlock, read `backlog/tasks/*.md` for discovery only and state that
   task metadata cannot be safely written until the CLI is available.
4. **Resolve dependencies.** Read each candidate with
   `pixi run --frozen bun x --bun backlog task <id> --plain`. A candidate is unblocked only when every item
   in `dependencies` is `Done`. Order by priority (high → medium → low), then `ordinal`; explain
   any spike that jumps the queue because it unlocks several tasks.
5. **Exclude already landed work.** Check `git log --oneline -15`, recent merged pull requests,
   open pull requests for the current branch, and current task status. Do not re-propose a result
   merely because its task record or sandbox branch is lagging; identify the stale record.
6. **Match scope to proof.** Separate locally provable slices from work requiring a push, CI,
   sandbox repack, real imagery, native platform, or maintainer action.

## 3. Propose the session, then stop

Fill **Session standup** from template §2 and wait for the user to choose or amend the scope.
Do not claim a task, edit files, or begin implementation before approval. A standup must include:

- environment mode and transport provenance;
- exact baseline evidence and any proof gaps;
- total open and unblocked work;
- 2–4 ordered candidates with dependencies and rationale;
- proposed slices, separating local proof from remote proof; and
- decisions that change the implementation shape, each with a recommendation.

The stop is part of the protocol: backlog order is evidence, not permission to choose product
scope for the user.

## 4. Execute approved work

Follow `AGENTS.md` as the authority and the relevant task's acceptance criteria as the boundary.

1. Claim the task through Backlog.md:
   `pixi run bun x --bun backlog task edit <id> -s "In Progress" -a @me --plan "<approved approach>"`.
2. Use the TDD skill for behavior changes: agree seams, obtain a meaningful red failure, implement
   the smallest green change, then use the refactor skill while tests remain green.
3. Keep domain behavior in Rust. Python and TypeScript remain thin adapters over the versioned
   `nadir-protocol` → `nadir-engine` dispatcher.
4. Test the narrow package first, then the affected language boundary, then the strongest
   baseline available in this sandbox. New behavior needs regression evidence; a command that
   could not run is listed, not silently omitted.
5. Update task notes through the CLI as evidence accumulates. Check an AC only after its proof:
   `--check-ac <n>`. Finish with a PR-ready `--final-summary`; use `task complete <id>` only when
   every criterion is proven.
6. Produce template §3 before committing. Make one focused Conventional Commit per task and run
   the commit-message check. Respect the hosting session's branch/push contract; push or open a PR
   only when the user has authorized that boundary.

Before hand-off, `git status --short` must contain only intentional paths, generated artifacts
must stay out of Git, and the evidence must name the exact command and result.

## 5. Close the session after work lands

A local commit, pushed branch, open PR, merged PR, and successful post-merge run are different
states. Report the one that exists.

1. If work merged or was pushed to `main`, inspect every triggered workflow. Nadir currently
   expects `ci` and `publish sandbox`; a red run belongs to this session until it is diagnosed.
2. After `publish sandbox` succeeds, inspect
   `sandbox/developer-linux-64:.pixi-sandbox/manifest.json`. Its `source.commit` should name the
   new `main` tip, and its lock/vendor metadata should match the merged locks.
3. Complete task records only when all AC evidence exists. Leave the task `In Progress` when a
   CI run, repack, real dataset, platform, or user action is still pending, and name that proof in
   notes and in the report.
4. Re-baseline the landed revision when this environment can do so without switching away from a
   platform-assigned branch. Otherwise use the successful CI run as remote evidence and preserve
   the local baseline separately.
5. Fill template §4. End it with template §1, fully populated: this is the paste-ready opening
   prompt for the next session.

A session that lands nothing still closes honestly: `Landed: nothing`, the current branch and
working-tree state, decisions made, proof gathered, and the next concrete move. Do not manufacture
a commit merely to make the report look complete.
