#!/usr/bin/env bash
# Tests for scripts/pane-restart (issue #746). Every external command (tmux, curl, pgrep, pfo,
# pane-ready) is a fake on PATH and HOME is a temporary directory, so no test touches a live pane.
# shellcheck disable=SC2016  # the key file path is passed, and written, unexpanded on purpose
# shellcheck disable=SC2012  # ls on our own backup names
set -uo pipefail
root=$(cd "$(dirname "$0")/../.." && pwd)
script="$root/scripts/pane-restart"
work=$(mktemp -d)
trap 'pkill -f "$work/bin/claude" 2>/dev/null; rm -rf "$work"' EXIT

fail=0
pass() { echo "ok   - $1"; }
bad() { echo "FAIL - $1"; fail=1; }
check() { # check <description> <command...>: passes when the command succeeds
  local desc=$1; shift
  if "$@" >/dev/null 2>&1; then pass "$desc"; else bad "$desc"; fi
}

# Fakes -------------------------------------------------------------------------------------------
bin="$work/bin"; mkdir -p "$bin"
# State lives in $FAKE: up (the pane's OpenCode server answers), status (/session/status body),
# tmux.log (every call), the knobs no_register, stuck, wrong_model, not_ready.
cat > "$bin/tmux" <<'EOF'
#!/usr/bin/env bash
echo "$*" >> "$FAKE/tmux.log"
kill_claude() { # the running fake Claude Code process ends and its session record goes with it
  [ -f "$FAKE/claude.pid" ] || return 0
  local pid; pid=$(cat "$FAKE/claude.pid"); kill "$pid" 2>/dev/null; rm -f "$HOME/.claude/sessions/$pid.json" "$FAKE/claude.pid"
}
case "$1" in
  has-session) [ -e "$FAKE/session-$3" ] ;;
  display-message) echo "${FAKE_OWN_SESSION:-hj-hub}" ;;
  capture-pane) cat "$FAKE/screen" 2>/dev/null ;;
  send-keys)
    target=$3; shift 3
    keys="$*"
    case "$keys" in
      "/exit Enter") echo "race: /exit and Enter in one call" >> "$FAKE/race.log" ;;
      /exit) : > "$FAKE/typed_exit" ;;
      Enter) if [ -e "$FAKE/typed_exit" ]; then rm -f "$FAKE/typed_exit"; [ -e "$FAKE/stuck" ] || { rm -f "$FAKE/up"; kill_claude; }; fi ;;
      C-c) [ "$(cat "$FAKE/stuck" 2>/dev/null)" = hard ] || { rm -f "$FAKE/up"; kill_claude; } ;;
      *'&& claude '*Enter)
        # a Claude Code launch line: run it for real under the fake `claude` on PATH, in the background
        [ -e "$FAKE/no_new_claude" ] || ( cd / && nohup bash -c "${keys% Enter}" >/dev/null 2>&1 & ) ;;
      *oc-holler*Enter)
        : > "$FAKE/up"
        model=$(printf '%s' "$keys" | sed -n 's/.* -m \([^ ]*\).*/\1/p')
        [ -e "$FAKE/wrong_model" ] && model=zai/glm-5.3
        echo "$model" > "$FAKE/live-model"
        if [ ! -e "$FAKE/no_register" ]; then
          echo ses_new > "$HOME/.holler-oc/$(echo "$target" | sed 's/^hj-/hj-/')/last-session"
          printf '[[session]]\nsession_id = "ses_new"\n' > "$HOME/.holler-oc/$target/sessions.toml"
        fi ;;
    esac ;;
esac
EOF
cat > "$bin/curl" <<'EOF'
#!/usr/bin/env bash
for a in "$@"; do url=$a; done
[ -e "$FAKE/up" ] || exit 7
case "$url" in
  */session/status) cat "$FAKE/status" ;;
  */path) echo '{"directory":"/proj/holler"}' ;;
  */message) m=$(cat "$FAKE/live-model"); echo "[{\"info\":{\"role\":\"assistant\",\"providerID\":\"${m%%/*}\",\"modelID\":\"${m#*/}\"}}]" ;;
