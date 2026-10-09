#!/usr/bin/env bash
# Herdr spike (issue #636): are pane ids stable across relayouts, a client attach and detach,
# and a full server restart, and how is the saved layout restored? Runs only in a scratch
# session it creates (see herdr-lib.sh). The attach step needs tmux and uses a private tmux
# server (tmux -L <scratch-session>), never the default one; without tmux it is skipped.
# Usage: bash scripts/spikes/herdr-restart.sh
set -euo pipefail
# shellcheck source=scripts/spikes/herdr-lib.sh
. "$(dirname "$0")/herdr-lib.sh"

spike_start

# The identity view: every pane's id, tab and label, and each tab's tree shape (ids only).
ids() {
  local snap trees="" t
  snap="$(h api snapshot)"
  for t in $(jq -r '.result.snapshot.tabs[].tab_id' <<<"$snap"); do
    trees+="$(hsock "{\"id\":\"e\",\"method\":\"layout.export\",\"params\":{\"tab_id\":\"$t\"}}" | jq -c '
      def shape: if .type == "pane" then .pane_id else {(.direction): [(.first|shape), (.second|shape)]} end;
      {(.result.layout.tab_id): (.result.layout.root | shape)}')"
  done
  jq -c '[.result.snapshot.panes[] | {pane_id, tab_id, label}] | sort_by(.pane_id)' <<<"$snap"
  printf '%s\n' "$trees" | jq -sc 'add'
}
area() { h api snapshot | jq -c '[.result.snapshot.layouts[] | {tab_id, area}]'; }

say "setup: two tabs, one pane started from an argv command (layout.apply), metadata and an agent report"
ws="$(h workspace create --label restart --cwd "$SPIKE_WORK" --no-focus)"
W="$(jq -r .result.workspace.workspace_id <<<"$ws")"
P1="$(jq -r .result.root_pane.pane_id <<<"$ws")"; h pane rename "$P1" first >/dev/null
P2="$(h pane split "$P1" --direction right --no-focus | jq -r .result.pane.pane_id)"; h pane rename "$P2" second >/dev/null
P3="$(h pane split "$P2" --direction down --no-focus | jq -r .result.pane.pane_id)"; h pane rename "$P3" third >/dev/null
A1="$(hsock "{\"id\":\"a\",\"method\":\"layout.apply\",\"params\":{\"workspace_id\":\"$W\",\"tab_label\":\"argv-tab\",\"root\":{\"type\":\"pane\",\"label\":\"argv\",\"command\":[\"sleep\",\"700\"],\"cwd\":\"$SPIKE_WORK\"}}}" | jq -r .result.layout.root.pane_id)"
h pane run "$P1" 'sleep 701'
h pane report-metadata "$P1" --source holler-display --title 'first r1c1' --token pos=r1c1
h workspace report-metadata "$W" --source holler-display --token profile=demo
h pane report-agent "$P2" --source spike636 --agent opencode --state working
base="$(ids)"; echo "$base"

say "relayout: resize, zoom on and off, swap, then close one pane"
hsock "{\"id\":\"r\",\"method\":\"pane.resize\",\"params\":{\"pane_id\":\"$P1\",\"direction\":\"right\",\"amount\":10}}" | jq -c '.result.resize.changed'
hsock "{\"id\":\"z\",\"method\":\"pane.zoom\",\"params\":{\"pane_id\":\"$P2\",\"mode\":\"on\"}}" | jq -c '.result.zoom | {changed}'
hsock "{\"id\":\"z\",\"method\":\"pane.zoom\",\"params\":{\"pane_id\":\"$P2\",\"mode\":\"off\"}}" | jq -c '.result.zoom | {changed}'
hsock "{\"id\":\"s\",\"method\":\"pane.swap\",\"params\":{\"source_pane_id\":\"$P2\",\"target_pane_id\":\"$P3\"}}" | jq -c '.result.swap.changed'
h pane close "$P3" >/dev/null
after_relayout="$(ids)"; echo "$after_relayout"
echo "ids of the surviving panes unchanged: $(jq -nc --argjson a "$(head -1 <<<"$base")" --argjson b "$(head -1 <<<"$after_relayout")" --arg gone "$P3" \
  '([$a[] | select(.pane_id != $gone)] == $b)')"

say "client attach and detach (a TUI client in a private tmux server)"
if command -v tmux >/dev/null; then
  before_attach="$(ids)"; echo "area before: $(area)"
  tmux -L "$SPIKE_TMUX" -f /dev/null new-session -d -s view -x 150 -y 45 "$SPIKE_BIN/h"
  spike_wait_for 10 sh -c "\"$SPIKE_BIN/h\" api snapshot | grep -qv '\"width\":120'" || true
  sleep 1
  echo "area attached: $(area)"
  attached="$(ids)"
  tmux -L "$SPIKE_TMUX" kill-server
  sleep 1
  detached="$(ids)"
  echo "ids and tree identical before attach / attached / after detach: $([ "$before_attach" = "$attached" ] && [ "$attached" = "$detached" ] && echo yes || echo NO)"
else
  echo "tmux not installed: attach check skipped"
fi

say "full server restart"
pids="$(for p in "$P1" "$A1"; do h pane process-info --pane "$p" | jq -c '.result.process_info | {pane_id, pid: .foreground_processes[0].pid, argv: .foreground_processes[0].argv}'; done)"
echo "$pids"
terms="$(h api snapshot | jq -c '[.result.snapshot.panes[] | {pane_id, terminal_id}]')"
sess_dir="$(dirname "$SPIKE_SOCKET")"
show "herdr --session <scratch-session> server stop"
spike_server_stop
echo "saved state: $(cd "$sess_dir" && for f in *; do case "$f" in *.sock|*.log) ;; *) printf '%s ' "$f" ;; esac; done)"
jq -c '{version, workspace_keys: (.workspaces[0] | keys), next_public_pane_number: .workspaces[0].next_public_pane_number,
  panes: [.workspaces[0].tabs[] | .panes | to_entries[] | {n: .key, v: (.value | keys)}]}' "$sess_dir/session.json"
for p in $(jq -r '.pid' <<<"$pids"); do kill -0 "$p" 2>/dev/null && echo "pid $p STILL ALIVE" || echo "pid $p ended with the server"; done
spike_start
after_restart="$(ids)"; echo "$after_restart"
echo "ids, labels and tree identical across the restart: $([ "$after_relayout" = "$after_restart" ] && echo yes || echo NO)"
for p in "$P1" "$A1"; do h pane process-info --pane "$p" | jq -c '.result.process_info | {pane_id, argv: .foreground_processes[0].argv}'; done
echo "terminal ids changed: $(jq -nc --argjson a "$terms" --argjson b "$(h api snapshot | jq -c '[.result.snapshot.panes[] | {pane_id, terminal_id}]')" '$a != $b')"
h pane get "$P1" | jq -c '.result.pane | {pane_id, label, title, tokens, restore_error}'
h pane get "$P2" | jq -c '.result.pane | {pane_id, agent, agent_status}'
h workspace get "$W" | jq -c '.result.workspace | {workspace_id, tokens}'
hsock "{\"id\":\"e\",\"method\":\"layout.export\",\"params\":{\"pane_id\":\"$A1\"}}" | jq -c '.result.layout.root | {pane_id, label, command}'
echo "next split gets: $(h pane split "$P1" --direction down --no-focus | jq -r .result.pane.pane_id) (closed ids are not reused)"
