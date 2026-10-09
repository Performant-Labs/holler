#!/usr/bin/env bash
# OpenCode pane spike (issue #635), capabilities 1 and 5: a headless `opencode serve`
# on which Holler creates the session of record through the HTTP API, lists it,
# reads it, aborts it (idle and busy) and deletes it. No model call, no real port,
# pane or session: see opencode-lib.sh for the safety contract.
#
#   bash scripts/spikes/opencode-api.sh
set -euo pipefail
# shellcheck source=scripts/spikes/opencode-lib.sh
. "$(dirname "${BASH_SOURCE[0]}")/opencode-lib.sh"

oc_init
PORT=$(oc_free_port)

# ---- 1. headless serve -------------------------------------------------------
oc_serve "$PORT"
pass "opencode serve --pure --port <port> --hostname 127.0.0.1 is healthy after ${OC_BOOT_MS} ms"
oc_http GET "$PORT" /global/health
info "GET /global/health -> $HTTP_CODE $HTTP_BODY"
check "GET /global/health reports healthy:true and a version" \
  test "$(printf '%s' "$HTTP_BODY" | jq -r '.healthy and (.version|length>0)')" = "true"
oc_guard_no_model "$PORT"

# The server publishes its OpenAPI document; the operations the adapter calls must be in it.
oc_http GET "$PORT" /doc
for op in global.health session.create session.list session.get session.update session.delete \
  session.status session.abort tui.selectSession tui.showToast; do
  check "GET /doc lists operation $op" \
    test "$(printf '%s' "$HTTP_BODY" | jq --arg op "$op" '[.paths[][] | objects | select(.operationId==$op)] | length')" = 1
done

oc_http GET "$PORT" /session
check "a fresh server has no session of its own (no stray 'ping' session)" \
  test "$(printf '%s' "$HTTP_BODY" | jq 'length')" = "0"

# ---- create the session of record ---------------------------------------------
oc_http POST "$PORT" /session '{"title":"spike-record"}'
SID=$(printf '%s' "$HTTP_BODY" | jq -r '.id // empty')
check "POST /session {title} -> 200 with an id starting ses_ (got $HTTP_CODE)" \
  test "$HTTP_CODE" = 200 -a "${SID#ses_}" != "$SID"
check "the new session's directory is the server's working directory" \
  test "$(printf '%s' "$HTTP_BODY" | jq -r '.directory')" = "$OC_DIR/proj"
check "the new session keeps the title Holler gave it" \
  test "$(printf '%s' "$HTTP_BODY" | jq -r '.title')" = "spike-record"

oc_http POST "$PORT" "/session?directory=$OC_DIR/proj" '{"title":"spike-other"}'
SID2=$(printf '%s' "$HTTP_BODY" | jq -r '.id // empty')
check "POST /session?directory=<dir> also creates a session (got $HTTP_CODE)" test -n "$SID2"

oc_http GET "$PORT" /session
check "GET /session lists exactly the two sessions Holler created" \
  test "$(printf '%s' "$HTTP_BODY" | jq -c '[.[].id]|sort')" = "$(jq -nc --arg a "$SID" --arg b "$SID2" '[$a,$b]|sort')"
oc_http GET "$PORT" "/session/$SID"
check "GET /session/:id -> 200 for the session of record" test "$HTTP_CODE" = 200
oc_http GET "$PORT" /session/status
info "GET /session/status with nothing running -> $HTTP_CODE $HTTP_BODY (idle sessions are absent)"
oc_http GET "$PORT" /api/session/active
info "GET /api/session/active -> $HTTP_CODE $HTTP_BODY"

oc_http PATCH "$PORT" "/session/$SID" "{\"title\":\"$SID\"}"
check "PATCH /session/:id {title} renames the session (got $HTTP_CODE)" \
  test "$(printf '%s' "$HTTP_BODY" | jq -r '.title')" = "$SID"

# ---- 5. abort -----------------------------------------------------------------
oc_http POST "$PORT" "/session/$SID/abort"
check "POST /session/:id/abort on an idle session -> 200 true (got $HTTP_CODE $HTTP_BODY)" \
  test "$HTTP_CODE" = 200 -a "$HTTP_BODY" = "true"
oc_http POST "$PORT" "/session/ses_doesnotexist0000000000000/abort"
info "POST /session/<unknown id>/abort -> $HTTP_CODE $HTTP_BODY"
oc_http POST "$PORT" "/api/session/$SID/interrupt"
info "POST /api/session/:id/interrupt (v2 API) on an idle session -> $HTTP_CODE $HTTP_BODY"

