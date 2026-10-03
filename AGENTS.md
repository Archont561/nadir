# AGENTS.md — machine instructions for this bundle

This is an **offline pixi sandbox**, not source code to merge.

- authoritative manifest: `.pixi-sandbox/manifest.json` (schema 2);
- environments: default (platform linux-64);
- verified bootstrap: `.pixi-sandbox/tools/linux-64/pixi-sandbox`; the branch root contains documentation only;
- never download tools at restore time; bundled tools are: pixi, pixi-sandbox, pixi-unpack;
- after restore, `.pixi/tools/linux-64/pixi` install --frozen --offline must be a no-op.
