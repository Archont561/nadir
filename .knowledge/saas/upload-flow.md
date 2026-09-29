---
type: User Interface
title: "Upload Flow — Drag-Drop to R2"
description: "The upload component: drag-drop UX, client-side validation, EXIF GPS preview, presigned parallel uploads straight to R2, and the completion event that triggers processing."
purpose: UI + API contract for the drone-photo upload flow
last_updated: 2026-09-30
status: stable
related:
  - data-flow.md
  - progress-channel.md
  - infrastructure.md
---

# Upload Flow — Drag-Drop to R2

## The User Experience

```
1. User drags 50 drone photos onto the upload zone
2. Client validates: file types (JPG/TIF), sizes (<50MB each), count (10–2000)
3. Client reads EXIF from first 5 files: GPS pins on a mini-map, camera info
4. User clicks "Start Upload"
5. Progress bar: "Uploading 23/50... 1.2 GB / 2.8 GB"
6. Upload completes → processing starts automatically
7. User is redirected to the progress page
```

## The Upload Component

A Svelte component at `src/lib/components/UploadZone.svelte` — SSR renders
the shell; interactivity hydrates on mount.

```svelte
<script lang="ts">
	// $state runes: file list, phase, per-file + total progress, GPS preview
	let phase = $state<'idle' | 'validating' | 'uploading' | 'complete' | 'error'>('idle');
	let files = $state<UploadFile[]>([]);
	let totalProgress = $state(0);
	let gpsPreview = $state<{ lat: number; lng: number }[]>([]);

	async function onDrop(e: DragEvent) {
		e.preventDefault();
		const dropped = Array.from(e.dataTransfer?.files ?? []);
		const valid = dropped.filter((f) =>
			/\.(jpe?g|tiff?)$/i.test(f.name) && f.size <= 50 * 1024 * 1024
		);
		if (valid.length < 10) return (phase = 'error'); // min 10 images
		files = valid.map((file) => ({ file, status: 'pending', progress: 0 }));

		// EXIF from first 5 files → mini-map pins (before upload!)
		const exifr = await import('exifr'); // ~30KB, lazy
		for (const f of valid.slice(0, 5)) {
			const gps = await exifr.gps(f);
			if (gps) gpsPreview.push({ lat: gps.latitude, lng: gps.longitude });
		}
	}

	async function startUpload() {
		// 1. Ask the gateway for presigned URLs + the project id
		const res = await fetch('/api/uploads/authorize', {
			method: 'POST',
			body: JSON.stringify({ name, pipeline, files: files.map(f => ({
				name: f.file.name, size: f.file.size, type: f.file.type
			}))})
		});
		const { projectId, uploads } = await res.json();

		// 2. Upload in parallel (4 concurrent) straight to R2
		const queue = [...uploads];
		await Promise.all(Array.from({ length: 4 }, async () => {
			while (queue.length) {
				const up = queue.shift()!;
				await fetch(up.url, { method: 'PUT', body: fileFor(up) });
				// update file status + totalProgress
			}
		}));

		// 3. Tell the gateway we're done → triggers the workflow
		await fetch('/api/uploads/complete', {
			method: 'POST',
			body: JSON.stringify({ projectId, completions: uploads })
		});
		goto(`/projects/${projectId}`);
	}
</script>
```

Files >32MB use **S3 multipart** (5MB chunks, presigned part URLs,
ETag collection, complete call) — same pattern as the knowledge-bundle
original.

## Client-Side Validation

| Check | Rule | Error |
|---|---|---|
| File type | `.jpg`, `.jpeg`, `.tif`, `.tiff` | "Only JPG and TIF supported" |
| File size | < 50MB per file | "File too large: {name}" |
| File count | 10–2000 | "Need 10–2000 images" |
| Total size | < 20GB (free), < 100GB (pro) | "Dataset too large for your plan" |
| Duplicate names | no exact duplicates | "Duplicate file: {name}" |

## EXIF Preview

The `exifr` library (~30KB, dynamically imported) reads GPS and camera info
client-side **before** upload, so the user sees a mini-map with GPS pins and
camera info (e.g. "DJI Mavic 3, 12.29mm") — confirming the dataset looks
right before committing to a 2GB upload.

## Server Contract

```
POST /api/uploads/authorize
  in : { name, pipeline, files: [{ name, size, type }] }
  out: { projectId, uploads: [{ key, url, mode: 'single'|'multipart', partUrls? }] }
  checks: auth → plan quota → R2 presign (gateway never buffers file bytes)

POST /api/uploads/complete
  in : { projectId, completions: [{ key, mode, uploadId?, parts? }] }
  out: { ok: true }
  effect: dataset row → emit dataset.uploadComplete → workflow starts
```

## Resumable Uploads (V2)

MVP uses presigned PUTs (simple, parallel). For field conditions (unreliable
mobile data), migrate to **tus.io** resumable uploads: a `tusd` sidecar
writes straight to R2, the gateway only handles the `post-finish` webhook.
Same completion event either way.

## See Also

- [Data flow](data-flow.md) — where uploads sit in the request lifecycle
- [Progress channel](progress-channel.md) — what happens after upload completes
- [Infrastructure](infrastructure.md) — R2 bucket layout and presigning
