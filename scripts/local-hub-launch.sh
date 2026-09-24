#!/usr/bin/env bash
# app:hub:launch -- build, then start the hub in the background. Refuses if a
# hub already holds this state dir's hub/serve.lock.
set -euo pipefail
. "$(dirname "${BASH_SOURCE[0]}")/local-lib.sh"

pid="$(lock_pid)"
if [ -n "$pid" ] && is_holler_hub "$pid"; then
  echo "error: a holler hub is already running (pid $pid) against $state_dir; stop it with ./scripts/run app:hub:stop" >&2
  exit 1
fi

ensure_built
bin="$(holler_path)"
mkdir -p "$state_dir/hub"
(nohup "$bin" hub serve --listen "$hub_listen" --advertise "$hub_advertise" > "$log_file" 2>&1 &)

# Ready when the hub answers on its control socket; give up if it died.
tries=0
until "$bin" hub status >/dev/null 2>&1; do
  tries=$((tries + 1))
  if [ "$tries" -gt 100 ]; then
    echo "error: the hub did not come up within 10s; see $log_file" >&2
    tail -n 20 "$log_file" >&2 || true
    exit 1
  fi
  sleep 0.1
done

echo "hub started (pid $(lock_pid)), log: $log_file"
"$bin" hub status
echo
echo "next steps:"
echo "  holler hub token mint --label <name>   # prints a ready-to-run 'body join' command (ws:// for a loopback advertise)"
echo "  holler roster                          # see what is connected (or ./scripts/run app:hub:status)"
echo "  ./scripts/run app:hub:stop             # stop this hub"
