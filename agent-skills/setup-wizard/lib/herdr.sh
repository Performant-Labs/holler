#!/usr/bin/env bash
# herdr.sh - the setup wizard's one door to Herdr (story #730, epic #726).
#
# Adds `--session <herdr_session>` to every Herdr command it runs, `server stop` and
# `session attach` included, so one instance's stage can never touch another instance's server
# or panes. A bare `herdr server stop` is never issued: which server it would stop is not
# established, and it could stop another instance's.
#
# UNVERIFIED until operator story #734 checks it with the real binary: what `--session` does
# (which server and socket it selects), what a pane sees in HERDR_PANE_ID / HERDR_SESSION, and
# the `session list` status words used by session-delete. Do not trust any of it blind.
#
# bash 3.2 compatible (no associative arrays, no GNU-only flags): runs on Linux and macOS.
#
# Configuration (environment, set from the instance table by the skill):
#   WIZARD_INSTANCE_NAME    the instance `name`; REQUIRED. An unset or empty value is refused
#                           (exit 2): the literal "default" is the default instance
#   HERDR_BIN               the herdr binary when it is not on PATH (default: herdr)
#   WIZARD_LIB              where ledger.sh lives (default: this script's own directory)
#   WIZARD_HERDR_SESSION    the instance `herdr_session` (empty = no named session)
#   WIZARD_INSTANCE_PREFIX  the instance `prefix` (default: the instance name)
#   WIZARD_LEDGER           the ledger file (default: ${WIZARD_STATE_DIR}/wizard-ledger.toml)
#   WIZARD_STATE_DIR        the instance state directory (default: $HOME/.holler)
#   WIZARD_LOG_DIR          where the server log goes (default: <state dir>/logs, created)
#
# Verbs:
#   run <herdr args...>   run `herdr --session <name> <args...>`
#   server-start          start the headless server in the background, logging to log-path;
#                         prints the server's pid alone on the first stdout line
#   check-session         refuse when the named session exists and the ledger did not create it
#   check-pane            refuse when this process runs in a pane of a different session
#   session-delete [name] delete this instance's own STOPPED session (`herdr session delete`);
#                         refuses a running or listed-live-in-the-ledger session, any other
#                         name, the default session and a session that is not listed
#   log-path              print the server log path for this instance
set -u

die() {
  echo "herdr.sh: $*" >&2
  exit 1
}

# An unset name must never act as the default instance: shell variables do not persist between
# the agent's commands, and a lost name would otherwise land on the operator's live default Herdr.
if [ -z "${WIZARD_INSTANCE_NAME:-}" ]; then
  echo "herdr.sh: WIZARD_INSTANCE_NAME is not set; refusing to run (use the literal 'default' for the default instance)" >&2
  exit 2
fi
instance="$WIZARD_INSTANCE_NAME"
session="${WIZARD_HERDR_SESSION:-}"
[ "$instance" = "default" ] && instance=""
herdr_bin="${HERDR_BIN:-herdr}"
lib_dir="${WIZARD_LIB:-$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)}"

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
    "$herdr_bin" --session "$session" "$@"
  else
    "$herdr_bin" "$@"
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

# The directory `ledger.sh` reads: the ledger file's own directory when WIZARD_LEDGER is set,
# else the instance state directory.
ledger_state_dir() {
  if [ -n "${WIZARD_LEDGER:-}" ]; then
    dirname "$WIZARD_LEDGER"
  elif [ -n "${WIZARD_STATE_DIR:-}" ]; then
    printf '%s\n' "$WIZARD_STATE_DIR"
  fi
}

# Exit 0 when `ledger.sh list` shows a LIVE `herdr` row for this session name (a stale row, a
# row of another role or of another session does not count).
ledger_created() {
  dir="$(ledger_state_dir)"
  [ -n "$dir" ] || return 1
  rows="$(HOLLER_STATE_DIR="$dir" bash "$lib_dir/ledger.sh" list 2>/dev/null)" || return 1
  tab="$(printf '\t')"
  while IFS="$tab" read -r _pid state role _stage sess _cmd; do
    if [ "$state" = "live" ] && [ "$role" = "herdr" ] && [ "$sess" = "$1" ]; then
      return 0
    fi
  done <<EOF3
$rows
EOF3
  return 1
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
    if [ -n "${HERDR_PANE_ID:-}" ]; then
      die "the driving agent is running inside a Herdr pane, so this instance's session '$session' cannot be established; run the wizard from a terminal outside any Herdr pane"
    fi
    die "running inside a Herdr pane whose session cannot be established; refusing to build (instance session: '$session')"
  fi
  if [ "$ps_" != "$session" ]; then
    die "running inside a pane of Herdr session '$ps_', not this instance's session '$session'; refusing to split any pane"
  fi
  return 0
}

# Deletes this instance's own stopped session, and nothing else.
session_delete() {
  target="${1:-$session}"
  [ -n "$session" ] || die "session-delete needs this instance's herdr_session; the default session is never deleted"
  [ "$session" != "default" ] || die "the default session is never deleted"
  [ "$target" = "$session" ] || die "session-delete only deletes this instance's own session '$session', not '$target'"
  listing="$(herdr_cmd session list 2>/dev/null)" || listing=""
  status=""
  while IFS= read -r line; do
    set -- $line
    if [ "${1:-}" = "$session" ]; then
      status="${2:-}"
    fi
  done <<EOF4
$listing
EOF4
  [ -n "$status" ] || die "no Herdr session named '$session' is listed; nothing to delete"
  [ "$status" = "stopped" ] || die "Herdr session '$session' is '$status', not stopped; refusing to delete it"
  if ledger_created "$session"; then
    die "the ledger shows a live Herdr server for session '$session'; refusing to delete it"
  fi
  herdr_cmd session delete "$session"
}

log_path() {
  if [ -n "${WIZARD_LOG_DIR:-}" ]; then
    dir="$WIZARD_LOG_DIR"
  elif [ -n "${WIZARD_STATE_DIR:-}" ]; then
    dir="$WIZARD_STATE_DIR/logs"
  else
    dir="${HOME:-.}/.holler/logs"
  fi
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
    # The default instance (empty session) starts an unnamed server; a named instance's server
    # always carries its session (an empty session on a non-default instance is refused above).
    log="$(log_path)"
    mkdir -p "$(dirname "$log")" || die "cannot create the log directory for $log"
    # nohup execs herdr, so $! is the server's own pid, not a wrapper's.
    if [ -n "$session" ]; then
      { nohup "$herdr_bin" --session "$session" server >"$log" 2>&1 & echo $!; }
    else
      { nohup "$herdr_bin" server >"$log" 2>&1 & echo $!; }
    fi
    ;;
  check-session) check_session ;;
  check-pane) check_pane ;;
  session-delete) session_delete "$@" ;;
  log-path) log_path ;;
  *) die "usage: herdr.sh run|server-start|check-session|check-pane|session-delete|log-path" ;;
esac
