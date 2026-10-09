# Handoff-T-red: #688 the pane test kit, slice c part 2: `FakeProfileScope` and the `ProfileScope` conformance suite

**Date:** 2026-10-09
**Branch:** issue-688-implementation
**Brief / wireframe reviewed:** docs/handoffs/688-brief.md (as amended at 98c250e); no wireframe (no UI surface)

## A precondition
Confirmed: A returned PASS on the plan (docs/handoffs/688/handoff-A.md, re-review after the amendment; warns W-7 and W-8 only).

## Tests authored
Both files start with `#![allow(clippy::unwrap_used, clippy::expect_used, clippy::panic)] // #688`. The testkit crate
auto-discovers `tests/*.rs` (no `autotests = false`), so no `[[test]]` entry is needed. Tier: integration over the two
fakes, the cheapest layer for a port composition. No sleeps: the one concurrency test bounds a possible deadlock with
`recv_timeout`.

`crates/holler-pane-testkit/tests/profile_scope_conformance_test.rs` (AC1, AC2; 6 tests)
- `the_fake_passes_the_profile_scope_conformance_suite`: the fake passes `run_profile_scope_conformance` (AC1).
- `the_suite_runs_the_documented_cases`: `profile_scope_cases()` is the 15 ids in order (AC1).
- `the_unbroken_wrapper_passes_so_a_mutant_fails_for_its_break_alone`: `Break::Nothing` passes (AC2).
- `a_scope_that_writes_the_profile_after_the_act_fails`: fails on `the-act-sees-the-edit` and `failed-act-restores-the-specs`.
- `a_scope_that_does_not_restore_on_a_failed_act_fails`: fails on `failed-act-restores-the-specs`.
- `a_scope_that_checks_membership_on_a_remove_fails`: fails on `remove-of-a-detached-spec-is-not-refused`.
  The mutants delegate to `FakeProfileScope`; the file holds no copy of the edit logic.

`crates/holler-pane-testkit/tests/fake_profile_scope_test.rs` (AC3, AC4; 18 tests)
- AC3: `a_wedged_profile_store_times_out_resolve_and_edit_spec`, `a_wedged_pane_store_times_out_resolve_and_edit_spec`,
  `a_corrupt_profile_store_fails_closed`, `a_failed_restore_returns_its_own_error`,
  `without_a_profile_a_wedged_profile_store_is_not_called`, `the_calls_follow_the_i8_order`.
- AC4: `a_set_whose_spec_names_another_pane_is_usage`, `removing_an_absent_entry_still_writes_the_profile`,
  `membership_compares_slugs` (a `Set`, not a `Remove`), `removing_a_detached_spec_is_not_refused` (also pins pane calls
  `[Get]` for the scope's own read), `a_hook_before_the_restore_makes_it_profile_conflict` (including one-shot),
  `resolve_of_a_pane_with_no_record_is_pane_not_in_profile`, `the_refusals_name_the_pane_and_the_profile`,
  `the_log_carries_the_scopes_actor`, `the_scope_is_send_and_sync`.
- Added from the plan review (A's W-8, and the hook's documented contract in the brief's Public API):
  `a_hook_stays_armed_until_a_failed_act_of_an_edit_with_a_profile` (a failing act without a profile, a conflicting first
  write and a succeeding act all leave the hook armed), `arming_the_hook_again_replaces_an_unused_one`, and
  `a_hook_may_arm_the_hook_again_without_deadlocking` (the hook runs outside the scope's own mutex; the edit runs on a
  thread and the test fails after 10 s instead of hanging).

AC5 to AC10 are F's and S's (no new test: AC5 is the unedited earlier targets, AC6 to AC8 are git/grep checks, AC9 the
CHANGELOG, AC10 the guards). The suite's own 15 cases are F's work (`conformance/profile_scope.rs`, `.../act.rs`); T pins
them through the id list, the fake passing, and the three mutants.

## RED confirmation
Run: `cargo test -p holler-pane-testkit --no-fail-fast`. Both new targets fail to build, and only because the items the
brief specifies do not exist yet:
```
error[E0432]: unresolved imports `holler_pane_testkit::conformance::profile_scope::profile_scope_cases`, `holler_pane_testkit::conformance::profile_scope::run_profile_scope_conformance`
error[E0432]: unresolved import `holler_pane_testkit::profile_scope::FakeProfileScope`   (both files)
error: could not compile `holler-pane-testkit` (test "profile_scope_conformance_test") due to 2 previous errors
error: could not compile `holler-pane-testkit` (test "fake_profile_scope_test") due to 1 previous error
```
These are the missing public API of the brief, not typos, a missing `[[test]]` or a setup error. For a test-kit story whose
API does not exist, an unresolved import is the only possible RED; to be sure the tests also fail (and pass) for the right
reasons once the API exists, T ran a throwaway probe, then reverted it (`git checkout -- crates/holler-pane-testkit/src`;
`git status` shows only the two new test files):
- A ~60-line reference fake written in the probe made all 18 `fake_profile_scope_test` tests pass, so each is satisfiable
  and consistent with the brief's "Fake behaviour".
- A variant of the probe that kept the hook's mutex guard alive while the hook ran made
  `a_hook_may_arm_the_hook_again_without_deadlocking` fail with `edit_spec returned: ... Timeout` after 10 s, so that test
  does detect W-8.
- With a stub suite (`profile_scope_cases()` empty, the runner returning `Ok(())`): `the_suite_runs_the_documented_cases`
  failed on the assertion (left `[]`, right the 15 ids), the three mutant tests failed on `a broken scope must not pass the
  suite`, and the two `Ok(())` tests passed. The mutants' targeted cases could not be exercised without F's suite; their
  logic was checked by reading against cases 8, 9 and 15.
- The earlier targets still build and pass: `pane_store_conformance_test` (10), `fake_pane_store_test` (22),
  `profile_store_conformance_test` (9), `fake_profile_store_test` (23), lib unit tests (4). `rustfmt --check --edition 2021`
  passes on both new files (173 and 506 lines, under the 900 lint).
- Clippy on the new test files cannot run until the API exists; F's GREEN run covers it.

## Ready for F
Confirmed: RED is valid (the only failures are the missing items of the brief's Public API). F may implement against these
tests. Notes for F: `rig()` seeds Alpha `[c1, c2, c3]` at g = 1, so the hook test expects generation 3 after the other
writer and 5 after a second, normal restore; `FakeProfileScope::new` must accept `Arc<FakeProfileStore>` and
`Arc<FakePaneStore>` (coerced to `Arc<dyn ...>`), and `before_next_restore` must run the hook outside the scope's mutex.
