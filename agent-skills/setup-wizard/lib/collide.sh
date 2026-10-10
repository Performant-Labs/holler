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
#   WIZARD_STATE_DIR            state_dir (empty = Holler's default; not compared). A state
#                               directory shown as `-` on a hub or body row is the default one
#                               ($HOME/.holler) and is compared as such
#   WIZARD_HERDR_SESSION        herdr_session (the hub host only)
#   WIZARD_BACKEND_PORTS        space-separated backend ports planned on this host
#   WIZARD_SESSION_NAMES        space-separated session names whose bodies run on this host
#                               (listed in the plan only: a body's session is not on its command
#                               line, so there is no per-session body check)
#   WIZARD_LEDGER               optional: this instance's wizard-ledger.toml on this host
#
# A process is "created by this instance" only if the ledger holds its pid with the same
# `started` and `cmd` (a live entry, per the epic's contract); every other process is foreign.
# A planned `tailscale serve` is the instance's own (never refused) when its target is
# http://127.0.0.1:<hub_port>, its port is WIZARD_SERVE_HTTPS_PORT and no live foreign process
# holds hub_port: "present, not touched" when the ledger has a live hub row, "this instance's
# port pair, no live hub" when it has not (a crash, reboot or teardown left the entry).
# `-` and `~/` state directories are resolved against the inventory's `home` line (the remote
# host's home), else this machine's $HOME. For the default instance, a running unnamed Herdr
# server that no live ledger row owns is foreign and refused (the operator may override that in
# Stage 3 by answering yes to "build in it"); for a named instance it is not.
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

# The default state directory, and a state directory in comparable form: `-` and `~/...` are
# resolved, a trailing slash dropped.
T_HOME=$(awk -F"$T" '$1 == "home" { print $2; exit }' "$inv")
home=${T_HOME:-${HOME:-~}}
default_sd="$home/.holler"
norm_sd() {
  local v="$1"
  [ "$v" = "-" ] && v=$default_sd
  case "$v" in "~/"*) v="$home/${v#"~/"}" ;; esac
  while [ "${#v}" -gt 1 ]; do
    case "$v" in */) v=${v%/} ;; *) break ;; esac
  done
  printf '%s' "$v"
}
plan_sd=""
[ -n "$state_dir" ] && plan_sd=$(norm_sd "$state_dir")
same_sd() { [ -n "$plan_sd" ] && [ "$(norm_sd "$1")" = "$plan_sd" ]; }

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
own_hub=0
while IFS="$T" read -r kind pid user sd key started cmd; do
  case "$kind" in
    hub | body | opencode | herdr)
      if ours "$pid" "$started" "$cmd"; then
        ours_pids="$ours_pids$pid "
        [ "$kind" = hub ] && own_hub=1
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

# own_target <target words>: 0 when a serve entry proxies to this instance's own hub port
own_target() {
  local w
  [ -n "$hub_port" ] || return 1
  for w in $1; do
    case "$w" in "http://127.0.0.1:$hub_port" | "http://127.0.0.1:$hub_port/"*) return 0 ;; esac
  done
  return 1
}

# foreign_hub_port: 0 when a live process that is not ours holds hub_port
foreign_hub_port() {
  local kind a b c d rest
  [ -n "$hub_port" ] || return 1
  while IFS="$T" read -r kind a b c d rest; do
    case "$kind" in
      port)
        if [ "$a" = "$hub_port" ] && { [ "$b" = "-" ] || ! is_ours_pid "$b"; }; then return 0; fi ;;
      hub)
        if [ "$d" = "$hub_port" ] && ! is_ours_pid "$a"; then return 0; fi ;;
    esac
  done <"$inv"
  return 1
}

# The planned serve port is the instance's own when a serve entry there proxies to its hub and
# no foreign process holds hub_port.
own_serve_port=""
if [ -n "$serve_port" ] && ! foreign_hub_port; then
  while IFS="$T" read -r kind a b c; do
    [ "$kind" = serve ] || continue
    if [ "$a" = "$serve_port" ] && own_target "$c"; then own_serve_port=$a; fi
  done <"$inv"
fi

echo "== collision preflight on $host (instance ${WIZARD_INSTANCE_NAME:-default}) =="
echo "-- already running here:"
anything=0
while IFS="$T" read -r kind a b c d e f; do
  [ "$kind" = home ] || anything=1
  case "$kind" in
    warn)
      echo "  warning: $a (the inventory is incomplete)" ;;
    port)
      if [ "$b" != "-" ] && is_ours_pid "$b"; then st="created by this instance"
      else st="present, not touched"; fi
      echo "  $st: port $a (${c}, pid $b)" ;;
    hub | body | opencode | herdr)
      if is_ours_pid "$a"; then st="created by this instance"
      else st="present, not touched"; fi
      what="$kind pid $a user $b"
      [ "$kind" = body ] && what="body with config $d (pid $a, user $b)"
      [ "$kind" = hub ] && what="hub on port $d (pid $a, user $b)"
      [ "$kind" = opencode ] && what="opencode backend on port $d (pid $a, user $b)"
      [ "$c" != "-" ] && what="$what, state dir $c"
      echo "  $st: $what" ;;
    serve)
      st="present, not touched"
      if [ "$own_serve_port" = "$a" ] && [ "$own_hub" = 0 ]; then
        st="this instance's port pair, no live hub"
      fi
      echo "  $st: tailscale serve on https port $a" ;;
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
      if same_sd "$c"; then
        refuse "state directory $c is already used by a hub (pid $a)" "state_dir"
      fi
      ;;
    body)
      is_ours_pid "$a" && continue
      if same_sd "$c"; then
        refuse "state directory $c is already used by a body (pid $a)" "state_dir"
      fi
      ;;
    opencode | herdr)
      is_ours_pid "$a" && continue
      if [ "$kind" = herdr ] && [ "$d" = "-" ] && [ "${WIZARD_INSTANCE_NAME:-}" = default ]; then
        case " $f " in
          *" server "*)
            refuse "an unnamed herdr server is already running (pid $a, user $b)" \
              'herdr_session (or answer yes to "build in it" in Stage 3)' ;;
        esac
      fi
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
  elif [ -n "$serve_port" ] && [ "$a" = "$serve_port" ] && [ "$own_serve_port" != "$a" ]; then
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
  [ "$own_serve_port" = "$a" ] && continue
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
