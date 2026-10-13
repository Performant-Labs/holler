#!/usr/bin/env bash
# OpenCode compaction spike (issue #878, brief Part B.1): what `POST /session/:id/summarize`
# and `POST /api/session/:id/compact` do against a running `opencode serve`, with no
# real model: the scratch config has ONE provider whose base URL is a local stub
# (opencode-stub-model.py) with a fixed `usage` block and a fixed summary text, a switch
# that holds a reply open, a switch that answers 500, and a log of each request's `model`.
#
#   bash scripts/spikes/opencode-compact.sh
#   OC_FIXTURE_DIR=docs/research/compaction-spike-fixtures/opencode bash scripts/spikes/opencode-compact.sh
#
# The safety contract is opencode-lib.sh's (env -i, scratch HOME and XDG_*, ports
# 127.0.0.1:48100-48199, a private tmux socket, an EXIT trap). The one difference: its
# dead-end provider is replaced by the stub, and oc_guard_stub_only refuses to go on
# unless the running server's GET /config shows the stub as the only provider. Compaction
# destroys history, so it only ever runs on sessions this probe created. No key, token
# or login value exists in this run (the stub needs none) or is printed.
#
# Every finding is for the version printed on the first line (`opencode --version`).
set -euo pipefail
# shellcheck source=scripts/spikes/opencode-lib.sh
. "$(dirname "${BASH_SOURCE[0]}")/opencode-lib.sh"

HERE="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
STUB_PY="$HERE/opencode-stub-model.py"
OC_HTTP_TIMEOUT=60
oc_init
FIX="${OC_FIXTURE_DIR:-}"
[ -z "$FIX" ] || mkdir -p "$FIX"

# ---- helpers -------------------------------------------------------------------
# Replace ids, the scratch dir and ports so output and fixtures name nothing real.
# Ids keep their order of appearance (ses_<id1>, ses_<id2> ...) so equality stays visible.
sanitize() {
  OC_DIR_S="$OC_DIR" python3 -I -c '
import os, re, sys
t = sys.stdin.read()
for d in (os.environ["OC_DIR_S"], os.path.realpath(os.environ["OC_DIR_S"])):
    t = t.replace(d, "<scratch-dir>")
seen = {}
def sub(m):
    k = m.group(0)
    seen.setdefault(k, len(seen) + 1)
    return "%s_<id%d>" % (m.group(1), seen[k])
t = re.sub(r"\b(ses|msg|prt)_[A-Za-z0-9]+", sub, t)
t = re.sub(r"(127\.0\.0\.1:)(4[0-9]{4})", r"\1<port>", t)
t = t.replace(os.path.basename(os.environ["OC_DIR_S"]), "<scratch-dir>")
t = re.sub(r"(\"(?:slug|username)\":)\"[^\"]*\"", "\\1\x22<name>\x22", t)
sys.stdout.write(t)'
}

# record <file-stem> : sanitize stdin, keep as fixture when OC_FIXTURE_DIR is set, echo nothing.
record() {
  local stem="$1" t
  t=$(sanitize)
  [ -z "$FIX" ] || printf '%s\n' "$t" >"$FIX/$stem.json"
  return 0
}

stub_ctl() { curl -s -m 5 -X POST -H 'content-type: application/json' -d "$1" "http://127.0.0.1:$SP/_ctl" >/dev/null; }
stub_n() { cat "$OC_DIR/logs/stub.log" 2>/dev/null | wc -l | tr -d ' '; }
stub_line() { sed -n "${1}p" "$OC_DIR/logs/stub.log"; } # the nth request, verbatim
stub_since() { tail -n +"$(($1 + 1))" "$OC_DIR/logs/stub.log" 2>/dev/null | jq -c '{kind,model,nmsg,held,failed,keys}' | tr '\n' ' '; }
stub_summary_model() { tail -n +"$(($1 + 1))" "$OC_DIR/logs/stub.log" 2>/dev/null | jq -r 'select(.kind=="summary") | .model' | head -1; }

