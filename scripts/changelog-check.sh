#!/usr/bin/env bash
# CI guard (issue #319): CHANGELOG.md must always carry an `## [Unreleased]`
# heading. A release that forgets to leave one behind (step 7 of
# docs/releasing.md) breaks the next release's ability to accumulate entries
# without hand-editing the file back into shape.
set -euo pipefail

file="CHANGELOG.md"

if [ ! -f "$file" ]; then
  echo "changelog-check: $file is missing"
  exit 1
fi

if ! grep -qE '^## \[Unreleased\]' "$file"; then
  echo "changelog-check: $file has no '## [Unreleased]' heading"
  exit 1
fi

echo "changelog-check: ok"
