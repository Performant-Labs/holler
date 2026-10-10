#!/usr/bin/env bash
# inventory.sh - READ-ONLY inventory of what already runs on THIS host (setup wizard, #728).
#
# It never writes, signals, starts or stops anything: it only runs `ss`/`lsof`, `ps`,
# `tailscale serve status` and `herdr session list`, and prints tab-separated lines:
#   home<TAB><this host's $HOME>      (always the first line)
#   port<TAB><port><TAB><pid|-><TAB><process name|->
#   hub<TAB><pid><TAB><user><TAB><state dir|-><TAB><port|-><TAB><started><TAB><cmd>
#   body<TAB><pid><TAB><user><TAB><state dir|-><TAB><--config path|-><TAB><started><TAB><cmd>
#   opencode<TAB><pid><TAB><user><TAB><state dir|-><TAB><port|-><TAB><started><TAB><cmd>
#   herdr<TAB><pid><TAB><user><TAB><state dir|-><TAB><session|-><TAB><started><TAB><cmd>
#   serve<TAB><https port><TAB><text of the tailscale serve line><TAB><proxy targets|->
#   herdr-session<TAB><name>          (only with WIZARD_INVENTORY_HERDR=1)
#   warn<TAB>missing tool <name>      (a tool this script needs and cannot find on PATH)
# `started` is the `LC_ALL=C ps -o lstart=` text exactly as ps prints it (the day of month is
# padded with a space on days 1 to 9) and `cmd` the command, the same values the wizard ledger
# records, so collide.sh can tell wizard-created processes from foreign ones. A body's session
# is not on its command line, so a body row names its `--config` path instead.
#
# The Herdr section (the `herdr session list` call and its `missing tool herdr` warning) runs
# only when WIZARD_INVENTORY_HERDR=1, which the skill sets on the Herdr host only. HERDR_BIN
# names the herdr binary when it is not on PATH.
#
# To run it on a remote host, pipe it over ssh: `ssh <remote-host> bash -s < inventory.sh`.
# WIZARD_INVENTORY_FIXTURE=<file> replays a fixture instead (for tests).
# bash 3.2 compatible: no associative arrays, no GNU-only flags; Linux and macOS.

if [ -n "${WIZARD_INVENTORY_FIXTURE:-}" ]; then
  cat "$WIZARD_INVENTORY_FIXTURE"
  exit $?
fi

T=$(printf '\t')

# The first line: this host's home, so a `-` or `~/` state directory can be resolved against it.
printf 'home\t%s\n' "${HOME:-}"

# --- tools this script needs ---------------------------------------------------------------
have() { command -v "$1" >/dev/null 2>&1; }
have_ss=0; have_lsof=0; have_ps=0; have_tailscale=0; have_herdr=0
have ss && have_ss=1
have lsof && have_lsof=1
have ps && have_ps=1
have tailscale && have_tailscale=1
herdr_bin=${HERDR_BIN:-herdr}
inventory_herdr=0
[ "${WIZARD_INVENTORY_HERDR:-}" = 1 ] && inventory_herdr=1
have "$herdr_bin" && have_herdr=1
[ "$have_ss" = 1 ] || [ "$have_lsof" = 1 ] || printf 'warn\tmissing tool ss\n'
[ "$have_ps" = 1 ] || printf 'warn\tmissing tool ps\n'
[ "$have_tailscale" = 1 ] || printf 'warn\tmissing tool tailscale\n'
[ "$inventory_herdr" = 0 ] || [ "$have_herdr" = 1 ] || printf 'warn\tmissing tool herdr\n'

# --- listening TCP ports ------------------------------------------------------------------
ports=""
[ "$have_ss" = 1 ] && ports=$(ss -ltnpH 2>/dev/null)
if [ -z "$ports" ]; then
  ports=""
  lsof_out=""
  [ "$have_lsof" = 1 ] && lsof_out=$(lsof -nP -iTCP -sTCP:LISTEN 2>/dev/null)
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

# `read` keeps the inner spacing of the last field, and lstart is 24 characters in the C locale
# ("Tue Oct  6 22:12:17 2026"), so started and cmd are cut by position, not split on blanks.
[ "$have_ps" = 1 ] && LC_ALL=C ps -ww -eo pid=,user=,lstart=,command= 2>/dev/null |
  while read -r pid user rest; do
    started=${rest:0:24}
    cmd=${rest:24}
    cmd=${cmd#"${cmd%%[![:space:]]*}"} # macOS pads the lstart column with extra blanks
    [ -n "$cmd" ] || continue
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
      body) key=$(flag_of "$cmd" --config) ;;
      herdr) key=$(flag_of "$cmd" --session) ;;
      opencode) key=$(flag_of "$cmd" --port) ;;
    esac
    case "$key" in '' | "$listen") key=- ;; esac
    printf '%s\t%s\t%s\t%s\t%s\t%s\t%s\n' "$kind" "$pid" "$user" "$sd" "$key" "$started" "$cmd"
  done

# --- tailscale serve -----------------------------------------------------------------------
# One row per https entry; its proxy targets (the `|-- / proxy <url>` lines under it) are the
# last field, space-separated, or "-".
if [ "$have_tailscale" = 1 ]; then
  tailscale serve status 2>/dev/null | {
    sp="" sl="" st=""
    flush_serve() {
      [ -n "$sp" ] && printf 'serve\t%s\t%s\t%s\n' "$sp" "$sl" "${st:--}"
      sp="" sl="" st=""
    }
    while IFS= read -r line; do
      case "$line" in
        https://*)
          flush_serve
          host=${line%% *}
          sp=443
          case "${host#https://}" in *:[0-9]*) sp=${host##*:} ;; esac
          sl=$line
          ;;
        *proxy\ *)
          [ -n "$sp" ] && st="${st:+$st }${line##*proxy }"
          ;;
      esac
    done
    flush_serve
  }
fi

# --- herdr sessions ------------------------------------------------------------------------
if [ "$inventory_herdr" = 1 ] && [ "$have_herdr" = 1 ]; then
  "$herdr_bin" session list 2>/dev/null | while read -r name _rest; do
    [ -n "$name" ] && printf 'herdr-session\t%s\n' "$name"
  done
fi
exit 0
