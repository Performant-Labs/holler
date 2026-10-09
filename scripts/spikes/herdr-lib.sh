# shellcheck shell=bash
# Shared harness for the Herdr spike scripts (issue #636). Source it; do not run it.
#
# Every Herdr call made through this file goes to ONE scratch session that this file creates
# and later stops. Two independent guards keep a real Herdr session out of reach:
#   1. Herdr runs with HOME and every XDG_* directory pointed into a fresh `mktemp -d` root,
#      and with every inherited HERDR_* variable removed, so even Herdr's *default* session
#      resolves to a socket inside the scratch root (a path no real server listens on).
#   2. Every call also names the scratch session explicitly (`--session spike636-<random>`).
# spike_start refuses to continue unless the running server reports that session and a socket
# inside the scratch root. spike_stop (an EXIT trap) stops only the server PID this file
# started, kills only the private tmux server it created, and removes the scratch root.
#
# Requirements: herdr, jq, python3 (for the raw socket client). tmux is optional (TUI checks).
set -euo pipefail

for _tool in herdr jq python3; do
  command -v "$_tool" >/dev/null || { echo "spike: $_tool is required" >&2; exit 2; }
done

# Unix socket paths are limited to ~108 bytes; keep the root short.
_tmp="${TMPDIR:-/tmp}"
[ "${#_tmp}" -le 40 ] || _tmp=/tmp
SPIKE_ROOT="$(mktemp -d "$_tmp/h636.XXXXXX")"
SPIKE_SESSION="spike636-$(od -An -N4 -tx1 /dev/urandom | tr -d ' \n')"
SPIKE_TMUX="$SPIKE_SESSION"            # private tmux server socket name (tmux -L), never the default
SPIKE_HOME="$SPIKE_ROOT/home"
SPIKE_WORK="$SPIKE_ROOT/work"          # cwd for every scratch pane
SPIKE_BIN="$SPIKE_ROOT/bin"
SPIKE_SERVER_PID=""
SPIKE_SOCKET=""
export SPIKE_ROOT SPIKE_SESSION
mkdir -p "$SPIKE_HOME/.config/herdr" "$SPIKE_HOME/.local/state" "$SPIKE_HOME/.local/share" \
  "$SPIKE_HOME/.cache" "$SPIKE_WORK" "$SPIKE_BIN" "$SPIKE_ROOT/run"
chmod 700 "$SPIKE_ROOT/run"

# Scratch config: no update or manifest checks (no network), /bin/sh panes with a neutral prompt,
# and sidebar rows that render custom metadata tokens (used by herdr-plugin.sh).
cat >"$SPIKE_HOME/.config/herdr/config.toml" <<'EOF'
onboarding = false
[terminal]
default_shell = "/bin/sh"
[update]
version_check = false
manifest_check = false
[ui.sidebar.agents]
rows = [["state_icon", "workspace", "pane"], ["$pos", "$shown"]]
[ui.sidebar.spaces]
rows = [["state_icon", "workspace"], ["$profile"]]
EOF

# The isolated herdr wrapper. tmux runs it too, so it must be a file, not a function.
cat >"$SPIKE_BIN/hx" <<EOF
#!/bin/sh
exec env -u HERDR_SOCKET_PATH -u HERDR_ENV -u HERDR_PANE_ID -u HERDR_TAB_ID -u HERDR_WORKSPACE_ID \\
  -u HERDR_CONFIG_PATH -u HERDR_HOME -u HERDR_SESSION -u TMUX \\
  HOME="$SPIKE_HOME" XDG_CONFIG_HOME="$SPIKE_HOME/.config" XDG_STATE_HOME="$SPIKE_HOME/.local/state" \\
  XDG_DATA_HOME="$SPIKE_HOME/.local/share" XDG_CACHE_HOME="$SPIKE_HOME/.cache" \\
  XDG_RUNTIME_DIR="$SPIKE_ROOT/run" SHELL=/bin/sh PS1='\$ ' \\
  herdr "\$@"
EOF
cat >"$SPIKE_BIN/h" <<EOF
#!/bin/sh
exec "$SPIKE_BIN/hx" --session "$SPIKE_SESSION" "\$@"
EOF
# Raw socket client: newline-delimited JSON, one request per connection.
# hsock.py SOCKET REQUEST [STREAM_SECONDS]: without STREAM_SECONDS it prints the first line;
# with it, it prints every line received in that many seconds (for events.subscribe).
cat >"$SPIKE_BIN/hsock.py" <<'EOF'
import socket, sys
path, req = sys.argv[1], sys.argv[2]
stream = float(sys.argv[3]) if len(sys.argv) > 3 else 0.0
s = socket.socket(socket.AF_UNIX, socket.SOCK_STREAM)
s.settimeout(stream if stream else 10)
s.connect(path)
s.sendall(req.encode() + b"\n")
buf = b""
try:
    while True:
        chunk = s.recv(65536)
        if not chunk:
            break
        buf += chunk
        if not stream and b"\n" in buf:
            buf = buf.split(b"\n", 1)[0] + b"\n"
            break
