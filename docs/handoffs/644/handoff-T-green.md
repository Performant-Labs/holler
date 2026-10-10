# Handoff-T-green: Phase 6 (Workflow phase 7) - #644 `holler pane launch` and `relaunch` (GREEN)

**Date:** 2026-10-10
**Branch:** issue-644-implementation (worktree `.claude/worktrees/0644-launch-relaunch`, head `5cf6d94`)
**Issue:** #644
**Handoff-F reviewed:** `docs/handoffs/644/handoff-F.md`
**Handoff-T-red:** `docs/handoffs/644/handoff-T-red.md`

## GREEN confirmation

`cargo test -p holler-cli --test pane_verbs -- launch:: relaunch::`

```
test result: ok. 57 passed; 0 failed; 0 ignored; 0 measured; 130 filtered out; finished in 0.40s
```

RED was 3 passed, 54 failed. F edited no test, and F's "Tests that look wrong (for T)" is "None", so no test was repaired.

- **Repeated:** the same command ran 20 times in a row and passed all 20 (`passed 20 of 20`).
- **The whole target:** `cargo test -p holler-cli --test pane_verbs` gives `187 passed; 0 failed`.

**Spot-check: the tests fail when the behaviour is removed.** Six mutations of F's code, each applied alone, then the launch/relaunch filter run, then the file restored:

| Mutation | Tests that failed |
|---|---|
| M1: drop O2, the check that Herdr still lists the pane (`tx_launch.rs`, `observe_live`) | `launch_fails_when_its_herdr_pane_vanishes`, and `launch_records_what_the_fakes_show` (its exact Herdr log) |
| M2: always append the reconcile step (`launch.rs`, `failure_body`) | `a_step_the_real_scope_printed_is_not_repeated` (AC 16k) |
| M3: never refuse an occupied cell (`refuse_occupied`) | `launch_refuses_a_cell_another_record_holds`, `launch_never_adopts_an_unrecorded_pane` |
| M4: B10 never closes the old pane (`close_old`) | `relaunch_with_grid_moves_the_pane`, `relaunch_records_the_move_before_closing_the_old_pane` |
| M5: B8 never keeps the session of record (`session_of_record`) | `relaunch_keeps_the_session_of_record` |
| M6: rollback never closes the created pane (`roll_back`) | 8 tests, among them `a_failed_attach_rolls_back`, `launch_aborts_when_the_server_never_gets_healthy`, `the_budget_bounds_a_slow_launch` and the three profile-conflict cases |

Restoring a file with `cp` gave it an older mtime than the mutated build, so cargo kept the mutated build: the first M2 run also showed M1's two failures. M2 was re-run alone after a `touch`, and that run is the one in the table. Both sources were then confirmed identical to `HEAD` (`git diff --quiet`) and touched, and the target and `holler-pane` were rebuilt and re-run green (187 passed; `holler-pane` 98 passed, 0 failed). `git status` is clean apart from this phase's handoff files.

## Tier 1 results

