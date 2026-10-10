# Handoff-T-red: Phase 4 - #644 `holler pane launch` and `relaunch` (RED)

**Date:** 2026-10-09
**Branch:** issue-644-implementation (worktree `.claude/worktrees/0644-launch-relaunch`, head `c6004ba` + this phase's staged changes)
**Brief / wireframe reviewed:** `docs/handoffs/644-brief.md` (as amended through `40f486e`); `docs/handoffs/644/handoff-A.md` (re-review, PASS); no wireframe (no UI surface)

## A precondition

Confirmed: A returned **PASS** on the amended plan (`handoff-A.md`, eight warns, no block).

Dependency preflight (Test plan), run on this branch, which holds #662a and #663:

- `profile_snapshot.rs` has `FIXED_PORT_POLICY_PREFIX: &str = "fixed:"` (line 18), `fixed_port_policy(port: u16) -> String` (line 21) and `spec_from_pane(pane: &Pane) -> ProfileSpec` (line 46).
- `profile_scope.rs` has `reconcile_step(profile: Option<&ProfileName>) -> String` (line 49), `pub struct StoreScope` (line 59) and `StoreScope::new(profiles, panes, actor)` (line 68).

Every pasted signature matches the merged code, so there is no `preflight-failed`.

## What T landed before the tests (no logic)

- **`crates/holler-pane/src/tx_launch.rs`**: every public item of the brief's API, with the constants at their values. That covers `PANE_EXISTS`, `GRID_OCCUPIED`, `PORT_IN_USE`, `DEFAULT_BUDGET`, `PROBE_TIMEOUT`, `HOST_NAME`, `OP_LAUNCH` and `OP_RELAUNCH`. It also has:
  - `TxOptions` and its `Default` (`now_ms` is `holler_proto::clock::now_millis`);
  - `LaunchRequest`, `RelaunchRequest`, `Launched`, `TxFailure` and `From<PaneError> for TxFailure`.

  `launch`, `relaunch` and `port_of_policy` are stubs marked `// stub (#644 RED): F fills`. They answer `NotImplemented` (`acted: false`).
- **`crates/holler-cli/src/pane/launch.rs` and `relaunch.rs`**: the new `Args`. Each has the positional `PANE`, a plain `String` at clap time; launch also has `--herdr-session NAME`. `run` still answers `not_implemented(644)`.
- **Surface**:
  - `tests/fixtures/cli-surface.txt`: each `# #644` line gains `demo-c1r1`, and one line adds `--herdr-session scratch`.
  - `docs/adr/ADR-0003.md`: rows 48-49 become `holler pane launch PANE [--herdr-session NAME] [SPEC FLAGS] [--profile NAME] [--spec-only]` and `holler pane relaunch PANE [SPEC FLAGS] [--profile NAME] [--spec-only]`.
  - `tests/pane_verbs/process/stub.rs`: the two `644` entries are deleted, and the `// #644` line is kept.

Deviations from the brief's text, each needed to keep a gate green or taken from a plan-review warn (journalled):

1. **`spec: Box<SpecFlags>`** in both `Args` (A-2/A-3 had a plain `SpecFlags`).
   - With the new positional, `PaneLaunch` and `PaneRelaunch` make the frozen `Command` enum (`cli.rs:102`) fail `clippy::large_enum_variant`: the largest variant is 408 bytes and the next is 176.
   - `cli.rs` and `pane/mod.rs` are frozen, so the box goes in the verbs' own files. clap 4.6 flattens a `Box<T: Args>` (`clap_builder` `derive.rs:366`).
   - `args.spec.validate()` still works by auto-deref.
2. **The positional field is named `pane`** (A warn 8; the brief's API says `name`). Tests go through argv and never see the field.
3. The fixture's `--port-policy fixed` line is now `fixed:48100`. The fixture is parse-only, and `fixed:48100` is the form launch accepts (decision 5).

## Tests authored

There are 57 tests: 54 behaviour tests, two kept #670 flag tests and one rig self-check. All are in the `pane_verbs` target (`cargo test -p holler-cli --test pane_verbs`), at the in-process verb tier: `run_verb_with` over the test kit's fakes, no subprocess. That is the cheapest tier that exercises the CLI parse, the effective spec, the engine and the output together. The four engine-only criteria (AC 15's policy round trip, 19b and 24) call `holler_pane::tx_launch::*` directly over the same rig, which is the tier the brief names for them.

**The rig** (`tests/pane_verbs/launch/rig.rs`, declared `pub(crate) mod rig;` in `launch.rs` and reached by `relaunch.rs` as `crate::launch::rig`, per A warn 4):

- It builds on #643's `crate::list::Rig`, whose fakes are used unchanged, with `FakeHerdr::new("scratch").with_workspace("main", 3, 3)`.
- **Linked host**: `stop_owned(name)` also calls `FakeHarness::kill(p)` for each port in 48100-48102 that was served for `name`. `Rig::unlinked()` turns this off (AC 22).
- **Harness hooks**: `rig.after(HarnessOp, |fakes, called| ..)` runs a closure once, after that method returns, with no lock held. Herdr hooks are not needed: the one Herdr scenario, the vanish, runs from the `attach_tui` hook.
- `with_store_scope()` swaps in #663's `StoreScope` (AC 16k).
- **Call-log span**: `run` and `engine` return the calls made during the run only (`Calls`, as in doctor's rig), so a test's own setup and post-run reads are never asserted.
- `assert_matches` (AC 2), `assert_no_keystroke` (I4: no `SendText` or `SendKeys` in the whole Herdr call log), and `assert_untouched` (nothing live was called, and neither store has a `CasPut`, `Delete` or `Rename`).
- **`seed_live` / `live(profile)`**: a pane as a successful `LAUNCH` (optionally with `--profile demo`) leaves it, built through the fakes' own port methods rather than the verb. Relaunch's cases start from it, so each relaunch test fails on relaunch alone, not on launch's setup. This is a deviation from the brief's "after `LAUNCH`", which made all 20 relaunch cases fail on their setup. `the_rig_seeds_a_pane_the_fakes_agree_with` checks the seed: `assert_matches` holds, and P's spec equals `LAUNCH`'s effective spec. It passes at RED because it tests the rig, not the feature.

**Format.** Most cases run with `--format json`, so the error code comes from `check_envelope`, which also checks the exit code against the code's class. AC 17 pins that a text run's stderr is exactly `error: ` + the JSON `error.message` + `\n`. The brief's "stderr carries / ends with the step" assertions (AC 6, 8, 16h-16k, 19c) are therefore made on the JSON message, which is what "Where the reconcile step lands" says is the same text.

| Test (module) | Criterion / behaviour pinned |
|---|---|
| `launch::launch_records_what_the_fakes_show` | AC 1 (the whole record as one `Pane` comparison; one session, no ping session; the TUI shows it; `driven: None`; harness log `[Health, Serve, Health, CreateSession, AttachTui, ShownSession]`, Herdr log `[Version, Snapshot, EnsurePane, Snapshot]`); also AC 16f (no profile call) and AC 11b's "without a command, `runs()` is empty" |
| `launch::launch_records_model_effort_env_and_ceilings` | AC 3 (C1 recorded) |
| `launch::launch_onto_a_frozen_server_times_out_before_any_session` | AC 4a |
| `launch::launch_aborts_when_the_server_never_gets_healthy` | AC 4b (`unavailable`, created pane closed) |
| `launch::launch_fails_when_its_herdr_pane_vanishes` | AC 5a (O2; message names the pane id) |
| `launch::launch_record_conflict_fails_loudly` | AC 6b (record conflict: no rollback, unscoped step, live state left) |
| `launch::a_crash_mid_launch_leaves_no_record` | AC 7 (`catch_unwind`; no `CasPut`; Herdr pane and tmux session left) |
| `launch::a_failed_attach_rolls_back` | AC 8 (code `session-not-found`, the step once, `Close`, tmux session stays with no process, server `Killed`) |
| `launch::the_command_reaches_the_host_as_argv` | AC 11b |
| `launch::every_grid_form_reaches_herdr_as_row_2_col_1` | AC 13a (text and JSON, fresh rig each) |
| `launch::exit_codes_equal_across_formats` | AC 17 (AC 1, 8, 9a, 10a, 11a, 13b, 15, 16e and 6a; each case's expected exit code is also asserted, so the stub cannot pass it) |
| `launch::the_budget_bounds_a_slow_launch` | AC 24 (engine; `Timeout { op: "pane.launch" }`, `acted: true`, under 3 s, harness log `[Health, Serve]`, Herdr log `[Version, Snapshot, EnsurePane, Close]`) |
| `launch::the_rig_seeds_a_pane_the_fakes_agree_with` | rig self-check (passes at RED) |
| `launch::pane_launch_accepts_every_spec_flag` | #670 AC 5, kept (passes) |
| `launch::guards::launch_of_a_recorded_name_is_pane_exists` | AC 9a |
| `launch::guards::launch_refuses_a_cell_another_record_holds` | AC 9b (occupant id and `demo-c2r1` named; Herdr log `[Version, Snapshot]`; also with the record's stored grid stale at `r3c1`) |
| `launch::guards::launch_ignores_a_stale_record_at_a_free_cell` | AC 9b (exit 0) |
| `launch::guards::launch_never_adopts_an_unrecorded_pane` | AC 9c ("no record" phrase; pane never closed) |
| `launch::guards::launch_never_adopts_a_running_server` | AC 9d (harness log `[Health]`) |
| `launch::guards::a_failing_probe_refuses_before_any_step` | AC 10a and AC 16d (merged: one scenario; P at generation 1, no `CasPut`, change log `[1]`) |
| `launch::guards::an_unscripted_probe_refuses` | AC 10b |
| `launch::guards::a_passing_probe_is_recorded` | AC 10c (`probe.last: Some(Ok)`; one call with `PROBE_TIMEOUT`) |
| `launch::guards::a_command_string_is_command_not_argv` | AC 11a |
| `launch::guards::an_env_value_is_refused_and_not_echoed` | AC 12 (text and JSON; the secret is absent from both streams) |
| `launch::guards::an_ambiguous_grid_is_refused_before_any_step` | AC 13b |
| `launch::guards::a_cell_outside_the_workspace_is_out_of_range` | AC 13c |
| `launch::guards::an_unsupported_herdr_is_refused` | AC 14 |
| `launch::guards::launch_names_every_missing_flag` | AC 15 (one `usage` line naming all eight flags) |
| `launch::guards::bad_policies_and_ceilings_are_usage` | AC 15 (`fixed`, `fixed:048100`, `--ctx-soft 2 --ctx-hard 1`) |
| `launch::guards::a_live_launch_needs_a_herdr_session` | AC 15 |
| `launch::guards::port_policy_round_trips_with_the_snapshot` | AC 15 / decision 5 (engine fn; plus `fixed`, `fixed:048100` and `auto` refused) |
| `launch::profiles::launch_with_profile_adds_the_spec_and_bumps_once` | AC 16a |
| `launch::profiles::a_failed_act_restores_the_profile_specs` | AC 16c |
| `launch::profiles::a_missing_profile_is_refused` | AC 16e |
| `launch::profiles::spec_only_changes_the_profile_and_nothing_live` | AC 16g, launch half (pane-store span exactly `[Get]`) |
| `launch::profiles::a_profile_conflict_after_the_act_fails_loudly` | AC 16h |
| `launch::profiles::the_reconcile_step_quotes_the_profile` | AC 16j |
| `launch::profiles::a_step_the_real_scope_printed_is_not_repeated` | AC 16k (both halves, over `StoreScope`) |
| `relaunch::relaunch_recreates_a_vanished_pane_at_its_cell` | AC 5b |
| `relaunch::relaunch_fails_on_a_stale_generation` | AC 6a |
| `relaunch::relaunch_with_profile_and_model_updates_the_spec` | AC 16b |
| `relaunch::relaunch_spec_only_changes_the_profile_and_nothing_live` | AC 16g, relaunch half |
| `relaunch::relaunch_refuses_a_pane_of_another_profile` | AC 16i |
| `relaunch::relaunch_without_grid_keeps_the_position` | AC 18 |
| `relaunch::a_failed_relaunch_never_closes_the_records_pane` | AC 18b |
| `relaunch::relaunch_with_grid_moves_the_pane` | AC 19 (the move) |
| `relaunch::relaunch_refuses_a_move_without_grid_and_a_new_directory` | AC 19 (the two `usage` refusals; the workspace message names `main`, `other` and `r2c1`) |
| `relaunch::relaunch_engine_enforces_its_rules` | AC 19b, the three named relaunch-engine cases, each live (`usage`, `acted: false`, untouched) and with `spec_only` (the first two accepted, P holds the drift, nothing live; the third still `usage`) |
| `relaunch::launch_engine_refuses_a_spec_for_another_pane_and_spec_only_without_a_profile` | AC 19b (`launch_engine_refuses_a_spec_for_another_pane`, `engine_refuses_spec_only_without_a_profile` for both engines; pane-store log empty) |
| `relaunch::relaunch_records_the_move_before_closing_the_old_pane` | AC 19c |
| `relaunch::relaunch_keeps_the_session_of_record` | AC 20 (kept sid; no `CreateSession`; record, SHOWN and TUI agree; `driven` stays `None`) |
| `relaunch::relaunch_replaces_a_deleted_session` | AC 20 |
| `relaunch::relaunch_keeps_the_stored_driven` | AC 20 / decision 22 |
| `relaunch::relaunch_leaves_other_panes_alone` | AC 21 (both panes run a command, so `ps` is not trivially empty) |
| `relaunch::relaunch_fails_when_the_old_server_survives` | AC 22 (unlinked host) |
| `relaunch::relaunch_of_a_missing_pane_is_refused` | AC 23 |
| `relaunch::pane_relaunch_accepts_every_spec_flag` | #670 AC 5, kept (passes) |

The two #670 stub-routing tests per verb (`*_stub_routes_*`, `pane_launch_json_mode_with_shared_flags_is_still_one_envelope`) are deleted. They ran the verbs with no `PANE`, which no longer parses, and they pinned the stub this story replaces.

AC 25-31 are greps and gates, not tests:

- AC 26's surface tests and greps already pass (below).
- AC 25 and AC 29 pass on the stubs; T-green re-runs them on F's code.
- AC 27 (the ADR-0021 edits), AC 28 (the CHANGELOG) and AC 30-31 (the gates on F's code, the public-repo check) are F's and S's.

## RED confirmation

`cargo test -p holler-cli --test pane_verbs -- launch:: relaunch::`

```
test result: FAILED. 3 passed; 54 failed; 0 ignored; 0 measured; 130 filtered out; finished in 0.06s
```

The three that pass are `the_rig_seeds_a_pane_the_fakes_agree_with`, `pane_launch_accepts_every_spec_flag` and `pane_relaunch_accepts_every_spec_flag`. Every one of the 54 fails on an assertion about the missing behaviour. None fails on a compile error, a missing target or a setup error. The first failing assertion of each falls into one of these groups:

- **Exit code 1 (`not-implemented`) where 0, 2 or 3 is expected**:
  - AC 3, 9a-9d, 10a-10c, 11a, 11b, 12, 13a-13c, 14, 15 (all three CLI cases), 16a, 16c, 16e, 16g (both), 16b, 16i, 18, 18b, 19 (both), 20 (all three), 21 and 23;
  - AC 1, from `assert_eq!((run.code, run.err.as_str()), (0, ""))`;
  - AC 17, as `AC 1: left 1, right 0`.

  For example, `launch_never_adopts_a_running_server` fails at `guards.rs:113` (left 1, right 3), with stdout `{"schema_version":1,"ok":false,"data":null,"error":{"code":"not-implemented","message":"not implemented (story #644)"}}`.
- **The right exit (1), the wrong code**: the code is `not-implemented` where the test expects:
  - `timeout` (AC 4a);
  - `unavailable` (AC 4b, 5a, 19c, 22);
  - `generation-conflict` (AC 6a, 6b);
  - `profile-conflict` (AC 16h, 16j, 16k).
- **AC 7**: `the run crashed: Ok(Run { outcome: Outcome { code: 1, ... err: "error: not implemented (story #644)\n" } ... })`. The stub returns without ever calling `serve`, so the hook's panic never fires.
- **The engine stubs**:
  - AC 24: `left: Err(TxFailure { error: NotImplemented, acted: false })`, `right: Err(TxFailure { error: Timeout { op: "pane.launch" }, acted: true })`;
  - AC 15 round trip: `left: Err(NotImplemented)`, `right: Ok(1)`;
  - AC 19b: `relaunch_engine_refuses_a_cell_change_without_grid: Err(TxFailure { error: NotImplemented, acted: false })` and `LaunchRequest { ..., pane: "demo-c2r1", ... }: Err(TxFailure { error: NotImplemented, ... })` (not the expected plan `usage`).

Every relaunch case starts from `seed_live`, which `the_rig_seeds_a_pane_the_fakes_agree_with` proves consistent. So each relaunch failure comes from the relaunch run itself: `rig.rs:546` is `data_of`'s exit-0 check on that run. None comes from setup.

**Gates on the RED tree**:

| Check | Result |
|---|---|
| `rustfmt --check --edition 2021` on the 8 touched `.rs` files | exit 0 |
| `cargo clippy --workspace --all-targets -- -D warnings` | exit 0 |
| `bash scripts/lint.sh` | exit 0; its only note on these files is a warn that `launch/rig.rs` is 711 lines |
| file sizes | rig 711, `launch.rs` 581, `relaunch.rs` 519, `guards.rs` 345, `profiles.rs` 205 lines; all under 800 |
| the other 133 `pane_verbs` tests | all pass |
| `cargo test -p holler-cli --test cli_surface_test --test docs_cli_test --test pane_cli_process` | 3 + 3 + 34 pass. Risk 4 is confirmed: clap's conflict and `requires` errors still win over the missing `PANE` |
| `cargo test -p holler-pane` | passes |
| `bash scripts/test-hooks.sh` | exit 0 |

AC 26's greps: `grep -nE '"(re)?launch", 644' .../stub.rs` prints nothing, and `grep -c '^    // #644$'` prints `1`. AC 25's greps print nothing, and so do AC 29's diffs.

## Ready for F

**RED is valid, and F may implement against these tests.**

Notes for F, from the RED tests:

- `PaneLaunch.spec` and `PaneRelaunch.spec` are `Box<SpecFlags>` (see above); keep the box.
- A conflict-path message must contain `to reconcile, run` exactly once. The no-record occupant message must contain the phrase `no record`. The B10 message must contain `was not closed` and the old pane id. The B2 message must contain `still answers`.
- Sanitizing ids with `findings::quoted` (A warn 1) is compatible with every assertion: each test checks only that the id is contained in the message.
- AC 16g pins the launch `--spec-only` run's pane-store call log as exactly `[Get]`, so the CLI must not read the pane record on launch.
