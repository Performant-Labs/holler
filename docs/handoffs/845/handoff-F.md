# Handoff-F: #845 switch-after-relaunch (option B)

**Date:** 2026-10-12
**Branch:** issue-845-implementation
**Issue:** #845

## What was done
- `crates/holler-adapter-opencode/src/attach.rs`: `select_session` sends the select request again after each poll whose
  query finds the TUI live but not showing the session, within the unchanged `settle` / `call` bounds. The reply rules
  moved into one private fn `select` (first send and every re-send). `Tui::watch` gained a per-poll `unseen` action;
  `attach_tui` passes a no-op. Module doc and `select_session` doc state the rule.
- `crates/holler-cli/tests/pane_integration/scenario.rs`: `switch_once_the_tui_takes_it` deleted (with its `eprintln`
  race note); step 4 is now `scratch.json(&["pane", "switch", C1, &target], 0)`. T's RED handoff names this as F's edit
  (criterion 6). `wait_for`, `check_envelope` and `Duration` are still used elsewhere in the file.
- `docs/adr/ADR-0021.md`: one sentence in "Switch and reset as built (#645)".
- `docs/pane-control.md`: one sentence in the `switch` entry (re-ask every 100 ms for at most 5 s; never shown ends
  `timeout`, records nothing). The existing `unavailable` sentence is left as is.
- `CHANGELOG.md`: `## [Unreleased]` / Bug Fixes entry linking #845 (and epic #633).

## Design decisions
- **Re-send through `watch`, not a second poll** (A warning 1): `watch(port, id, settle, unseen)`; `Seen` and the
  Shown/Dead/Gone classification stay in one place. `attach_tui` passes `|_| Ok(())`, so its behaviour is unchanged (ac30
  tests green).
- **Order: query, then re-send** (A warning 2). The first send stays before the watch (so `ac29a`, a title that already
  shows the target, still gets exactly one POST). Each step: NoPane -> `Gone`, Dead -> `Dead`, Live and shown -> `Shown`,
  Live and not shown -> re-send via `send_by(.., until)` then keep polling. A dead or missing TUI therefore ends the
  call with no further request (`ac29h`: exactly 2 POSTs).
- **One send-and-judge helper** (A warning 3): `select(call, session, until)`. The first send passes the call's
  `deadline`, which is exactly what `call.send` did; re-sends pass the poll's `until`. A 404 on any send is
  `session-not-found`; a non-`true` or HTML reply on any send is `unavailable` (A's stricter reading).
- Cadence: one re-send per 100 ms poll (A warning 6 accepted; no throttling added).

## Reuse / extend-vs-new
Extended `Tui::watch` (per-poll action) and reused the crate's one `poll`, `Call::send_by`, `Tui::query`, `known`,
`deadline_after`. No new poll, bound, subprocess kind or type. `lib.rs` untouched.

## Architecture notes for A
Adapter-internal only. `HarnessPort::select_session` signature, failure set and I3 rule (success only by title) are
unchanged; `tx_switch`, `relaunch`, `shown_session`, the Claude adapter, fakes, `holler-proto` untouched. `Tui::watch`
(private) gained a closure parameter. `archChanged`: false (no module boundary, public interface or dependency direction
changed).

## Deviations from spec / wireframe
None. `settle` and `call` were not raised.

## Tier 1 self-check (incl. tests now GREEN)
```
cargo test -p holler-adapter-opencode --test attach_test
test result: ok. 27 passed; 0 failed   (ac29f/g/h now pass; ac29a-e, ac29i, ac30* pass)
cargo test -q -p holler-adapter-opencode --test attach_test ac29   x5: 9 passed each run
cargo test -p holler-adapter-opencode          all test binaries ok (11, 27, 33, 12, 13 passed; 12 ignored opt-in)
cargo test -p holler-pane                      141 passed, 0 failed
cargo test -p holler-cli --test pane_verbs     347 passed, 0 failed
cargo test -p holler-cli --test pane_integration   71 passed, 4 ignored (opt-in)
cargo clippy -p holler-adapter-opencode --all-targets -- -D warnings          clean
cargo clippy -p holler-cli --lib --bins --test pane_integration --test pane_verbs -- -D warnings   clean
bash scripts/lint.sh                           exit 0 (warnings only, none in touched files; attach.rs 268 lines)
grep -rn "TUI-readiness race\|switch_once_the_tui_takes_it" crates   (no output)
```
Note: `cargo clippy -p holler-cli --all-targets -- -D warnings` fails on `crates/holler-cli/tests/token_load_test.rs:60`
(`redundant_iter_cloned`, a newer stable clippy lint). That file is unchanged against `origin/main`; pre-existing and
out of scope.

## Evidence appendix
`docs/handoffs/845/evidence.md` (`send_by`/`budget` bound a re-send by the poll's end; `poll`'s loop; the two callers).

## Tests that look wrong (for T)
None. F edited no test in `holler-adapter-opencode`. The only test-file edit is the scenario workaround removal
(criterion 6), which T's RED handoff assigned to F.

## Known issues
- Criterion 6's opt-in run (`HOLLER_TEST_PANE_INTEGRATION=1` on a scratch Herdr with OpenCode 1.19.0) was not run by F;
  the scenario compiles and its default (ignored) run is green. Its result still needs recording by the operator or an
  opted-in run.
- Risk 1 (is 5 s `settle` enough on 1.19.0) remains unmeasured (T's probe on 1.18.35 saw readiness within ~0.3 s).
  Nothing was added under "Needs operator decision if measured".
- `GREEN`-phase mutation check (remove the `settle` cap, see `ac29d` fail) is T's.

## Files changed
- `crates/holler-adapter-opencode/src/attach.rs`
- `crates/holler-cli/tests/pane_integration/scenario.rs` (workaround removal, criterion 6)
- `docs/adr/ADR-0021.md`
- `docs/pane-control.md`
- `CHANGELOG.md`
- `docs/handoffs/845/handoff-F.md`, `docs/handoffs/845/evidence.md`, `docs/handoffs/845/decisions.md`
