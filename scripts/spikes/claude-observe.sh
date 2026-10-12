#!/usr/bin/env bash
# Spike #830, question 4: what says working, idle, waiting for a question or permission,
# finished or dead, without parsing the screen; and how much a screen read can say.
#
#   O1. the status timeline of one session (idle -> busy -> idle) from `claude agents --json`,
#       joined with the hook events of the same turn (the screen has no heartbeat either)
#   O2. a permission wait: status, hook event, and the hub answering through a file
#   O3. a question wait (AskUserQuestion), and the launch flag that removes it
#   O4. a dead process (SIGKILL), as the listing reports it, and whether anything beats
#   O5. what the screen says in each state (the fallback)
#
# Scratch directory, private tmux socket, three short turns on the smallest model.
#
# Usage: scripts/spikes/claude-observe.sh
# shellcheck disable=SC2034 # CL_TARGET is read by the library
set -euo pipefail
HERE="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
# shellcheck source=claude-lib.sh
source "$HERE/claude-lib.sh"
cl_init

HLOG="$CL_DIR/hooks.log"; SLOG="$CL_DIR/status.log"; DEC="$CL_DIR/decision"
: >"$HLOG"; : >"$SLOG"
hook() { echo "{\"hooks\":[{\"type\":\"command\",\"command\":\"bash $HERE/claude-hook.sh $HLOG $1 ${2:-}\"}]}"; }
jq -n \
  --argjson ss "[$(hook SessionStart)]" --argjson up "[$(hook UserPromptSubmit)]" \
  --argjson pr "[$(hook PermissionRequest "$DEC")]" --argjson no "[$(hook Notification)]" \
  --argjson st "[$(hook Stop)]" --argjson se "[$(hook SessionEnd)]" \
  '{crossSessionInbound:"accept", hooks:{SessionStart:$ss, UserPromptSubmit:$up, PermissionRequest:$pr, Notification:$no, Stop:$st, SessionEnd:$se}}' \
  >"$CL_DIR/settings.json"

# Record every change of (status, waitingFor) of the target pane's process with a time.
poll_status() {
  local last="" cur
  while :; do
    cur=$(cl_agents | jq -r '(.[0] // {}) | "\(.status // "gone") \(.waitingFor // "-")"' 2>/dev/null || echo "gone -")
    if [ "$cur" != "$last" ]; then printf '{"t":%s,"event":"status","value":"%s"}\n' "$(now_ms)" "$cur" >>"$SLOG"; last="$cur"; fi
    sleep 0.15
  done
}
timeline() { # since_ms
  cat "$HLOG" "$SLOG" | jq -rs --argjson t0 "$1" 'map(select(.t >= $t0)) | sort_by(.t) | .[] |
    "      +\((.t - $t0))ms  \(.event)\(if .value then " = " + .value else "" end)\(if .notification_type then " (" + .notification_type + ")" else "" end)\(if .tool_name then " [" + .tool_name + "]" else "" end)\(if .verdict then " -> " + .verdict else "" end)"'
}
screen_tail() { cl_pane "$1" | sed 's/[[:space:]]*$//' | { grep -v '^$' || true; } | tail -"${2:-3}" | cut -c1-110 | sed 's/^/      | /'; }

U="$(uuidgen | tr 'A-Z' 'a-z')"
CL_TARGET=a
cl_launch a --session-id "$U" --settings "$CL_DIR/settings.json" --permission-mode manual
cl_wait_agents '.[0].status=="idle"' 30 >/dev/null || fail "the pane registered as idle"
PID=$(cl_pid a); SOCK=$(cl_inbox_of a)
poll_status & CL_EXTRA_PIDS+=($!)
sleep 1

ss=$(jq -rs 'map(select(.event == "SessionStart")) | .[0] // {}' "$HLOG")
check "the SessionStart hook is given the inbox socket path in its environment (a documented way to find it)" test "$(printf '%s' "$ss" | jq -r '.messaging_socket // ""')" = "$SOCK"
info "SessionStart hook: socket path matches the registry's, token present: $(printf '%s' "$ss" | jq -r '.messaging_token_present // false'), source: $(printf '%s' "$ss" | jq -r '.source // "-"')"

echo "== O5 (idle) what the screen says =="
screen_tail a 4

