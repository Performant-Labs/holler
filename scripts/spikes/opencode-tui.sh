#!/usr/bin/env bash
# OpenCode pane spike (issue #635), capabilities 2, 3 and 4 and the deleted-session
# case: a TUI started attached to a given server and session (`opencode attach -s`),
# switched to another session over the API (`POST /tui/select-session`) without a
# keystroke, and asked which session it shows (the terminal title it sets, read from
# tmux's #{pane_title}). The TUI runs on a private tmux socket; no key is ever sent to
# it. No model call: see opencode-lib.sh for the safety contract.
#
#   bash scripts/spikes/opencode-tui.sh
set -euo pipefail
# shellcheck source=scripts/spikes/opencode-lib.sh
. "$(dirname "${BASH_SOURCE[0]}")/opencode-lib.sh"

oc_init
PORT=$(oc_free_port)
oc_serve "$PORT"
oc_guard_no_model "$PORT"

# Holler titles each session with its own id, so the TUI's terminal title
# ("OC | <session title>") names the session it shows.
A=$(oc_create_session "$PORT" spike-a); oc_http PATCH "$PORT" "/session/$A" "{\"title\":\"$A\"}"
B=$(oc_create_session "$PORT" spike-b); oc_http PATCH "$PORT" "/session/$B" "{\"title\":\"$B\"}"
info "sessions of record: A=$A B=$B"

# ---- 2. TUI attached to this server and session A ------------------------------
oc_tui_start tui1 "$PORT" --session "$A"
if ms=$(oc_wait_title tui1 "OC | $A" 60); then
  pass "opencode attach http://127.0.0.1:<port> --dir <dir> --session A shows A (title 'OC | A' after $ms ms)"
else
  fail "opencode attach --session A shows A (title is '$(oc_tui_title tui1)')"
fi
check "the TUI's screen shows A's title too" oc_wait_screen tui1 "$A" 5
oc_http GET "$PORT" /session
check "attaching the TUI created no session of its own (still 2)" \
  test "$(printf '%s' "$HTTP_BODY" | jq 'length')" = 2

# ---- 3. switch without typing ------------------------------------------------------
oc_http POST "$PORT" /tui/select-session "{\"sessionID\":\"$B\"}"
check "POST /tui/select-session {sessionID:B} -> 200 true (got $HTTP_CODE $HTTP_BODY)" \
  test "$HTTP_CODE" = 200 -a "$HTTP_BODY" = true
if ms=$(oc_wait_title tui1 "OC | $B" 10); then
  pass "the TUI switched to B with no keystroke (title 'OC | B' after $ms ms)"
else
  fail "the TUI switched to B (title is '$(oc_tui_title tui1)')"
fi
oc_http POST "$PORT" "/tui/select-session?directory=$OC_DIR/proj" "{\"sessionID\":\"$A\"}"
if ms=$(oc_wait_title tui1 "OC | $A" 10); then
  pass "select-session with ?directory=<dir> switches back to A ($ms ms)"
else
  fail "select-session with ?directory=<dir> switches back to A (title '$(oc_tui_title tui1)')"
fi
oc_http POST "$PORT" /tui/select-session '{"sessionID":"ses_doesnotexist0000000000000"}'
check "select-session with an unknown id -> 404 (got $HTTP_CODE)" test "$HTTP_CODE" = 404
sleep 1
check "... and the TUI stays on A" test "$(oc_tui_title tui1)" = "OC | $A"
oc_http POST "$PORT" /tui/select-session '{"sessionID":"not-a-session"}'
check "select-session with a malformed id -> 400 (got $HTTP_CODE)" test "$HTTP_CODE" = 400

# ---- 4. which session is shown: the title, and its limits ----------------------------
oc_http PATCH "$PORT" "/session/$A" "{\"title\":\"hj-c1r1 holler pane title $A\"}"
sleep 1
info "a long session title is cut in the terminal title: '$(oc_tui_title tui1)'"
oc_http PATCH "$PORT" "/session/$A" "{\"title\":\"$A\"}"
if ms=$(oc_wait_title tui1 "OC | $A" 10); then
  pass "renaming the shown session updates the terminal title live ($ms ms)"
else
  fail "renaming the shown session updates the terminal title (title '$(oc_tui_title tui1)')"
fi

