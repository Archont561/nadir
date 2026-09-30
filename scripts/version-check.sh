#!/usr/bin/env bash
# Assert the project version is identical everywhere it is written.
#
# The authority is `[workspace.package] version` in the root Cargo.toml. Every other file
# that records a version — pixi.toml, the root package.json, the Bun packages, the Python
# pyproject — is a copy, and every copy is a place to forget to bump. Convco bumps them all
# in one pass (`.versionrc`); this script catches the hand-edit that skipped one.
#
# Runs as a string comparison rather than a TOML/JSON parse, deliberately: the check must
# not depend on a tool that might itself be out of date, and a file this script cannot parse
# is a file it should fail on rather than skip.
#
# It checks only files that exist. A package that is not in the tree yet is not a version
# mismatch, it is an absent file, and failing on that would make this task unusable during
# the scaffolding the repository is in.

set -uo pipefail

failures=0

# Print the first `version = "..."` or `"version": "..."` in a file.
#
# The last quoted string on the matching line is the value, not the first: a JSON line is
# `"version": "0.1.0"`, whose *first* quoted token is the key. Taking the tail is what
# makes one pattern handle both TOML and JSON.
read_version() {
  local file="$1"
  [[ -f "$file" ]] || return 0
  grep -oE '^[[:space:]]*(version[[:space:]]*=[[:space:]]*|"version"[[:space:]]*:[[:space:]]*)"[^"]+"' "$file" \
    | head -n 1 \
    | grep -oE '"[^"]+"' \
    | tail -n 1 \
    | tr -d '"'
}

# Compare one file against the authority.
expect() {
  local file="$1"
  if [[ ! -f "$file" ]]; then
    printf '  %-34s %s(absent, skipped)%s\n' "$file" "$dim" "$reset"
    return
  fi
  local found
  found="$(read_version "$file")"
  if [[ "$found" == "$AUTHORITY" ]]; then
    printf '  %-34s %s%s%s\n' "$file" "$ok" "$found" "$reset"
  else
    printf '  %-34s %s%s (expected %s)%s\n' "$file" "$bad" "${found:-<none>}" "$AUTHORITY" "$reset"
    failures=$((failures + 1))
  fi
}

if [[ -t 1 ]]; then
  ok=$'\033[32m'; bad=$'\033[31m'; dim=$'\033[2m'; reset=$'\033[0m'
else
  ok=""; bad=""; dim=""; reset=""
fi

# The authority, and the one file that is not a copy of it.
AUTHORITY="$(read_version Cargo.toml)"
if [[ -z "$AUTHORITY" ]]; then
  echo "version-check: could not read [workspace.package] version from Cargo.toml" >&2
  exit 1
fi

echo "version: $AUTHORITY"
echo
echo "  file                                  version"

# The Python pyproject and the root manifests. Workspace-member package.json files are
# checked too: they are the versions Bun and `npm`-style tooling report, and a mismatch
# between an SDK's declared version and the release it shipped in is a support ticket.
expect Cargo.toml
expect pixi.toml
expect package.json
expect python/nadir/pyproject.toml
expect crates/package.json
expect packages/sdk/package.json
expect packages/client/package.json
expect python/nadir/package.json

# The `version = "..."` on each intra-workspace path dependency in the root Cargo.toml.
# `cargo deny` cannot resolve a workspace dependency that has a path and no version, so
# these copies are forced to exist; this is what keeps them copies rather than becoming
# the place someone edits the version. Checked separately because the pattern is a third
# shape: `name = { path = "...", version = "..." }` rather than a bare `version = "..."`.
#
# Only lines that *define* a dependency are checked — those with a `path =`. The
# `nadir-core = { workspace = true }` lines in the root package's `[dependencies]` are
# consumers that deliberately carry no version, and a naive match on the crate name counts
# them as mismatches. They are verified the other way round instead, below.
echo
echo "  intra-workspace path dependencies (root Cargo.toml)"
while IFS= read -r line; do
  name=$(printf '%s' "$line" | grep -oE '^nadir-[a-z]+')
  found=$(printf '%s' "$line" | grep -oE 'version[[:space:]]*=[[:space:]]*"[^"]+"' | grep -oE '[0-9][^"]*' | tr -d '"')
  if [[ "$found" == "$AUTHORITY" ]]; then
    printf '  %-34s %s%s%s\n' "$name" "$ok" "$found" "$reset"
  else
    printf '  %-34s %s%s (expected %s)%s\n' "$name" "$bad" "${found:-<none>}" "$AUTHORITY" "$reset"
    failures=$((failures + 1))
  fi
done < <(grep -E '^nadir-[a-z]+[[:space:]]*=.*path[[:space:]]*=' Cargo.toml)

# The consumers must inherit rather than restate. A `version =` on a
# `{ workspace = true }` dependency is a second copy of the number that nothing checks and
# that convco does not bump — the exact failure this whole task exists to catch, so it is
# checked explicitly rather than left to a grep that would read it as a match.
consumer_issues=0
while IFS= read -r line; do
  if [[ "$line" == *"version"* ]]; then
    name=$(printf '%s' "$line" | grep -oE '^nadir-[a-z]+')
    printf '  %-34s %srestates a version instead of inheriting%s\n' "$name" "$bad" "$reset"
    failures=$((failures + 1))
  fi
  consumer_issues=$((consumer_issues + 1))
done < <(grep -E '^nadir-[a-z]+[[:space:]]*=.*workspace[[:space:]]*=[[:space:]]*true' Cargo.toml)

echo
if [[ "$failures" -gt 0 ]]; then
  echo "version-check: $failures file(s) disagree with Cargo.toml."
  echo "  Either bump them all with \`pixi run changelog\` (convco writes .versionrc's bumpFiles),"
  echo "  or fix the one that is wrong. Do not hand-edit a single copy: that is the failure."
  exit 1
fi
echo "version-check: all versions agree."
