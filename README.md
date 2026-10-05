# Offline sandbox (orphan branch)

Built 2026-10-05T15:47:37Z from commit `ee5e7d5` for platform `linux-64`.
`pixi.lock` sha256 `44bc67c4f57dd44c3f26ea6fbd2ed8762e0df1b8e7216a1511d40959e30c3a28`.

The verified self-bootstrap binary is stored at `.pixi-sandbox/tools/linux-64/pixi-sandbox`. The branch root intentionally contains documentation only.

| env | platform | packed | unpacked | files |
| --- | --- | ---: | ---: | ---: |
| `default` | linux-64 | 1077.5 MiB | 4622.1 MiB | 383 |

Cargo dependencies: **94 crates**, 79.3 MiB (loose) from `Cargo.lock` sha256 `81ad1feee753…`; restore materialises them to `.pixi-sandbox/vendor/`. Built with cargo 1.98.1 (797e8a9bc 2026-08-05); rustc 1.98.1 (48a229cea 2026-09-01).

## Restore on the disconnected machine

```bash
./.pixi-sandbox/tools/linux-64/pixi-sandbox doctor --branch-location . --verify
./.pixi-sandbox/tools/linux-64/pixi-sandbox restore --branch-location . --output-path <project> --force
# then, from <project> with no network, use pixi as the sole entrypoint:
pixi install --frozen --offline
pixi run --frozen -- cargo build --offline
```

Every manifest blob is verified before it is written into the working tree.