# ---- 3 (caveat). select-session is a broadcast to every TUI on that server ---------
oc_tui_start tui2 "$PORT" -s "$A" # the short form of --session
check "a second TUI started with -s A shows A" quiet oc_wait_title tui2 "OC | $A" 60
oc_http POST "$PORT" /tui/select-session "{\"sessionID\":\"$B\"}"
t1=$(oc_wait_title tui1 "OC | $B" 10 || echo no); t2=$(oc_wait_title tui2 "OC | $B" 10 || echo no)
info "two TUIs on one server, one select-session(B): tui1 -> $([ "$t1" = no ] && echo stayed || echo switched), tui2 -> $([ "$t2" = no ] && echo stayed || echo switched)"
check "select-session is not addressed to one TUI: both TUIs on the server switch" \
  test "$t1" != no -a "$t2" != no
oc_tmux kill-session -t tui2

# A server with no TUI at all still answers true: there is no delivery acknowledgement.
PORT2=$(oc_free_port)
oc_serve "$PORT2"
C=$(oc_create_session "$PORT2" lonely)
oc_http GET "$PORT" "/session/$C"
check "two servers on one data dir share sessions: C, made on the second, is readable on the first (got $HTTP_CODE)" test "$HTTP_CODE" = 200
oc_http POST "$PORT2" /tui/select-session "{\"sessionID\":\"$C\"}"
check "select-session on a server with no TUI attached still answers 200 true (no ack: observe the title instead)" \
  test "$HTTP_CODE" = 200 -a "$HTTP_BODY" = true

# ---- /tui/* control endpoints that need no model --------------------------------------
oc_http POST "$PORT" /tui/show-toast '{"message":"spike635-toast","variant":"info"}'
info "POST /tui/show-toast -> $HTTP_CODE $HTTP_BODY"
check "a /tui/show-toast message reaches the TUI's screen" oc_wait_screen tui1 spike635-toast 5
OC_HTTP_TIMEOUT=3 oc_http GET "$PORT" /tui/control/next
info "GET /tui/control/next (3 s client timeout) -> $HTTP_CODE ${HTTP_BODY:-<no body>}"

# ---- the TUI's session is deleted under it -------------------------------------------
oc_http DELETE "$PORT" "/session/$B"
check "DELETE the session the TUI shows -> 200 true" test "$HTTP_BODY" = true
if ms=$(oc_wait_title tui1 "OpenCode" 10); then
  pass "the TUI leaves the deleted session: title becomes 'OpenCode' (no session) after $ms ms"
else
  fail "the TUI reports the deleted session (title '$(oc_tui_title tui1)')"
fi
check "the TUI says 'The current session was deleted' on screen" oc_wait_screen tui1 "current session was deleted" 5
check "the TUI process is still running (it does not exit)" test "$(oc_tui_dead tui1)" = 0
# Both servers use the same data dir, so each lists the other's sessions: A and C remain.
EXPECT=$(jq -nc --arg a "$A" --arg c "$C" '[$a,$c]|sort')
ids() { printf '%s' "$HTTP_BODY" | jq -c '[.[].id]|sort'; }
oc_http GET "$PORT" /session
check "the TUI did not create a replacement session by itself (A and C left)" test "$(ids)" = "$EXPECT"
oc_http POST "$PORT" /tui/select-session "{\"sessionID\":\"$A\"}"
if ms=$(oc_wait_title tui1 "OC | $A" 10); then
  pass "after the delete, select-session(A) puts the TUI back on a live session ($ms ms)"
else
  fail "after the delete, select-session(A) recovers (title '$(oc_tui_title tui1)')"
fi

# ---- attach edge cases --------------------------------------------------------------
oc_tui_start tui3 "$PORT" --session ses_doesnotexist0000000000000
for _ in $(seq 1 50); do [ "$(oc_tui_dead tui3)" = 1 ] && break; sleep 0.2; done
check "attach --session <unknown id> exits instead of showing something else" test "$(oc_tui_dead tui3)" = 1
info "... exit status $(oc_tmux display-message -p -t tui3 '#{pane_dead_status}'), stderr: $(head -c 200 "$OC_DIR/logs/tui-tui3.err" | sed 's/\x1b\[[0-9;]*m//g' | tr '\n' ' ')"
oc_tmux kill-session -t tui3 2>/dev/null || true
oc_tui_start tui4 "$PORT"
if ms=$(oc_wait_title tui4 "OpenCode" 60); then
  pass "attach with no --session shows no session (title 'OpenCode', $ms ms)"
else
  fail "attach with no --session shows the home screen (title '$(oc_tui_title tui4)')"
fi
oc_tmux kill-session -t tui4 2>/dev/null || true
oc_http GET "$PORT" /session
check "no attach created a stray session (still A and C)" test "$(ids)" = "$EXPECT"
