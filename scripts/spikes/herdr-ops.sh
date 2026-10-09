#!/usr/bin/env bash
# Herdr spike (issue #636): which pane operations the socket API covers, the exact call for
# each, and the result shape. Runs only in a scratch session it creates (see herdr-lib.sh).
# Usage: bash scripts/spikes/herdr-ops.sh
set -euo pipefail
# shellcheck source=scripts/spikes/herdr-lib.sh
. "$(dirname "$0")/herdr-lib.sh"

spike_start

say "version: CLI status, socket ping, schema header"
show "herdr --session <scratch-session> status server --json"
h status server --json | jq -c '{version, protocol, capabilities}'
show 'socket: {"id":"1","method":"ping","params":{}}'
hsock '{"id":"1","method":"ping","params":{}}' | jq -c .result
show "herdr api schema"
h api schema | head -4
show "herdr api schema --json   (summary)"
h api schema --json | jq -c '{protocol, schema_version, schemas: (.schemas | keys),
  request_methods: ([.schemas.request.oneOf[].properties.method.const] | length)}'
h api schema --json | jq -r '[.schemas.request.oneOf[].properties.method.const
  | select(test("^(pane|layout|events|session|workspace|tab)\\."))] | join(" ")' | fold -s -w 110

say "wire: one JSON request per line, one request per connection; an unknown method is invalid_request"
python3 -I - "$SPIKE_SOCKET" <<'PY'
import socket, sys
def one(s, req):
    s.sendall(req)
    buf = b""
    while b"\n" not in buf:
        c = s.recv(65536)
        if not c:
            return "EOF"
        buf += c
    return buf.decode().strip()[:110]
s = socket.socket(socket.AF_UNIX); s.settimeout(3); s.connect(sys.argv[1])
print("first request:", one(s, b'{"id":"1","method":"ping","params":{}}\n'))
try:
    print("second request, same connection:", one(s, b'{"id":"2","method":"workspace.list","params":{}}\n'))
except OSError as e:
    print("second request, same connection:", type(e).__name__)
t = socket.socket(socket.AF_UNIX); t.settimeout(3); t.connect(sys.argv[1])
print("unknown method:", one(t, b'{"id":"x","method":"pane.nope","params":{}}\n'))
PY

say "create: workspace.create (CLI: workspace create)"
show "herdr workspace create --label ops --cwd <scratch-root>/work --no-focus"
ws="$(h workspace create --label ops --cwd "$SPIKE_WORK" --no-focus)"
jq -c '{type: .result.type, workspace: .result.workspace.workspace_id, tab: .result.tab.tab_id,
  root_pane: .result.root_pane.pane_id, root_pane_keys: (.result.root_pane | keys)}' <<<"$ws"
W="$(jq -r .result.workspace.workspace_id <<<"$ws")"
P1="$(jq -r .result.root_pane.pane_id <<<"$ws")"

say "split: pane.split (CLI: pane split); no command parameter, the new pane runs the default shell"
show "herdr pane split $P1 --direction right --no-focus"
split="$(h pane split "$P1" --direction right --no-focus)"
jq -c '{type: .result.type, pane_id: .result.pane.pane_id, tab_id: .result.pane.tab_id, terminal_id: .result.pane.terminal_id}' <<<"$split"
P2="$(jq -r .result.pane.pane_id <<<"$split")"

say "run: the CLI pane run types text + Enter into the pane's shell; the socket equivalent is pane.send_input"
show "herdr pane run $P1 'echo ran-636'"
h pane run "$P1" 'echo ran-636'; echo "exit=$? stdout=(empty)"
show "herdr pane wait-output $P1 --match ran-636 --timeout 5000"
h pane wait-output "$P1" --match ran-636 --timeout 5000 | jq -c '{type: .result.type, matched_line: .result.matched_line, read_keys: (.result.read | keys)}'
show "socket: pane.send_input {pane_id, text: \"echo input-636\", keys: [\"enter\"]}"
hsock "{\"id\":\"i\",\"method\":\"pane.send_input\",\"params\":{\"pane_id\":\"$P1\",\"text\":\"echo input-636\",\"keys\":[\"enter\"]}}" | jq -c .
h pane wait-output "$P1" --match input-636 --timeout 5000 >/dev/null && echo "input-636 seen in the pane"

