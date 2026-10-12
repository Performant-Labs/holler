# Handoff-T-green: Phase 7 (script numbering) - #667 profile apply on scratch Herdr, tmux and OpenCode

**Date:** 2026-10-11
**Branch:** issue-667-implementation (worktree `.claude/worktrees/0667-apply-scenario`, head `e22ce66`)
**Issue:** #667
**Handoff-F reviewed:** `docs/handoffs/667/handoff-F.md`
**Handoff-T-red:** `docs/handoffs/667/handoff-T-red.md`

F wrote no production code (brief decision 13, A's W-1). F's two edits are documentation: the CHANGELOG entry and the deleted #667 bullet in section 12 of `docs/pane-control.md`. F flagged no test as wrong, and no test was changed in this phase. `git diff bda1069 -- crates/` touches only `crates/holler-cli/tests/`.

## GREEN confirmation

| Command | Result |
|---|---|
| `cargo test -p holler-cli --test profile_apply_scenario` (inside the workspace run) | `ok. 5 passed; 0 failed; 2 ignored` |
| `cargo test -p holler-cli --test pane_integration` (inside the workspace run) | `ok. 68 passed; 0 failed; 4 ignored ... finished in 119.59s` (#649's 66 plus the 2 new guard pins) |
| `HOLLER_TEST_PANE_INTEGRATION=1 cargo test -p holler-cli --test profile_apply_scenario -- --ignored --test-threads=1`, run 1 (20:19 MDT) | `ok. 2 passed; 0 failed ... finished in 42.84s` |
| The same, run 2, straight after | `ok. 2 passed; 0 failed ... finished in 36.92s` |

Tools: herdr 0.9.1-preview.2026-09-21, tmux 3.7c, OpenCode 1.19.0.

**Nothing left behind (A1).**
- **Before:** there was no `hlr649`/`holler649` process. There were 10 `/tmp/hlr649-*` entries: T-red's 8 files and 2 directories, all from earlier in the day.
- **Right after the two runs:** there was no scratch process. Two more `/tmp/hlr649-*` directories had appeared at 20:19, each holding only an `opencode` trap script. They come from `pane_integration/wired.rs`'s `trap_dir()`, which a concurrent session in the `0835-import-leave-alone` worktree was running as part of `cargo test --workspace` (seen in `pgrep`). `profile_apply_scenario` does not include `wired.rs`.
- **After the mutant runs:** the two trap directories were gone, because that session's `TempDir`s dropped. The listing was back to the 10 entries of the baseline, and no scratch process existed. This holds even after the mutant runs that panicked partway through a scenario, so the scratch guard cleans up on panic too.

**The tests fail when the behaviour is removed: the A16 mutants.** Each mutant was applied by a script (`mutants.py`, kept in the session scratchpad) to a production file. The script then ran the named opted-in test with `--exact` and restored the file from a scratchpad copy. After each mutant, `git diff --stat -- crates/` was empty. Where a mutant would leave a variant or branch never built (the workspace denies `dead_code`), the script added the mutant behind `std::hint::black_box(true)`, so the original code still compiles.

| Mutant | Production edit | Test run | Where it failed |
|---|---|---|---|
| M1 | `tx_apply.rs` `choose`: a healthy `matches` pane becomes `Row::Relaunch` | A | `plan.rs:221`, step 2's dry run (`assert_plan_agrees_with_show`): `hlr649-ap1` `left: (Relaunch, Planned)`, `right: (Adopt, Planned)` |
| M2 | `tx_launch.rs` `probe()` returns `Ok(None)` first | B | `probe.rs:142`, phase (i): `hlr649-py` `left: (Create, Created)`, `right: (Create, Refused)` |
| M3 | `choose`: the `&& !take_over` of row a dropped (as written: refuse even with `--take-over`) | A | `scenario.rs:465`, step 5's take-over: `left: (Refuse, Refused)`, `right: (Adopt, Adopted)` |
| M3b | `choose`: row a never refuses (the other reading of M3's "is not refused") | A | `scenario.rs:455`, step 5 without `--take-over`: `hlr649-ap1` `left: (Adopt, Refused)`, `right: (Refuse, Refused)` |
| M4 | `tx_apply/observe.rs` `judge`: `health` treated as true | A | `scenario.rs:400`, step 4's dry run: `left: (Adopt, Planned)`, `right: (Relaunch, Planned)` |
| M5 | `apply()`: the live steps run when `dry_run` is set | A | `plan.rs:221`, step 2's dry run: `hlr649-ap1` `left: (Adopt, Adopted)`, `right: (Adopt, Planned)` |
| M6 | `tx_apply/run.rs` `take_over`: returns after the leave, skips the join | A | `scenario.rs:474`, step 5: `left: Null`, `right: String("hlr649-apply-q")`, "hlr649-ap1 moved to Q" |
| M7 | `holler-adapter-herdr/src/plan.rs` `below()`: `Direction::Down` becomes `Right` | A | `scratch/mod.rs:448`, step 1's `pane launch hlr649-ap2 --grid r2c1`: exit 1, `unavailable: Herdr put the new pane "w1:p2" at r1c2, not at r2c1` |

**No mutant survived.** Three of them fail at an earlier step than the brief names. Each earlier failure is still the scenario catching the broken behaviour:
- **M1 and M5** are caught by step 2's dry run, which agrees with `show` row by row.
- **M7** is caught by step 1. `pane launch` uses the same split planner as apply's create.
- **M1 at step 3:** step 3 also asserts the effect M1 targets, that `A1`'s pids are unchanged. Step 2 fails first, so that assertion is not reached under M1.

## Tier 1 results

| Command | Expected | Actual | Result |
|---|---|---|---|
| `bash scripts/lint.sh` | exit 0 | exit 0. 57 `warn:` lines, all pre-existing file-size warnings. None names a file of this story: the largest new file is `scenario.rs` at 551 lines, under the 600 warn. | PASS |
| `bash scripts/changelog-check.sh` | ok | exit 0 | PASS |
| `cargo clippy --workspace --all-targets -- -D warnings` | clean | exit 0 | PASS |
| `cargo test --workspace -- --skip roster_stays_accurate_under_concurrent_body_load` (CI's command, E69) | all pass | exit 0. 152 `test result: ok` groups, no FAILED | PASS |
| `cargo test -p holler-cli --test docs_cli_test` | pass | `ok. 3 passed` | PASS |
| `cargo test -p holler-cli --test docs_pane_control_test` | pass | `ok. 16 passed` | PASS |
| `cargo machete` | no unused deps | "didn't find any unused dependencies" | PASS |
| `cargo test -p holler-cli --test wire_selftest` (canary) | pass | `ok. 3 passed` | PASS |

**Cross-check of F's commands.** F's results are reproduced:
- `changelog-check`: ok;
- `docs_cli_test`: 3;
- `docs_pane_control_test`: 16;
- `pane_integration`: 68 passed, 4 ignored;
- `profile_apply_scenario`: 5 passed, 2 ignored;
- `lint.sh`: exit 0;
- clippy: clean.

F ran clippy on `-p holler-cli`. Here it was run on the whole workspace, as CI does. There is no discrepancy.

## Tier 2 results

| Check | Method | Result |
|---|---|---|
| A test per acceptance criterion | Mapped A1 to A17 to tests and checks (below) | PASS |
| Test quality (test-quality.md §7) | Read `scenario.rs`, `probe.rs`, `plan.rs`, `pins.rs` and the guard pins. Each step function names its behaviour, and each "unchanged" claim has a twin that changes (`X`'s new pids against `A1`'s, the marker absent in the dry run and present after the create, `B1`'s new pids in step 4). The pins are pure and at unit tier. The scenarios are at process tier because only real adapters show pids, cells and probes. No test repeats a fake-tier test: decision 2 keeps the assertion shape of `a_dry_run_and_profile_show_agree` but runs it on real state. The mutants show the assertions are behavioural. | PASS |
| Proportion | Two opted-in tests and seven default-run pins for a test-only story. Nothing is redundant enough to delete. | PASS |
| Type safety | The `Applied` reader is strict: missing keys and unknown names are refused (A14 pins). It maps names through production's `Action`/`Outcome::as_str`. The test code uses no `unsafe` and no loose `Value` comparisons of names. | PASS |
| Error paths | `usage` (no session), `pane-in-other-profile`, `probe-failed` (create and relaunch), `profile-not-found`, `pane-not-found`: each is asserted by code. | PASS |
| Data integrity | Generation deltas: unchanged (adopt), advanced (join), exactly +2 (take-over). `Still` equality around every no-op verb. Registry against Herdr's snapshot at the end (`assert_end_state`). | PASS |
| API contract | Every JSON verb goes through `Scratch::json`, which runs `check_envelope` (A10). The two text-mode runs are named in `scenario.rs`'s header. | PASS |
| Security | The `--herdr-session` argument guard runs on every verb (`Scratch::holler`), in both spellings (A13). `assert_no_credential_leaked` and `assert_tmux_env_is_scratch` run at the end of both tests. The binary runs under `env_clear` with `binary_env`. | PASS |
| Migration safety | No schema or migration change | N/A |
| Golden files / protocol | No protocol-visible change: `lint.sh`'s golden-diff summary printed nothing | N/A |
| Size and allows | All new files are under 600 lines. The five `#[allow]`s in `profile_apply_scenario/main.rs` each end `// #667`, and `lint.sh` checks the link. | PASS |
| Playwright / browser | None in this repo | N/A |
| Evidence appendix | The scenario asserts six unchanged-source facts literally: the step-4 detail text, show's `missing (no live pane)`, the dry-run text header, the `(workspace, row, col)` order, `usage` only when there is a create, and take-over's two CAS writes. Each was appended to `evidence.md` with a verbatim excerpt copied from source. | PASS |

## Acceptance criteria status

| Criterion | Status | Backed by |
|---|---|---|
| A1 | PASS | Both scenarios passed opted in twice in a row (42.84 s, 36.92 s), with no scratch process or directory left behind (see above) |
| A2 (step 1) | PASS | `scenario::step_1`, in the opted-in run |
| A3 (step 2) | PASS | `step_2`, `step_2_dry_runs`, `assert_text_plan`. M5 is killed here, and so is M1 |
| A4 (step 3) | PASS | `step_3`, `assert_joined_untouched`, `assert_created_after_its_probe`, `step_3_again_and_converge` |
| A5 (step 4) | PASS | `step_4`. M4 is killed here |
| A6 (step 5) | PASS | `step_5`. M3, M3b and M6 are killed here |
| A7 (step 7) | PASS | `step_7` |
| A8 (step 6, three phases) | PASS | `probe::a_failing_probe_refuses_the_create_and_the_relaunch_and_changes_nothing`. M2 is killed here |
| A9 (end state) | PASS | `end_state` and the end of test B (`assert_end_state`) |
| A10 | PASS | Every JSON verb goes through `Scratch::json` and `check_envelope` |
| A11 | PASS | #649's guards in the default run (68 passed), the new argument guard, and the credential and tmux environment checks at the end of both tests |
| A12 | PASS | `pins::every_scratch_test_is_ignored_and_names_the_opt_in_variable`, and its companion `pins::the_source_pin_finds_a_scratch_test_and_its_attributes`. RED was shown at T-red |
| A13 | PASS | `guards::a_herdr_session_argument_passes_only_when_it_names_the_scratch_session`. RED was shown at T-red for both spellings |
| A14 | PASS | `pins::the_applied_reader_*` (3) and `guards::launch_args_is_the_flag_list_launch_sent_before_the_move`. RED was shown at T-red |
| A15 | PASS | The workspace suite, `docs_pane_control_test`, `docs_cli_test`, workspace clippy and `lint.sh` are all clean |
| A16 | PASS | All seven mutants were killed (eight runs, counting M3b). See the table above |
| A17 | PASS | The CHANGELOG entry is present under `[Unreleased]` / `### Enhancements`. The #667 bullet is gone from section 12. `changelog-check.sh` is ok |

## Blocking issues

None.

## Advisory notes

- **M1, M5 and M7 fail at an earlier step than A16 names.** The scenario still catches each one, at step 2 for M1 and M5 and at step 1 for M7. Step 3's own create-placement check (decision 10) is not exercised by M7, because `pane launch` shares `below()` and fails first. If a later story wants step 3's check mutated on its own, it needs a mutant in apply's create path, not in the shared planner.
- **A12's source pin finds a scratch test only when its body calls `Scratch::start` and ends at a column-0 `}`.** T-red's decisions entry already records this limit. Both tests here fit it.
- **`pane_integration`'s `wired.rs` and `no_hub.rs` trap directories are `/tmp/hlr649-*`.** While a default run is going they look like scratch leftovers. A1's `/tmp` check needs to know when another worktree is running that target, as one was during this phase. This is not a defect: the `TempDir`s drop when the run ends.
- **`ACTIONS`/`OUTCOMES` in `plan.rs` list production's variants by hand.** A new production variant would be refused by the reader until it is added there. That fails loudly, as W-7 intends.

T-green complete, no blocking issues. No UI surface — U is N/A, ready for S.
