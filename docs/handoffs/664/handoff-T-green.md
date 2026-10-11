# Handoff-T-green: Phase 7 - `holler profile apply` (#664), fresh run after A round 4

**Date:** 2026-10-11 (11:28 MDT)
**Branch:** `issue-664-implementation`, head `0b26110` (F's commit), merge base `7284001` (#802)
**Issue:** #664
**Handoff-F reviewed:** `docs/handoffs/664/handoff-F.md` (fresh run)
**Handoff-T-red:** `docs/handoffs/664/handoff-T-red.md` (fresh run)
**First run's T-green:** replaced by this file. Read it with `git show 0b26110:docs/handoffs/664/handoff-T-green.md`.

## Test repair (F's "Tests that look wrong (for T)")

F listed none, and I found none. No test file changed in this phase. No production code was touched.

## GREEN confirmation

- `cargo test -p holler-cli --test profile_verbs`: `ok. 84 passed; 0 failed`, **10 of 10 runs**. The runs overlapped
  the full workspace run, so they were under load. The fresh-run RED,
  `apply::rows::a_refusal_after_the_act_began_names_the_reconcile_step`, now passes.
- **Spot-check: the tests fail when the behavior is removed.** I ran five mutants of F's real C25 code in
  `crates/holler-cli/src/profile/apply.rs`, not the RED prototype. After each one I restored the file from a copy. All
  five were killed, and the restored tree is GREEN again (84 of 84):

| Mutant (F's code) | Failing tests |
|---|---|
| M1: the `reconciled` pass is not called | `a_refusal_after_the_act_began_names_the_reconcile_step`, `a_failed_step_that_had_acted_names_the_reconcile_step` |
| M2: reconcile only `failed` acted steps | `a_refusal_after_the_act_began_names_the_reconcile_step` |
| M3: reconcile only `refused` acted steps | `a_failed_step_that_had_acted_names_the_reconcile_step` |
| M4: reconcile every step with an error, acted or not (row a included) | `a_refusal_after_the_act_began_names_the_reconcile_step` |
| M5: `stopped` builds its message from the error, not the reconciled detail | `a_failed_step_that_had_acted_names_the_reconcile_step`, `a_take_over_join_that_conflicts_leaves_the_pane_in_no_profile` |

The first run's five engine mutants (in the replaced T-green) cover code this run did not change.

## Tier 1 results

| Check | Command | Expected | Actual | Result |
|---|---|---|---|---|
| Whole workspace (the verify command) | `cargo test --workspace --no-fail-fast -- --skip roster_stays_accurate_under_concurrent_body_load` | exit 0 | exit 0. 148 binaries: 2157 passed, 0 failed, 25 ignored | PASS |
| Clippy | `cargo clippy --workspace --all-targets -- -D warnings` | no warning | exit 0 | PASS |
| Lint | `bash scripts/lint.sh` | exit 0 | exit 0. Warnings only. The story's only one is `apply/rig.rs` at 759 lines, under the 900-line limit | PASS |
| Changelog | `bash scripts/changelog-check.sh` | ok | `changelog-check: ok` | PASS |
| Unused dependencies | `cargo machete` | none | none found, exit 0 | PASS |
| Docs CLI | `cargo test -p holler-cli --test docs_cli_test` | pass | 3 passed | PASS |
| CLI surface | `cargo test -p holler-cli --test cli_surface_test` | pass | 3 passed | PASS |
| Canary | `cargo test -p holler-cli --test wire_selftest` | pass | 3 passed | PASS |
| Stub table | `cargo test -p holler-cli --test pane_cli_process` | pass | 36 passed | PASS |
| Hooks | `bash scripts/test-hooks.sh` | exit 0 | exit 0 | PASS |
| Rustdoc links | `RUSTDOCFLAGS="-D warnings" cargo doc -p holler-pane --no-deps` | clean | clean | PASS |
| Format of the story's files | `rustfmt --edition 2021 --check` on `profile/apply.rs`, `profile/show.rs`, `tx_apply.rs` (which follows `mod` into `observe.rs` and `run.rs`), and every story test file | exit 0 | exit 0. No `cargo fmt` was run (C24) | PASS |
| New `#[allow]` | `git diff origin/main` searched for added `#[allow` in code | none | none | PASS |

**Cross-check with F's results.** F reported 148 binaries, 2157 passed, 0 failed and 25 ignored, plus 84 of 84 for
`profile_verbs`, and `docs_cli_test` 3, `cli_surface_test` 3 and `pane_cli_process` 36. My counts are identical on
every command. There is no discrepancy.

## Tier 2 results

| Check | Method | Result |
|---|---|---|
| Coverage of each acceptance criterion | T-red's table (C14 entries 1 to 33, C6, C8, C16, plus this run's C21, C22 and C25 items) mapped to passing tests | PASS |
| Test quality | The C25 RED pins the behaviour on both outcome classes and both actions, and its negative (row a) kills the "every errored step" mutant. Each fresh-run test names one behaviour, and the mutants above show each half has teeth. Nothing is redundant to prune. | PASS |
| Type safety | Clippy is clean. There is no `unsafe` and no new `#[allow]`. `reconciled` is a total function over `Applied` | PASS |
| Error paths | A refused acted step (create and relaunch) and a failed acted step both carry the reconcile step. A refusal that ran nothing does not. The exit-1 O6 form is unchanged apart from its message | PASS |
| Data integrity | This run changes no write. `reconciled` edits only the output's `detail` strings | PASS |
| API contract | The step object keeps exactly `{pane, action, outcome, code, diff, detail}` (pinned in `one_run_takes_every_row_in_grid_order`). An acted step's `detail` now ends with the reconcile step, and ADR-0021's sentence says so | PASS |
| Security | `reconcile_step` single-quotes the profile name (`pane/profile_scope.rs:49-56`). No secret is handled | PASS |
| Protocol and goldens | No wire change | N/A |
| Migration | No schema change | N/A |
| Evidence appendix | F's 15 entries. I checked three C25 excerpts against source myself: `failure_body`, `reconcile_step`, and `TxFailure`. I added the one fact F left for me, the fault switch logging a call before it sleeps (`holler-pane-testkit/src/fault.rs:85-103`). The C21 test relies on it | PASS |
| Timing tests | The C21 test asserts an invariant of the call log, not a duration. 10 of 10 runs under load | PASS |

## Acceptance criteria status

All PASS. The first run's mapping still holds, and every test it names is in the 84 passing tests:

- rows a to f in grid order;
- idempotence;
- never kill, move or rewrite a healthy pane;
- `--take-over`;
- `--dry-run`;
- the `--herdr-session` gate;
- a failure stops the run (O6);
- health is observed and never stored;
- the I5 bound with no overall cap;
- errors before any plan.

This run adds:

- **C25, an acted step names the reconcile step:**
  - `a_refusal_after_the_act_began_names_the_reconcile_step` (refused, create and relaunch, JSON and text);
  - `a_failed_step_that_had_acted_names_the_reconcile_step` (failed, both formats).
- **C21, one thread per call:** `a_panes_health_and_shown_session_are_observed_at_once`.
- **C22, the step vocabulary:**
  - `one_run_takes_every_row_in_grid_order` (actions and exact keys);
  - `a_dry_run_calls_only_its_reads_and_plans_every_row` (`planned`);
  - `a_per_pane_usage_from_the_relaunch_rules_is_refused_and_the_run_goes_on` (`spec-not-launchable`);
  - the three `observe` failure tests.
- **A round 4 warn 2, the agent difference:** `a_dry_run_and_profile_show_agree` (`pane-g`).
- **C21's doc, C22 and C25's ADR sentence, `Step.acted`'s doc, and C23's `#818`:** doc wording that no test pins, so S
  audits it. I read each in F's diff, and each is present.

## Blocking issues

None.

## Advisory notes

- **`origin/main` moved during this phase.** #820 (the wizard skill rename) landed at 11:25 MDT. The branch's diff
  against the merge base is still A's 27 paths. Only `CHANGELOG.md` is touched by both, and `git merge-tree` against
  `origin/main` merges cleanly. The PR step should merge `main` or rebase onto it, and then re-run `changelog-check.sh`.
- **The "already contains" guard in `reconciled` is untested.** It is unreachable today: the engine's messages come
  from `holler-pane`, and the reconcile text is `holler-cli`'s. It mirrors `failure_body`, as A-dup expectation 6 asks,
  so it is not a defect.
- **F's Known issues stand.** These are the row-f window, the stale-P window, duplicate specs, and what an acted
  refusal leaves behind. They are the operator's, as C23 says.
- **Untested edge cases from T-red's "Not pinned" list remain for S.** These are a spec whose pane is not a valid
  `PaneName`, and a non-take-over step whose record moved to another profile during the run.

T-green complete, no blocking issues. No UI surface: U is N/A, ready for S.
