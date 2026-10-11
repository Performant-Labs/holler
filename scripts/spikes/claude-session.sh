#!/usr/bin/env bash
# Spike #830, questions 1, 5 and 6: the session of record, the login, and the process.
#
#   1. Does a launch choose the session id (--session-id), does a relaunch resume exactly
#      that session (--resume), and what does Claude Code expose as "the session this
#      pane shows"?
#   5. What does the process need to stay logged in (names only; nothing is read)?
#   6. Where does the process run, and what stops it?
#
# Uses a scratch directory and a private tmux socket (see claude-lib.sh). Two one-word
# model turns.
#
# Usage: scripts/spikes/claude-session.sh
# shellcheck disable=SC2034 # CL_TARGET is read by the library
set -euo pipefail
HERE="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
# shellcheck source=claude-lib.sh
source "$HERE/claude-lib.sh"
cl_init

U="$(uuidgen | tr 'A-Z' 'a-z')"
info "chosen session id: $U"

echo "== 5. login: what the process needs (names only) =="
auth_method() { # NAME=value... (the exit status of `auth status` is 1 when nobody is logged in)
  local out; out=$(cl_env "$@" "$CL_BIN" auth status 2>/dev/null || true)
  printf '%s' "$out" | jq -r '.authMethod // "none"' 2>/dev/null || echo none
}
info "authMethod with HOME, PATH, TERM, LANG only: $(auth_method)"
# Same login, found through a different route: a config directory with no login in it.
EMPTY="$CL_DIR/empty-config"; mkdir -p "$EMPTY"
info "authMethod with CLAUDE_CONFIG_DIR=<empty dir>: $(auth_method CLAUDE_CONFIG_DIR="$EMPTY")"
# A dummy value (not a key) shows which variable outranks the login. Never a real key.
info "authMethod with ANTHROPIC_API_KEY=<dummy>: $(auth_method ANTHROPIC_API_KEY=dummy-not-a-key)"
info "authMethod with CLAUDE_CODE_OAUTH_TOKEN=<dummy>: $(auth_method CLAUDE_CODE_OAUTH_TOKEN=dummy-not-a-token)"
if [ -f "$HOME/.claude/.credentials.json" ]; then
  info "login file present: <config dir>/.credentials.json (mode $(stat -c %a "$HOME/.claude/.credentials.json"); contents never read)"
fi

echo "== 1. a launch chooses the session id =="
cl_launch a --session-id "$U"
cl_wait_agents '.[0].status=="idle"' 30 >/dev/null || fail "the session registered as idle within 30 s"
PID=$(cl_pid a)
got=$(cl_agent_field '.sessionId')
check "claude agents --json reports the chosen id as the session this process holds" test "$got" = "$U"
info "agents --json entry keys: $(cl_agents | jq -c '.[0]|keys')"
info "kind=$(cl_agent_field '.kind') status=$(cl_agent_field '.status') name=$(cl_agent_field '.name')"
check "agents --json pid is the pane's process" test "$(cl_agent_field '.pid')" = "$PID"
REG="$HOME/.claude/sessions/$PID.json"
if [ -f "$REG" ]; then
  info "per-process registry file keys: $(jq -c 'keys' "$REG")"
  check "the registry file's sessionId is the chosen id" test "$(jq -r .sessionId "$REG")" = "$U"
  info "registry: peerProtocol=$(jq -c .peerProtocol "$REG") peerFeatures=$(jq -c .peerFeatures "$REG") entrypoint=$(jq -r .entrypoint "$REG")"
fi
check "no transcript exists before the first prompt" test ! -e "$(cl_transcript "$U")"
cl_keys a "Reply with the single word: pong" Enter
cl_wait_agents '.[0].status=="busy"' 10 >/dev/null || true
cl_wait_agents '.[0].status=="idle"' 60 >/dev/null || fail "the first turn finished"
for _ in $(seq 1 40); do [ -f "$(cl_transcript "$U")" ] && break; sleep 0.25; done
check "after the first turn the transcript is <config>/projects/<cwd-encoded>/<id>.jsonl" test -f "$(cl_transcript "$U")"
info "transcript lines: $(wc -l <"$(cl_transcript "$U")")"