| Check | Expected | Actual | Result |
|---|---|---|---|
| `cargo clippy --workspace --all-targets -- -D warnings` | exit 0 | exit 0 | PASS |
| `cargo test --workspace` (first run, fail-fast) | exit 0 | exit 101: `remote_admin_test::remote_say_wait_and_hub_query_json_match_local_after_masking`, `updates` 1 vs 2 (see Advisory 1) | flake, not this diff |
| `cargo test -p holler-cli --test remote_admin_test`, 5 runs | ok | 21 passed each time | PASS |
| `cargo test --workspace --no-fail-fast` | exit 0 | exit 0; 135 `test result` lines, 1676 passed, 0 failed, 14 ignored (the same as F's) | PASS |
| `cargo test -p holler-cli --test docs_cli_test --test wire_selftest --test cli_surface_test --test pane_cli_process` | ok | 3, 3, 3 and 34 passed | PASS |
| `cargo test -p holler-pane` | ok | 98 passed, 0 failed | PASS |
| `bash scripts/lint.sh` | exit 0 | exit 0; it warns that `launch/rig.rs` (711), `error.rs` (710, not this diff) and `tx_launch.rs` (786) are over 600 lines | PASS |
| `bash scripts/changelog-check.sh` | `changelog-check: ok` | `changelog-check: ok` | PASS |
| `cargo machete` | no unused dependencies | none found | PASS |
| `rustfmt --check --edition 2021` on the 3 production and 5 test files | exit 0 | exit 0 | PASS |
| `bash scripts/test-hooks.sh` | exit 0 | exit 0 | PASS |

**F's reported commands, re-run:** every Tier 1 row in handoff-F.md gives the same result here, with the same counts (57; 187, 3, 3, 34; 1676/0/14). The one difference is the first fail-fast workspace run's flake, which is outside the diff.

## Tier 2 results

| Check | Method | Result |
|---|---|---|
| Coverage | AC 1-24 each have a named test (see handoff-T-red's table). AC 16d is merged into 10a, and AC 16f into AC 1. AC 25-29 are checked by grep below. | PASS |
| Test quality | Every test pins one criterion through argv or the engine API, never F's private types. The mutations above show they fail when the behaviour goes. No test duplicates another: the AC 17 cases re-run other ACs' cases only to compare formats. The suite is proportionate: 57 tests for 31 ACs over two verbs and an engine. | PASS |
| Type safety | clippy `-D warnings` is clean. No new `#[allow]` is in the diff (`git diff d9eabbb HEAD -- crates \| grep '^\+.*#!?\[allow'` prints nothing). | PASS |
| Error paths | Every refusal code and failure code in the AC list is asserted with its exit class through `check_envelope`. | PASS |
| Data integrity | Generation conflicts on the record (AC 6a/6b) and on the profile (AC 16h/16k) are tested. A crash leaves no record (AC 7). A failed act restores P (AC 16c). | PASS |
| API contract | The JSON envelope and `data.pane.herdr.grid` (AC 13a, 17) are tested, and the surface tests pass (AC 26). | PASS |
| Security | AC 12: the secret value is absent from both streams. AC 25: no `send_text`/`send_keys`, shell literal, `Command::new` or `std::process` in the three files (both greps print nothing). The argv reaches the host verbatim (AC 11b). | PASS |
| Timing | No new fixed sleep in the tests (the only `sleep` in the diff is the argv `["sleep", "600"]`). The races are asserted as invariants. Repeated 20 times. | PASS (Advisory 2) |
| Migration | none: no schema, record or manifest change | N/A |
| Size | Every test file is under 900 lines (rig 711, `launch.rs` 583, `relaunch.rs` 519) | PASS |
| Evidence appendix | One T entry added to `evidence.md`: the rig's fakes and generation-1 seeds come from #643's unchanged `crate::list::Rig::new`. F's nine entries cover the rest. | done |
| Playwright / UI | none in this repo (no UI surface) | N/A |

## Acceptance criteria status

| AC | Status | Backed by |
|---|---|---|
| 1, 2 | PASS | `launch_records_what_the_fakes_show`; `assert_matches` in every case except 6, 7 and 18b |
| 3 | PASS | `launch_records_model_effort_env_and_ceilings` |
| 4a, 4b | PASS | `launch_onto_a_frozen_server_times_out_before_any_session`, `launch_aborts_when_the_server_never_gets_healthy` |
| 5a, 5b | PASS | `launch_fails_when_its_herdr_pane_vanishes`, `relaunch_recreates_a_vanished_pane_at_its_cell` |
| 6a, 6b | PASS | `relaunch_fails_on_a_stale_generation`, `launch_record_conflict_fails_loudly` |
| 7 | PASS | `a_crash_mid_launch_leaves_no_record` |
| 8 | PASS | `a_failed_attach_rolls_back` |
| 9a-9d | PASS | the four `guards::` cases, and `launch_ignores_a_stale_record_at_a_free_cell` |
| 10a-10c, 16d | PASS | `a_failing_probe_refuses_before_any_step`, `an_unscripted_probe_refuses`, `a_passing_probe_is_recorded` |
| 11a, 11b | PASS | `a_command_string_is_command_not_argv`, `the_command_reaches_the_host_as_argv` |
| 12 | PASS | `an_env_value_is_refused_and_not_echoed` |
| 13a-13c | PASS | `every_grid_form_reaches_herdr_as_row_2_col_1`, `an_ambiguous_grid_is_refused_before_any_step`, `a_cell_outside_the_workspace_is_out_of_range` |
| 14 | PASS | `an_unsupported_herdr_is_refused` |
| 15 | PASS | `launch_names_every_missing_flag`, `bad_policies_and_ceilings_are_usage`, `a_live_launch_needs_a_herdr_session`, `port_policy_round_trips_with_the_snapshot` |
| 16a-16k | PASS | the `profiles::` cases, `relaunch_with_profile_and_model_updates_the_spec`, `relaunch_spec_only_changes_the_profile_and_nothing_live`, `relaunch_refuses_a_pane_of_another_profile`; 16f in AC 1's test |
| 17 | PASS | `exit_codes_equal_across_formats` |
| 18, 18b | PASS | `relaunch_without_grid_keeps_the_position`, `a_failed_relaunch_never_closes_the_records_pane` |
| 19, 19b, 19c | PASS | `relaunch_with_grid_moves_the_pane`, `relaunch_refuses_a_move_without_grid_and_a_new_directory`, `relaunch_engine_enforces_its_rules`, `launch_engine_refuses_a_spec_for_another_pane_and_spec_only_without_a_profile`, `relaunch_records_the_move_before_closing_the_old_pane` |
| 20 | PASS | `relaunch_keeps_the_session_of_record`, `relaunch_replaces_a_deleted_session`, `relaunch_keeps_the_stored_driven` |
| 21 | PASS | `relaunch_leaves_other_panes_alone` |
| 22 | PASS | `relaunch_fails_when_the_old_server_survives` |
| 23 | PASS | `relaunch_of_a_missing_pane_is_refused` |
| 24 | PASS | `the_budget_bounds_a_slow_launch` |
| 25 | PASS | both greps print nothing |
| 26 | PASS | the surface tests pass; the stub grep prints nothing, and the `// #644` count is `1` |
| 27 | PASS | the four greps print lines 541, 345, 405 and 608. Against the merge base `d9eabbb`, the ADR diff's hunks are at lines 342 (section 8), 373 and 431 (section 9), 509 (section 12) and 572 ("Deferred", the mismatch bullet only). It touches no line of sections 3 (122-176) or 11 (504-523), nor the #647 or #662 bullets. |
| 28 | PASS | one `### Enhancements` entry linking #644; `changelog-check: ok` |
| 29 | PASS | against the merge base, no `+... unsafe` and no manifest change (F's Known issue 6 explains the two-dot form) |
| 30 | PASS | rustfmt, clippy, the `holler-pane`, `pane_verbs` and workspace tests, and `lint.sh` all pass; clippy enforces the size and complexity thresholds |
| 31 | for S | the public-repo check is S's |

## Blocking issues

None.

## Advisory notes

1. **A flake outside this diff:** `remote_admin_test::remote_say_wait_and_hub_query_json_match_local_after_masking` failed once under the full parallel workspace run. It saw `updates: 1` locally against `updates: 2` remotely, so the two stub chunks were coalesced on one side. It passed 5 of 5 alone and in the `--no-fail-fast` workspace run. This branch touches no file in that test's path. It is worth an issue if it is not tracked already: the parity assertion compares a chunk count that depends on timing.
2. **AC 24's margin:** the budget test has about 100 ms of margin on each side, by design (brief AC 24). If the 200 ms delayed `health` plus overhead exceeded 300 ms before A4, the run would time out before `serve`, and the exact `[Health, Serve]` log assertion would fail. That did not happen in 20 repeated runs or under the full parallel workspace load. If CI ever flakes here, widen the delay and budget together (for example 400 ms and 600 ms), not just one of them.
3. **Paths no test reaches:**
   - the rollback-failure note (`with_note`: `; rollback failed: host.stop_owned (<code>), herdr.close (<code>)`);
   - the `(Ok, None)` arm of `run` (a scope that skips the act);
   - `occupant`'s branch for a failed `pane_store.list()`.

   F names all three as design choices, and none is in the AC list. A test of the rollback note, with `fail_next(Close, ..)` after `fail_next(AttachTui, ..)`, would be cheap at this tier if S or A wants one.
4. F's Known issues 1-3 (the merge conflicts with `origin/main`, the `Timeout.op` wording, and the #640 "PROPOSED" mark) and 4 (the `--agent` amendment, #700) are for S and the MO, not for tests.