say "send text and keys: pane.send_text, pane.send_keys"
show "herdr pane send-text $P1 'echo typed-636'; herdr pane send-keys $P1 enter"
h pane send-text "$P1" 'echo typed-636'
h pane send-keys "$P1" enter
show "socket: pane.send_keys {pane_id, keys: [\"enter\"]}"
hsock "{\"id\":\"k\",\"method\":\"pane.send_keys\",\"params\":{\"pane_id\":\"$P1\",\"keys\":[\"enter\"]}}" | jq -c .
h pane wait-output "$P1" --match typed-636 --timeout 5000 >/dev/null && echo "typed-636 seen in the pane"

say "read: pane.read (CLI prints plain text; the socket returns JSON)"
show "herdr pane read $P1 --source recent --lines 4"
h pane read "$P1" --source recent --lines 4 | spike_redact
show "socket: pane.read {pane_id, source: recent, lines: 4}"
hsock "{\"id\":\"r\",\"method\":\"pane.read\",\"params\":{\"pane_id\":\"$P1\",\"source\":\"recent\",\"lines\":4}}" \
  | jq -c '.result | {type, read: (.read | del(.text)), text_lines: (.read.text | split("\n") | length)}'

say "what a pane runs: pane.process_info (CLI: pane process-info)"
h pane run "$P2" 'sleep 600'
spike_wait_for 5 sh -c "\"$SPIKE_BIN/h\" pane process-info --pane $P2 | grep -q '\"sleep\"'"
show "herdr pane process-info --pane $P2"
h pane process-info --pane "$P2" | jq -c '.result.process_info | {pane_id, shell_pid: (.shell_pid != null),
  foreground: [.foreground_processes[] | {name, argv, cwd}]}' | spike_redact

say "snapshot: session.snapshot (CLI: api snapshot)"
show "herdr api snapshot"
h api snapshot | jq -c '.result.snapshot | {version, protocol, keys: keys,
  layouts: [.layouts[] | {tab_id, area, panes: [.panes[] | {pane_id, rect}], splits: [.splits[] | {id, direction, ratio}]}]}'

say "events: events.subscribe (socket only; no CLI verb) while a pane is split, exits and is closed"
(hsock '{"id":"sub","method":"events.subscribe","params":{"subscriptions":[{"type":"pane.created"},{"type":"pane.closed"},{"type":"pane.exited"},{"type":"layout.updated"}]}}' 4 >"$SPIKE_ROOT/events.ndjson") &
sub=$!
sleep 1
P3="$(h pane split "$P2" --direction down --no-focus | jq -r .result.pane.pane_id)"
P4="$(h pane split "$P1" --direction down --no-focus | jq -r .result.pane.pane_id)"
h pane run "$P3" 'exit'
h pane close "$P4" >/dev/null
wait "$sub" || true
jq -c '{event: (.event // .result.type), pane: (.data.pane_id // .data.pane.pane_id // null)}' "$SPIKE_ROOT/events.ndjson"

say "close: pane.close (CLI: pane close); closing again is an error on stderr, exit 1"
P5="$(h pane split "$P1" --direction down --no-focus | jq -r .result.pane.pane_id)"
show "herdr pane close $P5"
h pane close "$P5"; echo " exit=$?"
show "herdr pane close $P5   (again)"
set +e; h pane close "$P5"; rc=$?; set -e; echo " exit=$rc"

say "layout.export (socket only): the tab as a split tree"
T="$(h pane get "$P1" | jq -r .result.pane.tab_id)"
hsock "{\"id\":\"e\",\"method\":\"layout.export\",\"params\":{\"tab_id\":\"$T\"}}" | jq -c .result.layout | spike_redact

say "layout.apply (socket only) into a NEW tab: argv commands run directly, without a shell"
apply="$(hsock "{\"id\":\"a\",\"method\":\"layout.apply\",\"params\":{\"workspace_id\":\"$W\",\"tab_label\":\"applied\",\"root\":{\"type\":\"split\",\"direction\":\"right\",\"ratio\":0.5,\"first\":{\"type\":\"pane\",\"label\":\"argv-sleep\",\"command\":[\"sleep\",\"601\"],\"cwd\":\"$SPIKE_WORK\"},\"second\":{\"type\":\"pane\",\"label\":\"plain\"}}}}")"
jq -c '.result.layout | {tab_id, root}' <<<"$apply" | spike_redact
AT="$(jq -r .result.layout.tab_id <<<"$apply")"
AP="$(jq -r .result.layout.root.first.pane_id <<<"$apply")"
AQ="$(jq -r .result.layout.root.second.pane_id <<<"$apply")"
h pane process-info --pane "$AP" | jq -c '.result.process_info | {pane_id, shell_is_foreground: (.shell_pid == .foreground_process_group_id), foreground: [.foreground_processes[] | {name, argv}]}'
argv_pid="$(h pane process-info --pane "$AP" | jq -r '.result.process_info.foreground_processes[0].pid')"

