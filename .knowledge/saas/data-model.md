---
type: Data Model
title: "SaaS Data Model — Six Tables, PostGIS, Credit Ledger"
description: "The Drizzle/Neon schema (users, projects, datasets, outputs, credit_transactions, workflow_state), index strategy, size estimates, the PostGIS spatial query catalog, and credit ledger operations."
purpose: Database schema and billing-ledger source of truth for the SaaS
last_updated: 2026-09-30
status: stable
related:
  - architecture.md
  - billing.md
  - auth.md
  - pipeline-workflow.md
---

# SaaS Data Model — Six Tables, PostGIS, Credit Ledger

## Metadata Only in Neon, Binaries in R2

NadrScan stores **metadata only** in Neon Postgres. All binary data (images,
point clouds, orthomosaics, meshes) lives in Cloudflare R2. Neon stores
references (R2 keys), processing state, user accounts, and billing records.

```
Neon Postgres (metadata, ~10MB per 100 users):
  ├── users               (Clerk accounts + credits)
  ├── projects            (processing jobs + PostGIS bounds)
  ├── datasets            (uploaded image sets)
  ├── outputs             (generated artifacts)
  ├── credit_transactions (billing ledger)
  └── workflow_state      (pipeline persistence)

Cloudflare R2 (binary data, ~25GB per 100 users):
  └── {userId}/{projectId}/raw/* and outputs/*
```

## The Six Tables (Drizzle)

Schema lives at `apps/nadirscan/src/lib/server/db/schema.ts` (Drizzle +
`drizzle-orm/neon-http`). Shape:

| Table | PK | Key columns | Notes |
|---|---|---|---|
| `users` | `text id` (Clerk `user_xxx`) | email (unique), name, role, **credits**, stripe_customer_id, plan | `credits` is a cached sum of the ledger |
| `projects` | `text id` (`proj_xxx`) | user_id FK, name, status, image_count, total_size_bytes, target_gsd, crs, pipeline, nadir_job_id, processing_time_ms, **bounds_geom** | status: uploading → queued → processing → postprocessing → complete / failed / cancelled |
| `datasets` | `text id` (`ds_xxx`) | project_id FK, storage_path (R2 prefix), image_count, total_size_bytes, gps_bounds (jsonb), camera_model, gsd_estimate, altitude_avg, overlap_estimate | one per upload |
| `outputs` | `text id` (`out_xxx`) | project_id FK, kind, storage_path (R2 key), content_hash (blake3), size_bytes, format, crs, metadata (jsonb) | kind: sparse_cloud, dense_cloud, dense_cloud_potree, mesh, mesh_glb, orthomosaic, orthomosaic_pmtiles, dsm, dtm, report, thumbnails |
| `credit_transactions` | `serial id` | user_id FK, amount (+/−), type, reference_id, description | **source of truth for billing** |
| `workflow_state` | `text id` | name ('photogrammetry.process'), status, bag (jsonb), current_step, error | makes the pipeline resumable |

PostGIS geometry column:

```typescript
const geometry = customType<{ data: string; driverData: string }>({
	dataType() { return 'geometry(POLYGON, 4326)'; }
});
// projects.boundsGeom — flight-area polygon, WGS 84
```

Extensions (once per database / migration `0000_init.sql`):

```sql
CREATE EXTENSION IF NOT EXISTS postgis;
CREATE EXTENSION IF NOT EXISTS postgis_topology;
```

Drizzle notes: `numeric(p,s)` maps to string in Drizzle — use
`doublePrecision` for GSD/altitude/overlap so the app treats them as
numbers. `timestamp` columns are `Date` in Drizzle and ISO strings at the
API boundary.

## Index Strategy

| Index | Type | Purpose |
|---|---|---|
| `users_email_idx` | UNIQUE | Login lookup |
| `users_stripe_idx` | BTREE | Webhook → user lookup |
| `projects_user_idx` | BTREE | Dashboard project list |
| `projects_status_idx` | BTREE | Admin: find stuck jobs |
| `projects_bounds_gist_idx` | **GIST** | **PostGIS spatial queries** |
| `datasets_project_idx` | BTREE | Project → dataset lookup |
| `outputs_project_idx` | BTREE | Project → outputs lookup |
| `outputs_kind_idx` | BTREE | Filter by artifact type |
| `credits_user_idx` | BTREE | Credit history |
| `credits_ref_idx` | UNIQUE (reference_id, type) | Idempotency — no double-charge / double-grant |
| `wf_status_idx` | BTREE | Find running workflows |

