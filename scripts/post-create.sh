#!/usr/bin/env bash
# Report the state of a devcontainer without changing anything.
#
# Called by `.devcontainer/devcontainer.json`'s postCreateCommand, after `pixi run setup`.
# Its job is to make the first five minutes in a new container cheap: print the tool
# versions that differ from a developer's own machine, the external engines that are present
# or absent, and the one command that proves the checkout works.
#
# Read-only on purpose. `setup` can fail on a network hiccup and leave a container that is
# built but looks broken; this script is what you run to find out whether it is actually
# broken or merely quiet, and it must be safe to re-run to answer that question.
#
# It prints, it does not assert. A missing engine is information, not a build failure —
# OpenMVS in particular is expected to be absent because there is no conda-forge package
# for it.

set -uo pipefail

# Colour only when stdout is a terminal, so the output stays readable in a CI log.
if [[ -t 1 ]]; then
  bold=$'\033[1m'; dim=$'\033[2m'; reset=$'\033[0m'
else
  bold=""; dim=""; reset=""
fi

section() { printf '\n%s%s%s\n' "$bold" "$1" "$reset"; }

# Print `label: value`, or a dim note when the tool is absent.
tool() {
  local label="$1"
  shift
  local version
  if version=$("$@" 2>/dev/null | head -n 1) && [[ -n "$version" ]]; then
    printf '  %-14s %s\n' "$label" "$version"
  else
    printf '  %-14s %snot found%s\n' "$label" "$dim" "$reset"
  fi
}

# Print a version the repository *pins* rather than one on PATH.
#
# `bunx biome --version` from the root is the trap this avoids: it resolves the unscoped
# `biome` package from the registry, which is a different project on a different version
# number, and prints it next to a workspace that uses `@biomejs/biome`. A report that names
# the wrong tool is worse than one that names none, so these read the pin out of the
# manifest — which is the number that actually decides what a contributor gets.
pinned() {
  local label="$1" file="$2" package="$3"
  local version
  version=$(grep -oE "\"$package\"[[:space:]]*:[[:space:]]*\"[^\"]+\"" "$file" 2>/dev/null |
    head -n 1 | grep -oE '[0-9][^"]*' | tr -d '"')
  if [[ -n "$version" ]]; then
    printf '  %-14s %s (pinned)\n' "$label" "$version"
  else
    printf '  %-14s %snot pinned%s\n' "$label" "$dim" "$reset"
  fi
}

section "Toolchain"
tool "pixi"      pixi --version
tool "bun"       bun --version
tool "python"    python --version
tool "rustc"     rustc --version
tool "cargo"     cargo --version
tool "ruff"      python -m ruff --version
pinned "turbo"   package.json "turbo"
pinned "biome"   packages/sdk/package.json "@biomejs/biome"

section "External engines"
# `nadir engines` already walks PATH and reports versions, and its output is the same thing
# this section would print. One implementation of the question, not two.
if [[ -x target/debug/nadir ]]; then
  target/debug/nadir engines
elif cargo run --quiet -- engines 2>/dev/null; then
  :
else
  printf '  %snot built yet; run `pixi run gates`%s\n' "$dim" "$reset"
fi

section "Workspace state"
printf '  %-14s %s\n' "git branch" "$(git rev-parse --abbrev-ref HEAD 2>/dev/null || echo unknown)"
printf '  %-14s %s\n' "git status" "$(git status --porcelain 2>/dev/null | wc -l | tr -d ' ') changed path(s)"

if [[ -f bun.lock ]]; then
  printf '  %-14s %spresent%s\n' "bun.lock" "" ""
else
  printf '  %-14s %sabsent — run `bun install`%s\n' "bun.lock" "$dim" "$reset"
fi
if [[ -d python/nadir/src/nadir ]]; then
  if python -c 'import nadir' 2>/dev/null; then
    printf '  %-14s %simportable%s\n' "python sdk" "" ""
  else
    printf '  %-14s %snot importable — run `pixi run py-install`%s\n' "python sdk" "$dim" "$reset"
  fi
fi

section "Next"
printf '  pixi run gates        # the full check: build, lint, typecheck, test, advisories\n'
printf '  cargo test -p nadir-cli   # just the CLI tests\n'
printf '  pixi run fmt          # rewrite with each formatter (cargo fmt, ruff, biome)\n'
printf '  %sThe devcontainer is configured; nothing else is required.%s\n' "$dim" "$reset"
