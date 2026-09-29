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
* [ADR-009: SvelteKit Gateway for NadrScan (apps/nadirscan)](decisions/009-sveltekit-gateway-for-nadrscan.md) - Records the accepted architecture decision to build the NadrScan SaaS gateway as apps/nadirscan with SvelteKit, superseding the Lintel BFF (NadirScan bundle) and the Astro demo stack.

## SaaS — NadrScan Application

The photogrammetry SaaS built on top of the Nadir engine — SvelteKit gateway at `apps/nadirscan` (ADR-009). Ported from the external NadirScan knowledge bundle (Archont561/NadirScan, OKF v0.2) and adapted from a Lintel BFF gateway to SvelteKit.

* [NadrScan SaaS — Project Context](saas/context.md) - One-page summary: SvelteKit + Nadir, the five core decisions, the three rules, cost envelope, monorepo placement
* [SaaS Architecture — SvelteKit + Nadir](saas/architecture.md) - Product definition, Lintel → SvelteKit primitive mapping, rendering strategy, cache layers, user journey → route map, phased build plan
* [SaaS Data Flow — Reads, Writes, Streams, Compute](saas/data-flow.md) - The four data movements mapped to SvelteKit, with lifecycles for dashboard, upload, processing, viewing, and SSE
* [Integration Bridge — SvelteKit ↔ Nadir](saas/integration-bridge.md) - The `services/nadir.ts` client, transport tiers, job submission, progress, artifact handoff, cancellation, error mapping
* [Photogrammetry Pipeline — 16-Stage Workflow](saas/pipeline-workflow.md) - Gateway-side orchestrator, ProcessingBag, step contracts, post-processing commands, pipeline variants, error compensation
* [SaaS Data Model — Six Tables, PostGIS, Credit Ledger](saas/data-model.md) - Drizzle schema, index strategy, size estimates, the PostGIS spatial query catalog, and ledger operations
* [Auth — Clerk in SvelteKit](saas/auth.md) - hooks.server.ts JWT verification, the provisioning webhook, the Clerk → Neon → Stripe → Resend chain, client components
* [Upload Flow — Drag-Drop to R2](saas/upload-flow.md) - The UploadZone component, client-side validation, EXIF GPS preview, presigned parallel uploads straight to R2
* [Progress Channel — Real-Time Processing Updates](saas/progress-channel.md) - SSE via +server.ts, channel payloads, the 13-stage timeline component, reconnection with snapshot catch-up
* [Viewers — MapLibre + PMTiles, Potree, Three.js](saas/viewers.md) - The three lazy-loaded result viewers (orthomosaic map, point cloud, textured mesh) as Svelte components
* [Infrastructure — R2, Neon, RunPod, Resend, Grafana, TiTiler, tus](saas/infrastructure.md) - Service-by-service catalog, env var table, dependency graph, demo-mode fallbacks
* [Billing — Stripe, Credits, and Cost Envelope](saas/billing.md) - Hybrid subscription + credits model, webhook idempotency, refunds, reconciliation, MVP cost envelope, scaling tiers
* [Deployment Topology — Edge, Gateway, Workers](saas/deployment-topology.md) - Three-zone model, one-image-two-entrypoints SvelteKit build, the Nadir GPU image, MVP deployment, scaling path

## Deployment

* [Demo Plan](deployment/demo-plan.md) - 4-week demo — Astro + WebcoreUI + Clerk + Postgres + Drizzle + Inngest + Docker worker (**UI stack superseded by ADR-009**)
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