write_config() { # with-pin(0|1)
  local pin='' lim='"limit":{"context":200000,"output":4000}'
  [ "$1" = 1 ] && pin='"agent":{"compaction":{"model":"stubprov/stub-compact"}},'
  cat >"$OC_DIR/config/opencode/opencode.json" <<JSON
{"\$schema":"https://opencode.ai/config.json","enabled_providers":["stubprov"],"disabled_providers":["opencode"],
 "model":"stubprov/stub-main","small_model":"stubprov/stub-main","autoupdate":false,"share":"disabled",
 $pin
 "provider":{"stubprov":{"npm":"@ai-sdk/openai-compatible","name":"stub model","options":{"baseURL":"http://127.0.0.1:$SP/v1"},
   "models":{"stub-main":{"name":"main",$lim},"stub-compact":{"name":"compact",$lim},"stub-other":{"name":"other",$lim}}}}}
JSON
}

# The analogue of oc_guard_no_model: the stub is the only provider and it is on loopback.
oc_guard_stub_only() { # port
  oc_http GET "$1" /config
  local got want ids
  got=$(printf '%s' "$HTTP_BODY" | jq -c '[.provider // {} | to_entries[] | {id: .key, url: .value.options.baseURL}]' 2>/dev/null || echo "?")
  want="[{\"id\":\"stubprov\",\"url\":\"http://127.0.0.1:$SP/v1\"}]"
  oc_http GET "$1" /config/providers
  ids=$(printf '%s' "$HTTP_BODY" | jq -c '[.providers[].id]' 2>/dev/null || echo "?")
  if [ "$got" = "$want" ] && [ "$ids" = '["stubprov"]' ]; then
    pass "guard: GET /config and /config/providers show the stub (127.0.0.1:<port>) as the only provider (no real model can be reached)"
  else
    fail "guard: unexpected providers $got / $ids; refusing to go on"
    exit 1
  fi
}

start_server() { # sets PORT, SRV_PID
  PORT=$(oc_free_port)
  oc_serve "$PORT"
  SRV_PID=$OC_LAST_PID
  oc_guard_stub_only "$PORT"
}

stop_server() {
  oc_kill_group "$SRV_PID" TERM
  local i
  for i in $(seq 1 25); do kill -0 -- "-$SRV_PID" 2>/dev/null || break; sleep 0.2; done
  oc_kill_group "$SRV_PID" KILL
}

turn() { # sid text -> HTTP_CODE
  oc_http POST "$PORT" "/session/$1/message" \
    "{\"model\":{\"providerID\":\"stubprov\",\"modelID\":\"stub-main\"},\"parts\":[{\"type\":\"text\",\"text\":\"$2\"}]}"
}

# New session with three short turns against the stub: sets SID.
mk_session() { # title
  oc_http POST "$PORT" /session "{\"title\":\"$1\"}"
  SID=$(printf '%s' "$HTTP_BODY" | jq -r '.id // empty')
  [ -n "$SID" ] || { fail "create session $1"; exit 1; }
  local i
  for i in 1 2 3; do
    turn "$SID" "turn $i, reply with a short acknowledgement"
    [ "$HTTP_CODE" = 200 ] || { fail "turn $i of $1 -> $HTTP_CODE"; exit 1; }
  done
}

msgs() { oc_http GET "$PORT" "/session/$1/message"; MSGS="$HTTP_BODY"; }
msgs_shape() { # sid -> compact one-line shape of the message list
  msgs "$1"
  SHAPE=$(printf '%s' "$MSGS" | jq -c '[.[] | {role: .info.role, mode: .info.mode, summary: .info.summary, model: .info.modelID, total: .info.tokens.total, parts: [.parts[].type]}]')
  printf '%s\n' "$SHAPE"
}
last_assistant_tokens() { printf '%s' "$MSGS" | jq -c '[.[] | select(.info.role=="assistant")] | last | .info.tokens'; }
ids_of() { printf '%s' "$MSGS" | jq -c '[.[].info.id]'; }

