---
type: Architecture
title: "SaaS Data Flow — Reads, Writes, Streams, Compute"
description: "The four data movements (read, write, stream, compute) mapped to SvelteKit primitives, with lifecycle walkthroughs for dashboard, upload, processing, and viewing."
purpose: Request/data lifecycle reference for the SaaS gateway
last_updated: 2026-09-30
status: stable
related:
  - architecture.md
  - upload-flow.md
  - progress-channel.md
  - integration-bridge.md
---

# SaaS Data Flow — Reads, Writes, Streams, Compute

## The Four Data Movements

Every operation in the SaaS is one of these four:

| Type | Direction | Transport | Latency budget |
|---|---|---|---|
| **Read** | Server → Browser | HTML (SSR `load`) | <500ms first byte |
| **Write** | Browser → Server | JSON (form action / `api/*`) | <2s round-trip |
| **Stream** | Server → Browser (push) | SSE (`+server.ts`) | <100ms per update |
| **Compute** | Gateway → Worker → Storage | HTTP (Nadir V0.5) + R2 | minutes to hours |

## Rule: HTML for reads, JSON for writes, CSS for visuals, GPU for compute

This maps directly onto SvelteKit:

```
Read   → +page.server.ts load()      (SSR HTML, streamed)
Write  → form actions / api/+server.ts (typed JSON RPC)
Stream → +server.ts SSE endpoint     (one-way push)
Visual → Svelte components + scoped CSS
Compute → $lib/server/pipeline.ts → services/nadir.ts → GPU → R2
```

## Lifecycle 1: Loading the Dashboard

```
Browser
  │ GET /dashboard
  ▼
CDN (Cloudflare)
  │ MISS (per-user page — not cacheable)
  ▼
SvelteKit gateway (hooks.server.ts)
  │ handle(): verify Clerk JWT (L1 cache: 5m)
  │ → event.locals.user
  ▼
+page.server.ts load()
  │ store.listProjects(userId)  → Neon (500ms cold, 20ms warm)
  │ → renders project list as HTML
  ▼
Response: text/html, streamed
  │ First byte: ~200ms
  ▼
Browser paints
  │ Interactivity hydrates lazily; heavy components mount on demand
```

**Data transferred**: ~15KB HTML + small component JS. Compare Next.js:
same page ships ~300KB JS + JSON payload.

## Lifecycle 2: Uploading Drone Photos

Uploads bypass the gateway entirely — files go **browser → R2 directly**.

```
Browser (UploadZone component)
  │ 1. POST api/uploads/authorize { files: [{ name, size, type }, ...] }
  ▼
SvelteKit gateway
  │ auth check (event.locals.user)
  │ billing quota check (Neon lookup, ~20ms)
  │ For each file: R2 presigned PUT URL (or multipart part URLs > 32MB)
  ▼
Response: { projectId, uploads: [{ key, url, mode }] }
  │
Browser uploads files in parallel (4 concurrent) straight to R2
  │ progress tracked client-side
  ▼
Browser: POST api/uploads/complete { projectId, completions }
  │ → dataset row created → workflow triggered
  │ → redirect to /projects/[id] (progress view)
```

## Lifecycle 3: Processing (Compute)

```
api/uploads/complete
  │ emits dataset.uploadComplete
  ▼
pipeline orchestrator ($lib/server/pipeline.ts)
  │ Phase 1 preflight (EXIF, GSD, overlap) — gateway, ~10s
  │ Phase 2 submitAndMonitor — Nadir on RunPod, 15min–4h
  │   poll GET /jobs/{id} every 5s
  │   each change → bus.emit → SSE subscribers
  │ Phase 3 post-processing (Potree, PMTiles, Draco, thumbnails) — parallel
  │ Phase 4 finalize (record artifacts, deduct credits, email)
  ▼
Every state change is persisted to workflow_state (resumable)
```

## Lifecycle 4: Viewing Results

```
GET /projects/[id]/results
  │ load(): project + outputs from Neon (metadata only, ~2KB)
  │ SSR renders page shell + output cards
  ▼
Browser lazy-loads viewers on tab click:
  │ Map tab    → import('maplibre-gl') + import('pmtiles')
  │ 3D tab     → import('three') (mesh) or Potree (point cloud)
  │ Tiles/octree fetched from R2 with HTTP Range requests ($0 egress)
```

## Lifecycle 5: SSE Progress

```
Nadir worker (RunPod)      emits stage progress every 5s
  ▼
pipeline orchestrator      polls /jobs/{id}, publishes to bus
  ▼
bus (EventEmitter / Redis pub-sub in V1)
  ▼
+server.ts GET api/projects/[id]/progress
  │ ReadableStream → text/event-stream, chunked
  ▼
Browser (EventSource)      progress component updates (~0.05ms per tick)
```

## Bandwidth Budget

| Flow | Size | Path |
|---|---|---|
| Dashboard HTML | ~15KB | CDN miss → gateway → browser |
| Raw photos (200 imgs) | ~2.8GB | browser → R2 (never touches gateway) |
| Progress events | ~100B / 5s | worker → gateway → browser (SSE) |
| Artifact downloads | 0.1–2GB | R2 → browser (presigned, $0 egress) |
| PMTiles/Potree tiles | KB-sized ranges | R2 → browser (Range requests) |

## See Also

- [Upload flow](upload-flow.md) — the upload component in detail
- [Progress channel](progress-channel.md) — SSE payloads and reconnection
- [Pipeline workflow](pipeline-workflow.md) — the 16-stage DAG
- [Integration bridge](integration-bridge.md) — the Nadir service client
