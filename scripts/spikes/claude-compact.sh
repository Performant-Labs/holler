#!/usr/bin/env bash
# Spike #878, part B.2: does `/compact` delivered over a Claude Code pane's inbox socket
# compact the session, which `agents --json` status shows it, and where can before/after
# token counts be read without typing?
#
# Safety contract = claude-lib.sh's, with these differences (see the header there):
#   - the scratch working directory is made INSIDE an already-trusted parent directory
#     (CL_TRUSTED_PARENT), so no trust dialog is expected; if one appears the probe stops
#     and records it, it never answers it;
#   - scratch paths are short (/tmp/hlc878-<pid>) so the unix socket paths stay short;
#   - cleanup removes only this run's tmux server, the scratch directories and the one
#     transcript directory that encodes the scratch working directory (no `claude purge`,
#     no other path under the config directory is listed or read);
#   - MODEL BUDGET: at most 3 model turns in total (seed, compaction, one follow-up).
#     The typed `/compact` control is a scratch-only person's action, counts as a turn
#     when the model is called, and is never an adapter action (invariant I4).
#
# Usage: CL_TRUSTED_PARENT=<an already trusted directory> scripts/spikes/claude-compact.sh
# shellcheck disable=SC2034 # CL_* are read by the library
set -euo pipefail
HERE="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
# shellcheck source=claude-lib.sh
source "$HERE/claude-lib.sh"

: "${CL_TRUSTED_PARENT:?set CL_TRUSTED_PARENT to an already trusted directory}"
TURNS=0
MAX_TURNS=3
SESSION_NAME=c

# Our own cleanup: only what this run made.
cl_cleanup() {
  local rc=$? left=""
  set +e
  [ -n "$CL_TMUX_SOCK" ] && cl_tmux kill-server >/dev/null 2>&1
  sleep 0.5
  [ -n "${CL_PROJECT:-}" ] && rm -rf "$CL_PROJECT"
  [ -n "$CL_WORK" ] && rm -rf "$CL_WORK"
  [ -n "$CL_DIR" ] && rm -rf "$CL_DIR"
  [ -e "${CL_PROJECT:-/nonexistent}" ] && left="$left project-dir"
  [ -e "$CL_WORK" ] && left="$left scratch-cwd"
  [ -e "$CL_DIR" ] && left="$left scratch-dir"
  if [ -n "${CL_CPID:-}" ] && kill -0 "$CL_CPID" 2>/dev/null; then left="$left claude-pid"; fi
  if [ -z "$left" ]; then echo "INFO  cleanup verified: tmux server, claude process, project transcript directory, scratch cwd and scratch dir are gone"
  else echo "FAIL  cleanup left:$left"; CL_FAILS=$((CL_FAILS + 1)); fi
  if [ "$rc" -eq 0 ] && [ "$CL_FAILS" -gt 0 ]; then rc=1; fi
  exit "$rc"
}

cl_require
CL_DIR="/tmp/hlc878-$$"
mkdir -p "$CL_DIR"
chmod 700 "$CL_DIR"
CL_WORK="$CL_TRUSTED_PARENT/scratch-878-claude"
mkdir -p "$CL_WORK"
CL_WORK="$(cd "$CL_WORK" && pwd -P)"
CL_PROJECT="$(cl_project_dir)"
CL_SOCK_DIR="$CL_DIR"
CL_TMUX_SOCK="$CL_DIR/tmux.sock"
HOOKDIR="$CL_DIR/h"
trap cl_cleanup EXIT
trap 'exit 130' INT TERM

info "claude version: $(cl_env "$CL_BIN" --version 2>/dev/null)"
info "scratch dir: <scratch-dir> (short /tmp path); scratch cwd inside a trusted parent; its transcript directory is removed on exit"
method=$(cl_env "$CL_BIN" auth status 2>/dev/null | jq -r '.authMethod // "unknown"' 2>/dev/null || echo unknown)
info "login method under the minimal environment: $method"
[ "$method" = "claude.ai" ] || { echo "no subscription login under the minimal environment; stopping" >&2; exit 2; }
info "variable NAMES passed to the probe process: HOME PATH TERM LANG HOLLER_CLAUDE_SOCKET_DIR (env -i; no CLAUDE_* or ANTHROPIC*)"

