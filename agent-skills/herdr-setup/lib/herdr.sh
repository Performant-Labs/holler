#!/usr/bin/env bash
# herdr.sh - the setup wizard's one door to Herdr (story #730, epic #726), and the door through
# which it runs Holler's pane and profile verbs (`holler pane ...`, `holler profile ...`, #803).
#
# Adds `--session <herdr_session>` to every Herdr command it runs, so one instance's stage can
# never touch another instance's server. Herdr itself is reached only for the read-only words
# and the server start: every pane is built, changed and read through Holler. A bare
# `herdr server stop` is never issued: which server it would stop is not established.
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
#   WIZARD_HOLLER_BIN       the holler binary when it is not on PATH (default: holler)
#   WIZARD_LIB              where ledger.sh lives (default: this script's own directory)
#   WIZARD_HERDR_SESSION    the instance `herdr_session` (empty = no named session)
#   WIZARD_INSTANCE_PREFIX  the instance `prefix` (default: the instance name)
#   WIZARD_LEDGER           the ledger file (default: ${WIZARD_STATE_DIR}/herdr/setup/wizard-ledger.toml)
#   WIZARD_STATE_DIR        the instance state directory (default: $HOME/.holler; `holler`
#                           requires it, absolute)
#   WIZARD_LOG_DIR          where the server log goes (default: <state dir>/logs, created)
#   WIZARD_HERDR_WORKSPACES the workspace and its extent, `<label>=<R>x<C>` (`panes.sh env`)
#   WIZARD_HERDR_SOCKET     the default instance's Herdr socket (`panes.sh env`); `holler`
#                           requires an absolute one for the default instance
#   WIZARD_TMUX_SOCKET      a named instance's own tmux socket (`panes.sh env`); `holler`
#                           requires an absolute one for any instance but the default
#
# Verbs:
#   run <words...>        `herdr --session <name> <words...>` for the read-only words only:
#                         `status`, `session list` and `workspace list`. Any other word (a
#                         pane or workspace change, `server stop`, `session attach`) exits 2
#                         and runs no Herdr: use `holler pane` through the `holler` verb
#   holler <args...>      run `holler <args...>` unchanged in a scrubbed environment: the base
#                         list below and the instance's literals, HOLLER_STATE_DIR,
#                         HOLLER_HERDR_SESSION (`default` for the default instance),
#                         HOLLER_HERDR_WORKSPACES and, when set, HOLLER_HERDR_SOCKET and
#                         HOLLER_TMUX_SOCKET. No HERDR_* and none of the caller's own HOLLER_*:
#                         the tmux server the first pane starts keeps this environment, and
#                         every pane inherits it
#   server-start          start the headless server in the background, logging to log-path,
#                         in a scrubbed environment (the base list plus HERDR_*); prints the
#                         server's pid alone on the first stdout line
#   check-session         refuse when the named session exists and the ledger did not create it
#   check-pane            refuse when this process runs in a pane of a different session
#   session-delete [name] delete this instance's own STOPPED session (`herdr session delete`);
#                         refuses a running or listed-live-in-the-ledger session, any other
#                         name, the default session and a session that is not listed
#   log-path              print the server log path for this instance
#
# The base list (both scrubbed environments): PATH HOME USER LOGNAME SHELL TERM LANG LC_* TMPDIR
# XDG_* plus the session set a pane needs: SSH_AUTH_SOCK DISPLAY WAYLAND_DISPLAY
# DBUS_SESSION_BUS_ADDRESS COLORTERM TZ EDITOR VISUAL LANGUAGE, the proxy variables in both cases
# and SSL_CERT_FILE SSL_CERT_DIR NODE_EXTRA_CA_CERTS; nothing else.
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
    printf '%s\n' "$WIZARD_STATE_DIR/herdr/setup/wizard-ledger.toml"
  fi
}

# The state directory `ledger.sh` reads: three levels above the ledger file when WIZARD_LEDGER
# is set (<state_dir>/herdr/setup/wizard-ledger.toml), else the instance state directory.
ledger_state_dir() {
  if [ -n "${WIZARD_LEDGER:-}" ]; then
    dirname "$(dirname "$(dirname "$WIZARD_LEDGER")")"
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

# The base list (header) into `keep`, as NAME=value words: what a shell and a pane need, and
# nothing of the driving agent's own (its credentials, its Herdr pane, its tmux client). Each verb
# adds its own: `server-start` HERDR_*, `holler` the instance's HOLLER_* literals (#771, #803).
base_env() {
  keep=()
  for name in $(compgen -e); do
    case "$name" in
      PATH | HOME | USER | LOGNAME | SHELL | TERM | LANG | LC_* | TMPDIR | XDG_* | \
        SSH_AUTH_SOCK | DISPLAY | WAYLAND_DISPLAY | DBUS_SESSION_BUS_ADDRESS | COLORTERM | TZ | \
        EDITOR | VISUAL | LANGUAGE | \
        HTTP_PROXY | HTTPS_PROXY | NO_PROXY | ALL_PROXY | \
        http_proxy | https_proxy | no_proxy | all_proxy | \
        SSL_CERT_FILE | SSL_CERT_DIR | NODE_EXTRA_CA_CERTS)
        keep+=("$name=${!name}")
        ;;
    esac
  done
}

