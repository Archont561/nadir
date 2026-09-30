---
okf_version: "0.2"
---

# Nadir Knowledge Bundle

Portable project knowledge for Nadir, organized for progressive disclosure.

## Start Here

* [Nadir Project Context](context.md) - Single-file project summary for LLM quick-loading
* [Nadir Knowledge Base Index](legacy-navigation.md) - Master map for LLM context loading — load this after /context.md

## Architecture

* [Artifact Model](architecture/artifact-model.md) - Artifact types, content-based blake3 hashing, caching, invalidation, provenance
* [Engine Registry & Trait API](architecture/engine-registry.md) - The 16 stable traits, config structs, engine resolution, and adapter pattern
* [Five Computational Domains](architecture/five-domains.md) - Detailed breakdown of Dataset, Reconstruction, Geometry, Surface, Cartography
* [Architecture Overview](architecture/overview.md) - Big-picture system design — 5 domains, 3 dependency tiers, build system metaphor
* [Pipeline & DAG Executor](architecture/pipeline-dag.md) - Declarative TOML pipelines, petgraph DAG, topological executor, concurrent branches
* [Worker Protocol](architecture/worker-protocol.md) - Worker API, protobuf schema, transport independence, distributed execution

## Architecture Decisions

* [ADR-001: Step-Scoped Tools vs ODM Monolith](decisions/001-step-scoped-vs-odm-monolith.md) - Records the accepted architecture decision on Step-Scoped Tools vs ODM Monolith.
* [ADR-002: Three-Language Split](decisions/002-three-language-split.md) - Records the accepted architecture decision on Three-Language Split.
* [ADR-003: Drop Python Adapter for MVP](decisions/003-drop-python-adapter-for-mvp.md) - Records the accepted architecture decision on Drop Python Adapter for MVP.
* [ADR-004: Inverted Container Model (No Docker-in-Docker)](decisions/004-inverted-container-no-dind.md) - Records the accepted architecture decision on Inverted Container Model (No Docker-in-Docker).
* [ADR-005: Per-Step Worker Topology](decisions/005-worker-topology-per-step.md) - Records the accepted architecture decision on Per-Step Worker Topology.
* [ADR-006: Pixi for Dependency Management](decisions/006-pixi-for-dependency-management.md) - Records the accepted architecture decision on Pixi for Dependency Management.
* [ADR-007: Engine Trait API Surface](decisions/007-engine-trait-api-surface.md) - Records the accepted architecture decision on Engine Trait API Surface.
* [ADR-008: WASM / Edge / Browser Strategy](decisions/008-wasm-edge-browser-strategy.md) - Records the accepted architecture decision on WASM / Edge / Browser Strategy.
* [ADR-009: One Manifest per Root](decisions/009-one-manifest-per-root.md) - Why `pixi.toml` is both the workspace root and the `nadir-cli` package manifest, and why there is no `crates/cli/pixi.toml`.
* [ADR-010: Bun, Not Node](decisions/010-bun-not-node.md) - Bun is the only JavaScript runtime, package manager and test runner; no `nodejs` and no `pnpm` in any feature.

## Deployment

* [Demo Plan](deployment/demo-plan.md) - 4-week demo — Astro + WebcoreUI + Clerk + Postgres + Drizzle + Inngest + Docker worker
* [Docker Strategy](deployment/docker-strategy.md) - Inverted container model, multi-stage Dockerfile, lean production variant
* [Hosting Comparison](deployment/hosting-comparison.md) - Railway vs Render vs Vercel vs Fly.io vs VPS for photogrammetry workloads
* [Pixi Setup](deployment/pixi-setup.md) - Full pixi.toml, environments, developer onboarding, Docker integration, CI

## Photogrammetry

* [COLMAP Integration](photogrammetry/colmap-integration.md) - CLI commands, binary format, adapter code, progress parsing, resume behavior
* [Engine Assignment](photogrammetry/engine-assignment.md) - Which tool handles which stage, why, and the V0→V3 replacement path
* [Geospatial Stack](photogrammetry/geospatial-stack.md) - GDAL, PROJ, PDAL, nalgebra, geo — what each does, Rust bindings, usage patterns
* [OpenMVS Integration](photogrammetry/openmvs-integration.md) - Tool chain, MVS scene format, COLMAP→OpenMVS conversion, adapter code
* [Photogrammetry Pipeline Stages](photogrammetry/pipeline-stages.md) - Full walkthrough from drone images to mapping products with math and diagrams

## References

* [Dependency Matrix](references/dependency-matrix.md) - All Rust crates and external tools with versions, tiers, and WASM status
* [External Sources](references/sources.md) - All documentation links for engines, libraries, and references

## Roadmap

* [Full Roadmap](roadmap/full-roadmap.md) - V0.1 → V3.0 complete checklist, dependency matrix, three rules
* [V0 MVP Details](roadmap/v0-mvp.md) - V0.1 and V0.2 milestones, deliverables, CLI output examples
* [V1 Platform Details](roadmap/v1-platform.md) - V1.0 adaptive planning, QC, recipes, resource scheduling
* [V2 Scale Details](roadmap/v2-scale.md) - V2.0 native engines, splitting, worker protocol, multi-language SDK
* [WASM / Edge / Browser Vision](roadmap/wasm-vision.md) - V2+++ pure Rust math, WASM compilation, browser/edge/serverless topologies
