---
type: Infrastructure
title: "Infrastructure — R2, Neon, RunPod, Resend, Grafana, TiTiler, tus"
description: "The external services the SaaS runs on: selection rationale, bucket layout, connection details, endpoint configuration, env var catalog, and the service dependency graph."
purpose: Service-by-service wiring reference for the SaaS
last_updated: 2026-09-30
status: stable
related:
  - context.md
  - deployment-topology.md
  - billing.md
  - integration-bridge.md
---

# Infrastructure — R2, Neon, RunPod, Resend, Grafana, TiTiler, tus

## Service Dependency Graph

```
                    Clerk (auth)
                        │
                        ▼
Browser ──→ SvelteKit gateway ──→ Neon (metadata, PostGIS)
                │    │
                │    ├──→ R2 (storage)
                │    │      ↑
                │    │      │ (direct upload from browser)
                │    │
                │    ├──→ RunPod ──→ Nadir engine ──→ R2 (read raw, write outputs)
                │    │                    │
                │    │                    ├── COLMAP (GPU)
                │    │                    ├── OpenMVS (GPU)
                │    │                    ├── GDAL/PROJ (CPU)
                │    │                    └── PDAL (CPU)
                │    │
                │    ├──→ Stripe (billing)      ├──→ Resend (email)
                │    └──→ Grafana (metrics)     └──→ TiTiler (V1+, dynamic tiles)
                │
                └──→ Redis (queues, cache, pub/sub — V1)
```

Every external client lives in `$lib/server/services/*.ts` — one module per
service, imported by actions and the pipeline, never shipped to the browser.

## Cloudflare R2 — Object Storage

**Why R2**: $0 egress is decisive. On S3, one active user viewing results
20×/month costs ~$1.80 in egress; on R2, $0. Storage ~$0.015/GB-mo.

```
nadrscan-data/
├── {clerk_user_id}/
│   └── {project_id}/
│       ├── raw/                    # uploaded drone photos
│       │   ├── 0001.jpg ...
│       └── outputs/                # Nadir + post-processor output
│           ├── sparse.ply, dense.laz, ortho.tif, dsm.tif, dtm.tif, mesh.obj
│           ├── potree/             # octree (directory of small files)
│           ├── ortho.pmtiles       # single-file tiles
│           ├── mesh.glb            # Draco GLB
│           ├── thumbnail.png, report.pdf
```

- Path starts with `{clerk_user_id}` — trivial prefix-scoped credentials later
- `raw/` and `outputs/` are siblings — independent lifecycle rules
  (delete raw after 90d, keep outputs forever)
- S3-compatible endpoint: `https://{ACCOUNT_ID}.r2.cloudflarestorage.com`,
  AWS SDK v3 + `@aws-sdk/s3-request-presigner`
- Public reads (PMTiles, Potree) via R2 custom domain (`cdn.nadrscan.app`)
  with CORS for Range requests

## Neon Postgres — Metadata + PostGIS

**Why Neon**: PostGIS is decisive (`ST_Intersects`, `ST_MakeEnvelope`,
`ST_DWithin` — impossible on D1/SQLite); autoscale to zero; branch databases
(every PR gets an isolated DB branch with production schema — hidden killer
feature); Supabase rejected as overlap (bundles auth/storage/realtime we
already have).

Connection details:

- **HTTP driver** (`@neondatabase/serverless` + `drizzle-orm/neon-http`),
  not WebSocket — stateless, cold-start-friendly, burns no idle compute
  hours on the free tier
- PostGIS enabled once per branch: `CREATE EXTENSION IF NOT EXISTS postgis;`
- Free tier: 0.5GB + 190 compute-hrs/mo (~3,000 users) → Launch $19/5GB

## RunPod — Serverless GPU for Nadir

**Why RunPod Serverless**: scale to zero (photogrammetry is sporadic),
bring-your-own Docker (Nadir is a Rust binary in a CUDA image), per-second
billing (a 12-min job costs 1/5 of an hourly minimum). Lambda Cloud = 24/7
VMs (V2+ training); Vast.ai = dev only; Modal = Python-first, poor fit.

