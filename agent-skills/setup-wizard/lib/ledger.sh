#!/usr/bin/env bash
# ledger.sh - the setup wizard's per-(host, instance) process ledger (epic #726, story #729).
#
# The ledger is <state_dir>/wizard-ledger.toml, where state_dir is $HOLLER_STATE_DIR, or
# $HOME/.holler when that is unset or empty (Holler's own default). Mode 0600, written
# atomically (temp file in the same directory, then rename). One [[process]] table per process
# the wizard started: pid, started, cmd, role, stage, session.
#
# Verbs:
#   record --pid <pid> --role <backend|hub|serve|body|herdr> --stage <4..9> [--session <name>]
#       Capture the process's start time and command from `ps` now and append the entry
#       (replacing any earlier entry for the same pid). Exit 1 if the pid does not exist.
#   list
#       One line per entry, tab separated:  pid  state  role  stage  session  cmd
#       state is `live` or `stale`; session may be empty; cmd is last.
#   owns <pid>
#       exit 0: recorded and live; 1: recorded but stale (dead or a reused pid);
#       2: not recorded (foreign). Prints nothing.
#
# A process is live when its pid exists and its current start time and command equal the
# recorded ones. Stale and foreign processes must never be signalled by the caller.
# bash 3.2 compatible; runs on Linux and macOS.

set -u

die() { printf 'ledger.sh: %s\n' "$*" >&2; exit 64; }

state_dir() {
  if [ -n "${HOLLER_STATE_DIR:-}" ]; then
    printf '%s' "$HOLLER_STATE_DIR"
  else
    [ -n "${HOME:-}" ] || die "neither HOLLER_STATE_DIR nor HOME is set"
    printf '%s/.holler' "$HOME"
  fi
}

LEDGER="$(state_dir)/wizard-ledger.toml"

trim() { sed -e 's/^[[:space:]]*//' -e 's/[[:space:]]*$//'; }

ps_started() { LC_ALL=C ps -o lstart= -p "$1" 2>/dev/null | trim; }
ps_cmd() { ps -o command= -p "$1" 2>/dev/null | trim; }

toml_escape() { printf '%s' "$1" | sed -e 's/\\/\\\\/g' -e 's/"/\\"/g'; }

