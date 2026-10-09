# Handoff-T-red: Phase 4 - #682 part 1, `FakeProfileStore` and the `ProfileStore` conformance suite

**Date:** 2026-10-09
**Branch:** issue-682-implementation (at 9e661ec)
**Brief / wireframe reviewed:** docs/handoffs/682-brief.md (no wireframe: no UI surface)

## A precondition
Confirmed: A returned PASS on the plan (docs/handoffs/682/handoff-A.md, six warns, no block). W-1 (the generation a
`Deleted` entry carries) was not re-settled by O before this phase, so the tests follow the brief: g + 1
(`the_clock_stamps_created_updated_and_at` expects the `Deleted` entry at generation 3 after an update to 2).

## Tests authored
Both files are new, start with `#![allow(clippy::unwrap_used, clippy::expect_used, clippy::panic)] // #682`, and are
auto-discovered (`holler-pane-testkit` has no `autotests = false`). rustfmt-clean. Neither exceeds 600 lines (239 and 593).
Tier for all: integration against the public API of the crate (the cheapest tier that reaches a public item).

`crates/holler-pane-testkit/tests/profile_store_conformance_test.rs` (AC1, AC2):
- `the_fake_passes_the_profile_store_conformance_suite`: AC1, the fake passes the suite.
- `the_suite_runs_the_documented_cases`: AC1, `profile_store_cases()` equals the 23 ids of the brief's table, in order.
- `the_unbroken_wrapper_passes_so_a_mutant_fails_for_its_break_alone`: AC2, `Break::Nothing` is `Ok(())`.
- Six mutants through `assert_suite_fails_on(broken, case)`, each wrapping `FakeProfileStore` and breaking one rule:
  `a_store_that_skips_the_generation_check_fails` (`stale-generation-conflicts`),
  `a_store_that_keeps_the_submitted_generation_fails` (`submitted-generation-ignored`),
  `a_store_that_checks_generation_before_existence_fails` (`delete-missing-is-profile-not-found`),
  `a_store_that_lets_a_same_slug_name_overwrite_fails` (`same-slug-other-name-is-profile-exists`),
  `a_store_that_forgets_the_log_on_delete_fails` (`log-is-append-only-and-oldest-first`),
  `a_store_that_repeats_on_resume_fails` (`watch-resumes-without-gap-or-repeat`).
  The slug mutant relies on the fake filing by slug (`get` of the other display name finds the stored record).
- Not repeated here: the pane suite's `the_guard_lives_for_the_case`. The runner shares `run_cases` with the pane suite,
  which that test already pins, so a second copy would be redundant.

`crates/holler-pane-testkit/tests/fake_profile_store_test.rs` (AC3, AC4):
- AC3: `a_wedged_store_times_out_every_method` (all 7 methods, nothing written, works after `set(None)`),
  `a_wedged_store_ends_an_open_watch`, `a_corrupt_store_fails_closed_everywhere` (records and log intact after),
  `fail_next_is_one_shot_and_per_op`, `a_slow_call_takes_at_least_the_delay` (lower bound only),
  `calls_are_recorded_in_order` (`[Get, CasPut, Log, List]`; `seeded`, `concurrent_*` add nothing),
  `port_op_names_are_port_dot_method` (all 8).
- AC4: `the_clock_stamps_created_updated_and_at`, `an_update_summarises_the_spec_count`,
  `the_submitted_slug_is_replaced_by_the_names_slug`, `a_same_slug_other_name_is_profile_exists_at_any_generation`,
  `a_concurrent_put_makes_the_next_cas_stale`, `a_concurrent_put_creates_a_profile_at_one`,
  `a_concurrent_delete_makes_the_profile_vanish`, `a_watch_ahead_of_the_head_is_usage`,
  `the_idle_wait_wakes_on_a_write` (lower and generous upper bound, no exact duration),
  `seeded_stores_hold_each_profile_at_generation_one`, `seeding_the_same_name_twice_is_a_conflict`,
  `seeding_a_same_slug_name_is_profile_exists`, `sample_profile_is_valid_and_deterministic`,
  `the_fake_is_send_and_sync_and_its_watch_is_send`.
- One test beyond the brief's list: `sample_spec_is_deterministic_harmless_and_agrees_with_sample_pane` pins every
  documented `sample_spec` value and that its model, context and cwd equal `sample_pane`'s (A's W-5: the two fixtures
  must not drift). It sits at the same tier as the other fixture test and duplicates nothing.

AC5 to AC10 are not test files: they are the existing slice-a tests (unchanged, still green, see below), the greps and the
diff checks for S and A, and the CHANGELOG and lint guards for F.

## RED confirmation
Command: `cargo test -p holler-pane-testkit --no-run`. Both new test targets fail to build, and only on the missing items
(the brief's stated RED shape for a test-kit story, where the fake and the suite are the public API under test):
```
error[E0432]: unresolved imports `holler_pane_testkit::conformance::profile_store::profile_store_cases`, `...::run_profile_store_conformance`
error[E0432]: unresolved import `holler_pane_testkit::profile_store::FakeProfileStore`            (profile_store_conformance_test.rs:17)
error[E0432]: unresolved imports `holler_pane_testkit::fixture::sample_profile`, `...::sample_spec`  (fake_profile_store_test.rs:18)
error[E0432]: unresolved imports `holler_pane_testkit::profile_store::FakeProfileStore`, `...::ProfileStoreOp`
error: could not compile `holler-pane-testkit` (test "profile_store_conformance_test") due to 2 previous errors
error: could not compile `holler-pane-testkit` (test "fake_profile_store_test") due to 2 previous errors
```
No other error appears. Import errors can mask type errors in the same file, so I also compiled both files against a
throwaway stub of the API (types and signatures only, then reverted with `git checkout -- crates/holler-pane-testkit/src`;
no stub is staged or left in the tree). With the stub both files compile with no error or warning, so F will meet no typo of
mine. Run against the stub, 21 of the 22 fake tests fail (the stub returns `NotImplemented` and its fixture is empty) and
`the_fake_is_send_and_sync_and_its_watch_is_send` passes, as a compile-time bound check is expected to.

Slice a is unaffected: `cargo test -p holler-pane-testkit --test pane_store_conformance_test --test fake_pane_store_test`
passes (fake_pane_store_test 22 tests, pane_store_conformance_test 10 tests).

## Ready for F
RED is valid: the two test targets do not build because `FakeProfileStore`, `ProfileStoreOp`, `profile_store_cases`,
`run_profile_store_conformance`, `sample_spec` and `sample_profile` do not exist. F may implement against them. Notes for F:
- `fake_profile_store_test.rs` is 593 lines, under lint.sh's 600-line warn and 900 fail, so do not add to it. The fix, if
  rustfmt grows it, is a split by T, not a trim of an assertion.
- The tests pin `ProfileNotFound { what: <display name> }` for `delete`, `concurrent_delete` and `log`, as the brief says
  ("`ProfileNotFound { what: name }`"), and `Conflict` (not `ProfileExists`) for seeding one name twice.
- `Deleted` carries the deleted generation + 1 (brief decision 4, A's W-1).
