---
type: User Interface
title: "Viewers — Map (MapLibre + PMTiles), Point Cloud (Potree), Mesh (Three.js)"
description: "The three result viewers as browser-only Svelte components: PMTiles protocol registration, basemap/opacity/hillshade controls, Potree octree loading with point budget, and the GLTFLoader mesh viewer."
purpose: Result-viewing components and their loading strategy
last_updated: 2026-09-30
status: stable
related:
  - data-flow.md
  - upload-flow.md
  - infrastructure.md
---

# Viewers — Map, Point Cloud, Mesh

## One Page, Three Tabs, Zero Upfront JS

The results page (`/projects/[id]/results`) SSRs metadata (output cards,
sizes, hashes, CRS, GSD) and lazy-loads each viewer **only when its tab is
first clicked**:

| Viewer | Library | Size | Data source |
|---|---|---|---|
| Orthomosaic map | MapLibre GL + PMTiles | ~200KB + ~10KB | `ortho.pmtiles` from R2 (Range requests) |
| Point cloud | Potree | ~800KB | Potree octree from R2 (Range requests) |
| Textured mesh | Three.js + GLTFLoader | ~600KB | `mesh.glb` from R2 |

All three mount inside `onMount` (browser-only — no SSR of WebGL), with
dynamic `import()` so they never enter the initial bundle.

## Map Viewer — MapLibre + PMTiles

```
┌─────────────────────────────────────────────────┐
│  [Satellite ▼] [OSM] [None]    Opacity: [===]  │
│  ┌─────────────────────────────────────────────┐│
│  │      orthomosaic (PMTiles) over basemap     ││
│  │      [DSM hillshade toggle]                 ││
│  └─────────────────────────────────────────────┘│
│  GSD: 2.5cm  |  CRS: EPSG:32634  |  12.3 ha     │
└─────────────────────────────────────────────────┘
```

```svelte
<script lang="ts">
	let { pmtilesUrl, bounds, gsd, crs, areaHa, dsmUrl } = $props();
	let container: HTMLDivElement;
	let opacity = $state(0.85);
	let basemap = $state<'satellite' | 'osm' | 'none'>('satellite');

	$effect(() => { /* map.setPaintProperty('ortho', 'raster-opacity', opacity) */ });
	$effect(() => { /* swap basemap source tiles */ });

	onMount(async () => {
		const [{ default: maplibregl }, { Protocol }] = await Promise.all([
			import('maplibre-gl'), import('pmtiles')
		]);
		const protocol = new Protocol();
		maplibregl.addProtocol('pmtiles', protocol.tile);

		const map = new maplibregl.Map({ container, /* style with
			raster source: `pmtiles://${pmtilesUrl}`, bounds, fitVBounds */ });
		return () => map.remove(); // cleanup on unmount
	});
</script>
```

PMTiles advantages: a **single-file** archive (one R2 object, one presigned
or public URL), HTTP Range requests fetch only visible tiles, zoom levels
z10–z18 baked at post-processing, $0 egress from R2. Basemap: Esri World
Imagery tiles (attribution required) or OSM.

## Point Cloud Viewer — Potree

- Data: `dense.laz` → PotreeConverter octree (BIN format, many small files
  under `outputs/potree/`) served from R2
- **Point budget** (default 2M points) caps GPU memory; LOD loads detail as
  you zoom
- **Eye-dome lighting** on for depth perception
- Camera presets: Orbit / Top / Front; measurement tools built in
  (distance, area, height)

```ts
onMount(async () => {
	const Potree = await import('potree-core'); // or the potree build
	// viewer.loadPointCloud(`${potreeUrl}/cloud.js`, 'dense', onLoaded)
});
```

## Mesh Viewer — Three.js

```ts
onMount(async () => {
	const THREE = await import('three');
	const { GLTFLoader } = await import('three/examples/jsm/loaders/GLTFLoader.js');
	const { OrbitControls } = await import('three/examples/jsm/controls/OrbitControls.js');
	// scene + ambient/directional lights + camera fitted to the model bbox
	// GLTFLoader().load(glbUrl, (gltf) => scene.add(gltf.scene))
	// wireframe toggle traverses meshes; resize handler; RAF loop
	// cleanup: cancelAnimationFrame, dispose renderer
});
```

Toolbar: wireframe toggle, camera presets (Orbit / Top / Front), file size.
`mesh.glb` is the Draco-compressed post-processed artifact (~4× smaller than
OBJ).

## Performance Notes

- Viewers mount on **tab click**, not page load — results page stays ~20KB
- Svelte `onMount` return function = cleanup (map remove, RAF cancel,
  renderer dispose) — no leaks when switching tabs
- PMTiles/Potree octrees are immutable + content-hashed →
  `Cache-Control: immutable` from R2, cached at L4 (browser)
- `data-` attributes + CSS for opacity/zoom badges (Tier 1 updates)

## See Also

- [Pipeline workflow](pipeline-workflow.md) — where PMTiles / Potree / GLB come from
- [Infrastructure](infrastructure.md) — R2 public bucket + CORS for tiles
- [Data flow](data-flow.md) — the viewing lifecycle
