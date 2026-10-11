#!/usr/bin/env bash
# Shared helpers for the Claude Code pane spike probes (issue #830). Sourced, not run.
#
# Unlike the OpenCode spike, these probes cannot give the process a fresh HOME: the
# subscription login lives under the config directory in HOME, and a scratch HOME has no
# login. The login is never read, copied, printed or moved by anything here. So the safety
# contract is different, and these are the parts that hold it:
#   - every Claude process runs under `env -i` with only HOME, PATH, TERM and LANG (plus
#     the env var NAMES a probe adds on purpose). Nothing from the caller's environment
#     reaches it: no CLAUDE_CODE_* variable, no token, no API key. A probe that wants to
#     show what a variable does sets that one variable by name and says so;
#   - the working directory is a fresh `mktemp -d` directory, so a session's transcript
#     lands in a project directory of its own under the config directory, and
#     cl_cleanup removes exactly that project with `claude purge <scratch dir>`;
#   - `--setting-sources project,local` keeps the operator's user settings (hooks, plugins,
#     MCP servers, permission mode) out of every scratch session. A probe's own hooks and
#     MCP config are passed with --settings / --mcp-config from the scratch directory;
#   - every listing of running sessions is filtered with `--cwd <scratch dir>`, so a probe
#     never reads, counts or prints anybody else's session;
#   - the TUI runs on a private tmux socket (`tmux -S`), never the default tmux server, and
#     TMUX / TMUX_PANE are not passed on;
#   - nothing here touches a Herdr session, a pane, a background-session supervisor that
#     is already running, or a session it did not create. cl_require_no_foreign_daemon
#     refuses to go on when a supervisor is already up (a --bg probe would add a job to it);
#   - model use is a few one-word turns on the smallest model (CL_MODEL, default haiku).
#
# Output is one line per observation: PASS / FAIL (an expectation the adapter would depend
# on) and INFO (a measured value or a behaviour worth recording). A probe exits 1 if any
# FAIL was printed. The login is checked with `claude auth status`, and only its
# `authMethod` field is ever looked at.

set -euo pipefail

CL_BIN="${CLAUDE_BIN:-$(command -v claude || true)}"
CL_MODEL="${CL_MODEL:-haiku}"
CL_BOOT_TIMEOUT="${CL_BOOT_TIMEOUT:-60}"
CL_TURN_TIMEOUT="${CL_TURN_TIMEOUT:-90}"
CL_FAILS=0
CL_EXTRA_PIDS=()
CL_DIR=""
CL_WORK=""
CL_SOCK_DIR=""
CL_TMUX_SOCK=""
CL_BG_IDS=()
CL_STARTED_DAEMON=0

pass() { echo "PASS  $*"; }
fail() { echo "FAIL  $*"; CL_FAILS=$((CL_FAILS + 1)); }
info() { echo "INFO  $*"; }
check() { # check "description" <command...>
  local what="$1"; shift
  if "$@"; then pass "$what"; else fail "$what"; fi
}

now_ms() { echo $(( $(date +%s%N) / 1000000 )); }

cl_require() {
  local t
  for t in jq tmux setsid timeout uuidgen; do
    command -v "$t" >/dev/null 2>&1 || { echo "missing tool: $t" >&2; exit 2; }
  done
  [ -n "$CL_BIN" ] && [ -x "$CL_BIN" ] || { echo "claude not found (set CLAUDE_BIN)" >&2; exit 2; }
}

# The environment of a scratch Claude process: an allowlist, then whatever the probe adds.
# Extra NAME=value pairs are passed as arguments before the command.
cl_env() {
  env -i HOME="$HOME" PATH="$PATH" TERM="${CL_TERM:-xterm-256color}" LANG=C.UTF-8 "$@"
}

cl_tmux() { cl_env tmux -S "$CL_TMUX_SOCK" -f /dev/null "$@"; }

# Scratch directory, private tmux socket, login check, cleanup trap.
cl_init() {
  cl_require
  CL_DIR="$(mktemp -d "${TMPDIR:-/tmp}/cl-spike.XXXXXX")"
  CL_DIR="$(cd "$CL_DIR" && pwd -P)"
  CL_WORK="$CL_DIR/work"
  mkdir -p "$CL_WORK" "$CL_DIR/logs"
  # A unix socket path is limited to about 100 bytes; keep tmux's short.
  if [ "${#CL_DIR}" -lt 80 ]; then CL_SOCK_DIR="$CL_DIR"; else CL_SOCK_DIR="$(mktemp -d /tmp/clspk.XXXXXX)"; fi
  CL_TMUX_SOCK="$CL_SOCK_DIR/tmux.sock"
  trap cl_cleanup EXIT
  trap 'exit 130' INT TERM
  info "claude version: $(cl_env "$CL_BIN" --version 2>/dev/null)"
  info "scratch dir: <scratch-dir> (mktemp -d; its project is purged on exit)"
  local method
  method=$(cl_env "$CL_BIN" auth status 2>/dev/null | jq -r '.authMethod // "unknown"' 2>/dev/null || echo unknown)
  info "login method under the minimal environment: $method"
  [ "$method" = "claude.ai" ] || { echo "no subscription login under the minimal environment; stopping" >&2; exit 2; }
}