echo "== O1. one turn: idle -> busy -> idle, with the hook events of the same turn =="
T0=$(now_ms)
cl_inbox_send "$SOCK" 'Run the shell command `sleep 3` and then reply with exactly: observed'
cl_wait_agents '.[0].status=="busy"' 15 >/dev/null || fail "the turn started"
sleep 1; echo "== O5 (busy) what the screen says =="; screen_tail a 4
cl_wait_agents '.[0].status=="idle"' 90 >/dev/null || fail "the turn ended"
sleep 1
timeline "$T0"
info "hook events seen in the turn: $(jq -rs 'map(select(.t >= '"$T0"') | .event) | unique | join(",")' "$HLOG")"

echo "== O2. a permission wait, answered by the hub through a file =="
rm -f "$DEC"; T1=$(now_ms)
cl_inbox_send "$SOCK" 'Run the shell command `touch permission-test.txt` using the Bash tool, then reply with exactly: ran-it'
if cl_wait_agents '.[0].status=="waiting"' 30 >/dev/null; then pass "status went to waiting"; else fail "status went to waiting"; fi
info "waitingFor = $(cl_agent_field '.waitingFor')"
sleep 1; echo "== O5 (waiting for permission) what the screen says =="; screen_tail a 6
touch_ms=$(now_ms); echo allow >"$DEC"
if cl_wait_agents '.[0].status=="busy" or .[0].status=="idle"' 30 >/dev/null; then pass "the hub's file answer ended the wait ($(( $(now_ms) - touch_ms )) ms), nobody typed"; else fail "the file answer ended the wait"; fi
cl_wait_agents '.[0].status=="idle"' 60 >/dev/null || true
sleep 1
timeline "$T1"
check "the command ran after the hub's answer" test -e "$CL_WORK/permission-test.txt"
rm -f "$DEC"

echo "== O3. a question wait (AskUserQuestion) =="
T2=$(now_ms)
cl_inbox_send "$SOCK" 'Use the AskUserQuestion tool to ask which of two colours I prefer, with the options red and blue. Do nothing else.'
if cl_wait_agents '.[0].status=="waiting"' 45 >/dev/null; then pass "status went to waiting for the question"; else fail "status went to waiting for the question"; fi
info "waitingFor = $(cl_agent_field '.waitingFor')"
sleep 1; echo "== O5 (waiting for an answer) what the screen says =="; screen_tail a 8
timeline "$T2"

echo "== O3b. the same request in a pane launched with --disallowed-tools AskUserQuestion =="
CL_TARGET=q
cl_launch q --settings "$CL_DIR/settings.json" --permission-mode manual --disallowed-tools AskUserQuestion
cl_wait_agents '.[0].status=="idle"' 30 >/dev/null || fail "the pane registered as idle"
cl_inbox_send "$(cl_inbox_of q)" 'Use the AskUserQuestion tool to ask which of two colours I prefer, with the options red and blue. Do nothing else.'
seen_waiting=no
for _ in $(seq 1 60); do
  st=$(cl_agent_field '.status'); [ "$st" = waiting ] && seen_waiting=yes
  [ "$st" = idle ] && [ "$seen_waiting" = no ] && [ -n "$(cl_pane q | grep -m1 'Another Claude session sent a message')" ] && sleep 4 && break
  sleep 0.5
done
st=$(cl_agent_field '.status')
check "the turn ended without the pane ever waiting for an answer from a person" test "$seen_waiting" = no -a "$st" = idle
cl_tmux kill-session -t q 2>/dev/null || true
CL_TARGET=a

echo "== O4. a dead process (SIGKILL) =="
REG="$HOME/.claude/sessions/$PID.json"
if [ -f "$REG" ]; then
  u1=$(jq -r .updatedAt "$REG"); s1=$(jq -r .statusUpdatedAt "$REG"); sleep 6; u2=$(jq -r .updatedAt "$REG")
  info "registry updatedAt after 6 s of waiting for an answer: $([ "$u1" = "$u2" ] && echo "unchanged (no heartbeat)" || echo "moved (a heartbeat)")"
fi
ends_before=$(grep -c '"event":"SessionEnd"' "$HLOG" || true)
kill -KILL "$PID"; sleep 2
info "SIGKILL: agents entries for the pane: $(cl_agents | jq length); registry file left behind: $([ -f "$REG" ] && echo yes || echo no)"
info "hook events at the end: $(jq -r .event "$HLOG" | sort | uniq -c | tr '\n' ' ')"
check "SessionEnd did not fire for the SIGKILLed process (a dead pane leaves no end event)" test "$(grep -c '"event":"SessionEnd"' "$HLOG" || true)" = "$ends_before"

echo
if [ "$CL_FAILS" -eq 0 ]; then echo "claude-observe: all expectations held"; else echo "claude-observe: $CL_FAILS expectation(s) failed"; fi
