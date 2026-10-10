#!/usr/bin/env bash
# collide.sh - compare a plan with one host's inventory and REFUSE any collision (#728).
#
#   collide.sh <host-label> <inventory-file>
#
# <inventory-file> is inventory.sh's output for that host. The plan comes from the contract's
# keys as environment variables (empty or unset = not planned on this host):
#   WIZARD_INSTANCE_NAME        name
#   WIZARD_HUB_PORT             hub_port (the hub host only)
#   WIZARD_SERVE_HTTPS_PORT     serve_https_port (the hub host only)
#   WIZARD_STATE_DIR            state_dir (empty = Holler's default; not compared)
#   WIZARD_HERDR_SESSION        herdr_session (the hub host only)
#   WIZARD_BACKEND_PORTS        space-separated backend ports planned on this host
#   WIZARD_SESSION_NAMES        space-separated session names whose bodies run on this host
#   WIZARD_LEDGER               optional: this instance's wizard-ledger.toml on this host
#
# A process is "created by this instance" only if the ledger holds its pid with the same
# `started` and `cmd` (a live entry, per the epic's contract); every other process is foreign.
# Prints the inventory beside the plan ("present, not touched" for foreign items), then one
# `REFUSED:` line per collision naming the item and the config key to change.
# Exit 0 = no collision, 1 = at least one collision, 2 = bad usage.
# Read-only: it runs no external program but awk and sed. bash 3.2 compatible.

host=${1:-}
inv=${2:-}
if [ -z "$host" ] || [ ! -r "$inv" ]; then
  echo "usage: collide.sh <host-label> <inventory-file>" >&2
  exit 2
fi
T=$(printf '\t')

hub_port=${WIZARD_HUB_PORT:-}
serve_port=${WIZARD_SERVE_HTTPS_PORT:-}
state_dir=${WIZARD_STATE_DIR:-}
herdr_session=${WIZARD_HERDR_SESSION:-}
backend_ports=${WIZARD_BACKEND_PORTS:-}
session_names=${WIZARD_SESSION_NAMES:-}
ledger=${WIZARD_LEDGER:-}

# Ledger entries as tab-separated: pid started cmd
ledger_rows=""
if [ -n "$ledger" ] && [ -r "$ledger" ]; then
  ledger_rows=$(awk -v T="$T" '
    function val(s) { sub(/^[^=]*=[ \t]*/, "", s); gsub(/^"|"[ \t]*$/, "", s); return s }
    function flush() { if (pid != "") print pid T started T cmd; pid = ""; started = ""; cmd = "" }
    /^\[\[process\]\]/ { flush(); next }
    /^pid[ \t]*=/ { pid = val($0) }
    /^started[ \t]*=/ { started = val($0) }
    /^cmd[ \t]*=/ { cmd = val($0) }
    END { flush() }
  ' "$ledger")
fi

# ours <pid> <started> <cmd>: 0 when a ledger entry matches all three
ours() {
  local p s c
  [ -n "$ledger_rows" ] || return 1
  while IFS="$T" read -r p s c; do
    if [ "$p" = "$1" ] && [ "$s" = "$2" ] && [ "$c" = "$3" ]; then return 0; fi
  done <<EOF2
$ledger_rows
EOF2
  return 1
}

# Pass 1: which pids and Herdr sessions are ours.
ours_pids=" "
ours_sessions=" "
while IFS="$T" read -r kind pid user sd key started cmd; do
  case "$kind" in
    hub | body | opencode | herdr)
      if ours "$pid" "$started" "$cmd"; then
        ours_pids="$ours_pids$pid "
        [ "$kind" = herdr ] && ours_sessions="$ours_sessions$key "
      fi
      ;;
  esac
done <"$inv"

is_ours_pid() { case "$ours_pids" in *" $1 "*) return 0 ;; esac; return 1; }
in_words() { case " $2 " in *" $1 "*) return 0 ;; esac; return 1; }

refusals=0
refuse() {
  refusals=$((refusals + 1))
  REFUSALS="${REFUSALS:-}REFUSED: $1 -- change $2${NL}"
}
NL='
'
REFUSALS=""
refused_ports=" "