# Full call with headers and timing. Sets F_CODE F_CT F_BODY F_MS.
full() { # method path [json-body] [timeout]
  local m="$1" path="$2" body="${3:-}" to="${4:-$OC_HTTP_TIMEOUT}" t0 hf bf out
  hf="$OC_DIR/logs/h.hdr"; bf="$OC_DIR/logs/b.body"; rm -f "$hf" "$bf"
  t0=$(now_ms)
  if [ -n "$body" ]; then
    out=$(curl -s -m "$to" -X "$m" -H 'content-type: application/json' -d "$body" -D "$hf" -o "$bf" -w '%{http_code}' "http://127.0.0.1:$PORT$path" 2>/dev/null || true)
  else
    out=$(curl -s -m "$to" -X "$m" -D "$hf" -o "$bf" -w '%{http_code}' "http://127.0.0.1:$PORT$path" 2>/dev/null || true)
  fi
  F_MS=$(( $(now_ms) - t0 ))
  F_CODE="${out:-000}"
  F_CT=$(grep -i '^content-type:' "$hf" 2>/dev/null | head -1 | tr -d '\r' | sed 's/^[^:]*: *//' || true)
  F_BODY=$(cat "$bf" 2>/dev/null || true)
}

# One summarize case on a fresh session: label, json body.
SUM_CASE() { # label body
  local label="$1" body="$2"
  mk_session "case-$label"
  msgs "$SID"
  CASE_N0=$(stub_n)
  full POST "/session/$SID/summarize" "$body"
  info "summarize[$label] request body $body -> $F_CODE content-type='${F_CT:-none}' after ${F_MS} ms; reply body: $(printf '%s' "$F_BODY" | sanitize)"
  info "summarize[$label] stub requests made by the call: $(stub_since "$CASE_N0")"
  printf '%s' "$F_BODY" | record "summarize-$label-reply"
  CASE_CODE="$F_CODE"; CASE_SID="$SID"
}

# ================================================================================
# 0. the stub, the version, and the routes
# ================================================================================
VER=$(oc_env "$OC_BIN" --version 2>/dev/null)
SP=$(oc_free_port)
python3 -I "$STUB_PY" "$SP" "$OC_DIR/logs/stub.log" >"$OC_DIR/logs/stub.out" 2>&1 </dev/null &
OC_EXTRA_PIDS+=($!)
for _ in $(seq 1 50); do (exec 3<>"/dev/tcp/127.0.0.1/$SP") 2>/dev/null && break; sleep 0.1; done
check "the stub model server listens on 127.0.0.1:<port> in 48100-48199" test "$SP" -ge 48100 -a "$SP" -le 48199
info "tested version: opencode $VER (the adapter was built against 1.19.0; every finding below is for $VER)"

write_config 0
start_server # unpinned server first: cases (a), (c), (d)
oc_http GET "$PORT" /doc
DOC="$HTTP_BODY"
has_sum=$(printf '%s' "$DOC" | jq '[.paths["/session/{sessionID}/summarize"].post // empty] | length' 2>/dev/null || echo 0)
has_cmp=$(printf '%s' "$DOC" | jq '[.paths["/api/session/{sessionID}/compact"].post // empty] | length' 2>/dev/null || echo 0)
check "OpenAPI lists POST /session/{sessionID}/summarize (session.summarize)" test "$has_sum" = 1
check "OpenAPI lists POST /api/session/{sessionID}/compact (v2.session.compact)" test "$has_cmp" = 1
if [ "$has_sum" != 1 ]; then
  info "summarize is absent from opencode $VER: the dependent steps are skipped"
  exit 0
fi
info "route schema, summarize (verbatim): $(printf '%s' "$DOC" | jq -c '.paths["/session/{sessionID}/summarize"].post | {operationId, requestBody: .requestBody.content["application/json"].schema, responses: (.responses | map_values(.description))}')"
info "route schema, summarize 200 content (verbatim): $(printf '%s' "$DOC" | jq -c '.paths["/session/{sessionID}/summarize"].post.responses["200"].content')"
if [ "$has_cmp" = 1 ]; then
  info "route schema, compact (verbatim): $(printf '%s' "$DOC" | jq -c '.paths["/api/session/{sessionID}/compact"].post | {operationId, requestBody: (.requestBody // "none"), responses: (.responses | map_values(.description))}')"
fi
oc_http GET "$PORT" /config
info "GET /config on the unpinned server, agent block: $(printf '%s' "$HTTP_BODY" | jq -c '.agent // "absent"')"