# Emit every entry as: pid,started,role,stage,session,cmd (unescaped), separated by the ASCII
# unit separator (a non-blank separator, so an empty session does not collapse).
entries() {
  [ -f "$LEDGER" ] || return 0
  awk '
    function unq(s,   out, i, c, n) {
      sub(/^[^"]*"/, "", s); sub(/"[[:space:]]*$/, "", s)
      out = ""; n = length(s)
      for (i = 1; i <= n; i++) {
        c = substr(s, i, 1)
        if (c == "\\" && i < n) { i++; c = substr(s, i, 1) }
        out = out c
      }
      return out
    }
    function flush() {
      if (have) printf "%s\037%s\037%s\037%s\037%s\037%s\n", pid, started, role, stage, session, cmd
      have = 0; pid = started = role = stage = session = cmd = ""
    }
    /^\[\[process\]\]/ { flush(); have = 1; next }
    /^pid[[:space:]]*=/ { v = $0; sub(/^[^=]*=[[:space:]]*/, "", v); pid = v; next }
    /^stage[[:space:]]*=/ { v = $0; sub(/^[^=]*=[[:space:]]*/, "", v); stage = v; next }
    /^started[[:space:]]*=/ { started = unq($0); next }
    /^cmd[[:space:]]*=/ { cmd = unq($0); next }
    /^role[[:space:]]*=/ { role = unq($0); next }
    /^session[[:space:]]*=/ { session = unq($0); next }
    END { flush() }
  ' "$LEDGER"
}

# live <pid> <started> <cmd>: 0 when the process exists with exactly that start and command.
is_live() {
  local now_started now_cmd
  now_started="$(ps_started "$1")"
  [ -n "$now_started" ] || return 1
  now_cmd="$(ps_cmd "$1")"
  [ "$now_started" = "$2" ] && [ "$now_cmd" = "$3" ]
}

cmd_list() {
  local us pid started role stage session cmd state
  us="$(printf '\037')"
  entries | while IFS="$us" read -r pid started role stage session cmd; do
    [ -n "$pid" ] || continue
    if is_live "$pid" "$started" "$cmd"; then state=live; else state=stale; fi
    printf '%s\t%s\t%s\t%s\t%s\t%s\n' "$pid" "$state" "$role" "$stage" "$session" "$cmd"
  done
}

cmd_owns() {
  [ $# -eq 1 ] || die "usage: ledger.sh owns <pid>"
  case "$1" in ''|*[!0-9]*) die "pid must be an integer" ;; esac
  local us pid started role stage session cmd found
  us="$(printf '\037')"
  found=2
  while IFS="$us" read -r pid started role stage session cmd; do
    [ "$pid" = "$1" ] || continue
    if is_live "$pid" "$started" "$cmd"; then return 0; fi
    found=1
  done <<EOF2
$(entries)
EOF2
  return "$found"
}

cmd_record() {
  local pid="" role="" stage="" session=""
  while [ $# -gt 0 ]; do
    case "$1" in
      --pid) [ $# -ge 2 ] || die "--pid needs a value"; pid="$2"; shift 2 ;;
      --role) [ $# -ge 2 ] || die "--role needs a value"; role="$2"; shift 2 ;;
      --stage) [ $# -ge 2 ] || die "--stage needs a value"; stage="$2"; shift 2 ;;
      --session) [ $# -ge 2 ] || die "--session needs a value"; session="$2"; shift 2 ;;
      *) die "unknown argument: $1" ;;
    esac
  done
  case "$pid" in ''|*[!0-9]*) die "--pid must be an integer" ;; esac
  case "$stage" in ''|*[!0-9]*) die "--stage must be an integer (4 to 9)" ;; esac
  [ "$stage" -ge 4 ] && [ "$stage" -le 9 ] || die "--stage must be 4 to 9"
  case "$role" in backend|hub|serve|body|herdr) ;; *) die "--role must be backend, hub, serve, body or herdr" ;; esac

  local started cmd
  started="$(ps_started "$pid")"
  cmd="$(ps_cmd "$pid")"
  if [ -z "$started" ] || [ -z "$cmd" ]; then
    printf 'ledger.sh: no process with pid %s; nothing recorded\n' "$pid" >&2
    return 1
  fi

  local dir lock tmp tries
  dir="$(dirname "$LEDGER")"
  mkdir -p "$dir" || die "cannot create $dir"
  lock="$LEDGER.lock"
  tries=0
  until mkdir "$lock" 2>/dev/null; do
    tries=$((tries + 1))
    [ "$tries" -le 100 ] || die "ledger is locked ($lock)"
    sleep 0.1 2>/dev/null || sleep 1
  done
  tmp="$(mktemp "$dir/.wizard-ledger.XXXXXX")" || { rmdir "$lock"; die "cannot create a temp file in $dir"; }
  chmod 600 "$tmp"
  {
    local us p s r st se c
    us="$(printf '\037')"
    while IFS="$us" read -r p s r st se c; do
      [ -n "$p" ] || continue
      [ "$p" = "$pid" ] && continue
      printf '[[process]]\npid = %s\nstarted = "%s"\ncmd = "%s"\nrole = "%s"\nstage = %s\nsession = "%s"\n\n' \
        "$p" "$(toml_escape "$s")" "$(toml_escape "$c")" "$(toml_escape "$r")" "$st" "$(toml_escape "$se")"
    done <<EOF2
$(entries)
EOF2
    printf '[[process]]\npid = %s\nstarted = "%s"\ncmd = "%s"\nrole = "%s"\nstage = %s\nsession = "%s"\n\n' \
      "$pid" "$(toml_escape "$started")" "$(toml_escape "$cmd")" "$role" "$stage" "$(toml_escape "$session")"
  } > "$tmp"
  if mv -f "$tmp" "$LEDGER"; then
    rmdir "$lock"
  else
    rm -f "$tmp"; rmdir "$lock"; die "cannot write $LEDGER"
  fi
}

verb="${1:-}"
[ $# -gt 0 ] && shift
case "$verb" in
  record) cmd_record "$@" ;;
  list) cmd_list ;;
  owns) cmd_owns "$@" ;;
  *) die "usage: ledger.sh record|list|owns ..." ;;
esac