echo "== 1. a second process cannot take a chosen id that is in use =="
cl_launch_hold b --session-id "$U"; sleep 4
info "while the first process is running: $(cl_show b 3 | tr '\n' '|' | cut -c1-240)"
cl_tmux kill-session -t b 2>/dev/null || true

echo "== 6. what stops it =="
before=$(cl_agents | jq length)
kill -TERM "$PID"
for _ in $(seq 1 40); do cl_alive a || break; sleep 0.25; done
check "SIGTERM ends the process and its tmux pane" bash -c '! tmux -S "$0" has-session -t a 2>/dev/null' "$CL_TMUX_SOCK"
info "agents entries before/after SIGTERM: $before / $(cl_agents | jq length)"
check "the per-process registry file is removed on a clean stop" test ! -e "$REG"
check "the transcript is kept" test -f "$(cl_transcript "$U")"

echo "== 1. a launch cannot reuse an id whose transcript exists =="
cl_launch_hold c --session-id "$U"; sleep 4
info "transcript exists, nothing running: $(cl_show c 3 | tr '\n' '|' | cut -c1-240)"
cl_tmux kill-session -t c 2>/dev/null || true

echo "== 1. a relaunch resumes exactly that session =="
CL_TARGET=d
cl_launch d --resume "$U"
cl_wait_agents '.[0].status=="idle"' 30 >/dev/null || fail "the resumed session registered as idle"
check "the resumed process holds the same session id" test "$(cl_agent_field '.sessionId')" = "$U"
check "the resumed process is a different process" test "$(cl_agent_field '.pid')" != "$PID"
if cl_pane d | grep -q pong; then pass "the pane shows the earlier conversation (the word 'pong')"; else fail "the pane shows the earlier conversation"; fi
info "transcript files in the project directory after the resume: $(find "$(cl_project_dir)" -maxdepth 1 -name '*.jsonl' | wc -l)"
check "no second transcript file was created" test "$(find "$(cl_project_dir)" -maxdepth 1 -name '*.jsonl' | wc -l)" -eq 1

echo "== 1. the shown session changes inside the TUI, and agents --json follows =="
cl_keys d "/clear" Enter
sleep 2
new=$(cl_agent_field '.sessionId')
info "after /clear typed into the pane the process holds: $([ "$new" = "$U" ] && echo "the same id" || echo "a different id")"
check "a session switch made by a person in the pane shows in agents --json" test "$new" != "$U"

echo "== 1. an id that was chosen but never prompted =="
UN="$(uuidgen | tr 'A-Z' 'a-z')"
CL_TARGET=n1
cl_launch n1 --session-id "$UN"
cl_wait_agents '.[0].status=="idle"' 30 >/dev/null || fail "the never-prompted pane registered as idle"
check "no transcript exists for it" test ! -e "$(cl_transcript "$UN")"
kill -TERM "$(cl_pid n1)"; sleep 2
cl_launch_hold n2 --resume "$UN"; sleep 4
info "--resume of the never-prompted id: $(cl_show n2 3 | tr '\n' '|' | cut -c1-200)"
cl_tmux kill-session -t n2 2>/dev/null || true
CL_TARGET=n3
cl_launch n3 --session-id "$UN"
cl_wait_agents '.[0].status=="idle"' 30 >/dev/null || fail "the same id launched again with --session-id"
check "--session-id of the never-prompted id is accepted again and holds that id" test "$(cl_agent_field '.sessionId')" = "$UN"
cl_tmux kill-session -t n3 2>/dev/null || true; sleep 1

echo "== 1. resume an unknown id =="
cl_tmux kill-session -t d 2>/dev/null || true
cl_launch_hold e --resume "$(uuidgen)"; sleep 4
info "unknown id: $(cl_show e 3 | tr '\n' '|' | cut -c1-240)"
cl_tmux kill-session -t e 2>/dev/null || true

echo
if [ "$CL_FAILS" -eq 0 ]; then echo "claude-session: all expectations held"; else echo "claude-session: $CL_FAILS expectation(s) failed"; fi
