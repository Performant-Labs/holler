#!/usr/bin/env bash
# A PreToolUse command hook for spike #830: the hub's abort at the next tool boundary.
#
# Usage (as the hook command):  claude-abort-hook.sh LOG FLAG_FILE
# Logs every tool call it is asked about. While FLAG_FILE exists it answers
# {"continue": false, "stopReason": ...}, which ends the turn instead of running the tool.
set -uo pipefail
LOG="$1"; FLAG="$2"
IN="$(cat)"
now_ms() { echo $(( $(date +%s%N) / 1000000 )); }
verdict=run
if [ -e "$FLAG" ]; then verdict=stop; fi
printf '%s' "$IN" | jq -c --arg t "$(now_ms)" --arg v "$verdict" '{t: ($t|tonumber), event:"PreToolUse", tool_name, verdict:$v}' >>"$LOG" 2>/dev/null || true
if [ "$verdict" = stop ]; then
  jq -cn '{continue:false, stopReason:"stopped by the hub"}'
fi
exit 0
