---
type: Workflow
title: "Photogrammetry Pipeline — 16-Stage Workflow"
description: "The gateway-side orchestrator for the 16-stage pipeline: DAG structure, ProcessingBag, step contracts, post-processing, pipeline variants, and error compensation."
purpose: Definition of the core SaaS workflow that drives Nadir
last_updated: 2026-09-30
status: stable
related:
  - integration-bridge.md
  - progress-channel.md
  - data-model.md
  - ../architecture/pipeline-dag.md
---

# Photogrammetry Pipeline — 16-Stage Workflow

## Overview

NadrScan's core product is a gateway-side workflow that orchestrates Nadir's
16-stage photogrammetry pipeline. It runs in the SvelteKit server (MVP:
in-process with `workflow_state` persistence; V1+: BullMQ workers), delegates
GPU compute to Nadir on RunPod, and streams progress to browsers via SSE.

```
event: dataset.uploadComplete
  ↓
workflow: photogrammetry.process   ($lib/server/pipeline.ts)
  │
  ├─ Phase 1: Preflight (gateway, ~10s)
  │    ├─ Step 1: inspectDataset    (EXIF, GSD, GPS)
  │    └─ Step 2: validateOverlap   (adaptive pipeline selection)
  │
  ├─ Phase 2: Processing (Nadir on RunPod, 15min–4hr)
  │    └─ Step 3: submitAndMonitor  (submits to Nadir, polls progress)
  │         Nadir internally runs stages 3–14:
  │           3. Feature extraction (COLMAP, GPU)
  │           4. Feature matching (COLMAP, GPU)
  │           5. Sparse reconstruction (COLMAP)
  │           6. Bundle adjustment (COLMAP/Ceres)
  │           7. Georeferencing (GDAL/PROJ)
  │           8. Dense MVS (OpenMVS, GPU)
  │           9. Point cloud filtering (PDAL)
  │          10. DSM generation (GDAL)
  │          11. DTM generation (GDAL)
  │          12. Mesh reconstruction (OpenMVS)
  │          13. Texture mapping (OpenMVS)
  │          14. Orthorectification (GDAL)
  │
  ├─ Phase 3: Post-processing (gateway worker, 5–15min)
  │    ├─ Step 4: convertPointCloud  (LAZ → Potree octree)
  │    ├─ Step 5: generatePMTiles    (COG → PMTiles)
  │    ├─ Step 6: compressMesh       (OBJ → Draco GLB)
  │    └─ Step 7: generateThumbnails (ortho → preview PNGs)
  │
  └─ Phase 4: Finalization (gateway, ~5s)
       ├─ Step 8: recordArtifacts    (Neon + R2 metadata)
       ├─ Step 9: deductCredits      (billing)
       └─ Step 10: notifyComplete    (email + cache invalidation)
```

## The ProcessingBag

```typescript
interface ProcessingBag {
	// Inputs (from trigger event)
	projectId: string;
	userId: string;
	datasetPath: string;        // R2 prefix: userId/projId/raw/
	outputPath: string;         // R2 prefix: userId/projId/outputs/
	imageCount: number;

	// Populated by Phase 1
	targetGsd?: number;
	crs?: string;
	pipeline?: 'fast' | 'standard' | 'survey';
	gpsBounds?: GeoJSON.Polygon;

	// Populated by Phase 2
	nadirJobId?: string;
	processingTimeMs?: number;
	artifactManifest?: NadirArtifact[];

	// Populated by Phase 3
	potreePath?: string;
	pmtilesPath?: string;
	glbPath?: string;

	// Status
	errorMessage?: string;
}
```

The bag is persisted to the `workflow_state` table after every step. If the
process crashes mid-pipeline, a restart resumes from the last completed step.

## Step Contracts

| Step | needs | provides | timeout | retry |
|---|---|---|---|---|
| inspectDataset | projectId, datasetPath, imageCount | targetGsd, crs, gpsBounds | 60s | 2× |
| validateOverlap | datasetPath, targetGsd, imageCount | pipeline | 30s | — |
| submitAndMonitor | datasetPath, outputPath, pipeline, targetGsd, crs, projectId | nadirJobId, processingTimeMs, artifactManifest | 4h | 2× exp. |
| convertPointCloud | artifactManifest, outputPath | potreePath | 30m | 1× |
| generatePMTiles | artifactManifest, outputPath | pmtilesPath | 20m | 1× |
| compressMesh | artifactManifest, outputPath | glbPath | 20m | 1× |
| generateThumbnails | artifactManifest, outputPath | — | 10m | 1× |
| recordArtifacts | projectId, artifactManifest, potreePath, pmtilesPath, glbPath | — | 60s | 3× |
| deductCredits | userId, projectId, imageCount, pipeline | — | 30s | 3× |
| notifyComplete | userId, projectId | — | 30s | 2× |

## DAG Visualization

