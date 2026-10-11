#!/usr/bin/env bash
# Spike #830, question 2: delivering a message to a pane's session without typing into it
# (epic decision 1, invariant I4), and what each way costs.
#
#   A. the session's inbox socket (a documented unix socket; the frame is the one the
#      client prints in its own debug log)
#   B. the same, into a --dangerously-skip-permissions pane (inbound policy)
#   C. a channel: an MCP server the session loads, pushed to by the hub
#   D. `claude -p --resume <id>` against a session a TUI already holds
#   E. a hub-owned `claude -p --input-format stream-json` process (no TUI)
#   F. `claude --bg` and a prompt sent to the running background session
#
# Scratch directory, private tmux socket, a handful of one-line turns on the smallest
# model. The "typed" stand-in is used only to answer a first-run dialog.
#
# Usage: scripts/spikes/claude-drive.sh [A B C D E F]   (default: all)
# shellcheck disable=SC2034 # CL_TARGET is read by the library
set -euo pipefail
HERE="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
# shellcheck source=claude-lib.sh
source "$HERE/claude-lib.sh"
cl_init
WANT="${*:-A B C D E F}"
want() { case " $WANT " in *" $1 "*) return 0 ;; esac; return 1; }

# The policy a hub-launched pane would carry: accept messages from the hub's own socket
# sends. Without it a pane that bypasses permission prompts holds every peer message
# behind a TUI dialog (section B).
echo '{"crossSessionInbound":"accept"}' >"$CL_DIR/accept.json"

U1="$(uuidgen | tr 'A-Z' 'a-z')"

if want A || want D; then
  echo "== A. inbox socket: deliver without typing =="
  CL_TARGET=a
  cl_launch a --session-id "$U1" --settings "$CL_DIR/accept.json" --allowed-tools 'Bash(sleep *)'
  cl_wait_agents '.[0].status=="idle"' 30 >/dev/null || fail "pane A registered as idle"
  SOCK=$(cl_inbox_of a)
  info "inbox socket: $(printf '%s' "$SOCK" | sed 's#[0-9][0-9]*#<pid>#') mode=$(stat -c '%a' "$SOCK") owner-only=$([ "$(stat -c %U "$SOCK")" = "$(id -un)" ] && echo yes || echo no)"
  t0=$(now_ms)
  cl_inbox_send "$SOCK" "Reply with exactly the word: delivered"
  cl_wait_agents '.[0].status=="busy"' 15 >/dev/null && info "status went busy $(( $(now_ms) - t0 )) ms after the socket write" || fail "the socket message started a turn"
  cl_wait_agents '.[0].status=="idle"' 60 >/dev/null || fail "the turn finished"
  check "the reply is in the pane" bash -c 'tmux -S "$0" capture-pane -p -t a | grep -q "● delivered"' "$CL_TMUX_SOCK"
  info "how the message reads in the pane: $(cl_pane a | grep -m1 'Another Claude session sent a message' | sed 's/[[:space:]]*$//')"
  info "transcript user entries for the turn: $(grep -c '"type":"user"' "$(cl_transcript "$U1")")"

  echo "-- a second message while a turn is running (sleep 6) is queued, not lost"
  cl_inbox_send "$SOCK" 'Run the shell command `sleep 6` and then reply with exactly: first-done'
  cl_wait_agents '.[0].status=="busy"' 15 >/dev/null || fail "the sleep turn started"
  sleep 1
  cl_inbox_send "$SOCK" "Reply with exactly the word: second-done"
  cl_wait_agents '.[0].status=="idle"' 90 >/dev/null || fail "turns finished"
  sleep 3
  cl_wait_agents '.[0].status=="idle"' 60 >/dev/null || true
  for w in first-done second-done; do
    check "the transcript holds a reply '$w' (queued message ran after the first turn)" grep -q "$w" "$(cl_transcript "$U1")"
  done
fi

