#!/usr/bin/env bash
# Shared helpers for the OpenCode pane spike probes (issue #635). Sourced, not run.
#
# Safety contract every probe inherits from here:
#   - every OpenCode process runs under `env -i` with HOME and XDG_* pointing into a
#     fresh `mktemp -d` directory, so the operator's real OpenCode config, auth and
#     session database are never read or written;
#   - the scratch config enables exactly one provider, "deadend", whose base URL is
#     http://127.0.0.1:9/v1 (a closed local port) and which has no key; the built-in free
#     "opencode" provider is disabled. oc_guard_no_model refuses to go on unless the
#     running server reports exactly that, so a model request (if anything ever made
#     one) fails locally: no model is reached and no credit is spent. A provider is
#     configured at all only because /session/:id/shell needs a model *name* to
#     attribute its message to; it never calls the model;
#   - ports are picked free from 48100-48199 on 127.0.0.1 only;
#   - the TUI runs on a private tmux socket (`tmux -S`), never the default tmux server;
#   - oc_cleanup (an EXIT trap) kills only the processes this run started (the server
#     process groups and their descendants, which OpenCode puts in sessions of their
#     own), kills the private tmux server and removes the temp directory.
#
# Output is one line per observation: PASS / FAIL (an expectation the adapter would
# depend on) and INFO (a measured value or a behaviour worth recording). A probe exits
# 1 if any FAIL was printed, so the same scripts can become #642's contract tests.

set -euo pipefail

OC_BIN="${OPENCODE_BIN:-$(command -v opencode || true)}"
OC_HTTP_TIMEOUT="${OC_HTTP_TIMEOUT:-10}"
OC_BOOT_TIMEOUT="${OC_BOOT_TIMEOUT:-240}"
OC_FAILS=0
OC_PGIDS=()
OC_EXTRA_PIDS=() # helper clients started in the background (killed by oc_cleanup)
OC_PORTS_TAKEN=" "
OC_TMUX_SOCK=""
OC_SOCK_DIR=""
OC_DIR=""

pass() { echo "PASS  $*"; }
fail() { echo "FAIL  $*"; OC_FAILS=$((OC_FAILS + 1)); }
info() { echo "INFO  $*"; }
check() { # check "description" <command...>
  local what="$1"; shift
  if "$@"; then pass "$what"; else fail "$what"; fi
}

quiet() { "$@" >/dev/null; }

now_ms() { echo $(( $(date +%s%N) / 1000000 )); }

oc_require() {
  local t
  for t in curl jq tmux setsid timeout; do
    command -v "$t" >/dev/null 2>&1 || { echo "missing tool: $t" >&2; exit 2; }
  done
  [ -n "$OC_BIN" ] && [ -x "$OC_BIN" ] || { echo "opencode not found (set OPENCODE_BIN)" >&2; exit 2; }
}

# Fresh scratch home, config with no provider, and the cleanup trap.
oc_init() {
  oc_require
  OC_DIR="$(mktemp -d "${TMPDIR:-/tmp}/oc-spike.XXXXXX")"
  mkdir -p "$OC_DIR"/{home,data,config/opencode,state,cache,proj,logs}
  # One provider, pointed at a closed local port, no key (see the header).
  if (exec 3<>/dev/tcp/127.0.0.1/9) 2>/dev/null; then
    echo "127.0.0.1:9 accepts connections; the dead-end provider would not be dead. Refusing." >&2
    exit 2
  fi
  cat >"$OC_DIR/config/opencode/opencode.json" <<'JSON'
{
  "$schema": "https://opencode.ai/config.json",
  "enabled_providers": ["deadend"],
  "disabled_providers": ["opencode"],
  "model": "deadend/none",
  "small_model": "deadend/none",
  "autoupdate": false,
  "share": "disabled",
  "provider": {
    "deadend": {
      "npm": "@ai-sdk/openai-compatible",
      "name": "dead end (closed local port)",
      "options": { "baseURL": "http://127.0.0.1:9/v1" },
      "models": { "none": { "name": "none" } }
    }
  }
}
JSON
  # A unix socket path is limited to about 100 bytes; keep tmux's short.
  if [ "${#OC_DIR}" -lt 80 ]; then OC_SOCK_DIR="$OC_DIR"; else OC_SOCK_DIR="$(mktemp -d /tmp/ocspk.XXXXXX)"; fi
  OC_TMUX_SOCK="$OC_SOCK_DIR/tmux.sock"
  trap oc_cleanup EXIT
  trap 'exit 130' INT TERM
  info "opencode version: $(oc_env "$OC_BIN" --version 2>/dev/null)"
  info "scratch dir: <scratch-dir> (mktemp -d; removed on exit)"
}

