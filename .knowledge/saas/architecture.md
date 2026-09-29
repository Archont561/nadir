---
type: Architecture
title: "SaaS Architecture — SvelteKit + Nadir"
description: "Product definition, why SvelteKit + Nadir, the primitive mapping, rendering strategy, five cache layers, the user journey mapped to routes, and the phased build plan."
purpose: Full system design of the NadrScan SaaS application
last_updated: 2026-09-30
status: stable
related:
  - context.md
  - data-flow.md
  - integration-bridge.md
  - deployment-topology.md
---

# SaaS Architecture — SvelteKit + Nadir

## The Product

A **photogrammetry SaaS**. Users upload drone imagery; the system produces:

- **Orthomosaics** (GeoTIFF / PMTiles) — georeferenced 2D imagery
- **Point clouds** (LAZ / Potree octree) — 3D dense reconstruction
- **Meshes** (OBJ / GLB) — textured 3D surface
- **DSM / DTM** (GeoTIFF) — elevation models
- **Reports** (PDF / JSON) — QC metrics, GSD, coverage

Target users: surveyors, agri-tech, construction, real-estate mapping.

## Why This Stack

The traditional stack for this problem is:

```
React/Next.js UI  →  Node.js BFF  →  Python worker  →  ODM monolith
```

That stack has four problems:

1. **ODM is a black box** — no caching, no resume, no per-stage control.
2. **Python worker is a bottleneck** — GIL, memory, subprocess management.
3. **Next.js ships ~300KB JS** for pages that are 95% content.
4. **The BFF is duplicated business logic** between UI and backend.

**Nadir + SvelteKit eliminates all four:**

| Problem | Nadir solution | SvelteKit solution |
|---|---|---|
| Black-box pipeline | Step-scoped engines with blake3 artifact hashing | — |
| Python bottleneck | Rust worker calls engines via `tokio::process` | — |
| Fat client bundle | — | Compiled Svelte components, SSR-first, no VDOM runtime |
| Duplicated BFF | — | The app IS the backend; `load()`, actions, and services in one codebase |

## Primitive Mapping (Lintel → SvelteKit)

The NadirScan bundle designed the app around 12 Lintel primitives. Under
ADR-009 each maps onto a SvelteKit construct:

```
html()       → +page.svelte with prerender = true      (marketing, docs)
server()     → +page.server.ts load()                  (dashboards, lists)
island()     → Svelte component                        (upload, viewers)
route()      → file-based routing
action()     → form actions + api/+server.ts endpoints
service()    → $lib/server/services/*.ts               (R2, RunPod, Stripe, Nadir)
resource()   → $lib/server/db (Drizzle + Neon PostGIS)
store()      → Svelte stores / $state runes
middleware() → hooks.server.ts handle()
channel()    → +server.ts SSE endpoint + EventSource
event()      → server EventEmitter / Redis pub-sub
workflow()   → $lib/server/pipeline.ts + workflow_state table
```

**The photogrammetry pipeline is one workflow.** Everything else is UI + glue.

## Rendering Strategy

| Tier | Mechanism | % of updates | Cost |
|---|---|---|---|
| 1 — CSS | class/attribute swap → CSS selector → paint | 70% | ~0.05ms |
| 2 — Text | `textContent = newValue` | 20% | ~0.5μs |
| 3 — DOM | Svelte reactivity (template update) | 10% | ~10–100μs |

The progress bar during processing? Tier 1 (a `data-stage` attribute + CSS).
The stage label updating? Tier 2. A new artifact appearing in the results
list? Tier 3. Svelte's compiled updates make Tier 3 cheap enough that the
distinction matters less than in React — but CSS-first remains the habit.

Heavy viewers (MapLibre ~200KB, Potree ~800KB, Three.js ~600KB) are
**dynamic imports inside `onMount`** — they never ship in the initial
bundle and never run during SSR.

## The 5 Cache Layers

```
L0 request dedup  →  L1 gateway  →  L2 fragment  →  L3 CDN  →  L4 browser
```

- **L4 (browser)** — static assets, PMTiles, Potree octree (immutable, content-hashed)
- **L3 (CDN, Cloudflare)** — prerendered marketing/docs pages, R2 objects via custom domain
- **L2 (fragment)** — dashboard project list (5-min stale) → Redis-cached `load` data (SvelteKit has no built-in fragment cache; cache the data, not the HTML)
- **L1 (gateway)** — Clerk user objects (5-min TTL), Neon query results
- **L0 (request dedup)** — SvelteKit's `event.fetch` deduplicates within a request automatically

Nadir's own artifact cache (blake3 content-addressed) sits **below** L1 —
it's the compute cache. Same input images → same artifact hash → skip work.

## User Journey → Route Map

```
1. Landing page            → /                (prerendered, CDN, 0KB JS)
2. Sign up                 → Clerk-hosted UI + POST /api/webhooks/clerk
                                                 → provisioning chain
3. Dashboard               → /dashboard       (+page.server.ts load, SSR)
4. Upload drone photos     → /projects/new    (UploadZone component →
                              POST api/uploads/authorize → browser → R2 direct)
5. Processing starts       → upload complete → workflow begins
6. Progress view           → /projects/[id]   (SSR snapshot + SSE live updates)
7. Results                 → /projects/[id]/results (metadata SSR +
                              MapLibre / Potree / Three.js lazy components)
8. Download                → POST api/outputs/[id]/link → R2 presigned URL
9. Billing                 → /billing         (plans, top-up, ledger history)
```

## Phased Build Plan

| Phase | Duration | Deliverable | SvelteKit surface | Nadir version |
|---|---|---|---|---|
| **MVP** | 6 weeks | Upload → process → download orthomosaic | routes, load, form actions, api endpoints, UploadZone | V0 fat worker, `standard.toml` |
| **V1** | 8 weeks | Progress UI, 3D viewer, billing | + SSE +server.ts, viewers, Stripe webhooks | V0 + progress parsing, artifact model |
| **V2** | 10 weeks | Multi-pipeline, GCP, exports, admin | + admin section, event bus, durable queues | V1 domain workers, QC gates, adaptive planner |
| **V3** | 12 weeks | API, MCP, AI suggestions, edge scale | + public API versioning, edge caching | V2 per-engine workers, protobuf protocol |

## What This SaaS Is NOT

- **Not a design tool** — no annotation, no CAD, no collaboration cursors.
- **Not a GIS platform** — no vector editing, no analysis workflows beyond photogrammetry.
- **Not a general 3D viewer** — Potree/Three.js for output display, not modeling.
- **Not real-time SLAM** — batch photogrammetry only, minutes-to-hours per job.

## See Also

- [Integration bridge](integration-bridge.md) — how the gateway talks to Nadir
- [Data flow](data-flow.md) — reads, writes, streams, compute lifecycles
- [Deployment topology](deployment-topology.md) — edge / gateway / compute zones
- [ADR-009](../decisions/009-sveltekit-gateway-for-nadrscan.md) — why SvelteKit
