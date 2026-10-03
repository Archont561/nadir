---
type: Architecture Decision Record
title: "ADR-011: Shared Transport and Thin Native Bindings"
description: "Keep Rust authoritative behind one versioned JSON transport exposed through PyO3 and N-API."
status: accepted
date: 2026-10-03
deciders: project lead
related:
  - 002-three-language-split.md
  - 003-drop-python-adapter-for-mvp.md
  - ../../../.knowledge/architecture/language-bindings.md
---

# ADR-011: Shared Transport and Thin Native Bindings

## Decision

Rust remains the sole implementation of Nadir's domain behavior. Python and TypeScript
reach it in-process through thin PyO3 and N-API crates. Both native adapters export exactly
one function taking JSON text and returning JSON text.

`nadir-protocol` owns a versioned request/response envelope and operation enum.
`nadir-engine` validates that envelope and dispatches operations to domain crates. Language
packages provide typed conveniences, response mapping, and idiomatic exceptions only.

## Why

Mirroring every Rust function as a separate FFI symbol couples all SDK releases to every
core API change and creates multiple places for behavior to drift. A stable transport makes
adding an operation a Rust protocol and dispatcher change rather than an ABI redesign in
every language.

The JSON envelope is not a remote-service commitment. It is an in-process compatibility
boundary that is easy to exercise identically from Rust, Python, and TypeScript.

## Consequences

- `python-native → engine → protocol + core` and `node-native → engine → protocol + core`.
- Native adapters contain no photogrammetry rules.
- Transport-breaking changes increment `TRANSPORT_VERSION`.
- Each language keeps a small hand-written DTO mirror, verified against the real addon.
- A future C, WebAssembly, JVM, or .NET face can reuse the dispatcher without duplicating
  the engine.
- ADR-003 still forbids a Python process between Rust and COLMAP/OpenMVS, but no longer
  forbids a Python consumer SDK.