say "layout.apply onto an EXISTING tab_id (referencing its own pane ids) replaces the tab"
re="$(hsock "{\"id\":\"a2\",\"method\":\"layout.apply\",\"params\":{\"tab_id\":\"$AT\",\"root\":{\"type\":\"split\",\"direction\":\"down\",\"ratio\":0.5,\"first\":{\"type\":\"pane\",\"pane_id\":\"$AQ\"},\"second\":{\"type\":\"pane\",\"pane_id\":\"$AP\"}}}}")"
jq -c '.result.layout | {tab_id, panes: [.root.first.pane_id, .root.second.pane_id]}' <<<"$re"
echo "tab before: $AT  panes before: $AP $AQ"
if kill -0 "$argv_pid" 2>/dev/null; then echo "argv process still alive"; else echo "argv process (sleep 601) is gone: apply replaced the tab and killed its panes"; fi
h tab list --workspace "$W" | jq -c '[.result.tabs[] | {tab_id, label}]'

say "argv launch at a chosen place: layout.apply into a staging tab, then pane.move into the target split"
st="$(hsock "{\"id\":\"s\",\"method\":\"layout.apply\",\"params\":{\"workspace_id\":\"$W\",\"tab_label\":\"staging\",\"root\":{\"type\":\"pane\",\"label\":\"placed\",\"command\":[\"sleep\",\"602\"],\"cwd\":\"$SPIKE_WORK\"}}}")"
SP="$(jq -r .result.layout.root.pane_id <<<"$st")"
pid_before="$(h pane process-info --pane "$SP" | jq -r '.result.process_info.foreground_processes[0].pid')"
hsock "{\"id\":\"m\",\"method\":\"pane.move\",\"params\":{\"pane_id\":\"$SP\",\"destination\":{\"type\":\"tab\",\"tab_id\":\"$T\",\"split\":\"right\",\"target_pane_id\":\"$P1\",\"ratio\":0.5}}}" \
  | jq -c '.result.move_result | {changed, previous_pane_id, pane_id: .pane.pane_id, tab_id: .pane.tab_id}'
pid_after="$(h pane process-info --pane "$SP" | jq -r '.result.process_info.foreground_processes[0].pid')"
[ "$pid_before" = "$pid_after" ] && echo "same process after the move (pid unchanged)"
h tab list --workspace "$W" | jq -c '[.result.tabs[] | {tab_id, label}]'

say "agent_status: a plain shell, then reported agent states (pane.report_agent), then release"
h pane get "$P1" | jq -c '.result.pane | {pane_id, agent, agent_status}'
(hsock "{\"id\":\"sub2\",\"method\":\"events.subscribe\",\"params\":{\"subscriptions\":[{\"type\":\"pane.agent_status_changed\",\"pane_id\":\"$P1\"},{\"type\":\"pane.agent_detected\"}]}}" 3 >"$SPIKE_ROOT/agent-events.ndjson") &
sub=$!
sleep 1
for state in working blocked idle; do
  h pane report-agent "$P1" --source spike636 --agent opencode --state "$state"
  printf 'reported %-8s -> ' "$state"; h pane get "$P1" | jq -c '.result.pane | {agent, agent_status}'
done
show "herdr agent list"
h agent list | jq -c '[.result.agents[] | {pane_id, agent, agent_status, state_change_seq}]'
h pane release-agent "$P1" --source spike636 --agent opencode
printf 'released          -> '; h pane get "$P1" | jq -c '.result.pane | {agent, agent_status}'
wait "$sub" || true
jq -c '{event, data: (.data // .result)}' "$SPIKE_ROOT/agent-events.ndjson"

say "display-only metadata: pane.report_metadata, workspace.report_metadata"
h pane report-metadata "$P1" --source holler-display --title 'demo r1c1' --token pos=r1c1 --token shown=ses_demo
h workspace report-metadata "$W" --source holler-display --token profile=demo
h pane get "$P1" | jq -c '.result.pane | {pane_id, title, tokens, revision}'
h workspace get "$W" | jq -c '.result.workspace | {workspace_id, tokens}'
