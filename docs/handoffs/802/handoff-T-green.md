# Handoff-T-green: Phase 7 (Workflow script; the role doc's Phase 6), round 2 - #802 the health wait checks the run's deadline before every ask

**Date:** 2026-10-11
**Branch:** issue-802-implementation (worktree `.claude/worktrees/0802-serve-command`, head `f27d5d0`)
**Issue:** #802 (closes #695)
**Handoff-F reviewed:** `docs/handoffs/802/handoff-F.md` (round 2)
**Handoff-T-red:** `docs/handoffs/802/handoff-T-red.md` (round 2)
**Brief:** `docs/handoffs/802-brief.md` as amended in `0dc4e8b`

The previous T-green (BLOCK on B-1) is at `git show dab3cd3:docs/handoffs/802/handoff-T-green.md`,
and the first one (PASS on round 1's code, with the per-AC table and mutation checks M1 to M6) at
`git show 3fe111d:docs/handoffs/802/handoff-T-green.md`. This round re-checks the whole suite and
what F changed for B-1, W-4, W-14, W-16 and the AC 12 advisory.

**VERDICT: PASS.** The suite is GREEN, B-1 is fixed where W-4 puts it, and Tier 1 and Tier 2 are
clean. No test was repaired: F listed none as wrong, and I found none.

## GREEN confirmation

```text
$ cargo test -p holler-cli --test pane_verbs -- launch::serve
test launch::serve::the_health_wait_asks_nothing_once_its_deadline_has_passed ... ok
test launch::serve::a_deadline_during_the_health_waits_sleep_ends_it_without_another_ask ... ok
test launch::serve::no_health_poll_starts_once_the_budget_has_run_out ... ok
test result: ok. 15 passed; 0 failed; 0 ignored; 0 measured; 246 filtered out; finished in 1.20s
```

**Spot-check (the tests pin the behaviour, not F's code).** I put `wait_healthy`'s loop back to
its round-1 form (ask first, check the time left after), ran the same command, then restored the
file from a copy (`git status` clean afterwards):

```text
test result: FAILED. 12 passed; 3 failed
  the_health_wait_asks_nothing_once_its_deadline_has_passed          left: [Health]                               right: []
  a_deadline_during_the_health_waits_sleep_ends_it_without_another_ask left: [Health, Health]                       right: [Health]
  no_health_poll_starts_once_the_budget_has_run_out                   left: [ServeCommand, Health, Health, Health] right: [ServeCommand, Health, Health]
```

These are the same three assertions as the RED in `handoff-T-red.md`.

## Tier 1 results

| Check | Command | Result |
|---|---|---|
| Verify command (whole workspace) | `cargo test --workspace --no-fail-fast -- --skip roster_stays_accurate_under_concurrent_body_load` | 148 binaries, passed=2073 failed=0 ignored=25, exit 0. PASS (F's count; includes `docs_cli_test`, `wire_selftest` and the test kit's conformance targets) |
| Clippy | `cargo clippy --workspace --all-targets -- -D warnings` (after touching `holler-pane` and `holler-adapter-opencode` crate roots so they are re-checked) | exit 0. PASS |
| Lint | `bash scripts/lint.sh` | exit 0, size warnings only (`tx_launch.rs` 820). PASS |
| Changelog | `bash scripts/changelog-check.sh` | `changelog-check: ok`. PASS |
| Unused deps | `cargo machete` | none. PASS |
| Rustdoc | `RUSTDOCFLAGS="-D warnings" cargo doc --no-deps -p holler-pane -p holler-pane-testkit -p holler-adapter-opencode` | clean. PASS |
| rustfmt | `rustfmt --edition 2021 --check` on all 40 `.rs` files the PR changes (`git diff --name-only 92fbc6e HEAD`) | clean. PASS |
| Hook tests | `bash scripts/test-hooks.sh` | exit 0. PASS |

F's opt-in `real_opencode_test` run (9/9 against opencode 1.19.0) was not repeated. It needs a
private opencode server and is not a CI step. The rig change it exercised (W-16) compiles under
`clippy --all-targets` and the workspace build.

**Cross-check of F's numbers.** Every command F lists in "Tier 1 self-check" gave the same result
here. F's one flake (`remote_admin_test::remote_admin_traffic_never_moves_the_bodys_last_seen`)
did not recur in my run.

## Tier 2 results

| Check | Method | Result |
|---|---|---|
| B-1 / W-4: no `health` call once the deadline has passed | Read `server.rs` `wait_healthy`: the deadline is checked at the top of the loop, so before every ask, the first after `run` included. The sleep is `min(poll, deadline - now)`, read after the ask. `start` has no second `budget.check()` (Phase 8's rule). | PASS |
| W-14: `check` and the wait agree | `Budget` holds one `end`, computed in `start`. `check` is `Instant::now() < end` and the wait stops at `Instant::now() >= deadline`, with `deadline() == end`: the same predicate. The halving loop ends because `checked_add(Duration::ZERO)` always succeeds. No caller passes such a budget, so no test, as T-red said. | PASS |
| W-16: rig imports `BOOT_POLL` | `rig.rs` now imports `tx_launch::BOOT_POLL`; its own `BOOT` (30 s) stays. An import in my test file, no logic or value changed. I accept it as it is. | PASS |
| AC 12 advisory (CHANGELOG) | The #802 entry now names both checks never-adopt rests on, and that the adapter's own-process check is gone. | PASS |
| Docs match behaviour | `wait_healthy`'s doc, `Budget`/`start`/`check`/`deadline` docs, step 3 of the server module doc, the I5 bullet of `tx_launch.rs`, and ADR-0021 section 2's health-wait paragraph all say "checked before every ask, none once the budget has run out". | PASS |
| Coverage per AC | AC 1-12 each have a test (the table in `3fe111d`'s T-green); amended Scope 2 / AC 6 now has the engine test and the two direct `wait_healthy` tests | PASS |
| Test quality | Each of the three tests names one behaviour and fails alone for the right reason (spot-check above). The engine test pins the bound of the run; the direct tests pin the rule in the shared function, which the engine test alone would not (a second `budget.check()` in `start` would pass it). No duplicates. The suite is proportionate: three tests for one rule at two tiers. | PASS |
| Timing | The assertions are call logs and outcomes, not durations. The sleep test's window (200 ms) is far shorter than its poll (60 s), so its result is fixed by ordering. `thread::sleep` never sleeps less than asked (std docs), so the wait's capped sleep always wakes at or past the deadline. | PASS |
| Type safety, error paths, data integrity, API contract, security, migration | Round 2 changes only `wait_healthy`'s loop and `Budget`'s fields. A `health` error is still returned as it is. Mutation checks M1 to M6 from `3fe111d` cover the unchanged rest. | PASS |
| Evidence appendix | Checked the two new entries myself against source: `harness.rs:346-349` (`FakeHarness::health` goes through `faults.enter`, which logs every call, `fault.rs:93-103`) and `fault.rs:79-83` (`set_delay`). Both excerpts are verbatim. Nothing to add. | PASS |
| Protocol, goldens, Playwright | No such change or surface | N/A |

## Acceptance criteria status

- AC 1-5 and AC 7-11: PASS, backed by the tests listed in `3fe111d`'s T-green, all GREEN in the
  workspace run above.
- AC 6 (with amended Scope 2): PASS. `the_budget_bounds_a_slow_launch` and the 4a/4b frozen-server
  tests in `launch.rs`, plus `no_health_poll_starts_once_the_budget_has_run_out`,
  `the_health_wait_asks_nothing_once_its_deadline_has_passed` and
  `a_deadline_during_the_health_waits_sleep_ends_it_without_another_ask`.
- AC 12: PASS. `changelog-check` passes, and the entry now says what never-adopt rests on.

## Blocking issues

None.

## Advisory notes

- `cargo fmt --all -- --check` reports drift in 191 files across the workspace. None of them is a
  file this PR changes, and CI does not run `cargo fmt`. Pre-existing, not #802's.
- `tx_launch.rs` is at 820 lines (lint warns, fails at 900). #644 part 2 and #664 also edit it.
- Carried over: W-11(a) (no test starts the server under a tmux pty, goes with #649), W-13 (merge
  order with #664), W-15 (no budget check between the plan's reads and `session_of_record`'s two
  calls; predates #802).
