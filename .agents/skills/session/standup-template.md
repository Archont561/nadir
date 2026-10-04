# Nadir session lifecycle templates

Use these four artifacts in order: the **opening prompt** begins a session, the **standup**
proposes its scope, a **task hand-off** proves each completed task, and the **session report**
closes the work. Keep the headings and field order so one session can be compared with the next.

The report's last block is the next opening prompt. Carry forward settled decisions and proof
gaps, not the full deliberation that produced them.

## 1. Session opening prompt

Written at close and pasted as the first message of the next session.

```text
Enter the Nadir sandbox and preserve the current working tree. Expect <full gates | Rust-only
offline baseline>: <observed counts/verdict>. The published transport was built from <source
commit>, carries Pixi <version> and pixi-sandbox <version>, and <matches | differs from> the
current pixi.lock because <reason>.

Read AGENTS.md, .knowledge/context.md, and <task/ADR paths that matter>. The current branch is
<branch>; <working-tree/PR/merge fact that prevents the next agent from guessing>.

Start with <ND-id or concrete decision>. <One sentence describing the deliverable and every
implementation choice already settled>. Prove <local slices> locally; <CI, sandbox repack, real
dataset, platform, or maintainer proof> remains behind <explicit authorization or event>.

Survey any changed remote or backlog state, propose the slice using the session skill, and stop
for confirmation before editing.
```

A useful opening prompt includes:

- the observed baseline, not a remembered count;
- the transport source and lock relationship;
- the exact task and already-settled decisions;
- the current branch/dirty/PR state;
- the local-versus-remote proof boundary; and
- the action requiring user authorization.

## 2. Session standup

Produced after startup and backlog survey, then stop.

```text
Session proposal — <YYYY-MM-DD>

Environment: <devcontainer | restored sandbox | host Pixi>; Pixi <version>.
Transport: source <sha>, pixi-sandbox <version>, <lock/vendor freshness>; activation via
<registered launcher | host Pixi | restored tool path>.
Baseline: <full gates or exact partial proof> → <counts/verdict and duration if useful>.
Proof gap: <none | unavailable suite and why>.
Branch: <name>, <ahead/behind/diverged from origin/main>, working tree <clean | intentional paths>.

Backlog: <X> To Do, <Y> unblocked. Candidates, in recommended order:
1. <ND-id> (<priority>, <type>) — <deliverable, dependencies, and why now>
2. <ND-id> …
3. <ND-id> …
Not this session: <ND-id> blocked by <ND-id>; <ND-id> deferred because <reason>.

Recommended scope: <task/slice and why it fits this session>.
Slices: <locally provable work> first; <push/CI/repack/real-data/platform proof> last and only
with <authorization/event>.
Decisions needed: <decision + recommendation, or "none">.

Done means: acceptance criteria proven, narrow and full available gates green, Backlog.md notes
and final summary current, one focused Conventional Commit, and remote evidence reported rather
than assumed.
Need from you: confirm this scope or choose another candidate.
```

## 3. Task hand-off

Produced after implementation and verification, before commit/push.

```text
<ND-id> — <title>

Changed:
- <path> — <why>
- <path> — <why>

Evidence:
- <narrow command> → <result>
- <boundary/integration command> → <result>
- <full or partial baseline command> → <counts/verdict; compare with opening baseline>

Acceptance criteria: <proven AC numbers>; <open AC and missing proof, or "all proven">.
Gates: <fmt> <lint> <typecheck> <test> <version-check>; <advisory/publish checks only if in scope>.
Sandbox impact: <none | pixi.lock/Cargo.lock changed; repack required>.
Left undone: <explicit remainder or "nothing within the approved scope">.
Proposed commit: <type(scope): subject>.
```

## 4. Session report

Produced after the actual landing state and remote checks are known.

```text
Session report — <YYYY-MM-DD>

Landed: <nothing | local commit sha | pushed branch | PR number/title | merged PR and main sha>.
Post-merge runs: ci <verdict/link>; publish sandbox <verdict/link and source.commit>; <other run>.
Tasks: <ND-id status and AC state>; <next task status>.

Baseline at open: <counts/verdict>.
Evidence at close: <counts/verdict on local or landed revision>.
Proof still open: <specific AC/event/machine, or "none">.
Sandbox state: <transport source/tool versions, lock/vendor freshness, activation path>.
Working tree: <branch and clean/intentional paths>.

Decisions carried forward:
- <settled choice and why the next session should not reopen it>

Next candidates, in order:
1. <ND-id> — <why next or what blocks it>
2. <ND-id> — <why next or what blocks it>

Next session should start with:

> <template §1, filled in as a paste-ready prompt>
```

When nothing merged, preserve the same shape. `Landed: nothing` plus an exact branch, diff, and
next action is a useful report; pretending local work reached `main` is not.
