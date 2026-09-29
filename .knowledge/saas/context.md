---
type: Project Context
title: "NadrScan SaaS — Project Context"
description: "One-page summary of the NadrScan SaaS: SvelteKit gateway + Nadir engine, the five core decisions, the three rules, and the MVP cost envelope. Ported from the NadirScan knowledge bundle and adapted to ADR-009."
purpose: Hot-path summary of the SaaS application for every LLM session
last_updated: 2026-09-30
status: stable
related:
  - ../context.md
  - architecture.md
  - integration-bridge.md
  - deployment-topology.md
---

# NadrScan SaaS — Project Context

## What Is This

A **photogrammetry SaaS** built by combining two projects:

- **Nadir** — Rust-based photogrammetry pipeline (COLMAP + OpenMVS +
  GDAL/PROJ/PDAL). Composes existing engines into a resumable, cacheable,
  content-hashed DAG. Lives in `crates/` of this monorepo.
- **NadrScan** — the SaaS application on top: **SvelteKit gateway** at
  `apps/nadirscan` (ADR-009), which handles auth, uploads, billing, progress
  streaming, and result viewing.

Users upload drone photos, get back orthomosaics, point clouds, meshes, and
DSM/DTM elevation models.

> Ported from the external NadirScan knowledge bundle (OKF v0.2, 34
> concepts, `Archont561/NadirScan`, fetched 2026-09-30). The bundle
> specified a **Lintel** BFF gateway; per ADR-009 the gateway is **SvelteKit**
> instead. Everything else — data model, pipeline, billing, storage, GPU
> topology — is carried over unchanged.

## Architecture in One Diagram

```
Browser (SSR pages + hydrated Svelte components)
   │
   ▼
SvelteKit gateway (apps/nadirscan, adapter-node)
   ├── hooks.server.ts — Clerk JWT auth, rate limiting
   ├── +page.server.ts load() — SSR reads (dashboard, results)
   ├── form actions + api/* — JSON writes (uploads, billing)
   ├── +server.ts — SSE progress stream
   └── $lib/server/pipeline.ts — 16-stage photogrammetry workflow
       └── services/nadir.ts → RunPod GPU workers (Nadir engine)
   │
   ├── Neon Postgres (PostGIS)  — metadata, users, credits
   ├── Cloudflare R2            — images, artifacts, PMTiles ($0 egress)
   ├── Redis + BullMQ           — job queues (V1; in-memory for MVP)
   └── Stripe + Resend          — billing + email
```

## The 5 Core Decisions

1. **SvelteKit replaces Next.js + BFF** (ADR-009) — one framework for
   gateway + UI; SSR reads, form-action writes, SSE progress. The original
   bundle's Lintel primitives map 1:1 onto SvelteKit constructs.
2. **Nadir replaces custom compute** — Rust workers wrap COLMAP/OpenMVS/GDAL
   via `tokio::process`, exposed over the V0.5 HTTP+JSON worker protocol.
3. **Cloudflare R2 for storage** — $0 egress is decisive for 1–10GB artifact
   downloads (point clouds, PMTiles).
4. **Neon over D1** — PostGIS is required for spatial queries
   (`ST_Intersects`, `ST_DWithin`, ...).
5. **Clerk for auth** — free tier (10k MAU), Clerk-hosted UI,
   webhook → Neon provisioning.

## The 3 Rules

- **HTML for reads, JSON for writes, CSS for visuals, GPU for compute.**
- **Server code stays in `$lib/server`** — the framework enforces the
  import boundary; everything else is shared or client code.
- **Static by default** — CDN serves 90%, gateway handles 10%, workers do
  the heavy work.

## Cost Envelope (MVP)

~$60/month at 50 jobs/month:

| Service | Cost |
|---|---|
| Clerk (auth, free tier) | $0 |
| Neon (Postgres + PostGIS, free tier) | $0 |
| R2 (500GB storage) | ~$7.50 |
| RunPod (50 GPU-hours) | ~$29 |
| Stripe fees | ~$18 |
| Fly.io gateway | ~$5 |
| Grafana Cloud free | $0 |

## Monorepo Placement

```
nadir/
├── apps/nadirscan/     # this SaaS (SvelteKit)
├── crates/             # Nadir engine (Rust)
├── pipelines/          # standard.toml, fast.toml, survey.toml
└── .knowledge/saas/    # this section
```

## Where To Go Next

- **System design** → [`architecture.md`](architecture.md)
- **Gateway ↔ Nadir bridge** → [`integration-bridge.md`](integration-bridge.md)
- **The 16-stage workflow** → [`pipeline-workflow.md`](pipeline-workflow.md)
- **Data model & credit ledger** → [`data-model.md`](data-model.md)
- **Deployment & scaling** → [`deployment-topology.md`](deployment-topology.md)
- **Why SvelteKit** → [`../decisions/009-sveltekit-gateway-for-nadrscan.md`](../decisions/009-sveltekit-gateway-for-nadrscan.md)

## See Also

- [Nadir project context](../context.md) — the engine this SaaS wraps
- [Worker protocol](../architecture/worker-protocol.md) — Nadir's side of the HTTP API
