#!/usr/bin/env bash
# Herdr spike (issue #636): what a plugin is, what it can call, and what can be shown in the
# sidebar. The plugin is written into the scratch root and linked into the scratch session's
# own config home only; nothing is installed into a real Herdr config. The sidebar check needs
# tmux and uses a private tmux server (tmux -L <scratch-session>); without tmux it is skipped.
# Usage: bash scripts/spikes/herdr-plugin.sh
set -euo pipefail
# shellcheck source=scripts/spikes/herdr-lib.sh
. "$(dirname "$0")/herdr-lib.sh"

spike_start

ws="$(h workspace create --label plugins --cwd "$SPIKE_WORK" --no-focus)"
W="$(jq -r .result.workspace.workspace_id <<<"$ws")"
P1="$(jq -r .result.root_pane.pane_id <<<"$ws")"

say "a minimal plugin: herdr-plugin.toml with one action and one event hook"
PLUG="$SPIKE_ROOT/plugin"; OUT="$SPIKE_ROOT/plugin-out"; mkdir -p "$PLUG" "$OUT"
cat >"$PLUG/herdr-plugin.toml" <<EOF
id = "spike636-display"
name = "spike636-display"
version = "0.0.1"
min_herdr_version = "0.9.1"
description = "spike: what a plugin process receives and may call"

[[actions]]
id = "probe"
title = "Probe the plugin environment"
contexts = ["global"]
# Records the NAMES of the HERDR_* variables it gets, then makes one mutating API call (a split).
command = ["/bin/sh", "-c", "env | grep '^HERDR_' | sed 's/=.*//' | sort > $OUT/env-names; herdr pane split $P1 --direction down --no-focus > $OUT/split.json 2>&1"]

[[events]]
on = "pane.created"
command = ["/bin/sh", "-c", "echo pane.created >> $OUT/events"]
EOF
sed "s#$SPIKE_ROOT#<scratch-root>#g" "$PLUG/herdr-plugin.toml"
show "herdr plugin link <scratch-root>/plugin"
h plugin link "$PLUG" | jq -c '.result.plugin | {plugin_id, enabled, actions: [.actions[].id], events: [.events[].on], warnings}'
h plugin list --json | jq -c '[.result.plugins[] | {plugin_id, version, min_herdr_version}]'

say "invoke the action: the plugin process gets the session socket and can change panes"
h plugin action invoke probe --plugin spike636-display | jq -c '.result | {type, context: (.context | keys)}'
spike_wait_for 10 test -s "$OUT/split.json" || true
echo "HERDR_* variable names given to the plugin process:"; tr '\n' ' ' <"$OUT/env-names"; echo
echo "the plugin's own 'herdr pane split' result: $(jq -c '{type: .result.type, new_pane: .result.pane.pane_id}' "$OUT/split.json")"
spike_wait_for 5 test -s "$OUT/events" || true
echo "event hook ran for: $(sort "$OUT/events" 2>/dev/null | uniq -c | tr -s ' ' | tr '\n' ' ')"

say "sidebar: display-only metadata tokens rendered by [ui.sidebar.*] rows in the scratch config"
grep -A1 'ui.sidebar' "$SPIKE_HOME/.config/herdr/config.toml"
h pane report-agent "$P1" --source spike636 --agent opencode --state working
h pane report-metadata "$P1" --source holler-display --token pos=r1c1 --token shown=ses_demo
h workspace report-metadata "$W" --source holler-display --token profile=demo-profile
if command -v tmux >/dev/null; then
  tmux -L "$SPIKE_TMUX" -f /dev/null new-session -d -s view -x 150 -y 30 "$SPIKE_BIN/h"
  spike_wait_for 10 sh -c "tmux -L \"$SPIKE_TMUX\" capture-pane -p -t view | grep -q ses_demo" || true
  echo "sidebar column of the attached TUI (first 26 columns, non-blank lines):"
  tmux -L "$SPIKE_TMUX" capture-pane -p -t view \
    | python3 -I -c 'import sys; [print(c) for c in (l.rstrip("\n")[:26] for l in sys.stdin) if c.strip(" │")]' | spike_redact
  tmux -L "$SPIKE_TMUX" kill-server
else
  echo "tmux not installed: sidebar capture skipped"
fi