# ================================================================================
# 3 (token sources) and 5 (reply shape): a model named in the body, no pin
# ================================================================================
mk_session "tokens"
TOK_SID="$SID"
msgs_shape "$TOK_SID" >/dev/null; BEFORE_SHAPE="$SHAPE"
info "token source before: last assistant message info.tokens = $(last_assistant_tokens)"
printf '%s' "$MSGS" | record "messages-before-summarize"
oc_http GET "$PORT" "/session/$TOK_SID"
info "token source before: session object .tokens = $(printf '%s' "$HTTP_BODY" | jq -c '.tokens'); keys = $(printf '%s' "$HTTP_BODY" | jq -c 'keys')"
info "messages before (shape): $BEFORE_SHAPE"
n0=$(stub_n)
full POST "/session/$TOK_SID/summarize" '{"providerID":"stubprov","modelID":"stub-other"}'
info "summarize (a) explicit stubprov/stub-other, no pin -> $F_CODE content-type='${F_CT:-none}' in ${F_MS} ms, body: $F_BODY"
printf '%s' "$F_BODY" | record "summarize-a-explicit-model-reply"
check "(a) summarize with a {providerID, modelID} body answers 200 (the one success status the OpenAPI document gives) (got $F_CODE)" test "$F_CODE" = 200
check "(a) the 200 body is the JSON boolean true" test "$F_BODY" = "true"
info "(a) stub requests made by the call: $(stub_since "$n0")"
check "(a) with no pin, the model named in the body is the model that ran (stub log)" test "$(stub_summary_model "$n0")" = stub-other
msgs_shape "$TOK_SID" >/dev/null; AFTER_SHAPE="$SHAPE"
info "messages after (shape): $AFTER_SHAPE"
printf '%s' "$MSGS" | record "messages-after-summarize"
info "token source after: last assistant message info.tokens = $(last_assistant_tokens)"
oc_http GET "$PORT" "/session/$TOK_SID"
info "token source after: session object .tokens = $(printf '%s' "$HTTP_BODY" | jq -c '.tokens')"
printf '%s' "$HTTP_BODY" | record "session-after-summarize"
oc_http GET "$PORT" /session
check "the session id is unchanged by summarize (the same single session is still listed)" \
  test "$(printf '%s' "$HTTP_BODY" | jq --arg s "$TOK_SID" '[.[] | select(.id==$s)] | length')" = 1
check "summarize only appended messages (history kept; the summary is new messages at the end)" \
  test "$(printf '%s' "$BEFORE_SHAPE" | jq length)" -lt "$(printf '%s' "$AFTER_SHAPE" | jq length)"
info "messages flagged summary:true after summarize = $(printf '%s' "$AFTER_SHAPE" | jq -c '[.[] | select(.summary==true)] | length')"
# Does the next turn see a shorter context? The stub logs the number of messages it was sent.
nb=$(stub_n)
turn "$TOK_SID" "turn after the summary"
info "the first turn after summarize reached the stub as $(stub_since "$nb"); the 3rd turn before it was $(stub_line 3 | jq -c '{nmsg, chars}')"
check "the turn after summarize is sent fewer messages than the third turn before it (the model context did shrink)" \
  test "$(tail -n +$((nb + 1)) "$OC_DIR/logs/stub.log" | jq -r '.nmsg' | head -1)" -lt "$(stub_line 3 | jq -r '.nmsg')"
msgs "$TOK_SID"
info "after that turn: last assistant message info.tokens = $(last_assistant_tokens) (the stub's usage is fixed, so this only shows which field a turn writes)"

# ================================================================================
# 4. which model runs it (unpinned server): (c) and (d), plus variant and auto
# ================================================================================
SUM_CASE c-empty-body '{}'
check "(c) no pin, body {}: refused with a 4xx and nothing sent to the stub (got $CASE_CODE)" \
  test "$CASE_CODE" -ge 400 -a "$CASE_CODE" -lt 500 -a "$(stub_n)" = "$CASE_N0"
