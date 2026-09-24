#!/usr/bin/env bash
# app:body:stop -- detach the body from the hub.
set -euo pipefail
. "$(dirname "${BASH_SOURCE[0]}")/local-lib.sh"

require_binary
exec "$(holler_path)" body detach
