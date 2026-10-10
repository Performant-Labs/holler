#!/usr/bin/env bash
# herdr.sh - the setup wizard's one door to Herdr (story #730, epic #726).
#
# Adds `--session <herdr_session>` to every Herdr command it runs, `server stop` and
# `session attach` included, so one instance's stage can never touch another instance's server
# or panes. A bare `herdr server stop` is never issued: which server it would stop is not
# established, and it could stop another instance's.
#
# bash 3.2 compatible (no associative arrays, no GNU-only flags): runs on Linux and macOS.
#
# Configuration (environment, set from the instance table by the skill):
#   WIZARD_INSTANCE_NAME    the instance `name` (empty or "default" = the default instance)
#   WIZARD_HERDR_SESSION    the instance `herdr_session` (empty = no named session)
#   WIZARD_INSTANCE_PREFIX  the instance `prefix` (default: the instance name)
#   WIZARD_LEDGER           the ledger file (default: ${WIZARD_STATE_DIR}/wizard-ledger.toml)
#   WIZARD_LOG_DIR          where the server log goes (default: ${TMPDIR:-/tmp})
#
# Verbs:
#   run <herdr args...>   run `herdr --session <name> <args...>`
#   server-start          start the headless server in the background, logging to log-path
#   check-session         refuse when the named session exists and the ledger did not create it
#   check-pane            refuse when this process runs in a pane of a different session
#   log-path              print the server log path for this instance
set -u

die() {
  echo "herdr.sh: $*" >&2
  exit 1
}

instance="${WIZARD_INSTANCE_NAME:-}"
session="${WIZARD_HERDR_SESSION:-}"
[ "$instance" = "default" ] && instance=""

# A non-default instance must name its Herdr session; never fall back to the default session.
if [ -n "$instance" ] && [ -z "$session" ]; then
  die "instance '$instance' is not the default but sets no herdr_session; refusing to run Herdr without a session"
fi
if [ -n "$session" ]; then
  case "$session" in
    [a-z]*) ;;
    *) die "herdr_session '$session' is not valid (^[a-z][a-z0-9-]{0,23}\$)" ;;
  esac
  case "$session" in
    *[!a-z0-9-]*) die "herdr_session '$session' is not valid (^[a-z][a-z0-9-]{0,23}\$)" ;;
  esac
  [ "${#session}" -le 24 ] || die "herdr_session '$session' is longer than 24 characters"
fi

# Runs herdr with the session flag when one is configured.
herdr_cmd() {
  if [ -n "$session" ]; then
    herdr --session "$session" "$@"
  else
    herdr "$@"
  fi
}

# GUESS, unverified until story #734 checks it with the real binary: a process inside a Herdr
# pane is assumed to see HERDR_PANE_ID, and its pane's session name in HERDR_SESSION. This is
# the only place that knowledge lives. Prints the pane's session ("" if unknown); returns 1
# when this process is not inside a pane at all.
pane_session() {
  if [ -z "${HERDR_PANE_ID:-}" ] && [ -z "${HERDR_SESSION:-}" ]; then
    return 1
  fi
  printf '%s\n' "${HERDR_SESSION:-}"
  return 0
}

ledger_file() {
  if [ -n "${WIZARD_LEDGER:-}" ]; then
    printf '%s\n' "$WIZARD_LEDGER"
  elif [ -n "${WIZARD_STATE_DIR:-}" ]; then
    printf '%s\n' "$WIZARD_STATE_DIR/wizard-ledger.toml"
  fi
}

# Exit 0 when the ledger records a `herdr` process for this session name.
ledger_created() {
  f="$(ledger_file)"
  [ -n "$f" ] && [ -f "$f" ] || return 1
  awk -v want="$1" '
    function val(s) { sub(/^[^=]*=[ \t]*/, "", s); gsub(/^"|"[ \t]*$/, "", s); return s }
    function flush() { if (role == "herdr" && sess == want) found = 1 }
    /^\[\[process\]\]/ { flush(); role = ""; sess = ""; next }
    /^\[/ { flush(); role = ""; sess = ""; next }
    /^role[ \t]*=/ { role = val($0) }
    /^session[ \t]*=/ { sess = val($0) }
    END { flush(); exit(found ? 0 : 1) }
  ' "$f"
}

check_session() {
  [ -n "$session" ] || return 0
  listing="$(herdr_cmd session list 2>/dev/null)" || listing=""
  exists=""
  while IFS= read -r line; do
    first="${line%%[[:space:]]*}"
    [ "$first" = "$session" ] && exists=1
  done <<EOF2
$listing
EOF2
  [ -n "$exists" ] || return 0
  if ledger_created "$session"; then
    return 0
  fi
  die "a Herdr session named '$session' already exists and the ledger did not create it; refusing to build in it. Stop and ask the operator."
}

check_pane() {
  [ -n "$session" ] || return 0
  if ! ps_="$(pane_session)"; then
    return 0
  fi
  if [ -z "$ps_" ]; then
    die "running inside a Herdr pane whose session cannot be established; refusing to build (instance session: '$session')"
  fi
  if [ "$ps_" != "$session" ]; then
    die "running inside a pane of Herdr session '$ps_', not this instance's session '$session'; refusing to split any pane"
  fi
  return 0
}

log_path() {
  dir="${WIZARD_LOG_DIR:-${TMPDIR:-/tmp}}"
  prefix="${WIZARD_INSTANCE_PREFIX:-$instance}"
  if [ -n "$prefix" ]; then
    printf '%s\n' "$dir/$prefix-herdr-server.log"
  else
    printf '%s\n' "$dir/herdr-server.log"
  fi
}

verb="${1:-}"
[ $# -gt 0 ] && shift
case "$verb" in
  run)
    [ $# -gt 0 ] || die "run needs a Herdr command"
    for a in "$@"; do
      case "$a" in
        --session | --session=*) die "do not pass --session; herdr.sh adds it" ;;
      esac
    done
    # Without a session name, commands that act on one server are refused outright.
    if [ -z "$session" ]; then
      case "$1 ${2:-}" in
        "server stop" | "session attach")
          die "'herdr $1 $2' without --session is never issued; configure herdr_session"
          ;;
      esac
    fi
    # Nothing that changes a layout runs from a pane of another session.
    case "$1 ${2:-}" in
      "pane split" | "pane run" | "pane send-keys") check_pane ;;
    esac
    herdr_cmd "$@"
    ;;
  server-start)
    [ -n "$session" ] || die "server-start needs a herdr_session"
    log="$(log_path)"
    nohup herdr --session "$session" server >"$log" 2>&1 &
    ;;
  check-session) check_session ;;
  check-pane) check_pane ;;
  log-path) log_path ;;
  *) die "usage: herdr.sh run|server-start|check-session|check-pane|log-path" ;;
esac