mk_session "case-c-no-body"
n0=$(stub_n)
full POST "/session/$SID/summarize"
info "summarize[c-no-body] no request body at all -> $F_CODE content-type='${F_CT:-none}'; reply body: $(printf '%s' "$F_BODY" | sanitize); stub requests: $(stub_since "$n0")"
printf '%s' "$F_BODY" | record "summarize-c-no-body-reply"
SUM_CASE c-provider-only '{"providerID":"stubprov"}'
SUM_CASE d-unknown-provider '{"providerID":"nosuchprovider","modelID":"x"}'
info "(d) unknown provider: stub request count moved by $(( $(stub_n) - CASE_N0 ))"
SUM_CASE d-unknown-model '{"providerID":"stubprov","modelID":"nosuchmodel"}'
info "(d) known provider, unknown model: stub request count moved by $(( $(stub_n) - CASE_N0 ))"
SUM_CASE e-variant '{"providerID":"stubprov","modelID":"stub-other","variant":"high"}'
SUM_CASE e-auto-true '{"providerID":"stubprov","modelID":"stub-other","auto":true}'
info "auto:true: message list afterwards $(msgs_shape "$CASE_SID")"
oc_http POST "$PORT" /session/ses_doesnotexist0000000000000/summarize '{"providerID":"stubprov","modelID":"stub-other"}'
info "summarize on an unknown session -> $HTTP_CODE $(printf '%s' "$HTTP_BODY" | sanitize)"
printf '%s' "$HTTP_BODY" | record "summarize-unknown-session-reply"
oc_http POST "$PORT" /session/notasession/summarize '{"providerID":"stubprov","modelID":"stub-other"}'
info "summarize on an id not starting with ses -> $HTTP_CODE $(printf '%s' "$HTTP_BODY" | sanitize)"

# ================================================================================
# 7. the compact route, on the unpinned server
# ================================================================================
mk_session "compact-route"
n0=$(stub_n)
full POST "/api/session/$SID/compact"
info "compact route -> $F_CODE content-type='${F_CT:-none}' in ${F_MS} ms, body: $(printf '%s' "$F_BODY" | sanitize)"
printf '%s' "$F_BODY" | record "compact-reply"
info "compact route: stub requests made: $(stub_since "$n0"); messages after: $(msgs_shape "$SID")"
full POST "/api/session/ses_doesnotexist0000000000000/compact"
info "compact route on an unknown session -> $F_CODE $(printf '%s' "$F_BODY" | sanitize)"
printf '%s' "$F_BODY" | record "compact-unknown-session-reply"
full GET "/api/session/$SID/context"
info "GET /api/session/:id/context -> $F_CODE $(printf '%s' "$F_BODY" | sanitize | head -c 400)"
full GET "/api/session/$SID"
info "GET /api/session/:id (v2) -> $F_CODE $(printf '%s' "$F_BODY" | sanitize | head -c 300)"

# ================================================================================
# restart pinned: (b), the shape of GET /config, the TUI title, timing, busy, failure
# ================================================================================
stop_server
write_config 1
start_server
oc_http GET "$PORT" /config
info "GET /config agent.compaction as the scratch config pins it (verbatim): $(printf '%s' "$HTTP_BODY" | jq -c '.agent.compaction')"
printf '%s' "$HTTP_BODY" | record "config-pinned"
check "GET /config shows agent.compaction.model = stubprov/stub-compact" \
  test "$(printf '%s' "$HTTP_BODY" | jq -r '.agent.compaction.model')" = "stubprov/stub-compact"

SUM_CASE b-pin-and-other-model '{"providerID":"stubprov","modelID":"stub-other"}'
check "(b) pinned server, body names stub-other: 200 (got $CASE_CODE)" test "$CASE_CODE" = 200
info "(b) the model that ran the summary (stub log) = $(stub_summary_model "$CASE_N0"); the body named stub-other; the pin is stub-compact"
check "(b) the pin shown by GET /config is the model that ran, not the one in the body" test "$(stub_summary_model "$CASE_N0")" = stub-compact
SUM_CASE b-pin-empty-body '{}'
info "(b) pinned server, body {}: model that ran = '$(stub_summary_model "$CASE_N0")'"
SUM_CASE b-pin-unknown-provider '{"providerID":"nosuchprovider","modelID":"x"}'
info "(b) pinned server, body names an unknown provider: model that ran = '$(stub_summary_model "$CASE_N0")'"

# ---- 5: timing, "returns when finished or at once", the TUI title -------------------
mk_session "timed"
TSID="$SID"
oc_tui_start tui "$PORT"
sleep 6
oc_http POST "$PORT" /tui/select-session "{\"sessionID\":\"$TSID\"}"
info "POST /tui/select-session -> $HTTP_CODE $HTTP_BODY"
oc_wait_title tui "OC | timed" 20 >/dev/null || true
info "TUI pane title before summarize: $(oc_tui_title tui)"
stub_ctl '{"hold_summary_only":true}'
n0=$(stub_n)
( curl -s -m 100 -X POST -H 'content-type: application/json' \
    -d '{"providerID":"stubprov","modelID":"stub-other"}' -o "$OC_DIR/logs/th.b" \
    -w '%{http_code}' "http://127.0.0.1:$PORT/session/$TSID/summarize" >"$OC_DIR/logs/th.r" 2>/dev/null ) &
