#!/usr/bin/env bash
# app:hub:stop -- stop the hub named in hub/serve.lock, but only if that pid is
# really a `holler hub serve` process.
set -euo pipefail
. "$(dirname "${BASH_SOURCE[0]}")/local-lib.sh"

pid="$(lock_pid)"
if [ -z "$pid" ]; then
  echo "no hub to stop: no pid in $lock_file" >&2
  exit 1
fi
if ! kill -0 "$pid" 2>/dev/null; then
  echo "no hub to stop: pid $pid from $lock_file is not running (stale lock)" >&2
  exit 1
fi
if ! is_holler_hub "$pid"; then
  echo "refusing to stop pid $pid: it is not a holler hub (pid in $lock_file; command: $(ps -o command= -p "$pid" 2>/dev/null || true))" >&2
  exit 1
fi

kill -TERM "$pid"
tries=0
while kill -0 "$pid" 2>/dev/null; do
  tries=$((tries + 1))
  if [ "$tries" -gt 100 ]; then
    echo "error: hub pid $pid did not exit within 10s after SIGTERM; not escalating" >&2
    exit 1
  fi
  sleep 0.1
done
echo "hub stopped (pid $pid)"
