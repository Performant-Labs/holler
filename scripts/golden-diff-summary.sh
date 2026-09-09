#!/usr/bin/env bash
# Golden-file diff summary (issue #246): `BLESS=1 cargo test -p holler-proto
# --test golden_test` re-serializes every golden JSON file with
# `serde_json::to_string_pretty`, which (via the crate's `preserve_order`
# feature having been dropped) reorders every file's keys alphabetically as a
# side effect — even files whose *value* didn't change. That cosmetic churn
# can bury a real, unrelated value-drift regression inside a huge "reorder"
# diff (the drift-laundering failure mode in
# docs/reviews/overlays/tests-as-evidence.md).
#
# This canonicalizes each changed golden file's old and new git blob (sorted
# keys, compact — jq -Sc is equivalent to golden_test.rs's own
# tests/common::canonical for this purpose) and compares them: identical
# canonical forms means a pure key-reorder (expected, harmless fallout of
# BLESS=1); different canonical forms means a real value change that deserves
# reviewer attention, called out by name instead of getting lost in the
# reorder noise.
#
# Informational only — never fails the build (exit 0 always). Called from
# scripts/lint.sh.
set -uo pipefail

golden_glob='crates/holler-proto/tests/golden/**/*.json'

if ! command -v jq >/dev/null 2>&1; then
  echo "lint: skipping golden-diff summary (#246) — jq not installed"
  exit 0
fi

# Pick a comparison base without assuming full history is present (CI
# checkouts are shallow by default). Two-dot diff (direct tree compare, not
# merge-base) so a shallow/unrelated-history clone still works.
base=""
if [ -n "${GITHUB_BASE_REF:-}" ]; then
  if git fetch --depth=1 origin "${GITHUB_BASE_REF}" >/dev/null 2>&1; then
    base="origin/${GITHUB_BASE_REF}"
  fi
elif git rev-parse --verify -q HEAD^ >/dev/null 2>&1; then
  base="HEAD^"
fi

if [ -z "$base" ]; then
  echo "lint: skipping golden-diff summary (#246) — no comparison base available (shallow clone with no parent, and not a PR)"
  exit 0
fi

changed=$(git diff --name-only "$base" HEAD -- 'crates/holler-proto/tests/golden/*.json' 'crates/holler-proto/tests/golden/**/*.json' 2>/dev/null | sort -u)

if [ -z "$changed" ]; then
  exit 0
fi

real_change=()
reorder_only=()
unreadable=()

while IFS= read -r gf; do
  [ -z "$gf" ] && continue
  old_raw=$(git show "${base}:${gf}" 2>/dev/null) || old_raw=""
  new_raw=$(cat "$gf" 2>/dev/null) || new_raw=""

  if [ -z "$old_raw" ]; then
    # New golden file (no prior blob) — nothing to compare against; not a
    # reorder, and not flagged as a "value change" either. Note it plainly.
    real_change+=("$gf (new file)")
    continue
  fi

  old_canon=$(printf '%s' "$old_raw" | jq -Sc . 2>/dev/null)
  new_canon=$(printf '%s' "$new_raw" | jq -Sc . 2>/dev/null)

  if [ -z "$old_canon" ] || [ -z "$new_canon" ]; then
    unreadable+=("$gf")
  elif [ "$old_canon" = "$new_canon" ]; then
    reorder_only+=("$gf")
  else
    real_change+=("$gf")
  fi
done <<<"$changed"

echo "lint: golden files with real value changes: ${#real_change[@]} — reorder-only: ${#reorder_only[@]}"
if [ "${#real_change[@]}" -gt 0 ]; then
  echo "lint:   real value change (review these):"
  for f in "${real_change[@]}"; do
    echo "lint:     $f"
  done
fi
if [ "${#reorder_only[@]}" -gt 0 ]; then
  echo "lint:   reorder-only (cosmetic BLESS=1 fallout, no reviewer action needed):"
  for f in "${reorder_only[@]}"; do
    echo "lint:     $f"
  done
fi
if [ "${#unreadable[@]}" -gt 0 ]; then
  echo "lint:   could not canonicalize (not valid JSON on one side?):"
  for f in "${unreadable[@]}"; do
    echo "lint:     $f"
  done
fi

exit 0