if want B; then
  echo "== B. inbox socket into a pane that bypasses permission prompts =="
  CL_TARGET=b
  cl_launch b --dangerously-skip-permissions
  cl_wait_agents '.[0].status=="idle"' 30 >/dev/null || fail "pane B registered as idle"
  SOCKB=$(cl_inbox_of b)
  cl_inbox_send "$SOCKB" "Reply with exactly the word: delivered"
  sleep 4
  info "status after the send: $(cl_agent_field '.status') waitingFor=$(cl_agent_field '.waitingFor')"
  check "the message was held, not delivered (status waiting, no turn)" test "$(cl_agent_field '.status')" = waiting
  info "pane says: $(cl_pane b | grep -m1 'Held peer message' | sed 's/[[:space:]]*$//' | cut -c1-200)"
  cl_tmux kill-session -t b 2>/dev/null || true; sleep 1

  echo "-- the same pane with crossSessionInbound=accept in --settings"
  CL_TARGET=b2
  cl_launch b2 --dangerously-skip-permissions --settings "$CL_DIR/accept.json"
  cl_wait_agents '.[0].status=="idle"' 30 >/dev/null || fail "pane B2 registered as idle"
  cl_inbox_send "$(cl_inbox_of b2)" "Reply with exactly the word: delivered"
  cl_wait_agents '.[0].status=="busy"' 15 >/dev/null && pass "accepted: the message started a turn in a bypassing pane" || fail "accepted: the message started a turn in a bypassing pane"
  cl_wait_agents '.[0].status=="idle"' 60 >/dev/null || true
  cl_tmux kill-session -t b2 2>/dev/null || true; sleep 1
fi

if want C; then
  echo "== C. a channel: an MCP server the session loads, pushed to by the hub =="
  CHSOCK="$CL_SOCK_DIR/ch.sock"; CHLOG="$CL_DIR/ch.log"
  jq -n --arg js "$HERE/claude-channel.mjs" --arg sock "$CHSOCK" --arg log "$CHLOG" \
    '{mcpServers:{hub:{command:"node",args:[$js],env:{CHANNEL_SOCK:$sock,CHANNEL_LOG:$log}}}}' >"$CL_DIR/mcp.json"
  U3="$(uuidgen | tr 'A-Z' 'a-z')"
  CL_TARGET=ch
  cl_launch ch --session-id "$U3" --mcp-config "$CL_DIR/mcp.json" --strict-mcp-config \
    --dangerously-load-development-channels server:hub --allowed-tools mcp__hub__reply
  dialogs=0
  for _ in $(seq 1 60); do
    text=$(cl_pane ch)
    if printf '%s' "$text" | grep -q 'I am using this for local development'; then
      dialogs=$((dialogs + 1)); info "a startup dialog blocks until a key is pressed: $(printf '%s' "$text" | grep -m1 -i 'development channels\|WARNING' | sed 's/^[[:space:]]*//' | cut -c1-120)"
      cl_keys ch Enter; sleep 1; continue
    fi
    if printf '%s' "$text" | grep -qi 'New MCP server found'; then
      dialogs=$((dialogs + 1)); info "a project-MCP consent dialog appeared"; cl_keys ch Enter; sleep 1; continue
    fi
    [ -S "$CHSOCK" ] && cl_agents | jq -e '.[0].status=="idle"' >/dev/null 2>&1 && break
    sleep 0.5
  done
  info "startup dialogs that needed a key: $dialogs"
  check "the channel server connected and listens" test -S "$CHSOCK"
  info "channel notice in the pane: $(cl_pane ch | grep -m1 -i 'channels' | sed 's/^[[:space:]]*//; s/[[:space:]]*$//' | cut -c1-160)"
  cl_unix_line "$CHSOCK" "Answer by calling the reply tool with the text: channel-ok"
  if ms=$(cl_wait_file "$CHLOG" '"ev":"reply"' 60); then pass "the session received the push and replied through the channel's tool ($ms ms)"; else fail "the session replied through the channel"; fi
  info "channel log events: $(jq -r .ev "$CHLOG" | sort | uniq -c | tr '\n' ' ')"
  cl_tmux kill-session -t ch 2>/dev/null || true; sleep 1
fi