# A --bg probe adds a job to the background-session supervisor. Refuse when one is
# already running (it may host somebody's sessions); if none is, the one the probe starts
# is the probe's own and cl_cleanup stops it.
cl_require_no_foreign_daemon() {
  if cl_env "$CL_BIN" daemon status >/dev/null 2>&1; then
    echo "a background-session supervisor is already running; refusing to add a scratch job to it" >&2
    exit 2
  fi
  CL_STARTED_DAEMON=1
}

cl_cleanup() {
  local rc=$? id p
  set +e
  for id in "${CL_BG_IDS[@]:-}"; do
    [ -n "$id" ] || continue
    cl_env "$CL_BIN" stop "$id" >/dev/null 2>&1
    cl_env "$CL_BIN" rm "$id" >/dev/null 2>&1
  done
  if [ "$CL_STARTED_DAEMON" = 1 ]; then
    # Only ours: cl_require_no_foreign_daemon saw none running before the probe began.
    cl_env "$CL_BIN" daemon stop --any >/dev/null 2>&1
  fi
  [ -n "$CL_TMUX_SOCK" ] && cl_tmux kill-server >/dev/null 2>&1
  for p in "${CL_EXTRA_PIDS[@]:-}"; do
    [ -n "$p" ] && kill "$p" >/dev/null 2>&1
  done
  sleep 0.3
  if [ -n "$CL_WORK" ] && [ -d "$CL_WORK" ]; then
    cl_env "$CL_BIN" purge "$CL_WORK" --yes >/dev/null 2>&1
  fi
  [ -n "$CL_SOCK_DIR" ] && [ "$CL_SOCK_DIR" != "$CL_DIR" ] && rm -rf "$CL_SOCK_DIR"
  [ -n "$CL_DIR" ] && rm -rf "$CL_DIR"
  if [ "$rc" -eq 0 ] && [ "$CL_FAILS" -gt 0 ]; then rc=1; fi
  exit "$rc"
}

# Start `claude ARGS` in a tmux session called NAME in the scratch directory.
# The command is exec'd, so the pane's pid is the Claude process.
cl_launch() { # name [claude args...]
  local name="$1"; shift
  cl_tmux new-session -d -s "$name" -x 160 -y 50 -c "$CL_WORK" -- \
    env -i HOME="$HOME" PATH="$PATH" TERM="${CL_TERM:-xterm-256color}" LANG=C.UTF-8 ${CL_LAUNCH_ENV:-} \
    "$CL_BIN" --setting-sources project,local --model "$CL_MODEL" "$@"
}

# Like cl_launch, but the pane stays after Claude exits and prints its exit code, so a
# refusal at startup can be read. The pane's pid is then the wrapper shell, not Claude.
cl_launch_hold() { # name [claude args...]
  local name="$1"; shift
  cl_tmux new-session -d -s "$name" -x 160 -y 50 -c "$CL_WORK" -- \
    bash -c 'env -i HOME="$1" PATH="$2" TERM=xterm-256color LANG=C.UTF-8 "$3" --setting-sources project,local --model "$4" "${@:5}"; echo "[exit $?]"; sleep 120' \
    bash "$HOME" "$PATH" "$CL_BIN" "$CL_MODEL" "$@"
}

# The non-blank lines of a pane, trailing spaces trimmed (first N, default 8).
cl_show() { # name [n]
  cl_pane "$1" | sed 's/[[:space:]]*$//' | { grep -v '^$' || true; } | head -"${2:-8}"
}

cl_pane() { cl_tmux capture-pane -p -t "$1" 2>/dev/null || true; }       # name
cl_pid() { cl_tmux display-message -p -t "$1" '#{pane_pid}' 2>/dev/null; } # name
cl_alive() { cl_tmux has-session -t "$1" 2>/dev/null; }                  # name

# A scratch-only stand-in for a person at a keyboard: answer a first-run dialog or compare
# the typed path with the hub-driven one. The adapter never does this (invariant I4).
cl_keys() { local n="$1"; shift; cl_tmux send-keys -t "$n" "$@"; }

# Wait until the pane text matches an extended regex; prints the milliseconds it took.
cl_wait_pane() { # name regex [timeout_s]
  local name="$1" re="$2" limit="${3:-$CL_BOOT_TIMEOUT}" t0
  t0=$(now_ms)
  while [ $(( ($(now_ms) - t0) / 1000 )) -lt "$limit" ]; do
    if cl_pane "$name" | grep -Eq -- "$re"; then echo $(( $(now_ms) - t0 )); return 0; fi
    sleep 0.3
  done
  return 1
}

