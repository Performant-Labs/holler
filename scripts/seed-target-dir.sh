#!/usr/bin/env bash
# Give this worktree its own target/, seeded from an existing warm one, so a
# build here reuses compiled third-party dependencies instead of compiling
# them from zero.
#
# Never point CARGO_TARGET_DIR at another worktree's target/ instead: cargo's
# fingerprints for workspace crates use workspace-relative paths plus source
# mtimes, so a checkout whose sources are older than the artifacts there is
# judged "fresh" and silently keeps a binary built from the other tree.
#
# Usage (from the workspace root):  scripts/seed-target-dir.sh <warm-target-dir>
set -euo pipefail

warm="${1:?usage: scripts/seed-target-dir.sh <warm-target-dir>}"
[ -f Cargo.toml ] || { echo "run from the workspace root" >&2; exit 2; }
[ -d "$warm" ] || { echo "no such directory: $warm" >&2; exit 2; }
[ -e target ] && { echo "target/ already exists here; remove it first" >&2; exit 2; }

# Copy-on-write clone: instant and free on APFS (macOS) and btrfs/xfs (Linux);
# elsewhere GNU cp falls back to a full copy, which is slower but still correct.
cp -Rc "$warm" target 2>/dev/null || cp -R --reflink=auto "$warm" target

# The cloned workspace-crate artifacts were built from the other tree's source.
# Remove them in both profiles so they rebuild from this tree; deps stay reused.
members=$(cargo metadata --no-deps --format-version 1 |
  python3 -c 'import json, sys; print(" ".join(p["name"] for p in json.load(sys.stdin)["packages"]))')
pkgs=()
for m in $members; do pkgs+=(-p "$m"); done
cargo clean "${pkgs[@]}" >/dev/null
cargo clean --release "${pkgs[@]}" >/dev/null

echo "seeded target/ from $warm; rebuilding: $members"