# Refuses (exit 2, nothing run) a `holler` call whose instance literals are missing: a lost value
# must never reach the default hub, the default instance's Herdr socket, or tmux's default server.
holler_guard() {
  case "${WIZARD_STATE_DIR:-}" in
    /*) ;;
    *)
      echo "herdr.sh: WIZARD_STATE_DIR is not set to an absolute path; refusing to run holler (it would reach the default hub)" >&2
      exit 2
      ;;
  esac
  if [ -n "$instance" ]; then
    case "${WIZARD_TMUX_SOCKET:-}" in
      /*) ;;
      *)
        echo "herdr.sh: WIZARD_TMUX_SOCKET is not set to an absolute path for instance '$instance' (panes.sh env prints it); refusing to run holler on tmux's default server" >&2
        exit 2
        ;;
    esac
  else
    case "${WIZARD_HERDR_SOCKET:-}" in
      /*) ;;
      *)
        echo "herdr.sh: WIZARD_HERDR_SOCKET is not set to an absolute path for the default instance (panes.sh env prints it); refusing to run holler" >&2
        exit 2
        ;;
    esac
  fi
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
    # Only Herdr's read-only words reach Herdr: every pane is Holler's (#803).
    case "$1 ${2:-}" in
      "status "* | "session list" | "workspace list") ;;
      *)
        echo "herdr.sh: 'run $1${2:+ $2}' is refused: run takes only status, session list and workspace list; build, change and read panes with holler pane, through herdr.sh holler" >&2
        exit 2
        ;;
    esac
    herdr_cmd "$@"
    ;;
  holler)
    [ $# -gt 0 ] || die "holler needs Holler's words, for example: holler pane list --format=json"
    holler_guard
    # Nothing runs from a pane of another Herdr session (the guard the pane verbs had).
    check_pane
    base_env
    keep+=("HOLLER_STATE_DIR=$WIZARD_STATE_DIR" "HOLLER_HERDR_SESSION=${session:-default}"
      "HOLLER_HERDR_WORKSPACES=${WIZARD_HERDR_WORKSPACES:-}")
    if [ -n "${WIZARD_HERDR_SOCKET:-}" ]; then keep+=("HOLLER_HERDR_SOCKET=$WIZARD_HERDR_SOCKET"); fi
    if [ -n "${WIZARD_TMUX_SOCKET:-}" ]; then keep+=("HOLLER_TMUX_SOCKET=$WIZARD_TMUX_SOCKET"); fi
    exec env -i "${keep[@]}" "${WIZARD_HOLLER_BIN:-holler}" "$@"
    ;;
  server-start)
    # The default instance (empty session) starts an unnamed server; a named instance's server
    # always carries its session (an empty session on a non-default instance is refused above).
    log="$(log_path)"
    mkdir -p "$(dirname "$log")" || die "cannot create the log directory for $log"
    # Only a named set of variables reaches the server (and so every pane it opens): the driving
    # agent's own variables must not leak into the operator's orchestrators.
    base_env
    for name in $(compgen -e); do
      case "$name" in
        HERDR_*) keep+=("$name=${!name}") ;;
      esac
    done
    # HERDR_BIN's directory stays reachable.
    case "$herdr_bin" in
      */*) keep+=("PATH=${PATH:-}:$(dirname "$herdr_bin")") ;;
    esac
    # env and nohup exec, so $! is the server's own pid, not a wrapper's.
    if [ -n "$session" ]; then
      { env -i "${keep[@]}" nohup "$herdr_bin" --session "$session" server >"$log" 2>&1 & echo $!; }
    else
      { env -i "${keep[@]}" nohup "$herdr_bin" server >"$log" 2>&1 & echo $!; }
    fi
    ;;
  check-session) check_session ;;
  check-pane) check_pane ;;
  session-delete) session_delete "$@" ;;
  log-path) log_path ;;
  *) die "usage: herdr.sh run|holler|server-start|check-session|check-pane|session-delete|log-path" ;;
esac
