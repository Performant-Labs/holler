# Handoff-T-green: #688 `FakeProfileScope` and the `ProfileScope` conformance suite

**Date:** 2026-10-09
**Branch:** issue-688-implementation
**Issue:** #688
**Handoff-F reviewed:** docs/handoffs/688/handoff-F.md
**Handoff-T-red:** docs/handoffs/688/handoff-T-red.md

## GREEN confirmation
`cargo test -p holler-pane-testkit`: every target "ok", 0 failed (`fake_profile_scope_test` 18, `profile_scope_conformance_test` 6,
earlier targets unedited: pane_store_conformance 10, fake_pane_store 22, profile_store_conformance 9, fake_profile_store 23).
The two new targets were run 5 more times in a row: 24/24 each time (no flake; there are no sleeps, the one concurrency test
is bounded by `recv_timeout`).

Spot-check that the tests pin behavior: replaced the fake's restoring `cas_put` with a no-op read (throwaway edit, reverted
from a backup; `git status` clean afterwards). 6 of 18 `fake_profile_scope_test` tests failed
(`a_failed_restore_returns_its_own_error`, `a_hook_before_the_restore_makes_it_profile_conflict`,
`a_hook_stays_armed_until_a_failed_act_of_an_edit_with_a_profile`, `arming_the_hook_again_replaces_an_unused_one`,
`the_log_carries_the_scopes_actor`, `the_calls_follow_the_i8_order`). F reported the three suite mutants rejected for the right reasons.

## Tier 1 results
| Command | Result |
|---|---|
| `cargo clippy --workspace --all-targets -- -D warnings` | PASS (exit 0) |
| `cargo test --workspace` | PASS (117 targets, 1279 passed, 0 failed, 5 ignored: same as F's report) |
| `bash scripts/lint.sh` | PASS (exit 0) |
| `bash scripts/changelog-check.sh` | PASS ("changelog-check: ok") |
| `cargo machete` | PASS (no unused dependencies) |
| `cargo test -p holler-cli --test docs_cli_test` | PASS (3) |
| `cargo test -p holler-cli --test wire_selftest` | PASS (3) |

F's reported numbers match mine; no discrepancy.

## Tier 2 results
- Coverage per criterion: every AC with a code surface has a test (AC1-AC4 below); AC5-AC10 are git/grep/CHANGELOG checks, PASS per F's output and the guards above. PASS.
- Test quality: each test names one behavior; the three mutant tests delegate to the fake, no copy of the edit logic; no duplicate of an earlier target. Proportionate (24 tests for 15 cases plus the fake's I8 order, hook, and fault paths). PASS.
- Error paths: wedged/corrupt profile and pane stores, failed restore, hook-induced conflict, `usage`, `pane-not-in-profile`. PASS.
- Timing: no fixed sleeps; invariants not durations. PASS.
- Size: test files 506 and 173 lines (under 900); no `#[allow]` in the new src files; test files carry `#[allow(...)] // #688`. PASS.
- Secrets: the fake handles no credentials; nothing asserted in logs. N/A.
- Protocol/golden: no protocol-visible change (test kit only, no manifest change). N/A.
- Evidence appendix: `evidence.md` (14 entries) covers the unchanged-code facts the tests rely on (port, `check_membership`, fault switch, seeds, `concurrent_put`, `feed::lock`, `run_cases`, `expect_code`). The tests rely on nothing further. PASS.
- Browser/Playwright: none in this repo. N/A.

## Acceptance criteria status
AC1 PASS (`the_fake_passes_the_profile_scope_conformance_suite`, `the_suite_runs_the_documented_cases`).
AC2 PASS (three mutant tests plus the unbroken-wrapper control).
AC3 PASS (wedged/corrupt/restore-failure/no-profile/I8-order tests).
AC4 PASS (usage, remove-absent, slug membership, detached remove, hook conflict, pane-not-in-profile, names, actor, Send+Sync, hook W-8 tests).
AC5-AC10 PASS per F's guard output; the workspace-level guards were re-run above (AC10) and agree.
Test repairs: none needed ("Tests that look wrong: None"); I changed no test.

## Blocking issues
None.

## Advisory notes
- The restore-failure point (a non-conflict error from the restoring write loses the act's error) is open by design and documented for #663.
- Issue #688's body is still the pre-review text (14 cases, 2 mutants); S audits against it, so O should add the "(amended 2026-10-09, plan review)" note A asked for.
