#!/usr/bin/env bash
# app:hub:run -- build, then run the hub in the foreground (Ctrl-C to stop).
set -euo pipefail
. "$(dirname "${BASH_SOURCE[0]}")/local-lib.sh"

ensure_built
exec "$(holler_path)" hub serve --listen "$hub_listen" --advertise "$hub_advertise"
