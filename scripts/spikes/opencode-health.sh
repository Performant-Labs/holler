#!/usr/bin/env bash
# OpenCode pane spike (issue #635), capability 6: survive or report a wedged server.
# Measures requests made while the server boots, health latency under load, and what
# a client and an attached TUI see when the server is frozen (SIGSTOP, a stand-in for
# a hung event loop), killed (SIGKILL) and restarted. The real 2026-10-07 wedge (days,
# 1.7 GB) cannot be reproduced here; SIGSTOP gives the same symptom to an HTTP client:
# the TCP connect succeeds and no answer ever comes. No model call: see opencode-lib.sh.
#
#   bash scripts/spikes/opencode-health.sh            (LOAD_N=500 LOAD_P=50 to change load)
set -euo pipefail
# shellcheck source=scripts/spikes/opencode-lib.sh
. "$(dirname "${BASH_SOURCE[0]}")/opencode-lib.sh"

LOAD_N="${LOAD_N:-500}"
LOAD_P="${LOAD_P:-50}"
oc_init
PORT=$(oc_free_port)

rss_kb() { # total RSS of the server's process group
  ps -o rss= -g "$1" 2>/dev/null | awk '{s+=$1} END {print s+0}'
}

# Time one GET; prints "<http code> <ms>".
timed_get() { # port path timeout
  local t0 code
  t0=$(now_ms)
  code=$(curl -s -o /dev/null -m "$3" -w '%{http_code}' "http://127.0.0.1:$1$2" 2>/dev/null || true)
  echo "${code:-000} $(( $(now_ms) - t0 ))"
}

# ---- requests made while the server is still starting ---------------------------------
# One GET /global/health every 50 ms from the moment of spawn, each with a 30 s limit,
# all in the background; then count those that never got an answer.
oc_serve_start "$PORT"
t0=$(now_ms)
for _ in $(seq 1 120); do
  (r=$(timed_get "$PORT" /global/health 30); echo "$(( $(now_ms) - t0 - ${r#* } )) $r" >>"$OC_DIR/logs/boot.tries") &
  OC_EXTRA_PIDS+=("$!")
  sleep 0.05
done
wait "${OC_EXTRA_PIDS[@]}" 2>/dev/null || true
healthy_ms=$(awk '$2=="200" {t=$1+$3; if (m=="" || t<m) m=t} END {print (m=="" ? -1 : m)}' "$OC_DIR/logs/boot.tries")
refused=$(awk '$2=="000" && $3<1000' "$OC_DIR/logs/boot.tries" | wc -l)
hung=$(awk '$2=="000" && $3>=29000' "$OC_DIR/logs/boot.tries" | wc -l)
slow=$(awk '$2=="200" && $3>=2000' "$OC_DIR/logs/boot.tries" | wc -l)
fast=$(awk '$2=="200" && $3<2000' "$OC_DIR/logs/boot.tries" | wc -l)
info "boot: first healthy answer $healthy_ms ms after spawn; of 120 health GETs sent from spawn: $refused refused, $hung accepted but never answered in 30 s, $slow answered after >2 s, $fast answered fast"
if [ "$hung" -gt 0 ]; then
  info "the never-answered GETs were sent at (ms after spawn): $(awk '$2=="000" && $3>=29000 {print $1}' "$OC_DIR/logs/boot.tries" | sort -n | tr '\n' ' ')"
fi
check "the server answers health within 10 s of spawn (I5's bound; got $healthy_ms ms)" \
  test "$healthy_ms" -ge 0 -a "$healthy_ms" -lt 10000
r=$(timed_get "$PORT" /doc 20)
info "first GET /doc after healthy -> ${r% *} in ${r#* } ms"
oc_wait_healthy "$PORT" 60 >/dev/null || { fail "server became healthy"; exit 1; }
SERVER=$OC_LAST_PID
oc_guard_no_model "$PORT"
A=$(oc_create_session "$PORT" "spike-health")
oc_http PATCH "$PORT" "/session/$A" "{\"title\":\"$A\"}"
info "server RSS at rest: $(rss_kb "$SERVER") kB"

# ---- load ------------------------------------------------------------------------
seq 1 20 | xargs -P 20 -I{} curl -s -o /dev/null -m 20 -X POST -H 'content-type: application/json' \
  -d '{"title":"load-{}"}' "http://127.0.0.1:$PORT/session"
oc_http GET "$PORT" /session
check "20 concurrent POST /session all landed (21 sessions)" test "$(printf '%s' "$HTTP_BODY" | jq 'length')" = 21

