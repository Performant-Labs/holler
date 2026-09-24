#!/usr/bin/env bash
# Shared helpers for scripts/local-*.sh (sourced, not run). Bash 3.2 / BSD safe.

root="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)"
state_dir="${HOLLER_STATE_DIR:-$HOME/.holler}"
hub_listen="${HOLLER_HUB_LISTEN:-127.0.0.1:41807}"
hub_advertise="${HOLLER_HUB_ADVERTISE:-$hub_listen}"
lock_file="$state_dir/hub/serve.lock"
log_file="$state_dir/hub/serve.log"

# The HOLLER_STATE_DIR the binary sees is the one we resolved here.
export HOLLER_STATE_DIR="$state_dir"

# Path of the holler binary: HOLLER_BIN, else the release build.
holler_path() {
  echo "${HOLLER_BIN:-${CARGO_TARGET_DIR:-$root/target}/release/holler}"
}

# Build first (unless HOLLER_BIN pins a binary), then require the binary.
ensure_built() {
  if [ -z "${HOLLER_BIN:-}" ]; then
    "$root/scripts/run" build >&2
  fi
  require_binary
}

# Require an existing binary without building (stop/status scripts).
require_binary() {
  local bin
  bin="$(holler_path)"
  if [ ! -x "$bin" ]; then
    echo "error: no holler binary at $bin; run ./scripts/run build (or set HOLLER_BIN)" >&2
    exit 1
  fi
}

# Print the pid in the serve lock, or nothing if there is no lock / no pid.
lock_pid() {
  local pid=""
  if [ -f "$lock_file" ]; then
    pid="$(tr -d '[:space:]' < "$lock_file")"
  fi
  case "$pid" in
    ''|*[!0-9]*) return 0 ;;
  esac
  echo "$pid"
}

# True if the pid is alive AND its command line is a `holler ... hub serve`.
# Never signal a pid for which this is false: a stale lock's pid may have been
# reused by an unrelated process.
is_holler_hub() {
  local pid="$1" cmdline argv0
  kill -0 "$pid" 2>/dev/null || return 1
  cmdline="$(ps -o command= -p "$pid" 2>/dev/null || true)"
  case "$cmdline" in
    *" hub serve"*) ;;
    *) return 1 ;;
  esac
  argv0="${cmdline%% *}"
  [ "${argv0##*/}" = "holler" ]
}