# Run a command in the scratch environment and nothing else from the caller's env.
oc_env() {
  env -i \
    PATH="$(dirname "$OC_BIN"):/usr/bin:/bin" \
    HOME="$OC_DIR/home" \
    XDG_DATA_HOME="$OC_DIR/data" \
    XDG_CONFIG_HOME="$OC_DIR/config" \
    XDG_STATE_HOME="$OC_DIR/state" \
    XDG_CACHE_HOME="$OC_DIR/cache" \
    TERM="${OC_TERM:-xterm-256color}" \
    OPENCODE_DISABLE_AUTOUPDATE=1 \
    OPENCODE_DISABLE_MODELS_FETCH=1 \
    OPENCODE_DISABLE_CLAUDE_CODE=1 \
    OPENCODE_DISABLE_LSP_DOWNLOAD=1 \
    OPENCODE_DISABLE_SHARE=1 \
    "$@"
}

# A free port in 48100-48199 on 127.0.0.1 that this run has not already handed out.
oc_free_port() {
  local p i start=$((RANDOM % 100))
  for i in $(seq 0 99); do
    p=$((48100 + (start + i) % 100))
    case "$OC_PORTS_TAKEN" in *" $p "*) continue ;; esac
    if ! (exec 3<>"/dev/tcp/127.0.0.1/$p") 2>/dev/null; then
      OC_PORTS_TAKEN="$OC_PORTS_TAKEN$p "
      echo "$p"
      return 0
    fi
  done
  echo "no free port in 48100-48199" >&2
  return 1
}

# Start `opencode serve` in its own process group (setsid), so cleanup can kill the
# group and nothing else. Sets OC_LAST_PID. Does not wait for health.
oc_serve_start() { # port
  local port="$1"
  # A backgrounded subshell is not a process-group leader, so setsid does not fork:
  # $! is the server's PID, its process group and its session id.
  (
    cd "$OC_DIR/proj"
    exec setsid env -i \
      PATH="$(dirname "$OC_BIN"):/usr/bin:/bin" HOME="$OC_DIR/home" \
      XDG_DATA_HOME="$OC_DIR/data" XDG_CONFIG_HOME="$OC_DIR/config" \
      XDG_STATE_HOME="$OC_DIR/state" XDG_CACHE_HOME="$OC_DIR/cache" \
      OPENCODE_DISABLE_AUTOUPDATE=1 OPENCODE_DISABLE_MODELS_FETCH=1 \
      OPENCODE_DISABLE_CLAUDE_CODE=1 OPENCODE_DISABLE_LSP_DOWNLOAD=1 OPENCODE_DISABLE_SHARE=1 \
      "$OC_BIN" serve --pure --port "$port" --hostname 127.0.0.1
  ) >"$OC_DIR/logs/serve-$port.log" 2>&1 </dev/null &
  OC_LAST_PID=$!
  disown "$OC_LAST_PID" 2>/dev/null || true # no "Killed" job notice when a probe kills it
  OC_PGIDS+=("$OC_LAST_PID")
}

# Wait until GET /global/health answers healthy; prints the milliseconds it took.
oc_wait_healthy() { # port [timeout_s]
  local port="$1" limit="${2:-$OC_BOOT_TIMEOUT}" t0 body
  t0=$(now_ms)
  while [ $(( ($(now_ms) - t0) / 1000 )) -lt "$limit" ]; do
    body=$(curl -s -m 2 "http://127.0.0.1:$port/global/health" 2>/dev/null || true)
    if [ "$(printf '%s' "$body" | jq -r '.healthy? // empty' 2>/dev/null)" = "true" ]; then
      echo $(( $(now_ms) - t0 ))
      return 0
    fi
    sleep 0.2
  done
  return 1
}

