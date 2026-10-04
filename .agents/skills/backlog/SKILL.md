---
name: backlog
description: Managing project work as Markdown tasks with Backlog.md. Use when creating, breaking down, tracking, or updating tasks; when the user mentions the backlog, a kanban board, a task ID (task-N), acceptance criteria (AC), or a Definition of Done (DoD).
---

Backlog.md keeps every task as a plain Markdown file under `backlog/`, versioned with the code, so task state is a git commit. Drive it through the CLI: prefer `backlog` commands over hand-editing task files, so IDs, filenames, and metadata stay consistent.

## Running it in this repo

This project has `backlog.md` as a root Bun-workspace devDependency. Every JavaScript CLI goes through the one generic `bun` Pixi task:

```
pixi run bun x --bun backlog <args>
```

- The generic task depends on `bun-install`, so the workspace installs itself on first use; `pixi run setup` also covers it. `--bun` overrides the `backlog` binary's `#!/usr/bin/env node` shebang because the `default` Pixi environment carries no Node.js (ADR-010). A bare `backlog` or `node_modules/.bin/backlog` therefore fails with `env: node: No such file or directory`.
- If `backlog/` does not exist yet, run `pixi run bun x --bun backlog init` once to scaffold it.

## Agent mode

Read commands default to an **interactive TUI** that never returns in a non-interactive shell. Always run non-interactively:

- Add `--plain` to every read (`task list`, `task <id>`, `search`) for stable text.
- Add `--json` when you will parse the output; `--json` is versioned and machine-readable.
- Never launch `backlog board` (bare), `backlog browser`, or `backlog task <id>` without `--plain` — they block on a UI.

## The loop

1. **Find work** — `pixi run bun x --bun backlog task list -s "To Do" --plain`, or `pixi run bun x --bun backlog search "<query>" --plain`.
2. **Read before coding** — `pixi run bun x --bun backlog task <id> --plain`. Read the acceptance criteria and any plan first; match test and interface names to the task's vocabulary.
3. **Claim and plan** — `pixi run bun x --bun backlog task edit <id> -s "In Progress" -a @me --plan "approach"`.
4. **Record progress in notes** (execution log), not comments — `pixi run bun x --bun backlog task edit <id> --notes "..."` then `--append-notes "..."` for more lines.
5. **Verify against AC** — mark each criterion with `--check-ac <n>`; write a PR-ready `--final-summary "..."` when the work is done.
6. **Close** — set `-s Done`, or `pixi run bun x --bun backlog task complete <id>` during cleanup (keeps the record and dependency links). Use `pixi run bun x --bun backlog task archive <id>` for cancelled, duplicate, or invalid work.

## Creating tasks

```
pixi run bun x --bun backlog task create "Title" -d "description" --ac "First,Second" -l area --priority high --dep task-1
```

Sub-task: add `-p <parent-id>`. Draft: add `--draft`, or `backlog draft create "..."` then `backlog draft promote <id>`. Full flag surface is in [`REFERENCE.md`](REFERENCE.md).

## Input gotchas

- **Multi-line** description/plan/notes: repeat the `--append-*` variant once per line (works in every shell, including agent sandboxes), or put real newlines inside double quotes. Do **not** use `$'line1\nline2'` — tree-sitter agent sandboxes reject it.
- **Literal backticks** in task text: single-quote the argument (`'Document `backlog init` setup'`), or the shell runs command substitution before Backlog.md sees the text.
- **Notes vs comments**: implementation notes and the final summary carry execution progress; comments are append-only review discussion (`--comment "..." --comment-author @you`). A standalone `---` line is reserved as a comment delimiter.

## More

For the exhaustive command and flag table (task edit AC/DoD operations, dependencies, milestones, drafts, board/export, browser, config, JSON shapes) see [`REFERENCE.md`](REFERENCE.md), and trust `pixi run bun x --bun backlog <command> --help` as the live source of truth over any cached list here.
