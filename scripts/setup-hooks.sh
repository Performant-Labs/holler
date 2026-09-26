#!/usr/bin/env bash
# Point this clone at the tracked hooks. core.hooksPath lives in local git
# config (never tracked), so every fresh clone must run this once.
set -euo pipefail
cd "$(git rev-parse --show-toplevel)"
git config core.hooksPath .githooks
echo "core.hooksPath=$(git config core.hooksPath)"