if want D; then
  CL_TARGET=a
  echo "== D. claude -p --resume <id> against a session a TUI already holds =="
  before=$(wc -l <"$(cl_transcript "$U1")")
  out=$(cd "$CL_WORK" && cl_env "$CL_BIN" --setting-sources project,local --model "$CL_MODEL" -p --resume "$U1" --output-format json "Reply with exactly the word: via-print" 2>&1 </dev/null || true)
  rid=$(printf '%s' "$out" | jq -r '.session_id // empty' 2>/dev/null || true)
  info "-p --resume result: session_id $([ "$rid" = "$U1" ] && echo "= the held session" || echo "is a NEW id") result=$(printf '%s' "$out" | jq -r '.result // empty' 2>/dev/null | cut -c1-60)"
  after=$(wc -l <"$(cl_transcript "$U1")")
  info "transcript lines of the held session before/after: $before / $after"
  info "transcript files in the project directory: $(find "$(cl_project_dir)" -maxdepth 1 -name '*.jsonl' | wc -l)"
  if [ -n "$rid" ] && [ "$rid" != "$U1" ] && [ -f "$(cl_transcript "$rid")" ]; then info "the new id has its own transcript ($(wc -l <"$(cl_transcript "$rid")") lines)"; fi
  if cl_pane a | grep -q via-print; then info "the TUI that holds the session shows the new turn"; else info "the TUI that holds the session does NOT show the new turn (it keeps its own view of the transcript)"; fi
  info "agents entries for the scratch dir (all sessions): $(CL_TARGET="" cl_agents | jq length)"
fi

if want E; then
  echo "== E. a hub-owned stream-json process (no TUI) =="
  U5="$(uuidgen | tr 'A-Z' 'a-z')"
  python3 "$HERE/claude-stream.py" "$CL_BIN" "$CL_WORK" "$CL_MODEL" --session-id "$U5" \
    "Reply with exactly the word: one" "What single word did you reply with just now?" | sed 's/^/      /'
  info "transcript for the stream-json session exists: $([ -f "$(cl_transcript "$U5")" ] && echo yes || echo no)"
fi

if want F; then
  echo "== F. claude --bg, and a prompt sent to the running background session =="
  cl_require_no_foreign_daemon
  # End the panes of the earlier sections: only the background session should be listed.
  cl_tmux kill-server 2>/dev/null || true; sleep 3
  CL_TARGET=""
  bgf() { cl_agents --all | jq -r "([.[] | select(.kind == \"background\")] | .[0] // {}) | ($1) // \"-\""; }
  REQ="$(uuidgen | tr 'A-Z' 'a-z')"
  out=$(cd "$CL_WORK" && cl_env "$CL_BIN" --bg --setting-sources project,local --model "$CL_MODEL" --session-id "$REQ" "Reply with exactly the word: bg-one" 2>&1 </dev/null || true)
  info "--bg --session-id: $(printf '%s' "$out" | head -4 | tr '\n' '|' | cut -c1-300)"
  id=$(printf '%s' "$out" | grep -oE '\b[0-9a-f]{8}\b' | head -1 || true)
  if [ -n "$id" ]; then
    CL_BG_IDS+=("$id")
    for _ in $(seq 1 120); do [ "$(bgf .state)" = "done" ] && break; sleep 0.5; done
    U6=$(bgf .sessionId)
    check "--bg ignores --session-id: the session id is assigned, not chosen" test "$U6" != "$REQ" -a "$U6" != "-"
    info "agents --json (bg entry): $(cl_agents --all | jq -c '[.[] | select(.kind == "background")] | .[0] | {kind, state, status, waitingFor, keys: (keys | length)}')"
    info "claude logs prints the session's terminal output (a screen dump with escape sequences): $(cd "$CL_WORK" && cl_env "$CL_BIN" logs "$id" 2>&1 | wc -c) bytes"
    echo "-- a prompt to the running background session with redirected output (how a hub would spawn it)"
    out2=$(cd "$CL_WORK" && cl_env "$CL_BIN" --resume "$U6" "Reply with exactly the word: bg-two" 2>&1 </dev/null | head -c 300 || true)
    info "no tty: $(printf '%s' "$out2" | tr '\n' '|' | cut -c1-260)"
    echo "-- the same from a terminal, with no other flags"
    cl_launch_raw bgsend --resume "$U6" "Reply with exactly the word: bg-two"
    sleep 8
    info "tty: $(cl_show bgsend 12 | tr '\n' '|' | cut -c1-420)"
    cl_tmux kill-session -t bgsend 2>/dev/null || true
    sleep 6
    info "after the terminal went away the background session is still listed: $(cl_agents --all | jq -c '[.[] | {kind, state, status}]')"
    check "the background session's transcript holds the second turn" bash -c 'grep -q bg-two "$0"' "$(cl_transcript "$U6")"
  else
    fail "--bg printed an id"
  fi
fi

echo
if [ "$CL_FAILS" -eq 0 ]; then echo "claude-drive: all expectations held"; else echo "claude-drive: $CL_FAILS expectation(s) failed"; fi
