---
title: "ADR-002: Three-Language Split"
status: accepted
date: 2025-02-23
deciders: project lead
related:
  - ../architecture/overview.md
  - ../architecture/worker-protocol.md
  - 003-drop-python-adapter-for-mvp.md
  - 007-engine-trait-api-surface.md
---

# ADR-002: Three-Language Split

## TL;DR

Nadir uses three languages with distinct responsibilities: **Rust** owns the
worker runtime, pipeline engine, and infrastructure; **TypeScript** owns the
developer-facing SDK, CLI, and web UI; **Python** owns future engine plugins
and scientific post-processing. The wire protocol between them is **protobuf**,
which serves as the single source of truth for all shared types.

## Context

The original architecture document proposed TypeScript as the sole
developer-facing layer, with Rust as infrastructure and Python as the ODM
adapter. This was challenged on two fronts:

1. **Python users are primary, not secondary.** Geospatial data scientists
   live in Python/Jupyter. Forcing them through TypeScript would be a mistake.

2. **Rust users deserve a first-class API.** The worker is Rust. Rust
   applications that embed Nadir should not need to go through TypeScript.

The question became: how do we offer SDKs in all three languages without
tripling the maintenance burden or creating three divergent type systems?

## Decision

**Three languages, one schema.** Each language gets an ergonomic SDK built on
top of shared protobuf types. The protobuf schema is the single source of
truth. Language-specific wrappers are thin (~200–500 lines each).

| Language | Responsibility | When |
|---|---|---|
| **Rust** | Worker runtime, pipeline engine, DAG executor, artifact store, process management, engine adapters, scheduler, storage | V0.1+ (primary) |
| **TypeScript** | Public SDK, workflow DSL, CLI, web UI, SaaS backend, API server | V2.0+ (SDK), V0.1 demo (BFF/UI) |
| **Python** | Engine plugins (ML, advanced geospatial), scientific post-processing | V2.0+ (optional) |

### The compilation flow is identical regardless of source language

```
Python user                    Rust user                   TS user
    │                              │                           │
    ▼                              ▼                           ▼
nadir.mapping(...)          Mapping::builder()          nadir.mapping(...)
    │                              │                           │
    ▼                              ▼                           ▼
MappingConfig protobuf      MappingConfig protobuf      MappingConfig protobuf
    │                              │                           │
    └──────────────┬───────────────┘───────────────────────────┘
                   │
                   ▼
            Worker Protocol (protobuf/Connect)
                   │
                   ▼
            Rust Worker (nadir-worker)
                   │
                   ▼
            Engine Adapters (COLMAP, OpenMVS, GDAL)
```

The worker does not know or care which language submitted the task. It
receives a `MappingConfig` protobuf message and executes it.

## Rationale

### Why protobuf as the canonical schema

- **One source of truth.** Add a field to the `.proto` file, regenerate, and
  all three languages get the new field automatically.
- **Wire compatibility.** Protobuf handles versioning, forward/backward
  compatibility, and efficient serialization.
- **Language-neutral.** `protoc` generates TypeScript (`@connectrpc`), Rust
  (`prost`), and Python (`protobuf`) bindings from the same schema.
- **No manual type reproduction.** We do not maintain JSON types in three
  languages and hope they stay in sync.

### Why Zod sits above protobuf in TypeScript

Protobuf gives wire compatibility. Zod gives the pleasant public API:

```typescript
const Mapping = z.object({
  input: z.string(),
  quality: z.enum(["fast", "standard", "survey"]),
  outputs: z.object({
    orthomosaic: z.boolean(),
    dsm: z.boolean(),
    dtm: z.boolean(),
  }),
});
```

The user sees ergonomic TypeScript. The wire representation is boring protobuf.

### Why each SDK is thin (~200–500 lines)

The heavy lifting is done by:
1. Generated protobuf types (data structures)
2. The Rust worker (execution)
3. The engine adapters (photogrammetry)

Each SDK only needs to provide:
- Builder/constructor ergonomics
- Validation (Zod/Pydantic/serde)
- Transport (local/remote worker client)
- Event streaming

This is ~200–500 lines per language, not thousands.

## Consequences

### Positive
- Python geospatial users get `from nadir import mapping`
- Rust users get `Mapping::builder().quality(Quality::Survey).build()`
- TypeScript users get `nadir.mapping({ quality: "survey" })`
- All three produce the same protobuf message
- Adding a new config field requires one `.proto` change + regeneration
- Community can contribute SDKs for other languages (Go, Java) using the same schema

### Negative
- Three SDKs to maintain (though each is thin)
- Protobuf adds a build step (code generation)
- Protobuf types are less ergonomic than native language types (hence the wrapper layer)

### Deferred
- Python SDK is not needed for MVP (V0.1–V1.0). Rust handles everything.
- TypeScript SDK is needed for the V0.1 demo BFF/UI but can start as a thin
  HTTP client before the full protobuf integration.
- Full protobuf integration is a V2.0 concern. V0.1 uses direct subprocess calls.

## See Also

- [ADR-003: Drop Python adapter for MVP](./003-drop-python-adapter-for-mvp.md) — Python is deferred to V2.0+
- [ADR-007: Engine trait API surface](./007-engine-trait-api-surface.md) — the 16 traits that all SDKs target
- [Worker protocol](../architecture/worker-protocol.md) — the protobuf schema design
