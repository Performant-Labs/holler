# Handoff-T-red: #683 the pane test kit, slice d (`FakeHerdr`, `FakeProber`, `HerdrPort` conformance suite)

**Date:** 2026-10-09
**Branch:** issue-683-implementation
**Brief / wireframe reviewed:** `docs/handoffs/683-brief.md` (amended after plan review); no wireframe (`uiSurface: false`)

## A precondition
Confirmed: A returned PASS on the plan (`docs/handoffs/683/handoff-A.md`, round 2; commit b076000).

## Tests authored
Three new files in `crates/holler-pane-testkit/tests/`. The crate has no `autotests = false`, so no `[[test]]` entry is
needed (and no manifest may change, AC 5). Each starts with the `#![allow(clippy::unwrap_used, clippy::expect_used,
clippy::panic)] // #683` line.

`herdr_conformance_test.rs` (494 lines; integration tier, the suite is the unit under test):
- `the_fake_passes_the_herdr_conformance_suite`, `the_fake_in_split_only_mode_passes_the_suite`: AC 1, both placements.
- `the_suite_runs_the_documented_cases`: `herdr_cases()` equals the 11 ids of the brief's table, in order.
- `the_suite_builds_a_fresh_fixture_per_case`: `fresh` called 11 times, 11 guards dropped, no port call after its guard.
- `the_unbroken_wrapper_passes_so_a_mutant_fails_for_its_break_alone`, plus one test per mutant (AC 2), each through
  `assert_suite_fails_on(broken, &[cases])`, which asserts `Err`, every named case among the failures and every `detail`
  non-empty. The wrapper `Mutant { inner: FakeHerdr, broken: Break }` implements `HerdrPort` in the test file only:
  - `a_transposing_herdr_fails` -> `ensure-r1c2-is-grid-out-of-range` and `ensure-r2c1-reads-back-as-r2c1`
  - `a_herdr_that_reads_back_transposed_fails` -> `ensure-r2c1-reads-back-as-r2c1`
  - `a_herdr_that_creates_on_every_ensure_fails` -> `ensure-is-idempotent`
  - `a_herdr_that_rebuilds_the_workspace_fails` -> `ensure-never-moves-another-pane`
  - `a_herdr_with_position_ids_fails` -> `closed-id-is-never-reused`
  - `a_herdr_that_renumbers_on_close_fails` -> `ids-unique-and-stable-when-a-sibling-closes`
  - `a_herdr_whose_close_is_idempotent_fails` -> `close-twice-is-pane-not-found` and `close-unknown-is-pane-not-found`
  - `a_herdr_whose_closed_panes_still_answer_fails` -> `calls-on-a-closed-pane-are-pane-not-found`
  - `a_herdr_that_ignores_max_lines_fails` -> `read-returns-at-most-max-lines`
  - `an_unsupported_herdr_fails_the_version_case` (no wrapper; also asserts that case is the only one failing)

`fake_herdr_test.rs` (538 lines; integration tier against the public API, the cheapest layer for a crate with no
in-module tests): every test named in AC 3, plus three the brief's text implies:
- `the_pane_counter_carries_into_two_digits_in_base_36` (the 36th pane is `w1:p10`, split out of the ids test)
- `split_only_mode_returns_an_occupant_without_a_split` (the occupied rule holds in both placements, brief step 3)
- `print_to_an_unknown_pane_is_pane_not_found`
The ids test also vanishes `w1:pA` and checks the next id is `w1:pE`, so a vanished id is not reused either.
Timing: `a_slow_call_takes_at_least_the_delay` asserts a lower bound only.

`fake_prober_test.rs` (139 lines; AC 4): the five named tests, plus `an_unscripted_error_names_the_argv` (the brief
fixes the message as `no probe scripted for {argv:?}`).

I dropped one test I had drafted (a freshness check that only tested my own closure): it proved nothing about the suite.

## RED confirmation
Command: `cargo test -p holler-pane-testkit --no-run`. The brief's test plan fixes the RED here as a build failure, because
the three stubs hold no item at all, so there is nothing for an assertion to run against. Every error is a missing item:
```
error[E0432]: unresolved imports `holler_pane_testkit::prober::FakeProber`, `holler_pane_testkit::prober::ProbeCall`
error[E0432]: unresolved imports `holler_pane_testkit::herdr::FakeHerdr`, `...::HerdrVersion`, `...::Placement`
error[E0432]: unresolved imports `holler_pane_testkit::herdr::FakeHerdr`, `...::HerdrOp`, `...::HerdrVersion`, `...::Placement`,
  `...::Sent`, `...::GRID_UNREACHABLE`, `...::PROTOCOL_22_VERSION`, `...::SUPPORTED_VERSIONS`, `...::UNSUPPORTED_VERSION`
error[E0432]: unresolved imports `holler_pane_testkit::conformance::herdr::herdr_cases`, `...::run_herdr_conformance`, `...::HerdrFixture`
error: could not compile `holler-pane-testkit` (test "herdr_conformance_test" / "fake_prober_test" / "fake_herdr_test")
```
No E0425/E0599/E0308 or other error: only the four expected unresolved-import groups, in exactly the three new files.

A compile-only RED hides type errors behind the resolution errors, so I also checked the tests against throwaway
signature-only stubs of the brief's API (bodies `todo!()`, reverted with `git checkout -- crates/holler-pane-testkit/src`;
nothing of them is staged or left in the tree). With them, all three files build with no rustc error or warning. `cargo clippy --all-targets -D warnings` stopped at
one lint caused only by the unit-struct stub (`FakeProber::default()` on a unit struct), so clippy over the tests is
not proven here and is left to T-green. The 22 `fake_herdr_test` tests then fail inside the stubs (`todo!()`), or, for
`port_op_names_are_herdr_dot_method`, on the assertion against the stub's empty `as_str`; the other two files were not
run against the stubs. So the tests build against the API as the brief specifies it.

Slice a is unharmed: `cargo test -p holler-pane-testkit --test fake_pane_store_test --test pane_store_conformance_test`
passes (22 and 10 tests). `bash scripts/lint.sh` exits 0 (largest new file 538 lines, under the 600 warning).

## Ready for F
RED is valid: the three new test files fail to build for one reason, the missing items the brief tells F to create, and
nothing else in them is wrong. F may implement against them. Two things for T-green, since the suite does not exist yet:
- The mutants cannot be run until `conformance/herdr.rs` exists; T-green runs each and repairs any that fails on a
  different case than its named one (brief, Risks).
- `a_vanished_pane...`, `ids_are_minted...` and `read_returns_the_last_lines...` pin details the brief states only
  implicitly (a vanished id is not reused; `read` of `"...ls\n\n"` ends with an empty last line, from `str::lines`).
  If F reads the brief differently, the fix is a test repair, not a weaker implementation.
