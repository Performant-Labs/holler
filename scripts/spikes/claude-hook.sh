#!/usr/bin/env bash
# A command hook for spike #830: records what Claude Code tells a hook, and (for a
# PermissionRequest) lets the hub answer through a file instead of a keypress.
#
# Registered by claude-observe.sh through --settings, never in anybody's real settings.
#
# Usage (as the hook command):  claude-hook.sh LOG EVENT [DECISION_FILE]
#   LOG            file to append one JSON line per event to
#   EVENT          the hook event name, for the log line
#   DECISION_FILE  PermissionRequest only: wait up to 25 s for this file to exist, then
#                  answer with its content ("allow" or "deny")
set -uo pipefail
LOG="${1:?log}"; EVENT="${2:?event}"; DECISION="${3:-}"
IN="$(cat)"
now_ms() { echo $(( $(date +%s%N) / 1000000 )); }
# The socket path is exported to hooks before SessionStart. Only the path and whether a
# token is present are logged: the token itself is never written anywhere.
printf '%s' "$IN" | jq -c --arg ev "$EVENT" --arg t "$(now_ms)" \
  --arg sock "${CLAUDE_CODE_MESSAGING_SOCKET:-}" --arg tok "${CLAUDE_CODE_MESSAGING_TOKEN:+yes}" '{
  t: ($t | tonumber), event: $ev,
  messaging_socket: (if $sock == "" then null else $sock end),
  messaging_token_present: (if $tok == "yes" then true else null end),
  hook_event_name, session_id, permission_mode,
  notification_type, message, tool_name, source, reason, prompt,
  stop_hook_active, last_assistant_message
} | with_entries(select(.value != null))' >>"$LOG" 2>/dev/null || true

if [ "$EVENT" = "PermissionRequest" ] && [ -n "$DECISION" ]; then
  for _ in $(seq 1 100); do
    if [ -f "$DECISION" ]; then
      verdict="$(tr -d '[:space:]' <"$DECISION")"
      [ "$verdict" = "deny" ] || verdict="allow"
      jq -cn --arg b "$verdict" '{hookSpecificOutput:{hookEventName:"PermissionRequest",decision:{behavior:$b}}}'
      printf '{"t":%s,"event":"%s-answered","verdict":"%s"}\n' "$(now_ms)" "$EVENT" "$verdict" >>"$LOG"
      exit 0
    fi
    sleep 0.25
  done
fi
exit 0