oc_serve() { # port -> starts, waits, sets OC_LAST_PID and OC_BOOT_MS
  oc_serve_start "$1"
  # shellcheck disable=SC2034 # read by the probe scripts
  OC_BOOT_MS=$(oc_wait_healthy "$1") || { fail "server on 127.0.0.1:$1 became healthy"; exit 1; }
}

# HTTP call with a hard client timeout. Sets HTTP_CODE ("000" = no answer) and HTTP_BODY.
oc_http() { # method port path [json-body]
  local method="$1" port="$2" path="$3" body="${4:-}" out
  if [ -n "$body" ]; then
    out=$(curl -s -m "$OC_HTTP_TIMEOUT" -X "$method" -H 'content-type: application/json' \
      -d "$body" -w $'\n%{http_code}' "http://127.0.0.1:$port$path" 2>/dev/null || true)
  else
    out=$(curl -s -m "$OC_HTTP_TIMEOUT" -X "$method" -w $'\n%{http_code}' \
      "http://127.0.0.1:$port$path" 2>/dev/null || true)
  fi
  HTTP_CODE="${out##*$'\n'}"
  HTTP_BODY="${out%$'\n'*}"
  [ "$HTTP_BODY" = "$out" ] && HTTP_BODY=""
  [ -n "$HTTP_CODE" ] || HTTP_CODE="000"
}

# The only provider must be the dead end on a closed local port. Refuse to probe otherwise.
oc_guard_no_model() { # port
  oc_http GET "$1" /config/providers
  local got
  got=$(printf '%s' "$HTTP_BODY" | jq -c '[.providers[] | {id, url: .options.baseURL}]' 2>/dev/null || echo "?")
  if [ "$got" = '[{"id":"deadend","url":"http://127.0.0.1:9/v1"}]' ] \
    && ! (exec 3<>/dev/tcp/127.0.0.1/9) 2>/dev/null; then
    pass "guard: the only provider is the dead end on closed 127.0.0.1:9 (no model can be reached)"
  else
    fail "guard: unexpected providers $got; refusing to go on"
    exit 1
  fi
}

oc_create_session() { # port title -> echoes id
  oc_http POST "$1" /session "{\"title\":\"$2\"}"
  printf '%s' "$HTTP_BODY" | jq -r '.id // empty'
}

# ---- TUI on a private tmux socket ----
oc_tmux() { tmux -S "$OC_TMUX_SOCK" -f /dev/null "$@"; }

oc_tui_start() { # name port [extra attach args...]
  local name="$1" port="$2"; shift 2
  local argv="" a
  for a in "$@"; do argv="$argv $(printf '%q' "$a")"; done
  local cmd
  cmd="cd $(printf '%q' "$OC_DIR/proj") && exec env -i PATH=$(printf '%q' "$(dirname "$OC_BIN"):/usr/bin:/bin") \
HOME=$(printf '%q' "$OC_DIR/home") XDG_DATA_HOME=$(printf '%q' "$OC_DIR/data") \
XDG_CONFIG_HOME=$(printf '%q' "$OC_DIR/config") XDG_STATE_HOME=$(printf '%q' "$OC_DIR/state") \
XDG_CACHE_HOME=$(printf '%q' "$OC_DIR/cache") TERM=xterm-256color \
OPENCODE_DISABLE_AUTOUPDATE=1 OPENCODE_DISABLE_MODELS_FETCH=1 OPENCODE_DISABLE_CLAUDE_CODE=1 \
$(printf '%q' "$OC_BIN") attach http://127.0.0.1:$port --dir $(printf '%q' "$OC_DIR/proj")$argv 2>$(printf '%q' "$OC_DIR/logs/tui-$name.err")"
  oc_tmux new-session -d -s "$name" -x 160 -y 45 "$cmd"
  oc_tmux set-option -t "$name" remain-on-exit on >/dev/null
}

