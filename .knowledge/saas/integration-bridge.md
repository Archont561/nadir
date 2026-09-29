---
type: Integration
title: "Integration Bridge — SvelteKit ↔ Nadir"
description: "The $lib/server/services/nadir.ts client: transport options, job submission, progress polling, artifact handoff, cancellation, and error mapping."
purpose: How the SvelteKit gateway talks to Nadir workers
last_updated: 2026-09-30
status: stable
related:
  - context.md
  - pipeline-workflow.md
  - ../architecture/worker-protocol.md
  - infrastructure.md
---

# Integration Bridge — SvelteKit ↔ Nadir

## The Problem

The SaaS gateway is TypeScript (SvelteKit). Nadir is Rust. They run in
different processes (gateway on Fly.io, workers on RunPod GPU instances).
They need to exchange:

- **Job submissions** (gateway → Nadir): "process this dataset with this pipeline"
- **Progress updates** (Nadir → gateway): "stage 7 of 16, 43% complete, ETA 12m"
- **Artifact references** (Nadir → gateway): "here are the R2 paths + blake3 hashes"
- **Cancellations** (gateway → Nadir): "user aborted, kill this job"

## The Bridge — One Server Module

The entire integration surface is one server-only module:

```typescript
// apps/nadirscan/src/lib/server/services/nadir.ts
// Server-only by SvelteKit guarantee ($lib/server cannot reach the client.)

import { env } from '$env/dynamic/private';

const BASE = env.NADIR_WORKER_URL; // e.g. RunPod endpoint proxy

async function call<T>(path: string, init?: RequestInit, timeoutMs = 30_000): Promise<T> {
	const ctrl = new AbortController();
	const t = setTimeout(() => ctrl.abort(), timeoutMs);
	try {
		const res = await fetch(`${BASE}${path}`, {
			...init,
			signal: ctrl.signal,
			headers: {
				'content-type': 'application/json',
				...(env.NADIR_WORKER_TOKEN
					? { authorization: `Bearer ${env.NADIR_WORKER_TOKEN}` }
					: {}),
				...init?.headers
			}
		});
		if (!res.ok) throw new NadirError(await res.json());
		return (await res.json()) as T;
	} finally {
		clearTimeout(t);
	}
}

export const nadir = {
	// POST /jobs → 202 { id, status: 'queued', estimatedDuration }
	submitPipeline: (input: SubmitPipelineInput) =>
		call<SubmitResponse>('/jobs', { method: 'POST', body: JSON.stringify(input) }),

	// GET /jobs/{id} → { status, stage, percent, eta, artifacts?, error? }
	getProgress: (jobId: string) => call<NadirJobStatus>(`/jobs/${jobId}`),

	// POST /jobs/{id}/cancel → { status: 'cancelled' }
	cancelJob: (jobId: string) => call(`/jobs/${jobId}/cancel`, { method: 'POST' }),

	// GET /_health → { status, version, engines, gpu, disk }
	health: () => call<HealthResponse>('/_health')
};
```

Everything downstream (workflow, UI, billing) imports `nadir` — no direct
HTTP, no retry logic scattered across the codebase. **When
`NADIR_WORKER_URL` is unset, the module swaps in a demo implementation**
that simulates the 16 stages in real time, so the whole product runs
without a GPU (see [deployment-topology.md](deployment-topology.md)).

## Transport Options

Nadir's roadmap defines worker-protocol tiers:

| Nadir version | Transport | Gateway side |
|---|---|---|
| **V0 (MVP)** | CLI subprocess (`nadir process ...`) | **Skipped** — too fragile for SaaS, Docker-in-Docker on the gateway |
| **V0.5 (current)** | HTTP + JSON (`nadir serve --http`) | `fetch` client above |
| **V1+ (planned)** | gRPC + protobuf (`nadir serve --grpc`) | `@grpc/grpc-js` client — same `nadir` interface |
| **V2+ (planned)** | bidirectional streaming | SSE endpoint bridges the stream directly |

## Job Submission

```typescript
// Inside the pipeline orchestrator (see pipeline-workflow.md)
const job = await nadir.submitPipeline({
	datasetPath: `r2://${bucket}/${userId}/${projectId}/raw/`,
	outputPath: `r2://${bucket}/${userId}/${projectId}/outputs/`,
	pipeline,                      // 'fast' | 'standard' | 'survey'
	projectId,
	imageCount,
	config: { gsd: targetGsd, crs }
});

await store.updateProject(projectId, { status: 'processing', nadirJobId: job.id });
```

## Progress Streaming

The orchestrator polls every 5s and republishes to the SSE bus:

```typescript
while (true) {
	await sleep(5_000);
	const status = await nadir.getProgress(jobId);

	bus.emit(projectId, { event: 'update', data: {
		phase: 'processing', stage: status.stage ?? '',
		stageLabel: labelFor(status.stage), percent: status.percent ?? 0,
		eta: status.eta ?? '', detail: status.detail ?? ''
	}});

	if (status.status === 'completed') return status; // → artifacts
	if (status.status === 'failed') throw new NadirError(status.error);
	if (status.status === 'cancelled') throw new CancelledError();
}
```

## Artifact Handoff

Nadir writes outputs directly to R2 (credentials passed per-job for V0.5;
worker-level env vars preferred from V1). On completion it returns:

```json
{
	"kind": "orthomosaic",
	"path": "r2://nadrscan-data/user_abc/proj_xyz/outputs/ortho.tif",
	"hash": "blake3:a1b2c3...",
	"size": 452000000,
	"metadata": { "gsd": 0.025, "crs": "EPSG:32633", "bounds": [19.85, 50.01, 20.05, 50.12] }
}
```

The gateway records this in the `outputs` table. Since the hash is
content-addressed, re-processing the same dataset with the same config
short-circuits: Nadir sees the hash exists in R2 and skips the work.

## Cancellation

```
User clicks "Cancel"
  → POST api/projects/[id]/cancel (form action)
    → nadir.cancelJob({ jobId })
      → Nadir sends SIGTERM to the engine subprocess (COLMAP, OpenMVS, ...)
      → 30s grace, then SIGKILL
    → project status = 'cancelled'
    → workflow undo chain → refund credits
```

## Error Handling

Every error from Nadir maps to a gateway error code:

| Nadir error | Gateway error | HTTP | User-facing action |
|---|---|---|---|
| `INSUFFICIENT_OVERLAP` | `VALIDATION_FAILED` | 422 | "Your photos don't overlap enough. Reshoot with 70%+ overlap." |
| `COLMAP_FEATURE_EXTRACTION_FAILED` | `WORKFLOW_STEP_FAILED` | 500 | "Feature extraction failed. Photos may be too blurry." |
| `OPENMVS_OOM` | `WORKFLOW_STEP_FAILED` | 500 | "Dataset too large for current tier. Upgrade or reduce image count." |
| `GDAL_INVALID_CRS` | `VALIDATION_FAILED` | 422 | "Coordinate system not recognized." |
| `DISK_FULL` | `INTERNAL_ERROR` | 500 | "Processing failed. Automatically retrying." |
| Worker crash / OOM | `SERVICE_ERROR` | 502 | "Processing failed. Automatically retrying." |

The service module owns timeout + retry; the orchestrator owns
classification; the QC preflight catches most user errors **before**
submission.

## See Also

- [Nadir worker protocol](../architecture/worker-protocol.md) — the Rust side of this API
- [Pipeline workflow](pipeline-workflow.md) — who calls this and when
- [Infrastructure](infrastructure.md) — RunPod endpoint configuration