echo "== collision preflight on $host (instance ${WIZARD_INSTANCE_NAME:-default}) =="
echo "-- already running here:"
anything=0
while IFS="$T" read -r kind a b c d e f; do
  anything=1
  case "$kind" in
    port)
      if [ "$b" != "-" ] && is_ours_pid "$b"; then st="created by this instance"
      else st="present, not touched"; fi
      echo "  $st: port $a (${c}, pid $b)" ;;
    hub | body | opencode | herdr)
      if is_ours_pid "$a"; then st="created by this instance"
      else st="present, not touched"; fi
      what="$kind pid $a user $b"
      [ "$kind" = body ] && what="body for session $d (pid $a, user $b)"
      [ "$kind" = hub ] && what="hub on port $d (pid $a, user $b)"
      [ "$kind" = opencode ] && what="opencode backend on port $d (pid $a, user $b)"
      [ "$c" != "-" ] && what="$what, state dir $c"
      echo "  $st: $what" ;;
    serve)
      echo "  present, not touched: tailscale serve on https port $a" ;;
    herdr-session)
      if in_words "$a" "$ours_sessions"; then st="created by this instance"
      else st="present, not touched"; fi
      echo "  $st: herdr session $a" ;;
  esac
done <"$inv"
[ "$anything" = 1 ] || echo "  (nothing found)"

# Pass 2: collisions of the plan with anything that is not ours.
hub_explained=""
while IFS="$T" read -r kind a b c d e f; do
  case "$kind" in
    hub)
      is_ours_pid "$a" && continue
      if [ -n "$hub_port" ] && [ "$d" = "$hub_port" ]; then
        refuse "a hub is already listening on port $d (pid $a, user $b)" "hub_port"
        hub_explained=$hub_port
      fi
      if [ -n "$state_dir" ] && [ "$c" = "$state_dir" ]; then
        refuse "state directory $c is already used by a hub (pid $a)" "state_dir"
      fi
      ;;
    body)
      is_ours_pid "$a" && continue
      if [ -n "$session_names" ] && in_words "$d" "$session_names"; then
        refuse "a body for session $d is already running (pid $a, user $b)" \
          "the [[session]] name (and its body's state_dir)"
      fi
      if [ -n "$state_dir" ] && [ "$c" = "$state_dir" ]; then
        refuse "state directory $c is already used by a body (pid $a)" "state_dir"
      fi
      ;;
    opencode | herdr)
      is_ours_pid "$a" && continue
      if [ -n "$state_dir" ] && [ "$c" = "$state_dir" ]; then
        refuse "state directory $c is already used by $kind (pid $a)" "state_dir"
      fi
      ;;
    herdr-session)
      in_words "$a" "$ours_sessions" && continue
      if [ -n "$herdr_session" ] && [ "$a" = "$herdr_session" ]; then
        refuse "a herdr session named $a already exists" "herdr_session"
      fi
      ;;
  esac
done <"$inv"

# Ports: a busy port that is not ours is a collision, whatever holds it.
while IFS="$T" read -r kind a b c; do
  [ "$kind" = port ] || continue
  [ "$b" != "-" ] && is_ours_pid "$b" && continue
  case "$refused_ports" in *" $a "*) continue ;; esac
  if [ -n "$hub_port" ] && [ "$a" = "$hub_port" ] && [ "$hub_explained" != "$a" ]; then
    refuse "port $a is in use by $c (pid $b)" "hub_port"
    refused_ports="$refused_ports$a "
  elif [ -n "$serve_port" ] && [ "$a" = "$serve_port" ]; then
    refuse "port $a is in use by $c (pid $b)" "serve_https_port"
    refused_ports="$refused_ports$a "
  elif [ -n "$backend_ports" ] && in_words "$a" "$backend_ports"; then
    refuse "port $a is in use by $c (pid $b)" \
      "backend_port_base (or that session's backend_port)"
    refused_ports="$refused_ports$a "
  fi
done <"$inv"

# A tailscale serve already on the planned https port (it may not show as a listener).
while IFS="$T" read -r kind a b; do
  [ "$kind" = serve ] || continue
  case "$refused_ports" in *" $a "*) continue ;; esac
  if [ -n "$serve_port" ] && [ "$a" = "$serve_port" ]; then
    refuse "tailscale serve already uses https port $a" "serve_https_port"
    refused_ports="$refused_ports$a "
  fi
done <"$inv"

echo "-- this run will create here:"
echo "  hub port ${hub_port:--}, serve https port ${serve_port:--}, state dir ${state_dir:-(default)}," \
  "herdr session ${herdr_session:-(none)}"
echo "  backend ports: ${backend_ports:--}; bodies for sessions: ${session_names:--}"

if [ "$refusals" -gt 0 ]; then
  printf '%s' "$REFUSALS"
  echo "REFUSED: $refusals collision(s) on $host; nothing was started. Fix the config and re-run."
  exit 1
fi
echo "OK: no collision on $host."
exit 0