# Make the session busy without a model: /session/:id/shell runs a shell command in the
# session (the TUI's `!` command). A unique sleep lets us find exactly our child.
MARK="37.$((RANDOM % 900 + 100))"
(curl -s -m 60 -X POST -H 'content-type: application/json' \
  -d "{\"agent\":\"build\",\"command\":\"sleep $MARK\"}" -w $'\n%{http_code}' \
  "http://127.0.0.1:$PORT/session/$SID/shell" >"$OC_DIR/logs/shell.out" 2>&1; printf '\ndone\n' >>"$OC_DIR/logs/shell.out") &
SHELL_CURL=$!
busy=""
for _ in $(seq 1 50); do
  oc_http GET "$PORT" /session/status
  busy=$(printf '%s' "$HTTP_BODY" | jq -r --arg s "$SID" '.[$s].type // empty')
  [ "$busy" = busy ] && break
  sleep 0.2
done
if [ "$busy" != busy ]; then
  wait "$SHELL_CURL" || true
  info "shell command did not make the session busy; shell answered: $(tr '\n' ' ' <"$OC_DIR/logs/shell.out" | cut -c1-300)"
  fail "a session can be made busy without a model (abort on a busy session is UNVERIFIED)"
else
  pass "POST /session/:id/shell {agent,command:'sleep $MARK'} makes /session/status report busy (no model)"
  child=""
  for _ in $(seq 1 50); do
    child=$(pgrep -f "^([^ ]*/)?sleep $MARK" | head -1 || true)
    [ -n "$child" ] && break
    sleep 0.1
  done
  check "the shell command is really running (sleep child found)" test -n "$child"
  info "the shell child pid ${child:-?} is in process group $(ps -o pgid= -p "${child:-1}" 2>/dev/null | tr -d ' '), not the server's ($OC_LAST_PID), so a kill of the server's group alone would not reach it"
  t0=$(now_ms)
  oc_http POST "$PORT" "/session/$SID/abort"
  check "POST /session/:id/abort on a busy session -> 200 true (got $HTTP_CODE $HTTP_BODY)" \
    test "$HTTP_CODE" = 200 -a "$HTTP_BODY" = "true"
  idle=""
  for _ in $(seq 1 50); do
    oc_http GET "$PORT" /session/status
    idle=$(printf '%s' "$HTTP_BODY" | jq -r --arg s "$SID" '.[$s].type // "idle"')
    [ "$idle" = idle ] && break
    sleep 0.1
  done
  check "after abort the session leaves /session/status (idle) within $(( $(now_ms) - t0 )) ms" test "$idle" = idle
  wait "$SHELL_CURL" || true
  info "the shell call returned HTTP $(sed -n 2p "$OC_DIR/logs/shell.out"); its tool output: $(head -1 "$OC_DIR/logs/shell.out" | jq -c '[.parts[]?|select(.type=="tool")|.state.output]' 2>/dev/null | cut -c1-120)"
  sleep 0.5
  if pgrep -f "^([^ ]*/)?sleep $MARK" >/dev/null; then
    fail "abort kills the shell child process (sleep $MARK still running)"
  else
    pass "abort kills the shell child process"
  fi
fi

# ---- delete ----------------------------------------------------------------------
oc_http DELETE "$PORT" "/session/$SID2"
check "DELETE /session/:id -> 200 true (got $HTTP_CODE $HTTP_BODY)" test "$HTTP_CODE" = 200 -a "$HTTP_BODY" = "true"
oc_http GET "$PORT" "/session/$SID2"
check "GET /session/:id after delete -> 404 (got $HTTP_CODE)" test "$HTTP_CODE" = 404
info "404 body: $HTTP_BODY"
oc_http DELETE "$PORT" "/session/$SID2"
info "DELETE of an already deleted session -> $HTTP_CODE $HTTP_BODY"

# ---- persistence: sessions survive a server restart on the same data dir ------
oc_kill_group "$OC_LAST_PID" TERM
for _ in $(seq 1 50); do kill -0 "$OC_LAST_PID" 2>/dev/null || break; sleep 0.1; done
PORT2=$(oc_free_port)
oc_serve "$PORT2"
oc_http GET "$PORT2" "/session/$SID"
check "the session of record survives a server restart (same XDG data dir, new port)" test "$HTTP_CODE" = 200
info "restart to healthy took ${OC_BOOT_MS} ms (warm)"