BG=$!; OC_EXTRA_PIDS+=("$BG")
t0=$(now_ms)
sleep 4
check "summarize does NOT return while the summary request is held open at the stub (still pending after 4 s)" \
  test ! -s "$OC_DIR/logs/th.r"
info "while held: stub log = $(stub_since "$n0"); TUI title = $(oc_tui_title tui)"
info "while held: message list shape = $(msgs_shape "$TSID")"
stub_ctl '{"hold_summary_only":false}'
wait "$BG" 2>/dev/null || true
HELD_MS=$(( $(now_ms) - t0 ))
hc=$(cat "$OC_DIR/logs/th.r" 2>/dev/null || echo 000)
info "held summarize returned $hc after ${HELD_MS} ms wall, body: $(cat "$OC_DIR/logs/th.b" 2>/dev/null)"
check "summarize returns only when the summary is finished (not before 4 s with the reply held; 200 on release)" \
  test "$HELD_MS" -ge 4000 -a "$hc" = 200
sleep 1
info "TUI pane title after summarize: $(oc_tui_title tui)"
info "TUI screen after summarize: lines showing the summary text = $(oc_tui_screen tui | grep -c 'fixed stub summary' || true)"
oc_http GET "$PORT" "/session/$TSID"
info "session object after summarize: id unchanged = $(printf '%s' "$HTTP_BODY" | jq -r --arg s "$TSID" '.id == $s'), title = $(printf '%s' "$HTTP_BODY" | jq -r '.title')"
oc_tmux kill-server 2>/dev/null || true

# ================================================================================
# 6. busy session (a turn in flight)
# ================================================================================
busy_probe() { # label route(summarize|compact)
  mk_session "busy-$1"
  local bsid="$SID" n0 n1 tb bs
  stub_ctl '{"hold":true}'
  n0=$(stub_n)
  ( curl -s -m 100 -X POST -H 'content-type: application/json' \
      -d '{"model":{"providerID":"stubprov","modelID":"stub-main"},"parts":[{"type":"text","text":"a long turn"}]}' \
      -o /dev/null "http://127.0.0.1:$PORT/session/$bsid/message" 2>/dev/null ) &
  tb=$!; OC_EXTRA_PIDS+=("$tb")
  for _ in $(seq 1 50); do [ "$(stub_n)" -gt "$n0" ] && break; sleep 0.1; done
  oc_http GET "$PORT" /session/status
  info "[$1] GET /session/status while the turn is held: $(printf '%s' "$HTTP_BODY" | sanitize)"
  n1=$(stub_n)
  rm -f "$OC_DIR/logs/bs.r" "$OC_DIR/logs/bs.b"
  if [ "$2" = summarize ]; then
    ( curl -s -m 100 -X POST -H 'content-type: application/json' -d '{"providerID":"stubprov","modelID":"stub-other"}' \
        -o "$OC_DIR/logs/bs.b" -w '%{http_code}' "http://127.0.0.1:$PORT/session/$bsid/summarize" >"$OC_DIR/logs/bs.r" 2>/dev/null ) &
  else
    ( curl -s -m 100 -X POST -o "$OC_DIR/logs/bs.b" -w '%{http_code}' "http://127.0.0.1:$PORT/api/session/$bsid/compact" >"$OC_DIR/logs/bs.r" 2>/dev/null ) &
  fi
  bs=$!; OC_EXTRA_PIDS+=("$bs")
  sleep 3
  if [ -s "$OC_DIR/logs/bs.r" ]; then
    info "[$1] busy: the call answered at once: $(cat "$OC_DIR/logs/bs.r") $(sanitize <"$OC_DIR/logs/bs.b"); stub requests since the held turn: $(( $(stub_n) - n1 ))"
    BUSY_VERDICT="answered at once while the turn was held"
  else
    info "[$1] busy: the call is still pending after 3 s; stub requests since the held turn: $(( $(stub_n) - n1 )) ($(stub_since "$n1"))"
    if [ "$(( $(stub_n) - n1 ))" -gt 0 ]; then BUSY_VERDICT="ran on top of the turn"; else BUSY_VERDICT="waited behind the turn (queued)"; fi
  fi
  info "[$1] busy verdict before the held turn is released: $BUSY_VERDICT"
  stub_ctl '{"hold":false}'
  wait "$tb" 2>/dev/null || true
  wait "$bs" 2>/dev/null || true
  info "[$1] after release: the call returned $(cat "$OC_DIR/logs/bs.r" 2>/dev/null) body '$(sanitize <"$OC_DIR/logs/bs.b" 2>/dev/null)'; stub requests for the case: $(stub_since "$n0")"
  info "[$1] message list after release: $(msgs_shape "$bsid")"
  sanitize <"$OC_DIR/logs/bs.b" 2>/dev/null | record "$2-busy-session-reply" || true
}
busy_probe summarize summarize
busy_probe compact compact