# The title OpenCode set; tmux's default title (the host name) is masked so no output
# names the machine.
oc_tui_title() {
  local t
  t=$(oc_tmux display-message -p -t "$1" '#{pane_title}' 2>/dev/null || true)
  case "$t" in "OC | "* | OpenCode*) echo "$t" ;; *) echo "<not set by opencode>" ;; esac
}
oc_tui_dead() { oc_tmux display-message -p -t "$1" '#{pane_dead}' 2>/dev/null || echo 1; }
oc_tui_screen() { oc_tmux capture-pane -p -t "$1" 2>/dev/null || true; }

# Wait until the pane title equals $2; echoes milliseconds waited. Returns 1 on timeout.
oc_wait_title() { # name expected [timeout_s]
  local t0 limit="${3:-30}"
  t0=$(now_ms)
  while [ $(( ($(now_ms) - t0) / 1000 )) -lt "$limit" ]; do
    if [ "$(oc_tui_title "$1")" = "$2" ]; then echo $(( $(now_ms) - t0 )); return 0; fi
    sleep 0.1
  done
  return 1
}

oc_wait_screen() { # name grep-pattern [timeout_s]
  local t0 limit="${3:-15}"
  t0=$(now_ms)
  while [ $(( ($(now_ms) - t0) / 1000 )) -lt "$limit" ]; do
    if oc_tui_screen "$1" | grep -q -- "$2"; then return 0; fi
    sleep 0.2
  done
  return 1
}

oc_kill_group() { # pgid signal
  kill "-$2" -- "-$1" 2>/dev/null || true
}

# Every descendant of a PID (OpenCode starts shell commands in sessions of their own,
# so killing the server's process group alone can leave them behind).
oc_descendants() { # pid
  local kids k
  kids=$(ps -o pid= --ppid "$1" 2>/dev/null || true)
  for k in $kids; do echo "$k"; oc_descendants "$k"; done
}

oc_cleanup() {
  local rc=$? g i
  set +e
  [ -n "$OC_TMUX_SOCK" ] && [ -S "$OC_TMUX_SOCK" ] && oc_tmux kill-server 2>/dev/null
  for g in "${OC_EXTRA_PIDS[@]:-}"; do [ -n "$g" ] && kill -TERM "$g" 2>/dev/null; done
  local desc=""
  for g in "${OC_PGIDS[@]:-}"; do [ -n "$g" ] && desc="$desc $(oc_descendants "$g" | tr '\n' ' ')"; done
  for g in $desc; do kill -TERM "$g" 2>/dev/null; done
  for g in "${OC_PGIDS[@]:-}"; do
    [ -n "$g" ] || continue
    oc_kill_group "$g" CONT
    oc_kill_group "$g" TERM
  done
  for i in $(seq 1 25); do
    local alive=0
    for g in "${OC_PGIDS[@]:-}"; do
      [ -n "$g" ] && kill -0 -- "-$g" 2>/dev/null && alive=1
    done
    [ "$alive" = 0 ] && break
    sleep 0.2
  done
  for g in "${OC_PGIDS[@]:-}"; do [ -n "$g" ] && oc_kill_group "$g" KILL; done
  for g in $desc; do kill -KILL "$g" 2>/dev/null; done
  if [ "${OC_SPIKE_KEEP:-0}" != "1" ]; then
    [ -n "$OC_DIR" ] && rm -rf "$OC_DIR"
    [ -n "$OC_SOCK_DIR" ] && [ "$OC_SOCK_DIR" != "$OC_DIR" ] && rm -rf "$OC_SOCK_DIR"
  fi
  echo "INFO  cleanup: killed process groups [${OC_PGIDS[*]:-}], private tmux server, temp dir removed"
  if [ "$rc" = 0 ] && [ "$OC_FAILS" -gt 0 ]; then rc=1; fi
  echo "RESULT  $( [ "$rc" = 0 ] && echo ok || echo "failed ($OC_FAILS FAIL)")"
  exit "$rc"
}
