# Sourced by every fake. FAKE_ROOT is the scenario's private registry directory; nothing here
# touches a real process, port or session. Registry files: pids, ports ("port pid name"),
# sessions ("name pid"), serve (tailscale serve lines), calls (one line per holler call).
alive() {
  local st
  st="$(ps -o stat= -p "$1" 2>/dev/null)"
  st="${st// /}"
  case "$st" in '' | Z*) return 1 ;; esac
  return 0
}
register_pid() { echo "$1" >>"$FAKE_ROOT/pids"; }
register_port() { echo "$1 $2 $3" >>"$FAKE_ROOT/ports"; }
# flag_value <--flag> <args...>: the word after the flag
flag_value() {
  local want="$1" w prev=""
  shift
  for w in "$@"; do
    if [ "$prev" = "$want" ]; then
      echo "$w"
      return 0
    fi
    prev="$w"
  done
  return 1
}