except socket.timeout:
    pass
sys.stdout.write(buf.decode())
EOF
chmod +x "$SPIKE_BIN/hx" "$SPIKE_BIN/h"

h() { "$SPIKE_BIN/h" "$@"; }
hsock() { python3 -I "$SPIKE_BIN/hsock.py" "$SPIKE_SOCKET" "$@"; }

# Replace machine-specific strings in anything printed, so output can be pasted into the note.
spike_redact() {
  sed -e "s#$SPIKE_ROOT#<scratch-root>#g" -e "s#$SPIKE_SESSION#<scratch-session>#g" \
    -e "s#$HOME#<home>#g" -e "s#$(id -un)#<user>#g" -e "s#$(hostname)#<host>#g"
}
say() { printf '\n### %s\n' "$*"; }
show() { printf '$ %s\n' "$*" | spike_redact; }

spike_wait_for() { # spike_wait_for SECONDS COMMAND... : poll until COMMAND succeeds
  local deadline=$((SECONDS + $1)); shift
  until "$@" >/dev/null 2>&1; do
    [ "$SECONDS" -lt "$deadline" ] || return 1
    sleep 0.2
  done
}

_session_running() {
  "$SPIKE_BIN/hx" session list 2>/dev/null | awk -v n="$SPIKE_SESSION" '$1==n && $2=="running"{f=1} END{exit !f}'
}

spike_start() {
  "$SPIKE_BIN/hx" --session "$SPIKE_SESSION" server >"$SPIKE_ROOT/server.log" 2>&1 &
  SPIKE_SERVER_PID=$!
  spike_wait_for 15 _session_running || { echo "spike: scratch server did not start" >&2; exit 1; }
  SPIKE_SOCKET="$("$SPIKE_BIN/hx" session list | awk -v n="$SPIKE_SESSION" '$1==n{print $4}')"
  spike_prove_target
}

spike_prove_target() {
  local st sess sock
  st="$(h status server --json)"
  sess="$(jq -r .session <<<"$st")"
  sock="$(jq -r .socket <<<"$st")"
  case "$sock" in "$SPIKE_ROOT"/*) ;; *) echo "spike: REFUSING, socket is outside the scratch root" >&2; exit 1 ;; esac
  [ "$sess" = "$SPIKE_SESSION" ] || { echo "spike: REFUSING, server reports another session" >&2; exit 1; }
  [ "$sock" = "$SPIKE_SOCKET" ] || { echo "spike: REFUSING, socket mismatch" >&2; exit 1; }
  case "$SPIKE_SESSION" in spike636-*) ;; *) echo "spike: REFUSING, not a spike session name" >&2; exit 1 ;; esac
  echo "target proven: session=$sess socket=$sock server_pid=$SPIKE_SERVER_PID" | spike_redact
}

spike_server_stop() { # stop only the server this file started, and wait for it to exit
  [ -n "$SPIKE_SERVER_PID" ] || return 0
  if kill -0 "$SPIKE_SERVER_PID" 2>/dev/null; then
    h server stop >/dev/null 2>&1 || true
    for _ in $(seq 1 50); do kill -0 "$SPIKE_SERVER_PID" 2>/dev/null || break; sleep 0.2; done
    kill -0 "$SPIKE_SERVER_PID" 2>/dev/null && kill "$SPIKE_SERVER_PID" 2>/dev/null || true
    wait "$SPIKE_SERVER_PID" 2>/dev/null || true
  fi
  SPIKE_SERVER_PID=""
}

spike_stop() {
  set +e
  if command -v tmux >/dev/null; then
    tmux -L "$SPIKE_TMUX" kill-server >/dev/null 2>&1
    # tmux can leave its socket file behind; remove only this private one, and only once its server is gone.
    local tsock
    tsock="${TMUX_TMPDIR:-/tmp}/tmux-$(id -u)/$SPIKE_TMUX"
    if [ -S "$tsock" ] && ! tmux -L "$SPIKE_TMUX" list-sessions >/dev/null 2>&1; then rm -f "$tsock"; fi
  fi
  spike_server_stop
  local stray
  stray="$(pgrep -f "$SPIKE_ROOT" || true)"
  if [ -n "$stray" ]; then echo "spike: WARNING, processes still reference the scratch root: $stray" >&2; fi
  case "$SPIKE_ROOT" in */h636.*) rm -rf "$SPIKE_ROOT" ;; esac
  echo "cleanup: scratch server stopped, private tmux server killed, scratch root removed"
}
trap spike_stop EXIT
