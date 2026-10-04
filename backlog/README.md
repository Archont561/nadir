# Nadir backlog

This directory owns project change over time. It contains executable work items and the
planning records that explain why those items exist.

- `tasks/` — Backlog.md task records (`nd-*`).
- `milestones/` — release milestones managed by Backlog.md.
- `docs/decisions/` — accepted architecture decision records.
- `docs/roadmaps/` — version roadmaps and milestone detail.
- `docs/plans/` — time-bound delivery plans.

Use `pixi run bun x --bun backlog task list --plain` to inspect work and the Backlog.md CLI to mutate
front matter. `.knowledge/` deliberately contains durable technical reference only:
architecture, photogrammetry, deployment, and dependency information.
