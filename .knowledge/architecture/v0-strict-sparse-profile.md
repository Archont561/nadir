---
type: Architecture
title: V0 Strict Sparse Reconstruction Profile
description: "The accepted V0 boundary: reproducible local sparse SfM with an immutable cache"
purpose: Defines the V0 product contract, safety boundary, cache semantics, qualification evidence, and explicit deferrals.
last_updated: 2026-10-05
status: accepted
related:
  - overview.md
  - artifact-model.md
  - engine-registry.md
  - pipeline-dag.md
  - worker-protocol.md
  - ../photogrammetry/colmap-integration.md
  - ../../backlog/docs/decisions/012-v0-strict-sparse-reconstruction-profile.md
  - ../../backlog/docs/roadmaps/v0-mvp.md
---

# V0 Strict Sparse Reconstruction Profile

## Decision in one sentence

V0 accepts exactly one bounded, calibrated ImageSet and produces one
byte-verified, local-coordinate `SparseScene v1` through a qualified CPU-only
COLMAP pipeline; it does not claim georeferencing, dense reconstruction, GPU
execution, worker service, or mapping products.

## V0 product boundary

```text
one immutable ImageSet snapshot
  → CPU-only feature extraction
  → CPU-only exhaustive matching
  → sparse reconstruction
  → canonical local SparseScene v1
```

The only qualified V0 engine path is COLMAP, resolved from a locked Pixi
developer profile or an official digest-pinned OCI runtime. Every invocation is
a separate immutable stage artifact. A completed cache entry contains exactly
the bytes named by its verified manifest and is never mutated. Identical
verified runs launch no COLMAP subprocesses on cache hits. User output is a
writable copy or copy-on-write reflink, never a cache path.

## Input and camera contract

Nadir accepts one input root, recursively discovers supported regular JPEG
files and explicitly calibrated PNG files, sorts normalized logical paths,
validates them against measured V0 bounds, and snapshots original bytes without
symlinks or ordinary hard links. Every run has one resolved intrinsics profile.
Mixed rigs, incompatible dimensions, missing calibration, focus/zoom changes,
and untracked orientation transforms fail.

## Coordinate and output contract

`SparseScene v1` contains `manifest.json`, canonically ordered `cameras.json`,
and binary little-endian `points.ply`. Its coordinate declaration is
`local_sfm`, with no CRS and arbitrary scale. COLMAP's world-to-camera
`x_camera = R × x_world + t` is converted to camera centre `-Rᵀ × t` and
camera-to-world orientation `Rᵀ`; quaternion signs are canonicalized.

## Artifact and cache semantics

An invocation key identifies a request; artifact/tree digests identify output
bytes; a cache entry maps the former to a verified manifest of the latter.
Keys include task and contract versions, named input digests, normalized
resolved configuration, qualified toolchain identity, and every byte-affecting
setting. Stages write unique staging directories and are structurally validated,
digested, manifested, and atomically promoted. Hits revalidate referenced bytes.
A per-invocation advisory lock coordinates the OS-user-local store.

Engine-private COLMAP databases and workspaces remain immutable and compatible
only with their adapter/toolchain profile. `SparseScene v1` is the canonical
exchange boundary. Structural validity is separate from a versioned quality
policy verdict.

## Strict acceptance policy

V0 accepts exactly one reconstruction model containing every admitted image,
with finite calibration, poses and geometry, a nonempty sparse cloud, and all
files passing `SparseScene v1` contract validation. Registration,
connectivity, feature, match, point, observation, track, and reprojection facts
are recorded.

## Runtime profiles and trust

The developer profile uses locked Pixi; an end-user profile uses an official OCI
image by digest; ambient system COLMAP is explicitly unqualified and isolated.
Qualified identity includes runtime/package digest, executable digest/version,
architecture, threads, locale, timezone, arguments, and GPU-disabled state.

Untrusted inputs require rootless OCI with no network, non-root execution,
read-only root/input mounts, dropped capabilities, and CPU, memory, process,
time, log, and scratch limits. The container writes only staging candidates;
the host rejects unsafe files, recomputes digests, validates, and imports them.
Trusted and untrusted namespaces do not cross-reuse in V0.

## Resource, lifecycle, and evidence

Qualification targets Linux x86-64, CPU-only execution, 32 GiB RAM, and 100 GiB
free cache space. Measured fixture evidence determines image, byte, pixel, pair,
disk and timeout limits. Lifecycle states are planned, waiting_for_lock,
staging, executing, validating, atomically_promoting, and completed, with
rejected, cancelled, failed, or abandoned terminal states.

Tests combine public-boundary integration tests, real pinned COLMAP runs, and
`proptest` coverage of canonicalization, paths, hashing, fingerprints, camera
normalization, lifecycle and cache invariants. Fixtures include owned synthetic
fault cases, a small CC BY CI set, and a larger qualification set. Qualification
requires two fresh stores with equal stage-tree digests and a same-store run
with zero COLMAP launches.

## Explicit V0 deferrals

Georeferencing, GPS/RTK/GCP and CRS claims; OpenMVS, PDAL, GDAL, DSM/DTM,
meshes, orthomosaics, GeoTIFF/COG outputs and all mapping products; GPU
execution and configurable/declarative DAG recipes; multi-camera rigs, RAW,
TIFF and video; HTTP serving, remote/shared workers and worker services; and
end-user Python/TypeScript workflow APIs are deferred until separately
qualified.
