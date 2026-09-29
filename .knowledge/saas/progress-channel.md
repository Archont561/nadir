---
type: User Interface
title: "Progress Channel — Real-Time Processing Updates"
description: "SSE architecture from the Nadir worker to the browser, the +server.ts endpoint, channel payloads, the stage-timeline component, and reconnection with snapshot catch-up."
purpose: Live progress streaming design for the processing view
last_updated: 2026-09-30
status: stable
related:
  - pipeline-workflow.md
  - upload-flow.md
  - integration-bridge.md
---

# Progress Channel — Real-Time Processing Updates

## Architecture

```
Nadir worker (RunPod)
  │ exposes stage progress (polled every 5s)
  ▼
pipeline orchestrator ($lib/server/pipeline.ts)
  │ bus.emit(projectId, { event, data })
  ▼
bus (EventEmitter in-process; Redis pub/sub from V1)
  │ fan-out to all subscribers
  ▼
SvelteKit +server.ts (GET /api/projects/[id]/progress)
  │ ReadableStream → text/event-stream, chunked
  ▼
Browser (EventSource)
  │ progress component updates stores
  ▼
CSS-first stage indicators (~0.05ms per tick)
```

## The SSE Endpoint

```typescript
// apps/nadirscan/src/routes/api/projects/[id]/progress/+server.ts
import { bus } from '$lib/server/events';
import { store } from '$lib/server/db';

export const GET = async ({ params, locals }) => {
	const project = await store.getProject(params.id);
	if (!project || project.userId !== locals.user?.id) {
		return new Response('Forbidden', { status: 403 });
	}

	const stream = new ReadableStream({
		start(controller) {
			const enc = new TextEncoder();
			const send = (msg: ChannelEvent) =>
				controller.enqueue(enc.encode(`event: ${msg.event}\ndata: ${JSON.stringify(msg.data)}\n\n`));

			// Catch-up snapshot first (fills any gap before subscribing)
			send({ event: 'update', data: snapshotFor(project) });

			const unsubscribe = bus.subscribe(params.id, send);
			const keepalive = setInterval(() => controller.enqueue(enc.encode(': ka\n\n')), 25_000);

			return () => { unsubscribe(); clearInterval(keepalive); controller.close(); };
		}
	});

	return new Response(stream, {
		headers: {
			'content-type': 'text/event-stream',
			'cache-control': 'no-cache',
			connection: 'keep-alive'
		}
	});
};
```

## Channel Payloads

| Event | Data | Source |
|---|---|---|
| `update` | `{ phase, stage, stageLabel, percent, eta, detail }` | stage changes |
| `percent` | `{ percent }` | throttled ~10% increments |
| `error` | `{ code, message, retryable, suggestion? }` | workflow failure |
| `done` | `{ status: 'complete' \| 'failed' \| 'cancelled' }` | terminal — client closes |

## The Timeline Component

```svelte
<!-- ProcessingTimeline.svelte -->
<script lang="ts">
	import { TIMELINE_STAGES } from '$lib/stages';
	let { projectId, initial } = $props();

	let snapshot = $state(initial);        // SSR-provided, no flash of empty
	const es = new EventSource(`/api/projects/${projectId}/progress`);
	es.addEventListener('update', (e) => (snapshot = JSON.parse(e.data)));
	es.addEventListener('error', (e) => (snapshot.error = JSON.parse(e.data)));
</script>

<div class="timeline" data-stage={snapshot.stage}>
	<div class="bar"><div class="fill" style:width="{snapshot.percent}%"></div></div>
	<div class="meta">
		<span>{snapshot.stageLabel}</span>
		<span>{snapshot.percent}%</span>
		<span>ETA: {snapshot.eta}</span>
	</div>
	{#each TIMELINE_STAGES as stage (stage.key)}
		<div class="row" data-stage={stage.key}>
			<span>{stage.icon}</span>
			<span>{stage.label}</span>
			<span class="status"></span>
		</div>
	{/each}
</div>
```

## The 13 Timeline Stages

```
🔍 Feature Extraction      🔗 Feature Matching      📐 Sparse Reconstruction
🎯 Bundle Adjustment       🌍 Georeferencing        ☁️ Dense Reconstruction
🧹 Point Cloud Filtering   🏔️ DSM / DTM Generation  🔺 Mesh Reconstruction
🎨 Texture Mapping         🗺️ Orthorectification    ⚙️ Post-Processing
✅ Complete
```

(The pipeline has 16 internal stages; `dtm` shares the DSM row and
`finalize` maps onto `complete`.)

## CSS-First Stage Indicators

One attribute swap, CSS handles the rest — no per-row re-render:

```css
/* completed + current + pending are derived from data-stage on the parent */
.timeline[data-stage="dense"] .row[data-stage="features"] .status::after { content: "✓"; color: green; }
.timeline[data-stage="dense"] .row[data-stage="dense"]   .status::after { content: "⟳"; animation: spin 1s linear infinite; }
.row .status::after { content: "—"; color: gray; }
```

## Reconnection

`EventSource` auto-reconnects, but events during the gap are lost. Two
safety nets:

1. **Snapshot on connect** — the endpoint sends a full snapshot before
   subscribing (shown in the code above)
2. **Server close on terminal state** — the `done` event ends the stream;
   the component closes the EventSource and shows the final state

## See Also

- [Pipeline workflow](pipeline-workflow.md) — what emits these events
- [Upload flow](upload-flow.md) — the step before this page
- [Integration bridge](integration-bridge.md) — where the progress data comes from
