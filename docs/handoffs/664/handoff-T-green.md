# Handoff-T-green: Phase 7 - `holler profile apply` (#664)

**Date:** 2026-10-11 (about 08:45 MDT)
**Branch:** `issue-664-implementation` (head `7f23e0f`, F's commit)
**Issue:** #664
**Handoff-F reviewed:** `docs/handoffs/664/handoff-F.md`
**Handoff-T-red:** `docs/handoffs/664/handoff-T-red.md`

## Test repair (F's "Tests that look wrong (for T)")

F was right about `crates/holler-cli/tests/pane_verbs/process/stub.rs`: `holler profile apply` is no longer a stub,
so its `STUBS` entry was stale. I deleted `("profile", "apply", 664),` and kept the `// #664` line, as the file's own
doc requires. That was the only test change. No production code was touched. The 38 apply tests needed no change.

## GREEN confirmation

- `cargo test -p holler-cli --test profile_verbs`: `ok. 81 passed; 0 failed` (38 apply tests plus 43 existing),
  **10 of 10 runs**.
- `cargo test -p holler-cli --test pane_cli_process`: `ok. 36 passed; 0 failed` (it was 34 of 36 before the repair).
- **Spot-check: the tests fail when the behavior is removed.** I ran five mutants of F's real code, not the RED
  prototype. Each one was reverted with `git checkout -- crates/holler-pane/src`, and every one was killed:

| Mutant (F's code) | `apply::` result |
|---|---|
| `listed` compares the pane id only, not the `(session, pane_id)` pair | 1 failed |
| a `failed` step does not stop the run | 7 failed |
| an adopt of a pane in no profile writes no join | 2 failed |
| a shown session of `Ok(None)` counts as unhealthy | 1 failed |
| no take-over move before a relaunch | 2 failed |

## Tier 1 results

| Check | Command | Expected | Actual | Result |
|---|---|---|---|---|
| Whole workspace (the verify command) | `cargo test --workspace --no-fail-fast -- --skip roster_stays_accurate_under_concurrent_body_load` | exit 0 | exit 0. 148 binaries: 2125 passed, 0 failed, 25 ignored | PASS |
| Clippy | `cargo clippy --workspace --all-targets -- -D warnings` | no warning | exit 0 | PASS |
| Lint | `bash scripts/lint.sh` | exit 0 | exit 0. Warnings only. `apply/rig.rs` is 742 lines, under the 900-line limit | PASS |
| Changelog | `bash scripts/changelog-check.sh` | ok | `changelog-check: ok` | PASS |
| Unused dependencies | `cargo machete` | none | none found | PASS |
| Docs CLI | `cargo test -p holler-cli --test docs_cli_test` | pass | 3 passed | PASS |
| CLI surface | `cargo test -p holler-cli --test cli_surface_test` | pass | 3 passed | PASS |
| Canary | `cargo test -p holler-cli --test wire_selftest` | pass | 3 passed | PASS |
| Hooks | `bash scripts/test-hooks.sh` | exit 0 | exit 0 | PASS |
| Pipeline adoption | `bash scripts/tests/pipeline-adoption-test.sh` | exit 0 | exit 0 | PASS |
| Format of the changed files | `rustfmt --edition 2021 --check` on every changed `.rs` file and `stub.rs` | exit 0 | exit 0 | PASS |
| Smoke | `holler profile apply`, run with no NAME | clap usage error, exit 2 | exit 2. `--help` shows the verb's doc | PASS |

**Cross-check with F's results.**

- F reported 2123 passed and 2 failed, with the 2 failures being the `STUBS` entry.
- After the repair the count is 2125 passed and 0 failed, which is F's count plus the 2 fixed tests. The results agree.
- F's `cargo fmt --check` note understates the drift. It reports diffs in many files across the workspace, not only
  `answerable.rs`, and none of those files is changed by this branch. CI does not run `cargo fmt --check`, so this does
  not block.

## Tier 2 results

| Check | Method | Result |
|---|---|---|
| Coverage of each acceptance criterion | T-red's table (C14 entries 1 to 33, C6, C8, C16) mapped to the passing tests | PASS |
| Test quality | Each test names one behavior and sits at the cheapest tier (engine over fakes; the verb only for exit codes and output). The RED prototype's 16 mutants and the 5 mutants above show the tests pin behavior. There are no redundant tests to prune. | PASS |
| Type safety | Clippy is clean. There is no `unsafe` and no new `#[allow]`. The public API is fully typed (`Action` and `Outcome` each have one `as_str` table) | PASS |
| Error paths | Tested: pre-plan errors (`usage`, `profile-not-found`, `unavailable`); per-step refusals (`pane-in-other-profile`, `grid-occupied`, `probe-failed`, `pane-exists`, `orchestrator-not-relaunched`, the per-pane usage refusal); failures (health, snapshot, cas conflict, timeout); the take-over join conflict (O4) | PASS |
| Data integrity | The take-over writes are CAS at the re-read generation, tested with an `NthCasPut` seam. A stale spec is never written back (entry 19). The twins `leave` and `join` in `run.rs` match `profile::delete::detach` and `profile::create::join`'s single-pane write line for line (compared by hand) | PASS |
| API contract | JSON `data` is `{profile, dry_run, steps:[{pane, action, outcome, code, diff, detail}]}`. The kit's envelope checker passes on it (`a_dry_run_prints_its_plan_as_the_steps_object`, `a_run_that_exits_0_prints_the_steps_object`). The exit-1 form matches O6 | PASS |
| Security | Every stored string passes through `findings::quoted` or `embedded` in a message, and through `show::text` in text output. This feature handles no secret | PASS |
| Protocol and goldens | There is no wire change, so no golden file and no `v2.md` change are needed | N/A |
| Migration | No schema change | N/A |
| Evidence appendix | F has 9 entries. I added 2 for facts my tests rely on in unchanged code: `tx_launch`'s observation calls `herdr.version` (`tx_launch.rs:288-291`), and the split-only fake refuses an unreachable cell (`holler-pane-testkit/src/herdr.rs:85-88`) | PASS |
| Timing tests | The bound tests compare durations only as properties the test checks itself (a run under 3 s; a run longer than one budget). There is no fixed sleep | PASS |

## Acceptance criteria status

All PASS, each backed by T-red's tests, now green:

- **Rows a to f, in grid order:**
  - `one_run_takes_every_row_in_grid_order`
  - `rows.rs` row tests
  - `row_b_missing_panes_are_created_by_splits_in_grid_order`
- **Idempotence:** `apply_twice_changes_nothing_the_second_time`
- **Never kill, move or rewrite a healthy pane; report a pane that differs:**
  - `row_d_a_healthy_pane_that_differs_is_reported_and_never_moved`
  - `a_dry_run_and_profile_show_agree`
- **`--take-over`:**
  - `takeover.rs` (7 tests, including O1 and O4)
  - `row_e_refuses_an_unhealthy_orchestrator_and_writes_nothing`
- **`--dry-run` makes only reads and prints the plan:** `dry_run.rs` (4 tests)
- **`--herdr-session` gate before any call:**
  - `a_create_without_herdr_session_is_usage_before_any_call`
  - `herdr_session_is_needed_for_a_create_alone`
- **Failure stops the run, and only the failure is printed (O6):**
  - `failures.rs` (5 tests)
  - `assert_stopped`, used by the verb half of those tests
- **Health is observed, never stored (`(session, pane_id)`, shown session):** `observe.rs` (3 tests)
- **Bound (I5), with no overall cap:** `bound.rs` (3 tests, plus the stale-spec test)
- **Errors before any plan:** `an_error_before_any_plan_is_the_verbs_own`

## Blocking issues

None.

## Advisory notes

- **Untested edge cases** (these were in T-red's "Not pinned" list; S may want to audit them):
  - A spec whose pane is not a valid `PaneName`. F refuses it as `spec-not-launchable` with action `refuse`.
  - A non-take-over relaunch or adopt whose record moved to another profile during the run.
  - Duplicate specs of one pane in P. F lists this under Known issues, and the profile verbs cannot produce it.
- **F's Known issues stand:**
  - The two follow-up issues are drafted, not filed.
  - The row-f window between the observation and the relaunch is open, and its remedy is the operator's decision.
  - The stale-P window of A's warn 5(a) remains.
- **Repo-wide `cargo fmt --check` drift** exists on `main` too and is not part of this change.

T-green complete, no blocking issues. No UI surface: U is N/A, ready for S.
