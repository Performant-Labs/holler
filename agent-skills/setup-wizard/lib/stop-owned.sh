#!/usr/bin/env bash
# stop-owned.sh - stop, restart or report a process, but only one the instance's own
# ledger recorded and whose recorded start time and command still match (epic #726).
#
# Usage:
#   stop-owned.sh stop       <state_dir> <pid>      signal one ledger process
#   stop-owned.sh restart    <state_dir> <pid>      stop it, then print its recorded command
#   stop-owned.sh check-port <state_dir> <port>     report who holds a planned port
#   stop-owned.sh teardown   <state_dir> [--purge-state]
#                                                   stop every live ledger process in
#                                                   reverse start order; say what is left
#
# Exit codes (stop, restart, check-port):
#   0 done / port free / port held by a live ledger process
#   1 stale: the pid exists but is not the recorded process (reused pid); not signalled
#   2 foreign: the pid is in no ledger for this instance; not signalled
#   3 the signal was sent but the process is still running after the grace period
#   4 usage or environment error
#
# Ledger access goes only through "$WIZARD_LIB/ledger.sh" (default: this script's own
# directory), with the instance's state directory passed as HOLLER_STATE_DIR (the way
# ledger.sh takes it): `owns <pid>` (0 live, 1 stale or gone, 2 not recorded) and `list`
# (one TAB-separated line per entry: pid, live|stale, role, stage, session, cmd; session may
# be empty, so it is parsed with awk -F'\t', never with read). Nothing is ever signalled by name or pattern, only by a recorded pid
# that `owns` just confirmed live. Bash 3.2 compatible.

set -u

SELF_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
LIB="${WIZARD_LIB:-$SELF_DIR}"
LEDGER="$LIB/ledger.sh"
GRACE="${STOP_OWNED_GRACE:-10}"

die() { echo "stop-owned: $*" >&2; exit 4; }

[ -f "$LEDGER" ] || die "ledger.sh not found in $LIB; refusing to signal anything"

is_num() { case "$1" in ''|*[!0-9]*) return 1 ;; *) return 0 ;; esac; }

# The ledger decides; anything it does not clearly answer is treated as foreign.
owns() {
  HOLLER_STATE_DIR="$1" bash "$LEDGER" owns "$2" >/dev/null 2>&1
  case $? in 0) return 0 ;; 1) return 1 ;; *) return 2 ;; esac
}

cur_cmd() { ps -o command= -p "$1" 2>/dev/null | sed 's/^[[:space:]]*//;s/[[:space:]]*$//'; }
cur_user() { ps -o user= -p "$1" 2>/dev/null | sed 's/^[[:space:]]*//;s/[[:space:]]*$//'; }

# A zombie has exited; it only waits for its parent to reap it.
is_running() {
  local st
  kill -0 "$1" 2>/dev/null || return 1
  st="$(ps -o stat= -p "$1" 2>/dev/null | sed 's/^[[:space:]]*//')"
  case "$st" in Z*) return 1 ;; esac
  return 0
}

valid_pid() {
  is_num "$1" || return 1
  [ "$1" -gt 1 ] || return 1
  [ "$1" -ne "$$" ] || return 1
  [ "$1" -ne "$PPID" ] || return 1
  return 0
}

ledger_list() { HOLLER_STATE_DIR="$1" bash "$LEDGER" list 2>/dev/null; }

# Recorded command (sixth TAB-separated field) of one pid.
recorded_cmd() {
  ledger_list "$1" | awk -F'\t' -v want="$2" '$1 == want { print $6; exit }'
}

# Recorded pids in start order: by stage, then pid (both numeric).
ledger_pids() {
  ledger_list "$1" | awk -F'\t' '$1 ~ /^[0-9]+$/ && $4 ~ /^[0-9]+$/ { print $4 " " $1 }' |
    sort -n -k1,1 -k2,2 | awk '{ print $2 }'
}

reverse_lines() {
  awk '{ l[NR] = $0 } END { for (i = NR; i >= 1; i--) print l[i] }'
}

# Signal one confirmed-live ledger pid. Returns 0 stopped, 3 still running.
signal_and_wait() {
  local pid="$1" i=0
  kill -TERM "$pid" 2>/dev/null
  while [ "$i" -lt $((GRACE * 10)) ]; do
    is_running "$pid" || return 0
    sleep 0.1 2>/dev/null || sleep 1
    i=$((i + 1))
  done
  is_running "$pid" && return 3
  return 0
}

# Prints a verdict line; returns 0 live, 1 stale, 2 foreign.
classify() {
  local sd="$1" pid="$2" rc
  owns "$sd" "$pid"; rc=$?
  case $rc in
    0) return 0 ;;
    1)
      echo "STALE pid $pid: the ledger recorded it but it is no longer that process (now: $(cur_cmd "$pid" | sed 's/^$/not running/')). Not signalled."
      return 1 ;;
    *)
      echo "FOREIGN pid $pid (owner $(cur_user "$pid"), command: $(cur_cmd "$pid")): not in this instance's ledger. Not signalled."
      return 2 ;;
  esac
}