```yaml
# runpod endpoint (conceptual)
name: nadir-engine
image: registry.nadrscan.app/nadir-engine:v0.5.0
gpu_type: RTX A5000          # 24GB VRAM
min_workers: 0               # scale to zero
max_workers: 5
idle_timeout: 60             # seconds → GPU released, billing stops
execution_timeout: 14400     # 4h max per job
env: R2_* (write-scoped token), RUST_LOG=info
```

GPU selection by stage: features/matching 4–12GB (A4000 ok), **dense MVS
12–20GB (A5000 recommended)**, meshing 8–16GB, SfM/ortho CPU-bound.
MVP: one A5000 endpoint covers 95% of jobs. Async `/run` + `/status/{id}`
polling (not `/runsync`).

Per-job cost example: 42-min standard job on A5000 Flex ≈ $0.41
($0.58/hr × 0.7h).

## Resend — Transactional Email

Five templates (welcome, processing complete, processing failed, low
credits, receipt) built with React Email, sent from the `emails` queue —
best-effort, never blocks the workflow. Domain: `mail.nadrscan.app`.
Webhook tracks bounces → mark users.

## Grafana Cloud — Monitoring (free tier)

- **Metrics**: prom-client in the gateway (`http_request_duration`,
  `sse_connections`, `workflow_active`, `credits_granted_total`) +
  Fly.io drain
- **Logs**: Loki via Fly log drain; Nadir workers ship `tracing` JSON
- **Traces**: Tempo (fetch + Nadir client spans)
- Dashboards: Gateway (p95, SSE), Jobs (queue depth, stage durations,
  GPU-hours), Business (signups, credits, revenue)
- Alerts: SSE error rate > 5%, workflow failure rate > 10%, RunPod queue
  depth > 10, Neon storage > 80%

## TiTiler — Dynamic COG Tiles (V1+)

Static PMTiles cover viewing. TiTiler adds **dynamic** operations on COGs
on-the-fly: NDVI (`/cog/statistics?url=…&bands=1,2`), hillshade,
band math, statistics — served for Pro/Team plans from a small Fly.io app
the gateway proxies. MVP skips it.

## tus.io — Resumable Uploads (V2)

MVP presigned PUTs assume decent connectivity. Field surveys on mobile data
need **tus** resumable uploads: `tusd` sidecar (Go binary) writes directly
to R2 with `--s3-bucket`, browser uses `tus-js-client`, the gateway handles
the `post-finish` webhook → same `dataset.uploadComplete` event. ~$0 (runs
alongside the gateway).

## Environment Catalog

| Group | Variables | Failure mode when unset |
|---|---|---|
| Storage | `R2_ACCOUNT_ID`, `R2_ACCESS_KEY_ID`, `R2_SECRET_ACCESS_KEY`, `R2_BUCKET`, `R2_PUBLIC_BASE_URL` | local upload sink (demo) |
| Database | `DATABASE_URL` | in-memory store with seed data (demo) |
| Compute | `NADIR_WORKER_URL`, `NADIR_WORKER_TOKEN` | simulated 16-stage worker (demo) |
| Auth | `CLERK_PUBLISHABLE_KEY`, `CLERK_SECRET_KEY`, `CLERK_ISSUER_URL`, `CLERK_WEBHOOK_SECRET`, `CLERK_SIGN_IN_URL` | fixed demo user |
| Billing | `STRIPE_SECRET_KEY`, `STRIPE_WEBHOOK_SECRET`, `STRIPE_PRICE_TOPUP`, `STRIPE_PRICE_PRO`, `STRIPE_PRICE_TEAM` | instant grants (demo) |
| Email | `RESEND_API_KEY` | emails logged, not sent |
| App | `APP_URL` | `http://localhost:5173` |

Every unset service degrades to an in-process demo implementation — the
whole product (upload → pipeline → results → billing) runs with **zero
credentials**, which is also how CI and screenshots work.

## See Also

- [Deployment topology](deployment-topology.md) — how these are deployed and scaled
- [Integration bridge](integration-bridge.md) — the RunPod/Nadir client
- [Billing](billing.md) — Stripe flows in detail
