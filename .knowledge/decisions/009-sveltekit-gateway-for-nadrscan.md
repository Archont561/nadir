---
type: Architecture Decision Record
title: "ADR-009: SvelteKit Gateway for NadrScan (apps/nadirscan)"
description: "Records the accepted architecture decision to build the NadrScan SaaS gateway as apps/nadirscan with SvelteKit."
status: accepted
date: 2026-09-30
deciders: project lead
related:
  - ../saas/context.md
  - ../saas/architecture.md
  - 002-three-language-split.md
  - ../architecture/worker-protocol.md
  - ../deployment/demo-plan.md
---

# ADR-009: SvelteKit Gateway for NadrScan (apps/nadirscan)

## TL;DR

The NadrScan photogrammetry SaaS — specified in the external NadirScan
knowledge bundle as a **Lintel** BFF gateway over the **Nadir** Rust engine —
will be built as **`apps/nadirscan`** in this monorepo using **SvelteKit**
(Svelte 5, `@sveltejs/adapter-node`). The 12 Lintel primitives map onto
native SvelteKit constructs (see table below). The Nadir Rust side of the
design is unchanged. This decision supersedes the UI stacks in
[`deployment/demo-plan.md`](../deployment/demo-plan.md) (Astro + WebcoreUI)
and the "Next.js UI" note in [`context.md`](../context.md).

## Context

Three documents currently specify three different UI stacks for the SaaS:

| Source | Stack |
|---|---|
| NadirScan knowledge bundle (`Archont561/NadirScan`, OKF v0.2, 34 concepts) | **Lintel** — bespoke TypeScript BFF framework (12 primitives, ~4.5KB client, CSS-first reactivity) |
| `deployment/demo-plan.md` | **Astro** + WebcoreUI + Clerk + Drizzle + Inngest |
| `context.md` (Current Status) | **Next.js** UI |

The NadirScan bundle (fetched 2026-09-30) contains the complete SaaS design:
data model (six Postgres tables + PostGIS), the 16-stage pipeline workflow,
Clerk auth chain, Stripe credit billing, R2 storage layout, RunPod GPU
topology, and the UI flows (upload, SSE progress, MapLibre/Potree/Three.js
viewers). All of that is engine-agnostic and worth keeping — it has been
ported into [`.knowledge/saas/`](../saas/context.md).

The open question was the gateway framework. Lintel does not exist as a
production-grade dependency; adopting it would mean building a framework
before building the product.

## Decision

**Build the NadrScan gateway as `apps/nadirscan` with SvelteKit.**

### Monorepo layout

```
nadir/
├── apps/
│   └── nadirscan/          # SvelteKit SaaS gateway (this decision)
│       └── src/
│           ├── routes/         # pages, form actions, +server.ts APIs
│           ├── lib/            # shared components + domain types
│           └── lib/server/     # server-only: db, services, pipeline
├── crates/                 # Rust workspace: nadir-cli, nadir-core, ...
├── pipelines/              # standard.toml, fast.toml, survey.toml
├── pixi.toml               # system deps (COLMAP, GDAL, ...) for the engine
└── .knowledge/             # this bundle (+ new saas/ section)
```

### Lintel primitives → SvelteKit constructs

The SaaS design survives intact; only the framework vocabulary changes:

| Lintel primitive | SvelteKit equivalent |
|---|---|
| `html()` (static page) | `+page.svelte` with `export const prerender = true` |
| `server()` (server fragment) | `+page.server.ts` `load()` — SSR HTML, streamed |
| `island()` (interactive island) | Svelte component — compiled, hydrates lazily |
| `route()` | File-based routing (`src/routes/…`) |
| `action()` (typed RPC) | Form actions (`actions` in `+page.server.ts`) + `+server.ts` JSON endpoints under `api/` |
| `service()` (external API client) | `$lib/server/services/*.ts` — plain TS modules, server-only by framework guarantee |
| `resource()` (typed data layer) | `$lib/server/db` — Drizzle ORM over Neon (PostGIS) |
| `store()` (reactive state) | Svelte stores / `$state` runes |
| `workflow()` (long-running DAG) | `$lib/server/pipeline.ts` orchestrator + `workflow_state` table (BullMQ/Inngest when durable queues are needed) |
| `channel()` (SSE/WebSocket) | `+server.ts` SSE endpoint (`ReadableStream`) + browser `EventSource` |
| `event()` (typed bus) | Server `EventEmitter` / Redis pub-sub for fan-out |
| `middleware()` | `hooks.server.ts` `handle()` (auth, rate limit, CORS) |