do_stop() {
  local sd="$1" pid="$2" rc
  valid_pid "$pid" || die "not a usable pid: $pid"
  classify "$sd" "$pid"; rc=$?
  [ "$rc" -eq 0 ] || return "$rc"
  local cmd; cmd="$(cur_cmd "$pid")"
  if signal_and_wait "$pid"; then
    echo "STOPPED pid $pid ($cmd)"
    return 0
  fi
  echo "STILL-RUNNING pid $pid ($cmd) after ${GRACE}s; sent SIGTERM only, nothing escalated."
  return 3
}

do_restart() {
  local sd="$1" pid="$2" cmd rc
  valid_pid "$pid" || die "not a usable pid: $pid"
  # Read the recorded command before the stop, in case the ledger drops the entry.
  cmd="$(recorded_cmd "$sd" "$pid")"
  do_stop "$sd" "$pid"; rc=$?
  [ "$rc" -eq 0 ] || return "$rc"
  echo "RESTART-CMD $cmd"
  return 0
}

do_check_port() {
  local sd="$1" port="$2" pids p worst=0 rc
  is_num "$port" || die "not a port: $port"
  pids=""
  if command -v lsof >/dev/null 2>&1; then
    pids="$(lsof -nP -iTCP:"$port" -sTCP:LISTEN -t 2>/dev/null | sort -u)"
  elif command -v ss >/dev/null 2>&1; then
    pids="$(ss -H -ltnp "sport = :$port" 2>/dev/null | sed -n 's/.*pid=\([0-9][0-9]*\).*/\1/p' | sort -u)"
  else
    die "need lsof or ss to look up who holds port $port"
  fi
  if [ -z "$pids" ]; then
    echo "FREE port $port"
    return 0
  fi
  for p in $pids; do
    classify "$sd" "$p"; rc=$?
    if [ "$rc" -eq 0 ]; then
      echo "OWNED port $port is held by pid $p, a live process of this instance's ledger."
    else
      echo "PORT $port is held by pid $p; the wizard stops and asks the user before doing anything about it."
      [ "$rc" -gt "$worst" ] && worst="$rc"
    fi
  done
  return "$worst"
}

do_teardown() {
  local sd="$1" purge="$2" pids p rc fail=0 n_stop=0 n_stale=0 n_foreign=0
  pids="$(ledger_pids "$sd" | reverse_lines)"
  if [ -z "$pids" ]; then
    echo "teardown: no ledger processes recorded for $sd"
  fi
  for p in $pids; do
    valid_pid "$p" || { echo "SKIP unusable recorded pid $p"; continue; }
    do_stop "$sd" "$p"; rc=$?
    case $rc in
      0) n_stop=$((n_stop + 1)) ;;
      1) n_stale=$((n_stale + 1)) ;;
      2) n_foreign=$((n_foreign + 1)) ;;
      *) fail=1 ;;
    esac
  done
  if [ "$fail" -ne 0 ]; then
    echo "teardown: a process is still running; instance state kept."
    return 3
  fi
  rm -f "$sd/wizard-ledger.toml"
  if [ "$purge" = "yes" ]; then
    case "$sd" in
      ''|/|"$HOME"|"$HOME"/) echo "LEFT state dir $sd (refused to purge a shared or top-level path)" ;;
      /*) rm -rf "$sd" && echo "REMOVED state dir $sd" ;;
      *) echo "LEFT state dir $sd (not an absolute path)" ;;
    esac
  else
    echo "LEFT state dir $sd (other files in it; pass --purge-state to remove it)"
  fi
  echo "teardown: stopped $n_stop, stale and not signalled $n_stale, foreign and not signalled $n_foreign."
  echo "LEFT every process and port that is not in this instance's ledger, and every other instance's state."
  return 0
}

verb="${1:-}"
case "$verb" in
  stop)       [ $# -eq 3 ] || die "usage: stop <state_dir> <pid>"; do_stop "$2" "$3" ;;
  restart)    [ $# -eq 3 ] || die "usage: restart <state_dir> <pid>"; do_restart "$2" "$3" ;;
  check-port) [ $# -eq 3 ] || die "usage: check-port <state_dir> <port>"; do_check_port "$2" "$3" ;;
  teardown)
    [ $# -ge 2 ] || die "usage: teardown <state_dir> [--purge-state]"
    purge=no
    if [ "${3:-}" = "--purge-state" ]; then purge=yes; elif [ $# -ge 3 ]; then die "unknown option: $3"; fi
    do_teardown "$2" "$purge" ;;
  *) die "usage: stop-owned.sh stop|restart|check-port|teardown <state_dir> ..." ;;
esac
exit $?