esac
EOF
cat > "$bin/pgrep" <<'EOF'
#!/usr/bin/env bash
[ -e "$FAKE/up" ]
EOF
cat > "$bin/pfo" <<'EOF'
#!/usr/bin/env bash
echo "$*" >> "$FAKE/pfo.log"
pane=$2
if [ "${3:-}" = "--set-registration" ]; then [ -e "$FAKE/no_fix" ] || [ -e "$FAKE/needs_body_restart" ] || echo "$4" > "$FAKE/registered"; echo "registered jupiter-hj-$pane/oc at $4"; exit 0; fi
if [ "$(cat "$FAKE/registered" 2>/dev/null)" = ses_new ]; then echo "jupiter-hj-$pane/oc: ok"; else echo "jupiter-hj-$pane/oc: 1 finding(s)"; echo "  drift  registered ses_old, the hub drives ses_new"; exit 1; fi
EOF
cat > "$bin/pane-ready" <<'EOF'
#!/usr/bin/env bash
echo "$*" >> "$FAKE/pane-ready.log"
if [ -e "$FAKE/not_ready" ]; then echo "== $1: NOT READY (1)"; exit 1; fi
echo "== $1: READY"
EOF
cat > "$bin/claude" <<'EOF'
#!/usr/bin/env bash
# A fake Claude Code: writes the per-process session record the real one writes (~/.claude/sessions/<pid>.json)
# and stays alive until killed. Its own arguments are its command line, which the script reads back.
args=("$@"); sid=""
for ((i = 0; i < $#; i++)); do [ "${args[$i]}" = --resume ] && sid=${args[$((i + 1))]}; done
[ -n "$sid" ] || sid="sess-new-$$"
mkdir -p "$HOME/.claude/sessions"
echo $$ > "$FAKE/claude.pid"
printf '{"pid":%s,"sessionId":"%s","cwd":"%s","tmux":"%s","status":"%s"}\n' "$$" "$sid" "$PWD" "${FAKE_CLAUDE_TMUX:-hj-c3r1:@1.%1}" \
  "$(cat "$FAKE/claude.status" 2>/dev/null || echo idle)" > "$HOME/.claude/sessions/$$.json"
while :; do sleep 0.2; done
EOF
chmod +x "$bin"/*

case_n=0
new_case() { # new_case <pane>: a fresh HOME and fake state with an idle pane on the old model
  case_n=$((case_n + 1))
  export FAKE="$work/case$case_n" HOME="$work/case$case_n/home"
  mkdir -p "$FAKE" "$HOME/.holler-oc/hj-$1" "$HOME/.holler-oc/keys"
  : > "$FAKE/session-hj-$1"; : > "$FAKE/up"; echo '{}' > "$FAKE/status"; echo zai/glm-5.3 > "$FAKE/live-model"
  echo ses_old > "$HOME/.holler-oc/hj-$1/last-session"
  : > "$HOME/.holler-oc/keys/anthropic.key"
  cat > "$HOME/.holler-oc/hj-$1/pane.env" <<'ENV'
# Review gate comment, kept as is
export DUAL_REVIEW=1
export OC_DEFAULT_AGENT=orchestrator
export OC_MODEL=zai/glm-5.3
export OC_EFFORT=high
export OC_ROLE_MODEL=zai/glm-5.3
ENV
  cp "$HOME/.holler-oc/hj-$1/pane.env" "$FAKE/pane.env.orig"
  unset TMUX_PANE FAKE_OWN_SESSION
}
run() { # run <args...>: the script under the fakes, fast timeouts; output in $FAKE/out, status in $FAKE/rc
  PATH="$bin:$PATH" PANE_RESTART_POLL=0 PANE_RESTART_TYPE_DELAY=0 PANE_RESTART_T_STOP=2 PANE_RESTART_T_UP=2 PANE_RESTART_T_REG=2 PANE_RESTART_T_MODEL=2 \
    bash "$script" "$@" > "$FAKE/out" 2>&1
  echo $? > "$FAKE/rc"
}
rc() { cat "$FAKE/rc"; }
env_file() { echo "$HOME/.holler-oc/hj-$1/pane.env"; }
send_lines() { grep '^send-keys' "$FAKE/tmux.log" 2>/dev/null; }
expect_untouched() { # expect_untouched <description> <pane>
  if untouched "$2"; then pass "$1"; else bad "$1"; fi
}
untouched() { # untouched <pane>: nothing changed and nothing typed
  cmp -s "$(env_file "$1")" "$FAKE/pane.env.orig" && ! send_lines | grep -q . && ! ls "$HOME/.holler-oc/hj-$1"/pane.env.bak-pre-restart-* >/dev/null 2>&1
}
model=anthropic/claude-sonnet-5-5
good=(--model "$model" --effort medium --key-file '$HOME/.holler-oc/keys/anthropic.key')

if [ ! -f "$script" ]; then bad "scripts/pane-restart exists"; exit 1; fi

# Happy path --------------------------------------------------------------------------------------
new_case c3r1
run c3r1 "${good[@]}"
check "happy path exits 0" test "$(rc)" = 0
check "happy path prints one OK line naming pane, model and the new session" \
  grep -qx 'c3r1: OK model=anthropic/claude-sonnet-5-5 session=ses_new' "$FAKE/out"
check "happy path prints exactly one line" test "$(wc -l < "$FAKE/out")" = 1
check "pane-ready ran for the pane" grep -qx 'c3r1' "$FAKE/pane-ready.log"
check "the drift is repaired with --set-registration for the new session" grep -qx 'doctor c3r1 --set-registration ses_new' "$FAKE/pfo.log"

# The 2026-10-10 failure: text typed without Enter ------------------------------------------------
bare=0; prev=""
while IFS= read -r l; do
  case "$l" in
    *Enter) ;;
    "send-keys -t hj-c3r1 C-c") ;;
    "send-keys -t hj-c3r1 /exit") ;;                       # the TUI's /exit: its Enter must be the very next call
    *) bare=$((bare + 1)) ;;
  esac
  [ "$prev" = "send-keys -t hj-c3r1 /exit" ] && [ "$l" != "send-keys -t hj-c3r1 Enter" ] && bare=$((bare + 1))
  prev=$l
done < <(send_lines)
check "every text typed into a pane is followed by Enter (the 2026-10-10 failure)" test "$bare" = 0 -a "$(send_lines | wc -l)" -ge 3
check "/exit and Enter are never in one call (the TUI's autocomplete swallows it)" test ! -e "$FAKE/race.log"
check "the launch line goes in one call with its Enter" bash -c "grep -c 'oc-holler.* Enter\$' '$FAKE/tmux.log' | grep -qx 1"
check "the launch line is OC_FRESH=1 oc-holler with the model, the derived session and the project the server had" \
  grep -q "^send-keys -t hj-c3r1 cd /proj/holler && OC_FRESH=1 OC_SESSION_NAME=oc .*oc-holler hj-c3r1 /proj/holler -- -m $model --agent orchestrator Enter\$" "$FAKE/tmux.log"
check "the session name is derived: no other hj- session is touched" bash -c "! grep -v 'hj-c3r1' '$FAKE/tmux.log' | grep -q 'hj-c3r2'"
check "the last /exit goes before the launch" bash -c "grep -n -e '/exit' -e 'oc-holler' '$FAKE/tmux.log' | head -2 | sed -n '1p' | grep -q '/exit'"

# pane.env ----------------------------------------------------------------------------------------
pe=$(env_file c3r1)
check "exactly one OC_MODEL, OC_EFFORT, OC_KEY_FILE, OC_ROLE_MODEL" \
  test "$(grep -c '^export OC_MODEL=' "$pe")$(grep -c '^export OC_EFFORT=' "$pe")$(grep -c '^export OC_KEY_FILE=' "$pe")$(grep -c '^export OC_ROLE_MODEL=' "$pe")" = 1111
check "pane.env has the new model, effort and an unexpanded key file path" \
  bash -c "grep -qx 'export OC_MODEL=$model' '$pe' && grep -qx 'export OC_EFFORT=medium' '$pe' && grep -qxF 'export OC_KEY_FILE=\$HOME/.holler-oc/keys/anthropic.key' '$pe'"
check "OC_ROLE_MODEL follows the model because the file had one" grep -qx "export OC_ROLE_MODEL=$model" "$pe"
check "other lines and comments are unchanged" bash -c "grep -qx '# Review gate comment, kept as is' '$pe' && grep -qx 'export DUAL_REVIEW=1' '$pe' && grep -qx 'export OC_DEFAULT_AGENT=orchestrator' '$pe'"
bak=$(ls "$HOME/.holler-oc/hj-c3r1"/pane.env.bak-pre-restart-* 2>/dev/null | head -1)
check "a backup of the original exists" cmp -s "$bak" "$FAKE/pane.env.orig"
check "the output never contains the key file path's contents or a key" bash -c "! grep -qi 'sk-' '$FAKE/out'"

# A second run keeps the first backup ------------------------------------------------------------
echo ses_old > "$HOME/.holler-oc/hj-c3r1/last-session"; : > "$FAKE/up"; rm -f "$FAKE/tmux.log"
run c3r1 "${good[@]}"
check "second run exits 0" test "$(rc)" = 0
check "second run leaves the first backup byte-identical and adds its own" bash -c "cmp -s '$bak' '$FAKE/pane.env.orig' && [ \$(ls '$HOME'/.holler-oc/hj-c3r1/pane.env.bak-pre-restart-* | wc -l) -ge 1 ]"
check "second run still has one of each OC_ line" test "$(grep -c '^export OC_MODEL=' "$pe")$(grep -c '^export OC_EFFORT=' "$pe")" = 11

# Absent lines are added; OC_ROLE_MODEL is not invented ------------------------------------------
new_case c1r2
printf 'export OC_DEFAULT_AGENT=orchestrator\n' > "$(env_file c1r2)"; cp "$(env_file c1r2)" "$FAKE/pane.env.orig"
run c1r2 "${good[@]}" --agent build
pe=$(env_file c1r2)
check "absent OC_ lines are added" bash -c "grep -qx 'export OC_MODEL=$model' '$pe' && grep -qx 'export OC_EFFORT=medium' '$pe'"
check "OC_ROLE_MODEL is not added when the file had none" bash -c "! grep -q OC_ROLE_MODEL '$pe'"
check "--agent overrides the pane.env default in the launch line" grep -q -- "--agent build Enter\$" "$FAKE/tmux.log"

# Refusals change nothing ------------------------------------------------------------------------
new_case c3r1; echo '{"busy":{"type":"busy"}}' > "$FAKE/status"
run c3r1 "${good[@]}"
check "a busy pane is refused (exit 1, nothing changed)" bash -c "[ '$(rc)' = 1 ]" ; expect_untouched "busy: pane.env identical, nothing typed" c3r1

new_case c3r1; rm -f "$FAKE/session-hj-c3r1"
run c3r1 "${good[@]}"
check "a missing tmux session is refused" test "$(rc)" = 1; expect_untouched "missing session: untouched" c3r1

new_case c3r1; rm -f "$FAKE/up"
run c3r1 "${good[@]}"
check "a dead port is refused" test "$(rc)" = 1; expect_untouched "dead port: untouched" c3r1

new_case c3r1
run c3 "${good[@]}"
check "a bad pane name is refused" test "$(rc)" = 1; expect_untouched "bad name: untouched" c3r1

new_case c4r2
run c4r2 "${good[@]}"
check "the MO pane c4r2 is refused" test "$(rc)" = 1; expect_untouched "c4r2: untouched" c4r2

new_case c3r1; export TMUX_PANE=%9 FAKE_OWN_SESSION=hj-c3r1
run c3r1 "${good[@]}"
check "the pane the script itself runs in is refused" test "$(rc)" = 1; expect_untouched "own pane: untouched" c3r1

new_case c3r1
run c3r1 --model 'bad model; rm -rf /' --effort medium
check "a model with shell characters is refused" test "$(rc)" = 1; expect_untouched "bad model: untouched" c3r1

new_case c3r1
run c3r1 --model "$model" --key-file '$HOME/.holler-oc/keys/missing.key'
check "a key file that does not exist is refused" test "$(rc)" = 1; expect_untouched "missing key file: untouched" c3r1

new_case c3r1
run c3r1
check "a missing --model is refused" test "$(rc)" = 1; expect_untouched "no model: untouched" c3r1

# Failures after a change are reported with the step and the backup ------------------------------
new_case c3r1; echo hard > "$FAKE/stuck"
run c3r1 "${good[@]}"
check "a TUI that never exits fails at the stop step (exit 2)" bash -c "[ '$(rc)' = 2 ] && grep -q '^c3r1: FAIL step 4 (stop)' '$FAKE/out'"
check "the stop failure names the backup" grep -q 'pane.env.bak-pre-restart-' "$FAKE/out"
check "no launch line was typed after a failed stop" bash -c "! grep -q 'oc-holler' '$FAKE/tmux.log'"

new_case c3r1; echo soft > "$FAKE/stuck"
run c3r1 "${good[@]}"
check "a TUI that needs one Ctrl-C still restarts" test "$(rc)" = 0

new_case c3r1; : > "$FAKE/no_register"
run c3r1 "${good[@]}"
check "a registration that never follows fails at the registration step" \
  bash -c "[ '$(rc)' = 2 ] && grep -q '^c3r1: FAIL step 7 (registration)' '$FAKE/out'"
check "no registration repair is attempted when no new session appeared" bash -c "! grep -q set-registration '$FAKE/pfo.log' 2>/dev/null"

new_case c3r1; : > "$FAKE/no_fix"
run c3r1 "${good[@]}"
check "a repair that does not take fails at step 7 and names the command" \
  bash -c "[ '$(rc)' = 2 ] && grep -q '^c3r1: FAIL step 7 (registration)' '$FAKE/out' && grep -q 'set-registration ses_new' '$FAKE/out'"

new_case c3r1; : > "$FAKE/wrong_model"
run c3r1 "${good[@]}"
check "a pane still answering on the old model fails at the model step" \
  bash -c "[ '$(rc)' = 2 ] && grep -q '^c3r1: FAIL step 8 (model)' '$FAKE/out'"

new_case c3r1; : > "$FAKE/not_ready"
run c3r1 "${good[@]}"
check "pane-ready failing fails at the readiness step" bash -c "[ '$(rc)' = 2 ] && grep -q '^c3r1: FAIL step 9 (pane-ready)' '$FAKE/out'"

# A pane that is fully down is started, not refused, when --project says where ---------------------
new_case c3r1; rm -f "$FAKE/up"
run c3r1 "${good[@]}" --project "$work"
check "a down pane with --project is started (exit 0)" test "$(rc)" = 0
check "a down pane is started without typing /exit" bash -c "! grep -q '/exit' '$FAKE/tmux.log' && grep -q 'oc-holler hj-c3r1 $work' '$FAKE/tmux.log'"
new_case c3r1; rm -f "$FAKE/up"
run c3r1 "${good[@]}"
check "a down pane without --project is refused and says so" bash -c "[ '$(rc)' = 1 ] && grep -q 'pass --project' '$FAKE/out'"

# --mo: the MO pane c4r2 --------------------------------------------------------------------------
mo_case() { # a fresh c4r2 case with MO's config files, busy as MO always is
  new_case c4r2
  mkdir -p "$HOME/fleet-agents/opencode-mo/agents"
  printf '{"provider":{"saluki-io":{"models":{"saluki":{}}}},"agent":{"compaction":{"model":"saluki-io/saluki"},"mo":{"variant":"max"}}}\n' > "$HOME/fleet-agents/opencode-mo/opencode.json"
  printf -- '---\nname: mo\nmode: primary\nmodel: saluki-io/saluki\ncolor: info\n---\nbody line, kept\n' > "$HOME/fleet-agents/opencode-mo/agents/mo.md"
  cp "$HOME/fleet-agents/opencode-mo/opencode.json" "$FAKE/mo.json.orig"; cp "$HOME/fleet-agents/opencode-mo/agents/mo.md" "$FAKE/mo.md.orig"
  echo '{"ses_x":{"type":"busy"}}' > "$FAKE/status"
}
mo_untouched() { cmp -s "$HOME/fleet-agents/opencode-mo/opencode.json" "$FAKE/mo.json.orig" && cmp -s "$HOME/fleet-agents/opencode-mo/agents/mo.md" "$FAKE/mo.md.orig" && ! send_lines | grep -q .; }
mo_model=anthropic/claude-haiku-5-5
mo_args=(--mo --model "$mo_model" --effort high --key-file '$HOME/.holler-oc/keys/anthropic.key')

mo_case
run c4r2 "${mo_args[@]}"
check "MO mode exits 0 on a busy MO pane" test "$(rc)" = 0
check "MO mode prints one OK line" grep -qx "c4r2: OK model=$mo_model session=ses_new (MO; pane-ready does not apply)" "$FAKE/out"
mj="$HOME/fleet-agents/opencode-mo/opencode.json"; mm="$HOME/fleet-agents/opencode-mo/agents/mo.md"
check "opencode.json gets the provider key as a file reference to the expanded path" \
  test "$(jq -r '.provider.anthropic.options.apiKey' "$mj")" = "{file:$HOME/.holler-oc/keys/anthropic.key}"
check "the compaction model and variant are pinned to the new model" \
  test "$(jq -r '.agent.compaction.model + " " + .agent.compaction.variant' "$mj")" = "$mo_model high"
check "MO's own variant is set in opencode.json, replacing the old one" test "$(jq -r '.agent.mo.variant' "$mj")" = high
check "opencode.json keeps its other providers and agents" test "$(jq -r '.provider["saluki-io"] != null and .agent.mo != null' "$mj")" = true
check "mo.md has the new model, and its other lines unchanged" \
  bash -c "grep -qx 'model: $mo_model' '$mm' && grep -qx 'name: mo' '$mm' && grep -qx 'color: info' '$mm' && grep -qx 'body line, kept' '$mm' && ! grep -q saluki '$mm'"
check "both config files were backed up byte-for-byte" bash -c "cmp -s '$FAKE/mo.json.orig' \$(ls '$mj'.bak-pre-restart-* | head -1) && cmp -s '$FAKE/mo.md.orig' \$(ls '$mm'.bak-pre-restart-* | head -1)"
check "the launch line names the mo session, the model and no --agent" \
  bash -c "grep -q 'OC_FRESH=1 OC_SESSION_NAME=mo .*oc-holler hj-c4r2 /proj/holler -- -m $mo_model Enter\$' '$FAKE/tmux.log' && ! grep -q -- '--agent' '$FAKE/tmux.log'"
check "MO's doctor uses the roster name jupiter-hj-c4r2/mo" grep -q 'doctor jupiter-hj-c4r2/mo --set-registration ses_new' "$FAKE/pfo.log"
check "pane-ready is not run for MO" test ! -e "$FAKE/pane-ready.log"
check "every text typed into MO's pane is followed by Enter" bash -c "! grep '^send-keys' '$FAKE/tmux.log' | grep -v 'Enter\$' | grep -v -e ' /exit\$' -e ' C-c\$'"

mo_case
run c4r2 --model "$mo_model" --effort high --key-file '$HOME/.holler-oc/keys/anthropic.key'
check "c4r2 without --mo is refused and says to pass --mo" bash -c "[ '$(rc)' = 1 ] && grep -q 'pass --mo' '$FAKE/out'"
if mo_untouched; then pass "c4r2 without --mo: nothing changed"; else bad "c4r2 without --mo: nothing changed"; fi

new_case c3r1
run c3r1 --mo "${good[@]}"
check "--mo on another pane is refused" test "$(rc)" = 1
expect_untouched "--mo on c3r1: untouched" c3r1

mo_case
run c4r2 --mo --model "$mo_model" --effort high
check "--mo without --key-file is refused" test "$(rc)" = 1
if mo_untouched; then pass "--mo without a key file: nothing changed"; else bad "--mo without a key file: nothing changed"; fi

mo_case; rm -f "$HOME/fleet-agents/opencode-mo/agents/mo.md"
run c4r2 "${mo_args[@]}"
check "missing MO config files are refused" test "$(rc)" = 1

mo_case
run c4r2 "${mo_args[@]}" --dry-run
check "MO --dry-run exits 0 and shows both diffs" bash -c "[ '$(rc)' = 0 ] && grep -q '^+model: anthropic/claude-haiku-5-5' '$FAKE/out' && grep -q '^+.*\"variant\": \"high\"' '$FAKE/out'"
if mo_untouched; then pass "MO --dry-run changes and types nothing"; else bad "MO --dry-run changes and types nothing"; fi

mo_case; : > "$FAKE/wrong_model"
run c4r2 "${mo_args[@]}"
check "MO still answering on the old model fails at step 8 and names both backups" \
  bash -c "[ '$(rc)' = 2 ] && grep -q '^c4r2: FAIL step 8 (model)' '$FAKE/out' && grep -q \"MO's config was changed, backup: .*opencode.json.bak-pre-restart-.* and .*mo.md.bak-pre-restart-\" '$FAKE/out'"

mo_case; : > "$FAKE/needs_body_restart"
printf 'import os,sys\nopen(os.environ["FAKE"]+"/registered","w").write(sys.argv[2])\nopen(os.environ["FAKE"]+"/helper.log","w").write(" ".join(sys.argv[1:]))\n' > "$FAKE/helper.py"
PANE_RESTART_BODY_HELPER="$FAKE/helper.py" run c4r2 "${mo_args[@]}"
check "MO whose body still drives the old session is fixed by the body restart helper" \
  bash -c "[ '$(rc)' = 0 ] && grep -qx 'jupiter-hj-c4r2/mo ses_new' '$FAKE/helper.log'"

mo_case; : > "$FAKE/needs_body_restart"
run c4r2 "${mo_args[@]}"
check "MO with no working helper fails at step 7 and names the helper" \
  bash -c "[ '$(rc)' = 2 ] && grep -q '^c4r2: FAIL step 7 (registration).*body restart helper' '$FAKE/out'"

# --dry-run ---------------------------------------------------------------------------------------
new_case c3r1
run c3r1 "${good[@]}" --dry-run
check "--dry-run exits 0" test "$(rc)" = 0
expect_untouched "--dry-run changes nothing and types nothing" c3r1
check "--dry-run prints the pane.env diff and both send-keys lines" \
  bash -c "grep -q '^[-+]export OC_MODEL=' '$FAKE/out' && grep -q 'send-keys -t hj-c3r1 /exit' '$FAKE/out' && grep -q 'oc-holler hj-c3r1' '$FAKE/out'"

# Claude Code panes (issue #784) ---------------------------------------------------------------------
# A pane whose tmux session holds a live Claude Code process (its record in ~/.claude/sessions/<pid>.json names the
# session as the tmux target) is restarted through its own command line, not through oc-holler and pane.env.
claude_case() { # claude_case <pane> [status]: an idle Claude Code pane on sonnet/medium, resumed from sess-old
  new_case "$1"
  rm -rf "$HOME/.holler-oc"; mkdir -p "$work/proj"; printf '> \n' > "$FAKE/screen"
  [ -z "${2:-}" ] || echo "$2" > "$FAKE/claude.status"
  ( cd "$work/proj" && FAKE_CLAUDE_TMUX="hj-$1:@1.%1" PATH="$bin:$PATH" nohup bash "$bin/claude" --dangerously-skip-permissions \
      --model sonnet --effort medium --name test-ff --resume sess-old >/dev/null 2>&1 & )
  local n=0; while [ ! -f "$FAKE/claude.pid" ] && [ $n -lt 100 ]; do sleep 0.05; n=$((n + 1)); done
  old_pid=$(cat "$FAKE/claude.pid"); export FAKE_CLAUDE_TMUX="hj-$1:@1.%1"
}
new_pid() { cat "$FAKE/claude.pid" 2>/dev/null; }
argv() { tr '\0' ' ' < "/proc/$1/cmdline" 2>/dev/null; }
alive() { kill -0 "$1" 2>/dev/null; }
claude_untouched() { alive "$old_pid" && [ "$(new_pid)" = "$old_pid" ] && ! send_lines | grep -q . && argv "$old_pid" | grep -q -- '--model sonnet --effort medium'; }

claude_case c3r1
run c3r1 --model opus --effort high
check "claude: restart exits 0" test "$(rc)" = 0
check "claude: prints one OK line naming pane, model, effort and the session" \
  bash -c "[ \"\$(wc -l < '$FAKE/out')\" = 1 ] && grep -q '^c3r1: OK model=opus effort=high session=sess-old' '$FAKE/out'"
check "claude: the old process is gone" bash -c "! kill -0 $old_pid 2>/dev/null"
np=$(new_pid)
check "claude: a new process is running" bash -c "[ -n '$np' ] && [ '$np' != '$old_pid' ] && kill -0 '$np'"
check "claude: the new command line has the new model and effort and resumes the same session" \
  bash -c "tr '\\0' ' ' < /proc/$np/cmdline | grep -q -- '--model opus --effort high' && tr '\\0' ' ' < /proc/$np/cmdline | grep -q -- '--resume sess-old'"
check "claude: the old model and effort are gone, and the other flags are kept" \
  bash -c "! tr '\\0' ' ' < /proc/$np/cmdline | grep -q -e 'sonnet' -e 'medium' && tr '\\0' ' ' < /proc/$np/cmdline | grep -q -e '--dangerously-skip-permissions' && tr '\\0' ' ' < /proc/$np/cmdline | grep -q -e '--name test-ff'"
check "claude: it starts in the session's own working directory" bash -c "jq -r .cwd '$HOME/.claude/sessions/$np.json' | grep -qx '$work/proj'"
check "claude: every text typed into the pane is followed by Enter, /exit and Enter in separate calls" \
  bash -c "! grep '^send-keys' '$FAKE/tmux.log' | grep -v 'Enter\$' | grep -v -e ' /exit\$' -e ' C-c\$' && [ ! -e '$FAKE/race.log' ]"
check "claude: the launch line goes in one call with its Enter" bash -c "grep -c 'claude .*Enter\$' '$FAKE/tmux.log' | grep -qx 1"
check "claude: no OpenCode file is read or made" test ! -e "$HOME/.holler-oc"
check "claude: only the pane's own session is typed into" bash -c "! grep '^send-keys' '$FAKE/tmux.log' | grep -v 'hj-c3r1'"

claude_case c3r1
run c3r1 --effort xhigh
check "claude: --effort alone keeps the model" bash -c "[ '$(rc)' = 0 ] && tr '\\0' ' ' < /proc/$(new_pid)/cmdline | grep -q -- '--model sonnet --effort xhigh'"
claude_case c3r1
run c3r1 --model opus
check "claude: --model alone keeps the effort" bash -c "[ '$(rc)' = 0 ] && tr '\\0' ' ' < /proc/$(new_pid)/cmdline | grep -q -- '--model opus --effort medium'"
claude_case c3r1
run c3r1 --model 'claude-opus-5-5[1m]' --fresh
check "claude: a full model name with a context suffix, and --fresh, start a new session (no --resume)" \
  bash -c "[ '$(rc)' = 0 ] && tr '\\0' ' ' < /proc/$(new_pid)/cmdline | grep -q -- '--model claude-opus-5-5\[1m\]' && ! tr '\\0' ' ' < /proc/$(new_pid)/cmdline | grep -q -e '--resume'"

claude_case c3r1
run c3r1 --model opus --effort high --dry-run
check "claude: --dry-run exits 0 and prints the stop and launch lines" \
  bash -c "[ '$(rc)' = 0 ] && grep -q 'send-keys -t hj-c3r1 /exit' '$FAKE/out' && grep -q 'claude .*--model opus --effort high' '$FAKE/out'"
if claude_untouched; then pass "claude: --dry-run changes and types nothing"; else bad "claude: --dry-run changes and types nothing"; fi

claude_case c3r1 busy
run c3r1 --model opus
check "claude: a busy session is refused (exit 1) and the reason is its record" bash -c "[ '$(rc)' = 1 ] && grep -q '^c3r1: REFUSED: the Claude Code session is busy' '$FAKE/out'"
if claude_untouched; then pass "claude: busy: nothing typed, process untouched"; else bad "claude: busy: nothing typed, process untouched"; fi
claude_case c3r1
printf 'working\n  esc to interrupt\n' > "$FAKE/screen"
run c3r1 --model opus
check "claude: a screen saying 'esc to interrupt' is refused even if the record says idle" bash -c "[ '$(rc)' = 1 ] && grep -q 'REFUSED: the Claude Code session is working' '$FAKE/out'"
if claude_untouched; then pass "claude: interrupt screen: untouched"; else bad "claude: interrupt screen: untouched"; fi
claude_case c3r1
printf 'Push the tag now?\n  Enter to select\n' > "$FAKE/screen"
run c3r1 --model opus
check "claude: a session waiting on a question is refused" bash -c "[ '$(rc)' = 1 ] && grep -q 'REFUSED: the Claude Code session is waiting on a question' '$FAKE/out'"
if claude_untouched; then pass "claude: question screen: untouched"; else bad "claude: question screen: untouched"; fi

while IFS='|' read -r bad_args why; do
  claude_case c3r1
  eval "run c3r1 $bad_args"
  check "claude: '$bad_args' is refused (exit 1) with: $why" bash -c "[ '$(rc)' = 1 ] && grep -q '^c3r1: REFUSED: .*$why' '$FAKE/out'"
  if claude_untouched; then pass "claude: '$bad_args': untouched"; else bad "claude: '$bad_args': untouched"; fi
done <<'ARGS'
--model a/b|is not a Claude model
--model 'x y'|is not a Claude model
--model opus --effort extreme|is not an effort level
--effort|is not an effort level
|needs --model or --effort
--model opus --key-file /x|are OpenCode options
--model opus --agent build|are OpenCode options
--model opus --mo|are OpenCode options
ARGS

claude_case c3r1
export FAKE_OWN_SESSION=hj-c3r1 TMUX_PANE=%9
run c3r1 --model opus
check "claude: the pane the script itself runs in is refused" bash -c "[ '$(rc)' = 1 ] && grep -q 'this script is running inside hj-c3r1' '$FAKE/out'"
if claude_untouched; then pass "claude: own pane: untouched"; else bad "claude: own pane: untouched"; fi

claude_case c3r1; : > "$FAKE/stuck"
run c3r1 --model opus
check "claude: a TUI that never exits fails at the stop step (exit 2)" bash -c "[ '$(rc)' = 2 ] && grep -q '^c3r1: FAIL step 4 (stop)' '$FAKE/out'"
check "claude: no launch line is typed after a failed stop" bash -c "! grep -q 'claude .*Enter\$' '$FAKE/tmux.log'"
claude_case c3r1; echo hard > "$FAKE/stuck"
run c3r1 --model opus
check "claude: a TUI that ignores Ctrl-C too fails at the stop step" bash -c "[ '$(rc)' = 2 ] && grep -q 'FAIL step 4' '$FAKE/out'"
claude_case c3r1; echo soft > "$FAKE/stuck"
run c3r1 --model opus
check "claude: a TUI that needs one Ctrl-C still restarts" test "$(rc)" = 0

claude_case c3r1; : > "$FAKE/no_new_claude"
run c3r1 --model opus
check "claude: a launch that never produces a session fails at step 6 and says the old one is gone" \
  bash -c "[ '$(rc)' = 2 ] && grep -q '^c3r1: FAIL step 6' '$FAKE/out'"
claude_case c3r1; echo busy > "$FAKE/claude.status"
rm -f "$FAKE/claude.status"; echo busy > "$FAKE/new_status"
run c3r1 --model opus
check "claude: a session that comes up idle ends with exit 0" test "$(rc)" = 0

claude_case c3r1
cp "$HOME/.claude/sessions/$old_pid.json" "$HOME/.claude/sessions/9999991.json"; sed -i 's/"pid":[0-9]*/"pid":'"$old_pid"'/' "$HOME/.claude/sessions/9999991.json"
run c3r1 --model opus
check "claude: two live records for one pane are refused as ambiguous" bash -c "[ '$(rc)' = 1 ] && grep -q 'REFUSED: more than one live Claude Code session' '$FAKE/out'"
rm -f "$HOME/.claude/sessions/9999991.json"
if claude_untouched; then pass "claude: ambiguous: untouched"; else bad "claude: ambiguous: untouched"; fi

claude_case c3r1
sed -i 's/"pid":[0-9]*/"pid":2999999/' "$HOME/.claude/sessions/$old_pid.json"
run c3r1 --model opus
check "claude: a record whose process is dead is ignored: the pane goes down the OpenCode path" \
  bash -c "[ '$(rc)' = 1 ] && grep -q 'is not PROVIDER/ID' '$FAKE/out' && ! grep -q 'Claude' '$FAKE/out'"
claude_case c3r2
run c3r1 --model opus
check "claude: a record for another pane's session is not used for this pane" bash -c "[ '$(rc)' = 1 ] && ! grep -q 'Claude' '$FAKE/out'"
if alive "$old_pid"; then pass "claude: another pane's process untouched"; else bad "claude: another pane's process untouched"; fi

exit $fail
