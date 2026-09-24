#!/usr/bin/env bash
# Fail if a built binary still contains a personal home-directory path.
#
# Usage: scripts/check-binary-paths.sh <binary> [<binary>...]
#
# Flags `/Users/<name>` and `/home/<name>` unless <name> is a generic build
# account (runner, linuxbrew, build, builder). Run it on every asset before it is
# attached to a release, and again on the files downloaded back from the release.
set -euo pipefail

[ "$#" -ge 1 ] || { echo "usage: $0 <binary>..." >&2; exit 2; }

status=0
for bin in "$@"; do
  hits="$(strings -a "$bin" \
    | grep -o -E '/(Users|home)/[A-Za-z0-9._-]+' \
    | grep -v -E '^/(Users|home)/(runner|linuxbrew|build|builder|\.cargo|\.rustup)$' \
    | sort | uniq -c | sort -rn || true)"
  if [ -n "$hits" ]; then
    echo "FAIL $bin embeds personal-looking paths:" >&2
    echo "$hits" >&2
    status=1
  else
    echo "ok   $bin"
  fi
done
exit "$status"
