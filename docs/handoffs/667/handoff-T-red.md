# Handoff-T-red: Phase 4 - #667 profile apply on scratch Herdr, tmux and OpenCode

**Date:** 2026-10-11
**Branch:** issue-667-implementation (worktree `.claude/worktrees/0667-apply-scenario`, head `bcb6031`)
**Brief / wireframe reviewed:** `docs/handoffs/667-brief.md`, `docs/handoffs/667/handoff-A.md`; no wireframe (no UI surface)

## A precondition

Confirmed: A returned PASS on the plan (Phase 3), with five binding warns. Each is handled:

- **W-1.** T-red wrote all the test code. Each pin is shown failing against the broken variant the Acceptance names, and the code is then restored. F owns the CHANGELOG entry and the one deleted bullet of `docs/pane-control.md` section 12. Neither is touched here.
- **W-2.** The guard refuses both `--herdr-session X` and `--herdr-session=X`.
- **W-3.** Both scenarios are made of step functions. No size-lint allow was added.
- **W-4.** No helper was copied. No `pane doctor` runs inside a `Still` window: doctor runs only in the end state.
- **W-5.** `launch_args` is a pure free function, pinned in `guards.rs`.

## Tests authored

**The moves (decision 3).** Each is a cut and paste with no change of assertion.

- **`pane_integration/out.rs` (new).** `Out` and `output_of`, moved from `main.rs`. Both targets include it, and `main.rs` re-exports it.
- **`pane_integration/scratch/verbs.rs` (new).** A second `impl Scratch`, so `mod.rs` stays at 539 lines (W-6). It holds:
  - `record` and `session_of_record`;
  - `findings`;
  - `assert_tmux_env_is_scratch`;
  - `kill_server(pane, port)`, the server kill of #649's `break_both`;
  - `assert_end_state(panes, workspaces)`, where `C1`, `C2` and `"hlr649"` became parameters.
- **`pane_integration/scenario.rs`.** Calls those methods. Its assertions are unchanged. `delete_strays` stays where it is, because #667 did not need it (see Notes).
- **`pane_integration/scratch/mod.rs`.**
  - `check_herdr_session_args`, the new guard, which `Scratch::holler` now calls.
  - `launch_args`, the pure argument builder. `Scratch::launch` and the new `Scratch::launch_argv` call it.

**The new target, `profile_apply_scenario/`:**

