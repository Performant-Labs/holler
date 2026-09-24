#!/usr/bin/env bash
# app:body:run -- build, then run the body in the foreground. The body must
# already be joined (`holler body join ...`); if not, holler says so and names
# that command.
set -euo pipefail
. "$(dirname "${BASH_SOURCE[0]}")/local-lib.sh"

config="${HOLLER_CONFIG:-}"
if [ -z "$config" ]; then
  if [ -f sessions.toml ]; then
    config="sessions.toml"
  elif [ -f session.toml ]; then
    config="session.toml"
  else
    echo "error: no body config: set HOLLER_CONFIG, or create sessions.toml (or session.toml) in $(pwd)" >&2
    exit 1
  fi
fi

ensure_built
exec "$(holler_path)" body run --config "$config"
