---
type: Architecture
title: "Deployment Topology — Edge, Gateway, Workers"
description: "The three-zone model, the one-image-two-entrypoint SvelteKit build, the separate Nadir GPU image, concrete MVP deployment, health checks and shutdown, and the scaling path."
purpose: Where the SaaS runs and how it grows
last_updated: 2026-09-30
status: stable
related:
  - context.md
  - infrastructure.md
  - ../deployment/docker-strategy.md
  - ../architecture/worker-protocol.md
---

# Deployment Topology — Edge, Gateway, Workers

## The Three-Zone Model

```
┌─ ZONE 1: EDGE (Cloudflare) ─────────────── 90% of traffic ─┐
│  - Prerendered pages (marketing, docs) + static assets      │
│  - R2 (images, artifacts, PMTiles, Potree octrees)          │
│  - $0 egress                                                │
└─────────────────────────────────────────────────────────────┘
                          │
                          │ 10% (dynamic + auth + uploads)
                          ▼
┌─ ZONE 2: GATEWAY (Fly.io, adapter-node) ───── ~$5–20/mo ───┐
│  - SvelteKit node build (apps/nadirscan)                    │
│  - hooks.server.ts: Clerk auth, rate limiting               │
│  - load()/actions: dashboard, uploads, downloads            │
│  - Pipeline orchestrator + SSE progress                     │
│  - No GPUs. No large-file buffering.                        │
└─────────────────────────────────────────────────────────────┘
                          │
             ┌────────────┼─────────────┐
             ▼            ▼             ▼
       ┌──────────┐  ┌─────────┐  ┌──────────┐
       │  Neon    │  │  Redis  │  │  RunPod  │
       │ Postgres │  │ BullMQ  │  │   GPU    │
       │ + PostGIS│  │ (V1)    │  │ workers  │
       └──────────┘  └─────────┘  └──────────┘
                                       │ Nadir engine
                                       ▼
                              ┌────────────────────┐
                              │ ZONE 3: COMPUTE    │
                              │ RunPod Serverless  │
                              │ COLMAP, OpenMVS,   │
                              │ GDAL/PROJ/PDAL,    │
                              │ PotreeConverter,   │
                              │ tippecanoe         │
                              │ Scale to zero      │
                              └────────────────────┘
```

## The SvelteKit Build: One Image, Two Entrypoints

`@sveltejs/adapter-node` produces a self-contained Node server. Gateway and
background worker share one Docker image; the worker overrides CMD:

```dockerfile
# apps/nadirscan/Dockerfile
FROM node:22-alpine AS builder
WORKDIR /app
COPY package*.json ./
RUN npm ci
COPY . .
RUN npm run build          # vite build → build/ (adapter-node)

FROM node:22-alpine
WORKDIR /app
COPY --from=builder /app/build ./build
COPY --from=builder /app/node_modules ./node_modules
COPY package.json ./
EXPOSE 3000
ENV NODE_ENV=production

# Gateway (default)
CMD ["node", "build"]

# Background worker (post-processing, billing, emails — MVP runs these
# in-process; separate container from V1):
#   docker run <image> node build/worker.js --queues=postprocess,billing
```

## The Nadir GPU Image — Separate

Nadir workers are a **separate image** (Rust + CUDA + engines, ~4GB — fine,
RunPod caches it after first pull):

```dockerfile
# Dockerfile.nadir (engine repo)
FROM nvidia/cuda:12.2.0-runtime-ubuntu22.04
RUN apt-get update && apt-get install -y \
    colmap openmvs gdal-bin proj-bin pdal \
    potreeconverter draco-transcoder tippecanoe
COPY --from=nadir-builder /build/target/release/nadir /usr/local/bin/nadir
ENTRYPOINT ["nadir", "serve", "--http", "--port=8080"]
```

Engine-side packaging details (Pixi-based build, inverted container model)
live in [../deployment/docker-strategy.md](../deployment/docker-strategy.md).

## Concrete Deployment (MVP)

```yaml
# Zone 1: Cloudflare (free tier + R2)
cloudflare:
  cdn: "*.nadrscan.app"                    # $0
  r2:
    bucket: nadrscan-data
    lifecycle:
      - path: "*/raw/*"      delete_after: 90d
      - path: "*/outputs/*" keep_forever: true

# Zone 2: Fly.io (gateway)
fly:
  app: nadrscan-gateway
  regions: [waw]                  # single region for MVP (Warsaw)
  size: shared-cpu-1x-1gb         # ~$5/mo
  env: CLERK_*, DATABASE_URL, R2_*, STRIPE_*, NADIR_WORKER_URL, APP_URL

# Zone 3: RunPod (compute)
runpod:
  endpoint: nadir-engine          # A5000, min 0 / max 5, idle 60s
```

## Health Checks & Shutdown

- **Gateway**: `GET /healthz` (built into the app; checks store + services,
  returns which services run in demo mode). Fly.io checks every 10s.
- **Nadir worker**: `GET /_health` → `{ status, version, engines, gpu, disk }`
- **Graceful shutdown**: gateway stops accepting SSE, drains in-flight
  actions, lets the workflow state persist; `SIGTERM` → 30s → `SIGKILL`.
  Running Nadir jobs are unaffected (they live on RunPod).

## Demo Mode

Every external service is optional at runtime: unset env vars fall back to
in-process implementations (in-memory store, simulated worker, local upload
sink, instant grants). `healthz` reports which services are live vs demo.
This keeps local dev, CI, and previews zero-config.

## Scaling Path

| Tier | Infra | Trigger to upgrade |
|---|---|---|
| **Starter** | 1 gateway (1x1gb), Neon free, R2 500GB, RunPod A5000 0–3, Redis free | gateway CPU >80% or SSE drops at ~200 streams |
| **Growth** | 3 gateways (autoscale), Neon Launch $19, 2 GPU endpoints (A4000 small jobs + A5000), Upstash $10, Grafana Pro | RunPod cold starts >30s, queue >10 |
| **Scale** | 9 gateways (3 regions: WAW/IAD/NRT), Neon Scale + read replicas, active GPU pool (min 2) + flex burst 30, TiTiler ×2 | GPU bill >$5k/mo |
| **Enterprise** | K8s, self-hosted GPUs (8×A6000 ≈ €1,500/mo vs $5k RunPod), 500TB R2, edge compute | — |

Full decision tree: see [billing.md](billing.md) (scaling section).

## See Also

- [Infrastructure](infrastructure.md) — the services referenced above
- [Docker strategy](../deployment/docker-strategy.md) — the engine-side image
- [Hosting comparison](../deployment/hosting-comparison.md) — why Fly.io for the gateway
