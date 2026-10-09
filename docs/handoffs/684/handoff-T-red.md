# Handoff-T-red: Phase 4 - #684 test kit slice e: `FakeHost`, `FakeHarness` and their conformance suites

**Date:** 2026-10-09
**Branch:** issue-684-implementation
**Brief / wireframe reviewed:** docs/handoffs/684-brief.md (no wireframe: no UI surface), docs/handoffs/684/handoff-A.md

## A precondition
Confirmed: A returned PASS on the plan (Phase 3, `handoff-A.md`, six `warn`s, none blocking). Findings honoured here: W-5(c) (no
private `pane_name` copy; tests use `PaneName::parse(..).unwrap()` and no suite code is written by T), W-4 (`server()` is pinned by
the frozen, thaw, killed, serve-again and serve-on-running tests), W-6(c) (not a T concern).

## Tests authored

Four new files under `crates/holler-pane-testkit/tests/` (no `[[test]]` entries needed: the testkit has default autotests, and
`Cargo.toml` is untouched). Each starts with `#![allow(clippy::unwrap_used, clippy::expect_used, clippy::panic)] // #684`.
All four are `rustfmt --check --edition 2021` clean and, against a signature-only copy of the API, `clippy --tests -D warnings`
clean. `scripts/lint.sh` exits 0 (largest file 576 lines, under the 600 warning).

| File | Lines | Tier |
|---|---|---|
| `tests/host_conformance_test.rs` | 224 | integration (suite as unit under test) |
| `tests/fake_host_test.rs` | 283 | integration (fake only) |
| `tests/harness_conformance_test.rs` | 259 | integration (suite as unit under test) |
| `tests/fake_harness_test.rs` | 576 | integration (fake only) |

Tier choice: the crate's behaviour is only reachable through its public API, so every test is a crate-level integration test, the
same layer as slice a's `pane_store_conformance_test.rs` and `fake_pane_store_test.rs`. There is no cheaper unit tier that the
brief's public API exposes.

### AC 1 and 2: `host_conformance_test.rs`
- `the_fake_passes_the_host_conformance_suite` (AC 1): `run_host_conformance(|| (FakeHost::new(), ()))` is `Ok(())`.
- `the_host_suite_runs_the_documented_cases` (AC 1): `host_cases()` equals the 9 ids in order.
- `the_unbroken_host_wrapper_passes` (AC 2): `Break::Nothing` wrapper is `Ok(())`, so a mutant fails for its break alone.
- Nine `a_host_whose_*_fails` tests, one per break, each asserting the named case is among the failures and every `detail` is
  non-empty: `PsOfMissingIsEmpty` -> `ps-of-missing-session-is-pane-not-found`; `RunCreatesMissingSession` ->
  `run-in-missing-session-is-pane-not-found`; `RunIsNoop` -> `run-adds-a-process`; `EnsureRecreates` ->
  `ensure-session-is-idempotent`; `RunsEmptyArgv` -> `run-empty-argv-is-usage`; `StopIsNoop` ->
  `stop-owned-stops-every-owned-process`; `StopMissingFails` -> `stop-owned-of-missing-session-is-ok`; `StopsEverySession` ->
  `stop-owned-leaves-other-sessions`; `PsListsEverySession` -> `ps-lists-only-its-session`. Mutants are built only from the
  fake's own port methods plus `end_session` and `sessions`.

### AC 3 and 4: `harness_conformance_test.rs`
- `the_fake_passes_the_harness_conformance_suite`, `the_harness_suite_runs_the_documented_cases` (15 ids in order),
  `the_sample_rig_is_two_ports_and_two_panes` (ports `[48100, 48101]`, panes `scratch:demo-c1r1`, `scratch:demo-c2r1`).
- Quirk and data-dir mutants (the fake itself, configured in `fresh`): `a_harness_that_acks_select_without_a_tui_fails` ->
  `select-without-tui-fails`; `a_harness_that_acks_abort_of_an_unknown_id_fails` -> `abort-unknown-is-session-not-found`;
  `a_harness_whose_servers_do_not_share_a_data_dir_fails` (`set_data_dir(48101, "other")`) -> `sessions-shared-across-servers`.
- Wrapper mutants: `the_unbroken_harness_wrapper_passes`, then `HealthAlwaysTrue` -> `health-of-unserved-port-is-false`,
  `PingSession` -> `fresh-server-has-no-sessions`, `ReusesSessionId` -> `create-session-is-listed`, `AttachFallsBackToLatest` ->
  `attach-unknown-is-session-not-found`, `SelectUnknownGoesHome` -> `select-unknown-is-session-not-found`, `SelectBroadcasts`
  -> `select-reaches-only-its-pane`. Mutants are exactly as the brief describes them.

### AC 5: `fake_host_test.rs`
`a_wedged_host_times_out_every_method`, `host_op_names_are_port_dot_method`, `calls_are_recorded_in_order` (+ a small
`exiting_a_process_is_not_a_call_through_the_port`, because the brief's sequence cannot reach a successful `exit_process`
without adding a `run` to the log), `run_records_each_argv_verbatim`, `a_refused_run_records_nothing`,
`ensure_session_keeps_the_first_cwd`, `pids_are_distinct_and_never_reused`, `a_process_that_exits_leaves_ps`,
`an_ended_session_is_missing_until_ensured_again`, `the_fake_host_is_send_and_sync`. One addition from Decision 7:
`a_fresh_session_has_no_process_and_survives_stop_owned` pins the fake's own empty `ps` and surviving session, which the
suite deliberately does not assert. Exact error values from the brief's table are asserted by equality.