## Data Size Estimates

| Table | Rows/user | 100 users | 10,000 users |
|---|---|---|---|
| users | 1 | 50 KB | 5 MB |
| projects | 10 | 1 MB | 100 MB |
| datasets | 10 | 800 KB | 80 MB |
| outputs | 80 | 8 MB | 800 MB |
| credit_transactions | 30 | 600 KB | 60 MB |
| workflow_state | 10 | 2 MB | 200 MB |
| **Total** | | **~13 MB** | **~1.2 GB** |

Neon free tier (0.5GB) supports ~3,000 users → upgrade to Launch ($19/mo,
5GB) at that point.

## Spatial Queries — PostGIS Catalog

Why PostGIS: users want "show projects on a map", "find projects near here",
"which projects overlap this AOI", "how much area have I scanned". SQLite/D1
cannot do this.

```sql
-- Bounds are set after the Nadir job completes:
UPDATE projects SET bounds_geom = ST_MakeEnvelope(west, south, east, north, 4326)
WHERE id = 'proj_abc';
```

The catalog (all GIST-indexed, <5ms at 10k projects):

1. **Viewport search** (map dashboard) — `ST_Intersects(bounds_geom, ST_MakeEnvelope(viewport..., 4326))`
2. **Proximity search** — `ST_DWithin(bounds_geom::geography, point::geography, radius_m)`; cast to `::geography` for meters (geometry distance is degrees — useless)
3. **Overlap detection** — `ST_Area(ST_Intersection(bounds_geom, ST_GeomFromGeoJSON(aoi)))` — "has this field been surveyed before?"
4. **Coverage aggregate** — `ST_Union(bounds_geom)` + `ST_Area(::geography)` — total km² scanned
5. **Extent query** — `ST_Extent(bounds_geom)` — zoom-to-all on the dashboard map
6. **Centroids** — `ST_Centroid(bounds_geom)` — map pins
7. **Temporal comparison** — same AOI over time: overlap query + `ORDER BY completed_at` for change detection

## Credit Ledger — Billing Source of Truth

**Golden rule: `credit_transactions` is the source of truth.** Stripe
Meters, Stripe Billing, and `users.credits` are derived views. If they
disagree, Neon wins.

```
Source of truth:  credit_transactions (Neon)
                        │
          ┌─────────────┼─────────────┐
          ▼             ▼             ▼
    users.credits   Stripe Meter   Dashboard
    (cached sum)    (invoicing)    (display)
```

### Transaction types

| Type | Sign | Trigger | Reference |
|---|---|---|---|
| `bonus` | + | User signup | `signup-bonus` |
| `purchase` | + | Stripe checkout (top-up) | Stripe `pi_xxx` |
| `subscription_grant` | + | Monthly plan renewal | Stripe `sub_xxx` |
| `processing` | − | Workflow completes | `proj_xxx` |
| `refund` | + | Workflow fails or cancelled | `proj_xxx` |
| `adjustment` | ± | Admin manual correction | `admin_xxx` |

### Ledger operations

Every operation is **idempotent** on `(reference_id, type)` — the unique
index rejects duplicates, so Stripe webhook retries and workflow retries
can never double-charge or double-grant:

```
grant(userId, amount, type, referenceId)     → insert if not exists, bump users.credits
deduct(userId, projectId, imageCount, pipeline)
    → cost = ceil(imageCount × rate[pipeline])
    → refuse if balance < cost (checked at submission)
refund(userId, projectId)                    → +|previous deduction|, reference proj_xxx
balance(userId)                              → users.credits (cache of Σ transactions)
history(userId)                              → listTransactions desc
```

The reconciliation job (daily, V1) recomputes `Σ transactions` per user and
repairs `users.credits` drift; Stripe Meter values are synced from the same
ledger.

## See Also

- [Billing](billing.md) — plans, Stripe flows, and how the ledger is fed
- [Auth](auth.md) — the Clerk → Neon provisioning chain that creates `users` rows
- [Pipeline workflow](pipeline-workflow.md) — what writes `workflow_state` and `outputs`
- [Infrastructure](infrastructure.md) — Neon connection details
