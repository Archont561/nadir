---
type: Domain Guide
title: COLMAP Integration
description: "ADR-012-qualified CPU-only COLMAP sparse path and post-V0 command map"
purpose: Defines the only accepted V0 COLMAP path, parse rules, cache boundaries and later COLMAP capabilities.
last_updated: 2026-10-05
status: stable
related:
  - engine-assignment.md
  - openmvs-integration.md
  - ../architecture/engine-registry.md
  - ../architecture/v0-strict-sparse-profile.md
  - ../../backlog/docs/decisions/012-v0-strict-sparse-reconstruction-profile.md
---

# COLMAP Integration

## TL;DR

ADR-012 qualifies exactly one V0 COLMAP path: CPU-only feature extraction,
CPU-only exhaustive matching and sparse reconstruction (`mapper`) from an
immutable calibrated ImageSet snapshot to a canonical local-coordinate
`SparseScene v1`. V0 does not expose raw COLMAP flags, alternate matchers,
model alignment, dense reconstruction, GPU execution or mapping products.

COLMAP workspaces and SQLite databases are engine-private stage artifacts. The
public exchange boundary is `SparseScene v1` only.

## V0 Qualified Path

```text
ImageSet snapshot
  → colmap feature_extractor   (CPU SIFT, fixed adapter-owned flags)
  → colmap exhaustive_matcher  (CPU matching, fixed adapter-owned flags)
  → colmap mapper              (sparse reconstruction)
  → SparseScene v1             (manifest.json, cameras.json, points.ply)
```

Required properties:

- COLMAP is resolved only from a qualified locked Pixi developer profile or an
  official OCI image by digest; ambient system COLMAP is unqualified.
- GPU use is explicitly disabled for extraction and matching.
- Runtime identity records package/runtime digest, executable digest/version,
  architecture, thread count, locale, timezone, arguments and GPU-disabled
  state.
- Each COLMAP invocation has its own invocation key and verified output-tree
  manifest. Invocation keys are not output digests.
- Cache hits revalidate referenced bytes and launch no COLMAP subprocesses.
- User output is a copy or copy-on-write reflink of `SparseScene v1`, never an
  engine workspace or cache path.

## V0 Command Skeleton

The adapter owns the exact flag set; callers select no raw COLMAP flags in V0.
The skeleton below documents the stage shape, not a user-facing CLI surface.

```bash
# Feature extraction: CPU only, one calibrated camera profile.
colmap feature_extractor \
  --database_path "$STAGING/database.db" \
  --image_path "$STAGING/images" \
  --ImageReader.single_camera 1 \
  --ImageReader.camera_model OPENCV \
  --SiftExtraction.use_gpu false

# Exhaustive matching: CPU only.
colmap exhaustive_matcher \
  --database_path "$STAGING/database.db" \
  --SiftMatching.use_gpu false

# Sparse reconstruction.
colmap mapper \
  --database_path "$STAGING/database.db" \
  --image_path "$STAGING/images" \
  --output_path "$STAGING/sparse"
```

Byte-affecting values, including defaults that COLMAP would otherwise infer,
become part of the invocation key. The actual implementation should pin every
accepted setting in one adapter-owned profile document/test fixture.

## V0 Acceptance Rules

Nadir accepts a COLMAP run only when all of these are true:

1. The mapper produced exactly one model directory.
2. That model contains every admitted image from the ImageSet snapshot.
3. Camera intrinsics, poses and sparse point coordinates are finite.
4. The sparse point cloud is nonempty.
5. Observations, track references and image/camera IDs are internally
   consistent.
6. All expected files pass the `SparseScene v1` structural validator.

Registration count, connectedness, feature counts, match counts, point counts,
track lengths and reprojection facts are recorded. A versioned quality-policy
verdict may reject or warn on those facts, but it is separate from structural
validity.

## SparseScene v1 Conversion

COLMAP writes a local sparse model as `cameras.bin`, `images.bin` and
`points3D.bin`. V0 converts those engine-private bytes into a canonical output
tree:

```text
sparse_scene_v1/
  manifest.json
  cameras.json
  points.ply
```

Canonicalization rules:

- The coordinate declaration is `local_sfm`; there is no CRS, scale, EPSG code,
  GPS transform or georeferencing claim.
- Cameras/images/points are ordered deterministically by normalized logical
  image path and stable numeric IDs.
- JSON is emitted in canonical field order with normalized floating-point
  validation rules.
- `points.ply` is binary little-endian with deterministic point ordering.
- Quaternion signs are canonicalized.

### Pose Semantics

COLMAP stores world-to-camera pose:

```text
x_camera = R × x_world + t
```

`SparseScene v1` stores camera-to-world orientation and camera centre:

```text
orientation_camera_to_world = Rᵀ
camera_center_world = -Rᵀ × t
```

Do not treat COLMAP's `t` as the camera position.

## Binary Parser Notes

### `cameras.bin`

Contains camera records with model ID, dimensions and parameter arrays. V0
supports only the camera models admitted by the calibrated ImageSet contract.
Unknown, truncated or parameter-count-mismatched records are typed validation
errors naming `cameras.bin`.

### `images.bin`

Contains image IDs, world-to-camera quaternions/translations, camera IDs,
image names and 2D observations. V0 validates that every admitted logical path
appears exactly once and that every registered image references the resolved
single camera profile.

### `points3D.bin`

Contains sparse point coordinates, RGB, reprojection error and track elements.
V0 validates finite coordinates/errors and that every track observation points
to an existing admitted image and 2D observation index.

## Stage-Level Caching

COLMAP's own resumability is not the V0 cache contract. Nadir stages write to
unique staging directories and only promote structurally valid manifests.
Promoted engine workspaces are immutable and compatible only with the adapter
and toolchain profile that produced them.

A same-store rerun with equal inputs and config must skip feature extraction,
matching and mapping by verified cache hit. Qualification evidence must prove
zero COLMAP launches on that rerun.

## Post-V0 COLMAP Capabilities

COLMAP supports many commands that Nadir may use after the strict sparse base is
qualified. These are not V0.1 requirements.

| Capability | COLMAP command | Roadmap |
|---|---|---|
| Spatial or sequential matching | `spatial_matcher`, `sequential_matcher` | V1 adaptive planning |
| Vocabulary-tree matching | `vocab_tree_matcher` | V1/V2 larger datasets |
| Pose-prior or hierarchical SfM | `pose_prior_mapper`, `hierarchical_mapper` | V1 adaptive planning |
| Model alignment/georeferencing | `model_aligner` | V1 georeferencing |
| Image undistortion and dense MVS | `image_undistorter`, `patch_match_stereo`, `stereo_fusion` | V1 dense alternatives |
| Mesh/texturing helpers | `poisson_mesher`, `delaunay_mesher`, `mesh_texturer` | V1 products |
| Model merge/partition support | `model_merger`, related tools | V2 scale |

## See Also

- [V0 strict sparse reconstruction profile](../architecture/v0-strict-sparse-profile.md)
- [Engine assignment](./engine-assignment.md) — why COLMAP owns sparse SfM
- [OpenMVS integration](./openmvs-integration.md) — post-V0 dense reconstruction
- [Engine registry](../architecture/engine-registry.md) — trait implementation
- [ADR-012](../../backlog/docs/decisions/012-v0-strict-sparse-reconstruction-profile.md)