### AC 6: `fake_harness_test.rs`
Every test the brief names, under its name: `a_wedged_harness_times_out_every_method`, `harness_op_names_are_port_dot_method`,
`calls_are_recorded_in_order`, `session_ids_are_ses_prefixed_and_distinct`,
`a_frozen_server_answers_health_false_and_times_out_its_calls`, `thaw_brings_a_frozen_server_back`,
`a_killed_server_answers_health_false_and_is_unavailable`, `serving_a_killed_port_again_keeps_the_sessions`,
`serve_on_a_running_port`, `controls_of_an_unserved_port_are_unavailable`, `a_session_deleted_under_a_tui_sends_it_home`,
`navigating_by_hand_changes_the_shown_session`, `closing_the_tui_leaves_no_shown_session`,
`select_without_a_tui_is_acked_with_the_quirk`, `abort_of_an_unknown_id_is_acked_with_the_quirk`,
`separate_data_dirs_do_not_share_sessions`, `a_seeded_session_is_listed_and_bypasses_the_log`,
`the_fake_harness_is_send_and_sync`. One addition: `attaching_again_replaces_the_tui_and_a_refused_attach_leaves_it` (the
brief's `attach_tui` row, unpinned by the suite for a pane that already has a TUI). A's W-4 is honoured: `server()` is asserted
(`ServerView { name, pid, state }`) in the frozen, thaw, killed, serve-again and serve-on-running tests. Beyond the brief's
bullets, the delete test also checks that a TUI on another session stays and that every TUI showing the id goes home, and the
data-dir test also checks that a session made before `set_data_dir` stays where it was and that `select_session` refuses an id
outside the TUI's data dir.

### AC 7 to 11
Not T's: ASSUMPTION comments (F writes them in `harness.rs`), the dependency rule, no other crate changes, CHANGELOG and the
guards. T re-checks 7 and 8 at GREEN.

## RED confirmation

Command: `cargo test -p holler-pane-testkit --no-run`. The brief's test plan defines RED for this slice as "fail to build because
the items do not exist"; the errors are all, and only, `E0432` on the missing items of the brief's API, not typos:

```
error[E0432]: unresolved imports `holler_pane_testkit::conformance::host::host_cases`, `...::run_host_conformance`   (host_conformance_test)
error[E0432]: unresolved import `holler_pane_testkit::host::FakeHost`                                                (host_conformance_test)
error[E0432]: unresolved imports `holler_pane_testkit::host::FakeHost`, `...::HostOp`                                (fake_host_test)
error[E0432]: unresolved imports `...::conformance::harness::{harness_cases, run_harness_conformance, HarnessRig}`   (harness_conformance_test)
error[E0432]: unresolved imports `holler_pane_testkit::harness::FakeHarness`, `...::Quirk`                           (harness_conformance_test)
error[E0432]: unresolved imports `holler_pane_testkit::harness::{FakeHarness, HarnessOp, Quirk, ServerState, ServerView, TuiView}`  (fake_harness_test)
error: could not compile `holler-pane-testkit` (test "fake_harness_test" | "fake_host_test" | "harness_conformance_test" | "host_conformance_test")
```

Because `E0432` stops the compiler before it type-checks a test body, a clean RED alone would hide a test that is itself
mis-typed. To rule that out, I temporarily replaced the four stubs with signature-only items (the brief's exact API, bodies
`unimplemented!()`/empty suites), ran `cargo test -p holler-pane-testkit --no-run` (all six test binaries built) and
`cargo clippy -p holler-pane-testkit --tests -- -D warnings` (no diagnostics), then restored the stubs with
`git checkout -- crates/holler-pane-testkit/src`. `git status` shows only the four new test files. The slice-a tests are
untouched and unaffected by the stubs being restored (they build once F fills the stubs).

Why the tests will fail for the right reason, not just fail to build: every conformance test asserts on `Ok(())` or on a
named case among `CaseFailure`s, and every fake-only test asserts on a returned value from the brief's tables; none can pass
against an empty or no-op implementation (the empty-suite stub, for instance, makes `the_host_suite_runs_the_documented_cases`
and every `a_*_fails` test fail on `expect_err`).

## Ready for F
RED is valid: the four files fail to build solely on the missing API, and compile and lint clean against the API's signatures.
F may implement against them. Notes for F:
- `run_host_conformance` / `run_harness_conformance` must return `Err` with a non-empty `detail` per failing case, and the
  case ids and order must match the tables exactly.
- The harness suite must work with the rig's `ports` on one shared data dir, and `SelectBroadcasts`/`PingSession` mutants use
  only `FakeHarness::tui`, `navigate` and `seed_session`, so those must work as specified.
- Each test file must stay under 900 lines; `fake_harness_test.rs` is at 576.
