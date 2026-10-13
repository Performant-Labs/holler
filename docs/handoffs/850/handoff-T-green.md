# Handoff-T-green: Phase 7 - #850: a body driver for a Claude Code pane

**Date:** 2026-10-12
**Branch:** issue-850-implementation
**Issue:** #850
**Handoff-F reviewed:** docs/handoffs/850/handoff-F.md
**Handoff-T-red:** docs/handoffs/850/handoff-T-red.md

## GREEN confirmation

`cargo test --workspace -- --skip roster_stays_accurate_under_concurrent_body_load` (the CI command): exit 0, 2934 passed, 0 failed (matches F).
Repeats of the timing-sensitive new targets: `claude_attach_driver_test` 13/13 and `claude_session_test` 12/12 three times running (12 s, 8 s each);
`claude_attach_mode_test` 7/7, `say_pane_scenario_test` 4/4.
Spot-check that a test pins behavior: `START_WAIT` in `claude_attach_driver/turn.rs` changed from 5 s to 1 s makes
`a_turn_too_short_to_see_ends_after_the_start_wait` FAIL (12 pass, 1 fail); reverted, working tree clean.
F flagged no test as wrong; no test was edited in this phase.

## Tier 1 results

| Command | Result |
|---|---|
| `cargo test --workspace -- --skip roster_stays...` | PASS (2934/0) |
| `bash scripts/lint.sh` | PASS (exit 0) |
| `bash scripts/changelog-check.sh` | PASS |
| `cargo test -p holler-cli --test docs_cli_test` | PASS (3/3) |
| `cargo test -p holler-cli --test wire_selftest` | PASS (3/3) |
| `cargo clippy --workspace --all-targets --keep-going -- -D warnings` | one error, `crates/holler-cli/tests/token_load_test.rs` (`unneeded cloning of iterator items`); the file is untouched by this branch (no diff against origin/main); every other target clean. Pre-existing, not #850. |
| `cargo machete` | not installed here (as T-red found); `holler-pane` is a normal dependency used by shipped `holler-body` code |

## Tier 2 results

- Coverage: each acceptance criterion 1 to 10 plus W-2 and W-5 has a test (see T-red table); all GREEN. PASS.
- Test quality: each test names a behavior; layers are the cheapest sufficient (parse, fake port, session task, in-process verbs, real hub and body only for `body_cmd`/holds); the scenario test replaces the deleted ignored step; no redundant test found. PASS.
- RED was valid by assertion (or unavoidable compile-RED for the two new public items), per T-red.
- Timing: new real-time tests wait on the clock only where the rule is the clock; readiness uses `wait_for`. Repeated 3x, no flake. PASS.
- File sizes: new test files 219 to 441 lines, under 900; lint passes. No `#[allow]` added in the new source files. PASS.
- Secrets: the CLI test asserts no login is read or printed (I7); `login_status` is never called (driver test). PASS.
- Protocol: no golden file, hub or proto source changed; v2.md section 5.3 and ADRs amended (`docs_claude_body_test` GREEN). PASS.
- Evidence appendix: `evidence.md` exists with eight excerpts; tests rely on nothing further outside the diff. PASS.
- No browser surface in this repo; none run.

## Acceptance criteria status

1 (row rule) `claude_row_test` PASS; 2 (driver) `claude_attach_driver_test` PASS; 3 (session task, rebind) `claude_session_test` PASS;
4 (process level) `claude_attach_mode_test` PASS; 5 (mixed scenario say) `say_pane_scenario_test` PASS; 6 (pane engine drive) `pane_claude`, `driven_single_process_test` PASS;
8, 10 (docs, wizard text) `docs_claude_body_test` PASS; W-2 `hold_single_path_test` PASS.

## Blocking issues

None.

## Advisory notes

- Clippy `--all-targets` fails on the pre-existing `token_load_test.rs:60` on this toolchain; CI behavior on its own toolchain not verified here.
- F's known issue stands for O: wizard Stage 8 still says the body config holds OpenCode rows "and nothing else", so a hand-written Claude row is not wizard-managed.
- `pane import` of a legacy `claude` attach row with an `endpoint` now blocks instead of skipping (T-red finding 2); O's call.
- `a_relaunch_during_a_turn_ends_it_cancelled` keeps a ~1e-4 per-run window (non-atomic fake relaunch); 3 repeats clean.