| File | Lines | What it holds |
|---|---|---|
| `main.rs` | 43 | Module doc, the run line, the includes (`support`, `out`, `scratch`), the five allows, each marked `// #667` |
| `plan.rs` | 355 | `Applied`/`Step` (strict reader over production's `Action`/`Outcome` `as_str`, per W-7), `assert_plan_agrees_with_show`, `Still`/`PaneStill`, `assert_one_herdr_pane_added`, `assert_on_binary_path` |
| `pins.rs` | 167 | A12 and A14 (reader) |
| `scenario.rs` | 551 | Test A |
| `probe.rs` | 213 | Test B |

**Each test, its criterion and its tier.**

| Test | Pins | Tier, and why |
|---|---|---|
| `pins::every_scratch_test_is_ignored_and_names_the_opt_in_variable` | A12 | Unit (source read). Only a read of the source sees an attribute. |
| `pins::the_source_pin_finds_a_scratch_test_and_its_attributes` | A12's reader is not vacuous: it finds a scratch test that lacks `#[ignore]` | Unit |
| `pins::the_applied_reader_reads_the_guides_shape` | A14: takes the guide's shape | Unit (pure parse) |
| `pins::the_applied_reader_refuses_a_missing_key` | A14: a missing top or step key | Unit |
| `pins::the_applied_reader_refuses_an_unknown_action_or_outcome` | A14: unknown names | Unit |
| `guards::a_herdr_session_argument_passes_only_when_it_names_the_scratch_session` | A13 and W-2. Passes: the scratch session in both forms, and no flag. Refuses: `default`, another `holler649-` name, no value, a flag in the value position, the empty `=` form, and the `=` forms of `default` and of another scratch name. A second flag that names `default` is refused in either order. | Unit (pure guard), beside #649's guards |
| `guards::launch_args_is_the_flag_list_launch_sent_before_the_move` | A14: `launch_args` against the literal pre-move list (`bda1069`, `scratch/mod.rs` 370-396) | Unit |
| `scenario::the_apply_scenario_runs_in_order_on_scratch_instances` (opt-in) | A1 to A7, A9 to A11, all on test A's set | Process-level, real adapters. Only a real Herdr, tmux and OpenCode show processes surviving, cells in Herdr, the probe as a process, the hub's writes, and exit codes. |
| `probe::a_failing_probe_refuses_the_create_and_the_relaunch_and_changes_nothing` (opt-in) | A1, A8, A10 and A11 on its own set | Process-level, real adapters, and a real `curl` |

**How test A's steps map to the criteria.** Each criterion keeps its control in the same run.

- **A2, step 1.** P's members are `[A1, A2]`; `Q` is created with `members []`.
  - `B1`'s profile is `null`.
  - `show P` is all `matches` with empty `differences`, and its text has no `differs`.
  - Control: `show Q` is two `missing` rows, and its text says `missing (no live pane)` twice.
- **A3, step 2.**
  - **Show before the dry runs:** `A2` `differs` with exactly `[{"field":"opencode_agent","spec":"plan","live":null}]`; `A1` `matches`; `X` and `B1` `missing`.
  - **Without `--herdr-session`:** the dry run exits 2 with `usage`.
  - **With the flag:** steps in the order `[A1, A2, B1, X]`, which `assert_plan_agrees_with_show` checks against show's rows. The `B1` exception is stated: `adopt`/`planned`, `diff` null.
  - **In text:** the header line, then exactly one `PANE: ACTION, OUTCOME` line per step.
  - **Nothing changed:** `Still` is equal before and after, and the marker file is absent.
- **A4, step 3.**
  - **`A1` and `A2`:** their `PaneStill` is unchanged. `A1` is adopted, and its pids are unchanged and non-empty. `A2` is reported.
  - **`B1`:** joins P, and its generation advances. Its pids, port state and session of record are the same.
  - **`X`:** created. It has a record in P and `probe.last` `"ok"`, the marker now exists, and its pids are new: they are the control for `A1`'s.
  - **Herdr:** the earlier panes keep their ids and cells, and exactly one pane is added, `X`'s, at `hlr649b` `{"row":2,"col":1}`.
  - **Second apply:** creates nothing, and `Still` is equal.
  - **`pane relaunch A2 --profile P`:** new pids, the same session of record, agent `plan`, and `show P` all `matches`.
- **A5, step 4.**
  - **After `kill_server(B1)`, the dry run** plans `B1` as `relaunch`/`planned`, with the detail `its harness server on port N did not answer`, and `Still` stays equal.
  - **The live run, without the flag,** relaunches `B1`. Its port answers again and its session is the same. Its pids are new compared with the pids before the kill.
  - **The other panes:** the other three keep their pids, and `A1`'s generation does not move.
- **A6, step 5.**
  - **`apply Q`:** `A1` and `A2` are `refuse`/`refused` with `pane-in-other-profile`, and `Still(Q)` is equal.
  - **`--take-over`:**
    - `A1` is adopted with generation +2, exactly. No relaxation was needed.
    - `A2` is reported (on `opencode_agent`).
    - Both are now in `Q`, and their pids are unchanged.
    - `B1` and `X` keep their `PaneStill` and stay in P.
- **A7, step 7.** `pane relaunch A2 --profile Q` sets the agent to `null` and gives new pids. `apply Q` then adopts `A1` and `A2`, `show Q` is `matches` twice, and every port answers.
- **A9, the end state.**
  - `assert_end_state([A1, A2, B1, X], ["hlr649", "hlr649b"])`;
  - `profile list` shows `live` 2 for P and 2 for Q;
  - `profile apply hlr649-nonexistent` exits 3 with `profile-not-found`;
  - then the tmux environment check and the credential check (A11).

**Test B (A8).** Ports come from `free_port()`. The bad port is proven `refused`. Both checks are `["curl","-s","--max-time","3","http://127.0.0.1:<port>/global/health"]` with `expect ["healthy"]`, so the bad and the good check differ only in the port.

- **(i)**
  - The order is `[W, Z, Y]`.
  - `Z` and `W` are adopted.
  - `Y` is `create`/`refused` with `probe-failed`.
  - `pane get Y` exits 3 with `pane-not-found`.
  - `Still` is equal: no record, no split and no process.
  - There is no tmux session `Y`, and `Y`'s port refuses.
- **(ii)** `kill_server(Z)`, then `Z`'s spec is given the bad check.
  - `Z` is `relaunch`/`refused` with `probe-failed`, and `Y` is refused again.
  - `Z`'s port still refuses, and its owned pids are empty.
  - `Still` is equal.
- **(iii)** Both checks are aimed at `W`'s port.
  - `Z` is relaunched and `Y` created, each with `probe.last` `"ok"`.
  - Exactly one Herdr pane is added, `Y`'s, at `hlr649h` r2c1.
- **Then:** `assert_end_state([Z, W, Y], ["hlr649g", "hlr649h"])`, the tmux environment check and the credential check.

## RED confirmation

There is no production code to write (brief decision 13 and A's W-1). The scenario runs against code that is already merged, so it passes once authored. The valid RED is each pin failing against the broken variant the Acceptance names. A script (`red.py`, kept in the scratchpad) applied one variant at a time, ran the pin, recorded the output, and restored the file. `git status` afterwards showed only the authored changes.

```text
## A12: one #[ignore] removed (probe.rs)
$ cargo test -p holler-cli --test profile_apply_scenario pins::every_scratch_test_is_ignored
test pins::every_scratch_test_is_ignored_and_names_the_opt_in_variable ... FAILED
panicked at crates/holler-cli/tests/profile_apply_scenario/pins.rs:73:9:
a_failing_probe_refuses_the_create_and_the_relaunch_and_changes_nothing starts a scratch set but is not #[ignore]d naming HOLLER_TEST_PANE_INTEGRATION: ["#[test]"]

## A13: a guard that ignores the flag
$ cargo test -p holler-cli --test pane_integration guards::a_herdr_session_argument
test guards::a_herdr_session_argument_passes_only_when_it_names_the_scratch_session ... FAILED
panicked at crates/holler-cli/tests/pane_integration/guards.rs:174:9:
["profile", "apply", "p", "--herdr-session", "default"] is refused

## A13 (W-2): a guard that sees only the two-token spelling
$ cargo test -p holler-cli --test pane_integration guards::a_herdr_session_argument
test guards::a_herdr_session_argument_passes_only_when_it_names_the_scratch_session ... FAILED
panicked at crates/holler-cli/tests/pane_integration/guards.rs:174:9:
["profile", "apply", "p", "--herdr-session=default"] is refused

## A14: launch_args without the --herdr-session pair
$ cargo test -p holler-cli --test pane_integration guards::launch_args_is_the_flag_list
test guards::launch_args_is_the_flag_list_launch_sent_before_the_move ... FAILED
panicked at crates/holler-cli/tests/pane_integration/guards.rs:214:5:
assertion `left == right` failed
  left: ["pane", "launch", "hlr649-ap1", "--project", "/tmp/hlr649-abcdef/oc/proj", ...]
 right: ["pane", "launch", "hlr649-ap1", "--herdr-session", "holler649-0123abcd", "--project", ...]

## A14: a reader that takes a missing key as null
$ cargo test -p holler-cli --test profile_apply_scenario pins::the_applied_reader_refuses_a_missing_key
test pins::the_applied_reader_refuses_a_missing_key ... FAILED
panicked at crates/holler-cli/tests/profile_apply_scenario/pins.rs:145:9:
a step with no "code" is refused

## A14: a reader that takes an unknown name as the first variant
$ cargo test -p holler-cli --test profile_apply_scenario pins::the_applied_reader_refuses_an_unknown
test pins::the_applied_reader_refuses_an_unknown_action_or_outcome ... FAILED
panicked at crates/holler-cli/tests/profile_apply_scenario/pins.rs:165:9:
action "launch" is refused
```

Each fails on the pin's own assertion about the broken behaviour. None fails on a compile error, a missing `[[test]]` or a harness error. The `missing key` variant fails at `"code"`, the first key whose loss a null-tolerant reader would accept. The keys before it (`pane`, `action`, `outcome`) are still refused by their type checks, which is correct.

**The scenario can fail (a liveness check, not the A16 run).**
- **The mutant.** M2 (`probe()` returns `Ok(None)` when the spec has a check) was applied to `crates/holler-pane/src/tx_launch.rs` and the file restored. `git diff --stat -- crates/holler-pane/` was empty afterwards.
- **The result.** Test B failed at `probe.rs:142` in phase (i): `Step { pane: "hlr649-py", action: Create, outcome: Created, ... }`, `left: (Create, Created)`, `right: (Create, Refused)`.
- **The other six mutants** (M1, M3 to M7) are T-green's, per A16.

**Restored state: GREEN of the authored suite.**

| Command | Result |
|---|---|
| `cargo test -p holler-cli --test profile_apply_scenario` | 5 passed, 2 ignored. The two scenarios list as `ignored, opt-in: ... HOLLER_TEST_PANE_INTEGRATION=1 (#667)`. |
| `cargo test -p holler-cli --test pane_integration` | 68 passed, 4 ignored. That is #649's 66 from the baseline taken before the moves, plus the 2 new guard pins. |
| `HOLLER_TEST_PANE_INTEGRATION=1 cargo test -p holler-cli --test profile_apply_scenario -- --ignored --test-threads=1` | 2 passed in 43.13 s |
| `HOLLER_TEST_PANE_INTEGRATION=1 cargo test -p holler-cli --test pane_integration -- --ignored --test-threads=1` | #649's 4 scenarios, all passing after the moves, in 54.74 s |
| `cargo clippy -p holler-cli --all-targets -- -D warnings` | clean |
| `bash scripts/lint.sh` | exit 0, no warning on any file of this story |

The tools were herdr 0.9.1-preview, tmux 3.7c, OpenCode 1.19.0 and `/usr/bin/curl`. After the run there was no `hlr649`/`holler649` process and no new `/tmp/hlr649-*` entry. The ten that exist date from #649's runs earlier today.

## Notes for F, T-green and O

**Deviations from the brief's step table.** Each was decided here and is journalled.

1. **Test B (ii) and (iii) pass `--herdr-session`.** The table writes `profile apply hlr649-probe` without it. `Y` has no record until (iii) succeeds, so every run has a `create` and needs the flag (E32).
2. **The checks use `--check-json`, not `--check-arg=-s`.** The argv stored is the same, and JSON needs no `=` form for `-s` or `-c`.
3. **`Still.registry` is `pane list`'s `data.panes`, not the whole `data`.** The unmanaged table carries `idle_seconds`, which moves with the clock.
4. **`delete_strays` was not moved.** It stays private to #649's scenario because #667 never needed it: doctor was clean at both end states. `findings()` was moved, as decision 3 lists, though no step of #667 calls it.
5. **`scenario.rs` is 551 lines**, against the brief's "about 450". That is under lint's warning at 600 and the gate at 900. The two clippy complexity splits added helpers.

**Assumptions of A's, proven in the opted-in run:**
- doctor finds no stray session after apply's relaunches;
- `owned_pids` is empty after `kill_server`;
- every create and relaunch fits the 10 s budget;
- `curl` and `sh` are on the binary's `PATH`;
- step 5's generation goes up by exactly 2.

**For F:**
- Nothing under `crates/` is F's.
- Decision 14's CHANGELOG entry and the deletion of the #667 bullet of `docs/pane-control.md` section 12 are F's, per W-1.
- `scripts/changelog-check.sh`, `docs_pane_control_test` and `docs_cli_test` were not run here.

**For T-green:**
- A1's second consecutive opted-in run, with the `ps` and `/tmp` check;
- the A16 mutants M1 and M3 to M7;
- the docs tests once F's edits land.

## Ready for F

Confirmed RED is valid. F may implement against these tests. F's part is documentation only (W-1).