```
inspectDataset ──┐
                 ├──▶ validateOverlap ──▶ submitAndMonitor ──┬──▶ convertPointCloud ──┐
                 │                                           ├──▶ generatePMTiles ────┤
                 │                                           ├──▶ compressMesh ───────┤
                 │                                           └──▶ generateThumbnails ─┤
                 │                                                                     ▼
                 │                                                              recordArtifacts
                 │                                                                     │
                 │                                                              ┌──────┴──────┐
                 │                                                              ▼             ▼
                 │                                                       deductCredits  notifyComplete
```

**Parallelism**: Steps 4–7 (post-processing) run concurrently — they depend
only on `artifactManifest` and `outputPath`, not on each other. Saves ~10
minutes on a typical job.

## Step 1–2: Preflight Details

**inspectDataset** samples 10 images from R2 and reads EXIF (GPS, altitude,
focal length, sensor width), then computes:

```
GSD = (sensorWidth × altitude) / (focalLength × imageWidth)
CRS = UTM zone from GPS centroid (EPSG:326xx north / 327xx south)
bounds = GPS bounding box → GeoJSON polygon
```

**validateOverlap** picks the pipeline adaptively and enforces preflight
gates:

```
imageCount < 100 && gsd > 0.02  → fast
imageCount > 500 || gsd < 0.01  → survey
otherwise                       → standard

Errors: < 10 images → "Minimum 10 images required"
        gsd > 1.0 m → "GSD too high — check altitude and camera settings"
```

## Phase 3: Post-Processing Commands

| Input | Command | Output | Duration |
|---|---|---|---|
| `dense.laz` | `PotreeConverter in.laz -o potree/ --output-format BIN --material RGB --diagonal-fraction 250` | Potree octree (many small files) | 1–12 min |
| `ortho.tif` | `gdal2tiles.py --zoom 10-18 --processes 4 --webp --resampling average ortho.tif tiles/` then `pmtiles convert tiles/ ortho.pmtiles` | single PMTiles archive | 2–10 min |
| `mesh.obj` | Draco encoder → GLB | `mesh.glb` (~4× smaller) | 1–5 min |
| `ortho.tif` | GDAL overview → PNG | `thumbnail.png` (~50KB) | seconds |

Post-processing is **CPU-only** — it runs on gateway workers, freeing GPU
instances for the next job. Zoom z10–z18 covers ~10m overview to ~2.5cm
detail; survey-grade (<1cm GSD) extends to z20 at 3× file size.

## Pipeline Variants

| Stage | Fast | Standard | Survey |
|---|---|---|---|
| Feature extraction | SIFT, 4,000 max | SIFT, 8,000 max | SIFT, 16,000 max |
| Matching | Exhaustive <100, vocab tree >100 | Vocab tree | Exhaustive |
| Bundle adjustment | 1 iteration | 3 iterations | 5 iterations + GCP |
| Dense MVS | Quality: low | Quality: medium | Quality: high |
| Mesh | Skip | Standard | High-poly |
| Ortho | 2× GSD | 1× GSD | 0.5× GSD |
| **Typical time (200 imgs)** | **~15 min** | **~45 min** | **~2.5 hr** |
| **Credit cost** | **0.5/img** | **1/img** | **2/img** |

## Error Handling & Compensation

Four error categories:

| Category | Examples | Strategy |
|---|---|---|
| **User errors** | Too few images, no GPS, blurry photos | Fail fast, clear message, no retry |
| **Transient infra** | RunPod cold start timeout, R2 503, Neon cold start | Retry 2–3× with backoff |
| **Persistent infra** | GPU OOM, disk full, RunPod quota | Retry once on different GPU, then fail |
| **Data errors** | COLMAP can't match, degenerate geometry | Fail, suggest reshoot, refund credits |

**Undo chains** run in reverse order of completed steps when a workflow
fails after side effects:

```
submitAndMonitor  → undo: cancelJob
deductCredits     → undo: refundCredits
```

Post-processing outputs are valid artifacts even on later failure — they
have no undo. Only actions with external side effects (billing, running
jobs) need compensation.

**Checkpointing**: Nadir writes artifacts to R2 after each stage. On retry,
Nadir skips stages whose output hash already exists in R2 — effective resume
without an explicit checkpoint protocol (V1+ adds an explicit skip API).

**Dead letter queue** (V1, with BullMQ): steps that exhaust retries land in
`dlq` with the full bag for admin inspection and manual retry.

## Admin Visibility

```
GET  /api/admin/workflows                 → list instances
GET  /api/admin/workflows/{id}            → state + bag
POST /api/admin/workflows/{id}/cancel     → cancel
POST /api/admin/workflows/{id}/retry      → retry from failed step
```

## See Also

- [Integration bridge](integration-bridge.md) — the Nadir client used by step 3
- [Progress channel](progress-channel.md) — how step updates reach the browser
- [Pipeline & DAG executor](../architecture/pipeline-dag.md) — the Nadir-internal DAG
- [Data model](data-model.md) — `workflow_state` and `outputs` tables
