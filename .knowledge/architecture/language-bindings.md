# Language binding architecture

> **V0 clarification (ADR-012):** this is the durable binding design, but V0
> exposes reconstruction through the Rust CLI first. Python and TypeScript
> reconstruction APIs wait for a qualified workflow contract.

Nadir uses the same monorepo boundary convention as `Archont561/geoquery`: the Rust engine
is authoritative and language packages are FFI faces, not parallel implementations.

```text
Python package ─→ crates/python-native ─┐
                                       ├─→ crates/engine ─→ crates/core
TypeScript SDK → crates/node-native ───┘          │
                                                  └─→ crates/protocol
```

`nadir-protocol` defines a versioned JSON envelope. Each native adapter exports exactly one
`invoke` function taking JSON text and returning JSON text. `nadir-engine` validates the
envelope and dispatches operations to the core. Convenience APIs in Python and TypeScript
only construct envelopes, map responses, and turn failed responses into exceptions.

This indirection is intentional:

- domain rules are implemented and tested once in Rust;
- adding an operation does not change every native ABI;
- every language receives the same errors and semantics;
- transport compatibility can be rejected explicitly with `TRANSPORT_VERSION`;
- native crates remain ordinary Cargo members, covered by the workspace lock and gates.

## Adding an operation

1. Add the operation to `crates/protocol::Operation`.
2. Dispatch it in `crates/engine`; call domain code rather than implementing behavior there.
3. Add engine contract tests.
4. Add typed convenience wrappers and boundary tests in each language package.

Never add domain behavior to `python-native`, `node-native`, or a language package.
