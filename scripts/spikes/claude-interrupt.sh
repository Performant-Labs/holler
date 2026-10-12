#!/usr/bin/env bash
# Spike #830, question 3: what stops a turn in flight without killing the session.
#
#   A. SIGINT to the process of a pane that is in the middle of a tool call, then a relaunch
#   B. SIGINT to an idle pane
#   C. a hub-owned stream-json process: control_request interrupt
#   D. the frames the inbox socket was tried with, as the client classifies them
#   E. a hub-controlled PreToolUse hook that ends the turn at the next tool boundary
#
# Scratch directory, private tmux socket, a handful of short turns on the smallest model.
#
# Usage: scripts/spikes/claude-interrupt.sh [A B C D E]   (default: all)
# shellcheck disable=SC2034 # CL_TARGET is read by the library
set -euo pipefail
HERE="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
# shellcheck source=claude-lib.sh
source "$HERE/claude-lib.sh"
cl_init
WANT="${*:-A B C D E}"
want() { case " $WANT " in *" $1 "*) return 0 ;; esac; return 1; }
echo '{"crossSessionInbound":"accept"}' >"$CL_DIR/accept.json"

# Is $1 a descendant of $2? (walk the parent chain in /proc)
is_descendant() {
  local p="$1" anc="$2"
  for _ in $(seq 1 20); do
    p=$(awk '{print $4}' "/proc/$p/stat" 2>/dev/null || echo 0)
    [ "$p" = "$anc" ] && return 0
    [ "$p" -le 1 ] && return 1
  done
  return 1
}
find_sleep() { # seconds ancestor-pid -> pid of that sleep under the ancestor
  local p
  for p in $(pgrep -fx "sleep $1" || true); do is_descendant "$p" "$2" && { echo "$p"; return 0; }; done
  return 1
}

if want A || want B; then
  U="$(uuidgen | tr 'A-Z' 'a-z')"
  CL_TARGET=a
  cl_launch a --session-id "$U" --settings "$CL_DIR/accept.json" --allowed-tools 'Bash(sleep *)'
  cl_wait_agents '.[0].status=="idle"' 30 >/dev/null || fail "pane registered as idle"
  PID=$(cl_pid a); SOCK=$(cl_inbox_of a)
fi

if want A; then
  echo "== A. SIGINT in the middle of a tool call =="
  cl_inbox_send "$SOCK" 'Run the shell command `sleep 47` and then reply with exactly: finished-47'
  cl_wait_agents '.[0].status=="busy"' 15 >/dev/null || fail "the turn started"
  for _ in $(seq 1 120); do SL=$(find_sleep 47 "$PID" || true); [ -n "${SL:-}" ] && break; sleep 0.25; done
  if [ -n "${SL:-}" ]; then pass "the tool call is running as a child of the Claude process"; else info "the tool call was not seen as a child process (the model may have backgrounded it); the turn is busy: $(cl_agent_field '.status')"; fi
  info "status when the signal is sent: $(cl_agent_field '.status') (the model decides whether the command runs in the foreground)"
  t0=$(now_ms)
  kill -INT "$PID"
  for _ in $(seq 1 60); do kill -0 "$PID" 2>/dev/null || break; sleep 0.1; done
  if kill -0 "$PID" 2>/dev/null; then
    fail "SIGINT ends the Claude process (a signal is not the Ctrl-C key the TUI handles)"
  else
    pass "SIGINT ends the Claude process, $(( $(now_ms) - t0 )) ms after it is sent (a signal is not the Ctrl-C key the TUI handles)"
  fi
  sleep 1
  if [ -n "${SL:-}" ]; then check "the child tool process is gone" bash -c '! kill -0 "$0" 2>/dev/null' "$SL"; fi
  check "the transcript is kept" test -f "$(cl_transcript "$U")"
  check "no 'finished-47' reply was written" bash -c '! grep -q "finished-47\"" "$0" || ! grep -q "text\":\"finished-47" "$0"' "$(cl_transcript "$U")"
  echo "-- relaunch with --resume: the session continues, the cut-off turn is not re-run"
  cl_tmux kill-session -t a 2>/dev/null || true
  CL_TARGET=a2
  cl_launch a2 --resume "$U" --settings "$CL_DIR/accept.json" --allowed-tools 'Bash(sleep *)'
  cl_wait_agents '.[0].status=="idle"' 30 >/dev/null || fail "the resumed pane registered as idle (not running the old turn)"
  check "the resumed process holds the same session id" test "$(cl_agent_field '.sessionId')" = "$U"
  PID=$(cl_pid a2); SOCK=$(cl_inbox_of a2)
  cl_inbox_send "$SOCK" "Reply with exactly the word: after-interrupt"
  cl_wait_agents '.[0].status=="busy"' 15 >/dev/null || fail "a message after the relaunch started a turn"
  cl_wait_agents '.[0].status=="idle"' 60 >/dev/null || true
  check "the reply to the later message is in the pane" bash -c 'tmux -S "$0" capture-pane -p -t a2 | grep -q "● after-interrupt"' "$CL_TMUX_SOCK"
  info "the pane after the relaunch shows the earlier turn as: $(cl_pane a2 | grep -m1 -i 'interrupt' | sed 's/^[[:space:]]*//; s/[[:space:]]*$//' | cut -c1-120)"