# ================================================================================
# 8. failed summary: the stub answers 500
# ================================================================================
mk_session "fail"
FSID="$SID"
msgs_shape "$FSID" >/dev/null; F_SHAPE_BEFORE="$SHAPE"; F_IDS_BEFORE=$(ids_of)
oc_http GET "$PORT" "/session/$FSID"; F_TOK_BEFORE=$(printf '%s' "$HTTP_BODY" | jq -c '.tokens')
stub_ctl '{"fail_summary_only":true}'
n0=$(stub_n)
full POST "/session/$FSID/summarize" '{"providerID":"stubprov","modelID":"stub-other"}' 240
stub_ctl '{"fail_summary_only":false}'
info "failed summary: summarize -> $F_CODE content-type='${F_CT:-none}' after ${F_MS} ms, body: $(printf '%s' "$F_BODY" | sanitize)"
info "failed summary: the stub saw $(( $(stub_n) - n0 )) summary request(s): $(stub_since "$n0")"
printf '%s' "$F_BODY" | record "summarize-stub-500-reply"
FAIL_CODE="$F_CODE"
oc_http GET "$PORT" /session/status
info "GET /session/status once the failed call has returned: $(printf '%s' "$HTTP_BODY" | sanitize)"
msgs_shape "$FSID" >/dev/null; F_SHAPE_AFTER="$SHAPE"
info "failed summary: messages before $F_SHAPE_BEFORE"
info "failed summary: messages after  $F_SHAPE_AFTER"
printf '%s' "$MSGS" | record "messages-after-failed-summarize"
check "after a failed summary every earlier message is still there, in order" \
  test "$(printf '%s' "$MSGS" | jq -c --argjson b "$F_IDS_BEFORE" '[.[].info.id] | .[0:($b|length)] == $b')" = true
info "failed summary: messages added by the failed attempt = $(( $(printf '%s' "$F_SHAPE_AFTER" | jq length) - $(printf '%s' "$F_SHAPE_BEFORE" | jq length) )); summary:true messages = $(printf '%s' "$F_SHAPE_AFTER" | jq '[.[] | select(.summary==true)] | length')"
oc_http GET "$PORT" "/session/$FSID"
info "failed summary: session tokens before $F_TOK_BEFORE, after $(printf '%s' "$HTTP_BODY" | jq -c '.tokens')"
nb=$(stub_n)
turn "$FSID" "does this session still answer after the failed summary"
check "the session still answers a turn after the failed summary (got $HTTP_CODE)" test "$HTTP_CODE" = 200
msgs "$FSID"; printf '%s' "$MSGS" | jq -c '[.[] | {role: .info.role, mode: .info.mode, summary: .info.summary, parts: [.parts[].type]}]' | sed 's/^/INFO  messages after the turn that followed the failed summary: /'
info "that turn reached the stub with nmsg=$(tail -n +$((nb + 1)) "$OC_DIR/logs/stub.log" | jq -r '.nmsg' | head -1) (the third turn before had $(stub_line 3 | jq -r '.nmsg' ))"
oc_http GET "$PORT" /session/status
info "GET /session/status after the failure: $(printf '%s' "$HTTP_BODY" | sanitize)"
info "failure status of a summary whose model call fails: $FAIL_CODE"
exit 0