# 20 open event streams, as 20 attached clients would hold.
for _ in $(seq 1 20); do
  curl -s -N -m 120 "http://127.0.0.1:$PORT/event" >/dev/null 2>&1 &
  OC_EXTRA_PIDS+=("$!")
done
t0=$(now_ms)
seq 1 "$LOAD_N" | xargs -P "$LOAD_P" -I{} curl -s -o /dev/null -m 20 -w '%{http_code}\n' \
  "http://127.0.0.1:$PORT/session" >"$OC_DIR/logs/load.codes" &
LOADER=$!
OC_EXTRA_PIDS+=("$LOADER")
lat=""
for _ in $(seq 1 10); do
  r=$(timed_get "$PORT" /global/health 5)
  lat="$lat ${r#* }"
  sleep 0.1
done
wait "$LOADER" || true
total=$(( $(now_ms) - t0 ))
ok=$(grep -c '^200$' "$OC_DIR/logs/load.codes" || true)
info "load: $LOAD_N GET /session at concurrency $LOAD_P with 20 open /event streams: $ok x 200 in $total ms"
info "GET /global/health latency during load (ms):$lat"
check "every request under load answered 200" test "$ok" = "$LOAD_N"
r=$(timed_get "$PORT" /global/health 2)
check "health answers within 2 s after the load (${r#* } ms)" test "${r% *}" = 200
info "server RSS after load: $(rss_kb "$SERVER") kB"

# ---- a TUI on the session, for the failure cases --------------------------------------
oc_tui_start tui1 "$PORT" --session "$A"
oc_wait_title tui1 "OC | $A" 60 >/dev/null || fail "TUI attached to A"

# ---- frozen server (SIGSTOP): what a wedge looks like from outside -------------------
oc_kill_group "$SERVER" STOP
r=$(timed_get "$PORT" /global/health 2)
check "a frozen server is detected by a client timeout, not an error: health -> ${r% *} after ${r#* } ms (2 s limit)" \
  test "${r% *}" = 000
if (exec 3<>"/dev/tcp/127.0.0.1/$PORT") 2>/dev/null; then
  info "a TCP connect to the frozen server still succeeds (the kernel accepts it): only an HTTP timeout tells"
fi
sleep 3
info "the attached TUI while the server is frozen: pane dead=$(oc_tui_dead tui1), title '$(oc_tui_title tui1)' (it cannot tell)"
oc_kill_group "$SERVER" CONT
if ms=$(oc_wait_healthy "$PORT" 20); then
  pass "after SIGCONT the server answers health again ($ms ms)"
else
  fail "server recovers after SIGCONT"
fi
oc_http POST "$PORT" /tui/select-session "{\"sessionID\":\"$A\"}"
check "the TUI still obeys select-session after the freeze" test "$HTTP_BODY" = true

# ---- killed server (SIGKILL) ------------------------------------------------------------
oc_kill_group "$SERVER" KILL
for _ in $(seq 1 50); do kill -0 "$SERVER" 2>/dev/null || break; sleep 0.1; done
r=$(timed_get "$PORT" /global/health 2)
check "a killed server is refused at once: health -> ${r% *} in ${r#* } ms" test "${r% *}" = 000 -a "${r#* }" -lt 1000
sleep 5
info "the attached TUI 5 s after its server died: pane dead=$(oc_tui_dead tui1), title '$(oc_tui_title tui1)'"
info "its screen mentions: $(oc_tui_screen tui1 | grep -o -i -E 'reconnect[a-z]*|disconnect[a-z]*|connection[a-z ]*|error[^ ]*|unable[a-z ]*' | sort -u | tr '\n' ',' || true)"

# ---- restart on the same port and data dir: does the TUI come back? --------------------
oc_serve "$PORT"
info "restart on the same port: healthy after ${OC_BOOT_MS} ms"
oc_http GET "$PORT" "/session/$A"
check "the session of record is still there after kill and restart (got $HTTP_CODE)" test "$HTTP_CODE" = 200
oc_http POST "$PORT" /tui/select-session "{\"sessionID\":\"$A\"}"
sleep 3
info "after the restart the old TUI: pane dead=$(oc_tui_dead tui1), title '$(oc_tui_title tui1)'"
oc_http POST "$PORT" /tui/show-toast '{"message":"spike635-after-restart","variant":"info"}'
if oc_wait_screen tui1 spike635-after-restart 5; then
  info "the old TUI re-subscribed to the restarted server (a toast reached it)"
else
  info "the old TUI did NOT receive a toast from the restarted server: relaunch the TUI after a server restart"
fi