fi

if want B; then
  echo "== B. SIGINT while idle =="
  [ -n "${PID:-}" ] && kill -0 "$PID" 2>/dev/null || { CL_TARGET=a; cl_launch a --session-id "$U" --settings "$CL_DIR/accept.json"; cl_wait_agents '.[0].status=="idle"' 30 >/dev/null; PID=$(cl_pid a); }
  kill -INT "$PID"; sleep 2
  if kill -0 "$PID" 2>/dev/null; then fail "one SIGINT at an idle prompt ends the process"; else pass "one SIGINT at an idle prompt ends the process too (an abort must never be a signal)"; fi
fi

if want D; then
  echo "== D. does the inbox socket take an interrupt? (frames tried, as the client's debug log classifies them) =="
  UD="$(uuidgen | tr 'A-Z' 'a-z')"
  CL_TARGET=d
  cl_launch d --session-id "$UD" --settings "$CL_DIR/accept.json" --allowed-tools 'Bash(sleep *)' --debug-file "$CL_DIR/d.debug"
  cl_wait_agents '.[0].status=="idle"' 30 >/dev/null || fail "pane D registered as idle"
  SD=$(cl_inbox_of d)
  FRAMES=(
    '{"type":"control_request","request_id":"x1","request":{"subtype":"interrupt"}}'
    '{"type":"interrupt"}'
    '{"type":"abort"}'
    '{"type":"control","action":"interrupt"}'
    '{"type":"control","action":"abort"}'
  )
  for frame in "${FRAMES[@]}"; do cl_unix_line "$SD" "$frame"; sleep 0.5; done
  info "client classification of the ${#FRAMES[@]} frames:"
  grep -E "unhandled|Unhandled" "$CL_DIR/d.debug" | sed 's/^[0-9TZ:.-]* //' | cut -c1-120 | sed 's/^/      /'
  check "every interrupt-shaped frame was classified unhandled by the client" test "$(grep -cE 'Received unhandled message type|Unhandled control action' "$CL_DIR/d.debug")" -eq "${#FRAMES[@]}"
  info "control actions the client does handle (from its own code, version-pinned): notify_when_idle, artifact reply yield/unyield"
  cl_tmux kill-session -t d 2>/dev/null || true
fi

if want E; then
  echo "== E. a hub-controlled PreToolUse hook stops the turn at the next tool boundary =="
  UE="$(uuidgen | tr 'A-Z' 'a-z')"
  HLOG="$CL_DIR/abort-hook.log"; FLAG="$CL_DIR/abort.flag"
  jq -n --arg cmd "bash $HERE/claude-abort-hook.sh $HLOG $FLAG" \
    '{crossSessionInbound:"accept", hooks:{PreToolUse:[{hooks:[{type:"command",command:$cmd}]}]}}' >"$CL_DIR/abort-settings.json"
  CL_TARGET=e
  cl_launch e --session-id "$UE" --settings "$CL_DIR/abort-settings.json" --allowed-tools 'Bash(sleep *)'
  cl_wait_agents '.[0].status=="idle"' 30 >/dev/null || fail "pane E registered as idle"
  cl_inbox_send "$(cl_inbox_of e)" 'Run these shell commands one at a time, each as its own separate Bash call: `sleep 6`, then `sleep 6`, then `sleep 6`. After the third, reply with exactly: all-done'
  cl_wait_agents '.[0].status=="busy"' 15 >/dev/null || fail "the turn started"
  cl_wait_file "$HLOG" '"verdict":"run"' 30 >/dev/null || fail "the hook was consulted for the first tool call"
  t0=$(now_ms); touch "$FLAG"
  if cl_wait_agents '.[0].status=="idle"' 40 >/dev/null; then pass "the turn ended $(( $(now_ms) - t0 )) ms after the hub raised the flag (at the next tool boundary)"; else fail "the turn ended after the flag"; fi
  check "the process is alive and holds the same session" test "$(cl_agent_field '.sessionId')" = "$UE"
  check "the third command did not run (no 'all-done')" bash -c '! tmux -S "$0" capture-pane -p -t e | grep -q "● all-done"' "$CL_TMUX_SOCK"
  info "hook consulted: $(jq -r .verdict "$HLOG" | sort | uniq -c | tr '\n' ' ')"
  info "pane marker: $(cl_pane e | grep -m1 -i 'stopped\|hub' | sed 's/^[[:space:]]*//; s/[[:space:]]*$//' | cut -c1-120)"
  cl_tmux kill-session -t e 2>/dev/null || true
fi

if want C; then
  echo "== C. hub-owned stream-json: control_request interrupt =="
  CL_TARGET=
  U3="$(uuidgen | tr 'A-Z' 'a-z')"
  python3 "$HERE/claude-stream.py" "$CL_BIN" "$CL_WORK" "$CL_MODEL" --session-id "$U3" --interrupt-after 4 \
    --allow-tools 'Bash(sleep *)' 'Run the shell command `sleep 45` and then reply with exactly: finished-45' \
    'Reply with exactly the word: after-interrupt' | sed 's/^/      /'
fi

echo
if [ "$CL_FAILS" -eq 0 ]; then echo "claude-interrupt: all expectations held"; else echo "claude-interrupt: $CL_FAILS expectation(s) failed"; fi
