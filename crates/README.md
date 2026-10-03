# Rust workspace

Nadir follows the same boundary convention as `Archont561/geoquery`: Rust owns the domain,
and each language binding is a thin face over one versioned JSON transport.

## Layers

| Layer | Crates | Responsibility |
| --- | --- | --- |
| Domain | `core`, pipeline-stage crates | Photogrammetry types and rules |
| Transport | `protocol` | Versioned request/response DTOs and operations |
| Dispatch | `engine` | Translate operations into core calls |
| FFI | `python-native`, `node-native` | One JSON-in/JSON-out function per ABI |
| Applications | root `nadir-cli` | User-facing commands |

Dependency arrows point inward: `*-native → engine → protocol + core`. An FFI crate must not
contain domain behavior or expose a parallel list of native functions. Add an operation to
`protocol`, implement it once in `engine`, then add language-level convenience wrappers.
