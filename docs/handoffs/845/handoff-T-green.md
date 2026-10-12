# Handoff-T-green: #845 switch-after-relaunch (option B)

**Date:** 2026-10-12
**Branch:** issue-845-implementation
**Issue:** #845
**Handoff-F reviewed:** docs/handoffs/845/handoff-F.md
**Handoff-T-red:** docs/handoffs/845/handoff-T-red.md

## GREEN confirmation
`cargo test -p holler-adapter-opencode --test attach_test`: 27 passed, 0 failed (ac29f/g/h now pass; ac29a-e, ac29i, ac30* stay green). Repeated 8 times: 27/27 each run, no flake.

Mutation spot-checks (production edited temporarily, restored, tree clean afterwards):
- Re-send removed (`unseen(until)?` neutralised): ac29f, ac29g, ac29h FAIL ("1 select requests were sent").
- Re-send's 404 swallowed: ac29g FAILS (49 select requests were sent).
- `settle` cap removed (poll bound x40): ac29d FAILS.

## Tier 1 results
| Command | Result |
|---|---|
| `cargo test -p holler-adapter-opencode` | all binaries ok (11, 27, 33, 12, 13; 12 opt-in ignored) PASS |
| `cargo test -p holler-pane` | all ok PASS |
| `cargo test -p holler-cli --test pane_verbs` | 347 passed PASS |
| `cargo test -p holler-cli --test pane_integration` | 71 passed, 4 ignored (opt-in) PASS |
| `cargo test -p holler-cli --test docs_cli_test` | 3 passed PASS |
| `cargo test -p holler-cli --test wire_selftest` | 3 passed PASS |
| `cargo clippy -p holler-adapter-opencode --all-targets -- -D warnings` | clean PASS |
| `bash scripts/lint.sh` | exit 0 PASS |
| `bash scripts/changelog-check.sh` | ok PASS |
| `cargo machete` | not installed here; not run |

I did not re-run the workspace-wide `cargo test`/clippy; F's reported `token_load_test.rs:60` clippy failure is in a file unchanged against origin/main (pre-existing, out of scope).

## Tier 2 results
- Coverage: each criterion with a hermetic path has a test (1: ac29f; 2/3: ac29a-e; 4: ac29g, ac29h; bound: ac29d, ac29i). PASS
- Test quality: tests name behaviors, assert invariants (request counts, outcomes) not durations except the bound pins with SLACK; no blind sleeps; no duplicate. Suite proportionate. PASS
- Error paths: 404 on re-send, dead and missing pane between sends, hung request, never-shown. PASS
- Secrets: ac29h asserts no secret echoed. PASS
- Protocol/golden: no wire or protocol change; no goldens affected. PASS
- Reuse: re-send goes through the existing `Tui::watch`/`poll`/`send_by`; no new poll or bound. PASS
- Cross-check of F's commands: results match F's reported numbers.
- Evidence appendix: evidence.md exists from F; tests rely on nothing further outside the diff.

## Acceptance criteria status
1 re-ask until title shows: PASS (ac29f). 2/3 first-send and already-shown behaviour unchanged: PASS (ac29a-e). 4 404 / gone while re-asking: PASS (ac29g, ac29h). Bounds unchanged: PASS (ac29d, ac29i). 5 `attach_tui` unchanged: PASS (ac30*). 6 scenario workaround removed: PASS statically (compiles, no leftover references); the opt-in run is NOT done. 7 docs/ADR/CHANGELOG: present.

## Blocking issues
None. No test was repaired this phase (F flagged none).

## Advisory notes
- Criterion 6's live run (`HOLLER_TEST_PANE_INTEGRATION=1`, scratch Herdr, OpenCode 1.19.0) and risk 1 (is 5 s `settle` enough on 1.19.0) remain unmeasured; operator or opted-in run must record the result.
- Re-send cadence is one POST per 100 ms poll for up to 5 s (at most ~50); accepted by A.
