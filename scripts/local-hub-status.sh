#!/usr/bin/env bash
# app:hub:status -- the roster and the hub's own status.
set -euo pipefail
. "$(dirname "${BASH_SOURCE[0]}")/local-lib.sh"

require_binary
bin="$(holler_path)"
"$bin" roster
"$bin" hub status