### The 3 rules, preserved

`HTML for reads, JSON for writes, CSS for visuals, GPU for compute` maps
directly: SSR `load()` functions for reads, form actions / `api/*` endpoints
for writes, scoped Svelte CSS for visuals, Nadir workers on RunPod for
compute.

## Rationale

| Factor | Lintel | SvelteKit | Next.js / Astro |
|---|---|---|---|
| Maturity | Unbuilt concept | Mature, production-proven | Mature |
| Client JS | ~4.5KB ideal | Compiled components, no VDOM runtime — close in practice | 300KB+ typical (Next.js) |
| SSR + islands | Core idea | Native (SSR default, hydrate what needs it) | Next: RSC; Astro: islands |
| Routing | `route()` | File-based, zero config | File-based |
| Server-only enforcement | Convention | `$lib/server` is a build-time guarantee | Convention |
| Typed RPC | `action()` | Form actions with progressive enhancement | Server actions (Next) |
| SSE streaming | `channel()` | `+server.ts` + `ReadableStream` | Route handlers |
| Hiring / ecosystem | None | Large (Svelte community) | Largest |
| Risk | Framework must be built first | None | None |

SvelteKit is the closest production-grade equivalent to what the NadirScan
bundle actually wanted from Lintel: server-rendered HTML by default, tiny
hydrated islands, one TypeScript codebase for gateway + UI, and no
duplicated BFF layer.

## Consequences

### Positive
- One mainstream framework instead of three competing plans; docs now agree
- Svelte compiler output preserves the "static by default, small client"
  goal well enough (~10–40KB per interactive page vs ~300KB React)
- `$lib/server` gives a hard import boundary for secrets (R2, Stripe, Clerk)
- `adapter-node` → one Docker image, `node build` gateway + worker via CMD
  override (same one-image-two-entrypoint pattern the bundle prescribed)
- The ported `.knowledge/saas/` section keeps every non-UI decision
  (data model, billing, storage, GPU topology) intact
- Demo-mode fallbacks let the app run with zero external credentials

### Negative
- Client bundle is larger than Lintel's theoretical 4.5KB — mitigated by
  lazy-loading heavy viewers (MapLibre, Potree, Three.js) via dynamic import
- SvelteKit has no built-in durable workflow engine — the 16-stage pipeline
  orchestrator is app-level code with `workflow_state` persistence; adopt
  BullMQ or Inngest (per demo-plan heritage) when multi-process durability
  is required
- Fragment cache (Lintel L2) has no direct equivalent — use CDN caching of
  prerendered content + Redis-cached `load` data instead

### Deferred
- MVP: single process, in-memory queue + SSE bus, demo-mode services
- V1: Redis + BullMQ workers, Clerk + Stripe live, RunPod endpoint
- V2: multi-region gateways, TiTiler, tus resumable uploads

## See Also

- [SaaS project context](../saas/context.md) — the ported NadrScan summary
- [SaaS architecture](../saas/architecture.md) — full system design on SvelteKit
- [ADR-002: Three-language split](./002-three-language-split.md) — TS app / Rust worker split this fits into
- [Worker protocol](../architecture/worker-protocol.md) — the Nadir side of the bridge
- [Demo plan](../deployment/demo-plan.md) — superseded UI stack (Astro)