# The adapter's settings: accept cross-session messages, SessionStart hook writes the socket path.
HOOK='umask 077 && [ -n "$CLAUDE_CODE_MESSAGING_SOCKET" ] && [ -n "$HOLLER_CLAUDE_SOCKET_DIR" ] && d="$HOLLER_CLAUDE_SOCKET_DIR" && mkdir -p "$d" && chmod 700 "$d" && t="$d/.$PPID.tmp" && printf "%s\n" "$CLAUDE_CODE_MESSAGING_SOCKET" > "$t" && mv -f "$t" "$d/$PPID" || true'
jq -cn --arg c "$HOOK" '{crossSessionInbound:"accept",hooks:{SessionStart:[{hooks:[{type:"command",command:$c}]}]}}' >"$CL_DIR/settings.json"

SID="$(uuidgen | tr 'A-Z' 'a-z')"
TR="$(cl_transcript "$SID")"
CL_TARGET=$SESSION_NAME
CL_LAUNCH_ENV="HOLLER_CLAUDE_SOCKET_DIR=$HOOKDIR"
cl_launch "$SESSION_NAME" --session-id "$SID" --settings "$CL_DIR/settings.json" --disallowed-tools AskUserQuestion
CL_CPID=$(cl_pid "$SESSION_NAME")
info "claude pid started by this run: <pid>; start time $(ps -o lstart= -p "$CL_CPID" 2>/dev/null | sed 's/  */ /g'), parent is the private tmux server: $([ "$(ps -o ppid= -p "$CL_CPID" 2>/dev/null | tr -d ' ')" = "$(cl_tmux display-message -p '#{pid}')" ] && echo yes || echo no)"

if ! cl_wait_agents '.[0].status=="idle"' 60 >/dev/null; then
  if cl_pane "$SESSION_NAME" | grep -Eqi 'trust|Do you want|\(y/n\)|Yes,'; then
    fail "a dialog appeared at startup (recorded, NOT answered): $(cl_show "$SESSION_NAME" 6 | tr '\n' '|' | cut -c1-300)"
  else
    fail "the session registered as idle within 60 s: $(cl_show "$SESSION_NAME" 6 | tr '\n' '|' | cut -c1-300)"
  fi
  exit 1
fi
pass "agents --json lists the scratch session idle (no dialog)"
info "agents --json entry keys: $(cl_agents | jq -r '.[0] | keys | join(",")')"

SOCK=""
for _ in $(seq 1 40); do [ -f "$HOOKDIR/$CL_CPID" ] && break; sleep 0.25; done
if [ -f "$HOOKDIR/$CL_CPID" ]; then
  SOCK=$(head -1 "$HOOKDIR/$CL_CPID")
  pass "the SessionStart hook wrote the inbox socket path under the claude pid"
else
  fail "the SessionStart hook wrote the inbox socket path"; exit 1
fi

