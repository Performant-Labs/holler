#!/usr/bin/env bash
# inventory.sh - READ-ONLY inventory of what already runs on THIS host (setup wizard, #728).
#
# It never writes, signals, starts or stops anything: it only runs `ss`/`lsof`, `ps`,
# `tailscale serve status` and `herdr session list`, and prints tab-separated lines:
#   port<TAB><port><TAB><pid|-><TAB><process name|->
#   hub<TAB><pid><TAB><user><TAB><state dir|-><TAB><port|-><TAB><started><TAB><cmd>
#   body<TAB><pid><TAB><user><TAB><state dir|-><TAB><session|-><TAB><started><TAB><cmd>
#   opencode<TAB><pid><TAB><user><TAB><state dir|-><TAB><port|-><TAB><started><TAB><cmd>
#   herdr<TAB><pid><TAB><user><TAB><state dir|-><TAB><session|-><TAB><started><TAB><cmd>
#   serve<TAB><https port><TAB><text of the tailscale serve line>
#   herdr-session<TAB><name>
# `started` is the `ps` lstart text and `cmd` the command, the same values the wizard ledger
# records, so collide.sh can tell wizard-created processes from foreign ones.
#
# To run it on a remote host, pipe it over ssh: `ssh <remote-host> bash -s < inventory.sh`.
# WIZARD_INVENTORY_FIXTURE=<file> replays a fixture instead (for tests).
# bash 3.2 compatible: no associative arrays, no GNU-only flags; Linux and macOS.

if [ -n "${WIZARD_INVENTORY_FIXTURE:-}" ]; then
  cat "$WIZARD_INVENTORY_FIXTURE"
  exit $?
fi

T=$(printf '\t')

# --- listening TCP ports ------------------------------------------------------------------
ports=$(ss -ltnpH 2>/dev/null)
if [ -z "$ports" ]; then
  ports=""
  lsof_out=$(lsof -nP -iTCP -sTCP:LISTEN 2>/dev/null)
  if [ -n "$lsof_out" ]; then
    echo "$lsof_out" | while read -r name pid _user _fd _type _dev _size _node addr _rest; do
      [ "$name" = "COMMAND" ] && continue
      port=${addr##*:}
      case "$port" in '' | *[!0-9]*) continue ;; esac
      printf 'port\t%s\t%s\t%s\n' "$port" "$pid" "$name"
    done
  fi
else
  echo "$ports" | while read -r _state _rq _sq local _peer users; do
    port=${local##*:}
    case "$port" in '' | *[!0-9]*) continue ;; esac
    pid=-
    name=-
    case "$users" in
      *'(("'*)
        name=${users#*'(("'}
        name=${name%%'"'*}
        pid=${users#*pid=}
        pid=${pid%%[!0-9]*}
        [ -n "$pid" ] || pid=-
        ;;
    esac
    printf 'port\t%s\t%s\t%s\n' "$port" "$pid" "$name"
  done
fi

# --- holler hub / holler body / opencode / herdr processes ---------------------------------
# The state directory is only visible where /proc exists (Linux); elsewhere it is "-".
state_dir_of() {
  local v
  if [ -r "/proc/$1/environ" ]; then
    v=$(tr '\0' '\n' <"/proc/$1/environ" 2>/dev/null | sed -n 's/^HOLLER_STATE_DIR=//p' | head -n 1)
    [ -n "$v" ] && { printf '%s' "$v"; return; }
  fi
  printf -- '-'
}

# value of `--flag value` or `--flag=value` in a command line, "-" when absent
flag_of() {
  local cmd="$1" flag="$2" prev="" w
  for w in $cmd; do
    case "$w" in
      "$flag="*) printf '%s' "${w#*=}"; return ;;
    esac
    if [ "$prev" = "$flag" ]; then printf '%s' "$w"; return; fi
    prev=$w
  done
  printf -- '-'
}

LC_ALL=C ps -eo pid=,user=,lstart=,command= 2>/dev/null |
  while read -r pid user d1 d2 d3 d4 d5 cmd; do
    [ -n "$cmd" ] || continue
    started="$d1 $d2 $d3 $d4 $d5"
    first=${cmd%% *}
    rest=${cmd#"$first"}
    rest=${rest# }
    sub=${rest%% *}
    exe=${first##*/}
    kind=""
    case "$exe" in
      holler)
        case "$sub" in hub) kind=hub ;; body) kind=body ;; esac ;;
      opencode) kind=opencode ;;
      herdr) kind=herdr ;;
    esac
    [ -n "$kind" ] || continue
    sd=$(state_dir_of "$pid")
    case "$kind" in
      hub)
        listen=$(flag_of "$cmd" --listen)
        key=${listen##*:}
        ;;
      body | herdr) key=$(flag_of "$cmd" --session) ;;
      opencode) key=$(flag_of "$cmd" --port) ;;
    esac
    case "$key" in '' | "$listen") key=- ;; esac
    printf '%s\t%s\t%s\t%s\t%s\t%s\t%s\n' "$kind" "$pid" "$user" "$sd" "$key" "$started" "$cmd"
  done

# --- tailscale serve -----------------------------------------------------------------------
tailscale serve status 2>/dev/null | while IFS= read -r line; do
  case "$line" in
    https://*)
      host=${line%% *}
      port=443
      case "${host#https://}" in *:[0-9]*) port=${host##*:} ;; esac
      printf 'serve\t%s\t%s\n' "$port" "$line"
      ;;
  esac
done

# --- herdr sessions ------------------------------------------------------------------------
herdr session list 2>/dev/null | while read -r name _rest; do
  [ -n "$name" ] && printf 'herdr-session\t%s\n' "$name"
done
exit 0