# Running sessions started under the scratch directory, as the CLI reports them.
# When CL_TARGET names a tmux session, only that session's process is returned (several
# sessions share the scratch directory in some probes).
# shellcheck disable=SC2120 # callers may pass --all
cl_agents() { # [--all]
  local out pid
  out=$(cl_env "$CL_BIN" agents --json --cwd "$CL_WORK" "$@" 2>/dev/null || echo '[]')
  if [ -n "${CL_TARGET:-}" ]; then
    pid=$(cl_pid "$CL_TARGET" 2>/dev/null || true)
    printf '%s' "$out" | jq --argjson p "${pid:-0}" '[.[] | select(.pid == $p)]'
  else
    printf '%s' "$out"
  fi
}

# One field of the (single) running session under the scratch directory.
cl_agent_field() { # jq-expression
  cl_agents | jq -r "(.[0] // {}) | ($1) // \"-\"" 2>/dev/null
}

# Poll cl_agents until a jq predicate over the array holds; prints the milliseconds.
cl_wait_agents() { # jq-predicate [timeout_s]
  local pred="$1" limit="${2:-$CL_TURN_TIMEOUT}" t0
  t0=$(now_ms)
  while [ $(( ($(now_ms) - t0) / 1000 )) -lt "$limit" ]; do
    if cl_agents | jq -e "$pred" >/dev/null 2>&1; then echo $(( $(now_ms) - t0 )); return 0; fi
    sleep 0.5
  done
  return 1
}

# The project directory Claude Code derives from the scratch directory (documented rule:
# every non-alphanumeric character becomes "-"), and a session's transcript in it.
cl_project_dir() { echo "$HOME/.claude/projects/$(printf '%s' "$CL_WORK" | sed 's/[^A-Za-z0-9]/-/g')"; }
cl_transcript() { echo "$(cl_project_dir)/$1.jsonl"; } # session-id

# Dismiss the first-run dialogs of a fresh directory (workspace trust) so the prompt box
# shows. Prints the dialogs it saw, by their first line. Scratch-only (see cl_keys).
cl_pass_first_run() { # name
  local name="$1" t0 text
  t0=$(now_ms)
  while [ $(( ($(now_ms) - t0) / 1000 )) -lt "$CL_BOOT_TIMEOUT" ]; do
    text=$(cl_pane "$name")
    if printf '%s' "$text" | grep -Eqi 'trust (the files|this folder)|Yes, I trust'; then
      info "first-run dialog: workspace trust"
      cl_keys "$name" Enter
      sleep 1
      continue
    fi
    if printf '%s' "$text" | grep -Eq '^[[:space:]]*[│|>❯]'; then return 0; fi
    sleep 0.3
  done
  return 1
}

# Write one line to a unix socket (the inbox socket, the channel's control socket).
cl_unix_line() { # socket line
  python3 - "$1" "$2" <<'PY'
import socket, sys, time
s = socket.socket(socket.AF_UNIX)
s.settimeout(5)
s.connect(sys.argv[1])
s.sendall((sys.argv[2] + "\n").encode())
time.sleep(0.4)
PY
}

# A peer message in the shape the client documents in its own startup log:
# {"type":"user","message":{"role":"user","content":"..."}}.
cl_inbox_send() { # socket text
  cl_unix_line "$1" "$(jq -cn --arg t "$2" '{type:"user",message:{role:"user",content:$t}}')"
}

# Wait until a file contains a regex; prints the milliseconds it took.
cl_wait_file() { # file regex [timeout_s]
  local f="$1" re="$2" limit="${3:-$CL_TURN_TIMEOUT}" t0
  t0=$(now_ms)
  while [ $(( ($(now_ms) - t0) / 1000 )) -lt "$limit" ]; do
    if [ -f "$f" ] && grep -Eq -- "$re" "$f"; then echo $(( $(now_ms) - t0 )); return 0; fi
    sleep 0.25
  done
  return 1
}

# The inbox socket path of the process in a tmux pane, from its registry file.
cl_inbox_of() { # tmux-name
  local pid; pid=$(cl_pid "$1")
  jq -r .messagingSocketPath "$HOME/.claude/sessions/$pid.json"
}

# Like cl_launch_hold, but adds no flags of its own: some commands (resuming a running
# background session with a prompt) are refused when any session-configuring flag is present.
cl_launch_raw() { # name [claude args...]
  local name="$1"; shift
  cl_tmux new-session -d -s "$name" -x 160 -y 50 -c "$CL_WORK" -- \
    bash -c 'env -i HOME="$1" PATH="$2" TERM=xterm-256color LANG=C.UTF-8 "$3" "${@:4}"; echo "[exit $?]"; sleep 120' \
    bash "$HOME" "$PATH" "$CL_BIN" "$@"
}