# Helpers ---------------------------------------------------------------------------
status() { cl_agent_field '.status'; }
sampler_start() { # logfile
  ( while :; do printf '%s %s\n' "$(now_ms)" "$(status)"; sleep 0.15; done ) >"$1" 2>/dev/null &
  SAMPLER=$!
}
sampler_stop() { kill "$SAMPLER" 2>/dev/null || true; wait "$SAMPLER" 2>/dev/null || true; }
statuses_seen() { awk '{print $2}' "$1" | uniq | tr '\n' '>' | sed 's/>$//'; }
tr_lines() { [ -f "$TR" ] && wc -l <"$TR" || echo 0; }
tr_bytes() { [ -f "$TR" ] && wc -c <"$TR" || echo 0; }
# One line per transcript entry: shape only (types, flags, usage numbers), no message text.
tr_shape() {
  [ -f "$TR" ] || return 0
  jq -c '{type, subtype, isCompactSummary, isSidechain, role: .message.role,
          usage: (.message.usage // null | if . then {in:.input_tokens, out:.output_tokens, cache_read:.cache_read_input_tokens, cache_create:.cache_creation_input_tokens} else null end),
          compact: (.compactMetadata // null)}
         | with_entries(select(.value != null))' "$TR" 2>/dev/null
}
last_usage() { # the last assistant entry's usage numbers
  jq -c 'select(.type=="assistant" and .message.usage != null) | .message.usage | {in:.input_tokens, out:.output_tokens, cache_read:.cache_read_input_tokens, cache_create:.cache_creation_input_tokens}' "$TR" 2>/dev/null | tail -1
}
model_turn() { TURNS=$((TURNS + 1)); info "model turns used so far: $TURNS of $MAX_TURNS"; }
session_now() { cl_agent_field '.sessionId'; }

# 1. Seed turn ---------------------------------------------------------------------
echo "== 1. seed turn through the inbox =="
cl_inbox_send "$SOCK" "Reply with the single word: ok"
cl_wait_agents '.[0].status=="busy"' 20 >/dev/null || fail "the seed message started a turn"
model_turn
cl_wait_agents '.[0].status=="idle"' 90 >/dev/null || fail "the seed turn finished"
check "the seed reply is in the transcript" bash -c 'grep -q "\"type\":\"assistant\"" "$0"' "$TR"
BEFORE_LINES=$(tr_lines); BEFORE_BYTES=$(tr_bytes); BEFORE_USAGE=$(last_usage)
info "after the seed: transcript $BEFORE_LINES lines, $BEFORE_BYTES bytes; last assistant usage $BEFORE_USAGE"
info "transcript entry shapes after the seed: $(tr_shape | tr '\n' ' ')"
info "session id before == launch id: $([ "$(session_now)" = "$SID" ] && echo yes || echo no)"

# 2. /compact through the inbox ---------------------------------------------------------
echo "== 2. /compact delivered through the inbox (the adapter's frame) =="
sampler_start "$CL_DIR/inbox.samples"
t0=$(now_ms)
cl_inbox_send "$SOCK" "/compact"
sleep 12
cl_wait_agents '.[0].status=="idle"' 90 >/dev/null || fail "status returned to idle after the inbox /compact"
sleep 1
sampler_stop
info "status values over time after the inbox /compact (first sample at write): $(statuses_seen "$CL_DIR/inbox.samples")"
info "all distinct status values seen: $(awk '{print $2}' "$CL_DIR/inbox.samples" | sort -u | tr '\n' ' ')"
AFTER_LINES=$(tr_lines)
info "transcript after the inbox /compact: $AFTER_LINES lines (was $BEFORE_LINES), $(tr_bytes) bytes (was $BEFORE_BYTES)"
info "new transcript entry shapes: $(tr_shape | tail -n +"$((BEFORE_LINES + 1))" | tr '\n' ' ')"
info "last assistant usage now: $(last_usage)"
info "pane after the inbox /compact: $(cl_show "$SESSION_NAME" 14 | cut -c1-140 | tr '\n' '|')"
if tr_shape | tail -n +"$((BEFORE_LINES + 1))" | grep -q '"type":"assistant"'; then
  INBOX_MODEL_REPLIED=yes; model_turn
  info "the model replied to the inbox /compact as ordinary text (it was NOT run as a command)"
else
  INBOX_MODEL_REPLIED=no
fi
if tr_shape | grep -Eq 'compact_boundary|isCompactSummary|compactMetadata'; then
  INBOX_COMPACTED=yes; pass "the inbox /compact compacted: the transcript holds a compaction boundary or summary entry"
else
  INBOX_COMPACTED=no; fail "the inbox /compact compacted the session (no boundary or summary entry in the transcript)"
fi
info "session id after the inbox /compact: $([ "$(session_now)" = "$SID" ] && echo unchanged || echo CHANGED)"

# 3. Control: a person types /compact into the scratch TUI -----------------------------------
echo "== 3. control: /compact typed by hand into the scratch TUI (a person's action, not an adapter action) =="
if [ "$INBOX_COMPACTED" = yes ]; then
  info "control skipped: the inbox /compact already compacted; no further model turn spent"
elif [ "$TURNS" -ge "$MAX_TURNS" ]; then
  info "control skipped: the turn budget ($MAX_TURNS) is spent"
else
  sampler_start "$CL_DIR/typed.samples"
  PRE=$(tr_lines)
  cl_keys "$SESSION_NAME" "/compact" Enter
  sleep 12
  cl_wait_agents '.[0].status=="idle"' 90 >/dev/null || fail "status returned to idle after the typed /compact"
  sleep 1
  sampler_stop
  model_turn
  info "status values over time after the typed /compact: $(statuses_seen "$CL_DIR/typed.samples")"
  info "all distinct status values seen (typed): $(awk '{print $2}' "$CL_DIR/typed.samples" | sort -u | tr '\n' ' ')"
  info "transcript after the typed /compact: $(tr_lines) lines (was $PRE), $(tr_bytes) bytes"
  info "new entry shapes (typed): $(tr_shape | tail -n +"$((PRE + 1))" | tr '\n' ' ')"
  info "session id after the typed /compact: $([ "$(session_now)" = "$SID" ] && echo unchanged || echo "CHANGED (same process now holds another id)")"
  info "pane after the typed /compact: $(cl_show "$SESSION_NAME" 14 | cut -c1-140 | tr '\n' '|')"
  if tr_shape | grep -Eq 'compact_boundary|isCompactSummary|compactMetadata'; then
    info "the typed /compact compacted: a boundary or summary entry exists"
  else
    info "the typed /compact left no boundary or summary entry in the transcript"
  fi
fi

# 4. Token source ------------------------------------------------------------------------
echo "== 4. where can before/after token counts be read without typing? =="
info "agents --json fields: $(cl_agents | jq -r '.[0] | keys | join(",")') (any with token/context/usage in the name: $(cl_agents | jq -r '.[0] | keys[] | select(test("token|context|usage";"i"))' | tr '\n' ' '))"
info "compaction metadata in the transcript (numbers only): $(jq -c 'select(.compactMetadata != null or .subtype=="compact_boundary") | (.compactMetadata // .)' "$TR" 2>/dev/null | jq -c 'with_entries(select(.value|type=="number" or type=="string" and length<40))' 2>/dev/null | tail -2 | tr '\n' ' ')"
info "assistant usage entries in order: $(jq -c 'select(.type=="assistant" and .message.usage != null) | [.message.usage.input_tokens, .message.usage.cache_read_input_tokens, .message.usage.cache_creation_input_tokens, .message.usage.output_tokens]' "$TR" 2>/dev/null | tr '\n' ' ')"

# 5. Follow-up turn -----------------------------------------------------------------------
echo "== 5. one follow-up turn: does the conversation continue after compaction? =="
if [ "$TURNS" -ge "$MAX_TURNS" ]; then
  info "follow-up skipped: a fourth model turn would be needed (budget $MAX_TURNS spent)"
else
  cl_inbox_send "$SOCK" "Reply with the single word: again"
  cl_wait_agents '.[0].status=="busy"' 20 >/dev/null || fail "the follow-up started a turn"
  model_turn
  cl_wait_agents '.[0].status=="idle"' 90 >/dev/null || fail "the follow-up finished"
  info "usage after the follow-up: $(last_usage)"
  check "the follow-up reply is in the transcript" bash -c 'grep -q "\"type\":\"assistant\"" "$0"' "$TR"
fi
info "total model turns used: $TURNS of $MAX_TURNS"

echo
if [ "$CL_FAILS" -eq 0 ]; then echo "claude-compact: all expectations held"; else echo "claude-compact: $CL_FAILS expectation(s) failed (a FAIL here is an answer, see the evidence file)"; fi
