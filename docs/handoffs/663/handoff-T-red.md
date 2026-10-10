# Handoff-T-red: Phase 4 - #663 the `--profile` helper (`StoreScope`) and the probe runner (`run_probe`)

**Date:** 2026-10-09
**Branch:** issue-663-implementation (head `2dad8bf` plus the staged RED files below)
**Brief / wireframe reviewed:** `docs/handoffs/663-brief.md` (as of `2dad8bf`); wireframe N/A (no UI surface)

## A precondition

Confirmed: A returned PASS on the plan (`docs/handoffs/663/handoff-A.md`, second pass, commit `2dad8bf`), with warns W-7 to
W-12 left for F within the existing ACs. None of them changes what a test asserts.

## What T landed besides tests (the brief's Test plan)

The brief's Test plan has T land a compile stub in `crates/holler-cli/src/pane/profile_scope.rs` first, so that a missing
symbol cannot pass for a RED. The stub has the exact public items of Decisions 1 and 8:

- `pub struct StoreScope { profiles: Arc<dyn ProfileStore>, panes: Arc<dyn PaneStore>, actor: Actor }` and `StoreScope::new`;
- an `impl ProfileScope` whose two methods answer `Err(PaneError::NotImplemented)`;
- `pub fn reconcile_step(&ProfileName) -> String`, returning `String::new()`;
- `pub const RECONCILE_STEP_UNSCOPED: &str = ""`.

The stub's bodies read their fields and parameters into `let _`, so `dead_code = "deny"` does not fail the build. F
replaces every body and the module docs. `crates/holler-pane/src/probe.rs`'s stub body is unchanged; T only appended the
test module.

## Tests authored

Every test is inline (Decision 21; the blast radius has no test file). Each module opens with
`#[allow(clippy::unwrap_used, clippy::expect_used, clippy::panic)] // #663`.

### `crates/holler-cli/src/pane/profile_scope.rs`, `mod tests` (unit tier, over the test kit's fakes)

The fixture for AC 2-7 is `Demo Alpha` (`demo-c1r1`, `demo-c2r1`), seeded by the actor `conformance`, so it is at
generation 1. The two pane records name `Demo Alpha`. `s'` is `sample_spec(demo-c1r1)` with `context.soft + 1`.

| Test | Pins | Why this tier |
|---|---|---|
| `store_scope_passes_the_profile_scope_conformance_suite` | AC 1: all 15 cases of #638's suite against `StoreScope` (the issue's first acceptance bullet) | The suite runs over in-memory fakes, so no process is needed. It reuses the suite and copies none of its cases. |
| `restore_failure_keeps_its_code_and_names_the_unrestored_edit` | AC 2 / Decision 5. The test runs once each for `timeout`, `unavailable` and `store-corrupt` on the restoring write. It checks that:<br>- the code is kept;<br>- the message names `Demo Alpha`, `demo-c1r1`, `unavailable: act` and both reconcile command lines, on one line;<br>- the act ran once;<br>- the calls are exactly `[Get, CasPut, CasPut]`;<br>- P is at generation 2 and holds `s'`. | Unit. The open point is not pinned by the suite. |
| `restore_conflict_names_the_act_error_and_the_reconcile_step` | AC 3 / Decision 7. The code is `profile-conflict`, and the message has the same five parts and no `\n`. | Unit. Case 11 pins only the code and the name. |
| `first_write_timeout_says_the_edit_may_have_landed` | AC 4 / Decision 6. A first-write `timeout` gets `may hold the edit` and the doctor step, and the act runs 0 times. A first-write `unavailable` is passed through exactly, and the act runs 0 times. | Unit |
| `reconcile_step_single_quotes_the_profile_name` | AC 5 / Decision 8. It checks:<br>- the exact text for `Demo Alpha`;<br>- POSIX quoting of `It's $(id) Demo`, with no `\n`;<br>- `RECONCILE_STEP_UNSCOPED` equals #644's text exactly. | Unit, a pure function |
| `set_of_a_spec_for_another_pane_is_usage_before_any_write` | AC 6 / Decision 9. The code is `usage`, the act runs 0 times, and the profile store sees no `CasPut`. | Unit |
| `pane_store_fault_fails_a_remove_before_the_profile_write` | AC 7 / Decision 10. A pane-store `Get` fault on a `Remove` is answered exactly, the act runs 0 times, there is no `CasPut`, and P stays at generation 1. | Unit |

### `crates/holler-pane/src/probe.rs`, `mod tests` (unit tier, a real harmless child process each)

`Scratch` is a private guard. It creates `std::env::temp_dir()/hlr-probe-663-<pid>-<n>` and removes it on `Drop`.
`assert_gone_within_2s` is a counted poll of at most 40 rounds of `ps -o stat= -p <pid>`, 50 ms apart. A pid passes when the
output is empty, `ps` exits non-zero, or the state starts with `Z`. Both helpers stay private and minimal, as W-11
pre-ruled. The tests send no signal themselves.

| Test | Pins |
|---|---|
| `all_expected_strings_present_is_ok` | 8a: `printf` with `alpha` and `beta` is `Ok`. |
| `one_missing_string_is_failed_naming_it` | 8b: the answer is exactly `Failed { missing: ["gamma"] }`. |
| `no_expect_and_exit_zero_is_ok` | 8c: `true` with an empty `expect` is `Ok`. |
| `non_zero_exit_is_error_even_with_every_string` | 8d / Decision 14: `exit 3` with every string present is `Error` with `exited with status 3`; `false` gives `exited with status 1`. |
| `hung_command_is_error_at_the_timeout` | 8e / Decision 15 and C7: `sleep 30` at 300 ms. It asserts `elapsed >= 300 ms` first, then `timed out`, then `elapsed < 2,300 ms`. |
| `timeout_kills_the_whole_process_group` | 8f / Decisions 15-16: `sh -c 'sleep 30 & echo $! > "$0"; wait' <dir>/pid` at 500 ms. It asserts that `pid` is absent before the call. After the return, in this order, it asserts that the file **exists**, then `timed out`, then that the `sleep` is gone or a zombie within 2 s. |
| `background_child_holding_stdout_is_a_timeout` | 8g: the same order with `...; echo up` and `expect ["up"]`. The answer must be `Error` with `timed out`, never `Ok`. |
| `argv_is_never_given_to_a_shell` | 8h / Decision 13. `;`, `$(touch m1)` and `x; touch m2` arrive literally, so the answer is `Ok` and neither marker exists. The one-element `["printf hello"]` gives `could not be started`. |
| `output_over_the_cap_is_error` | 8i / Decision 17: `yes` gives `more than 1 MiB` in under 5 s. |
| `reasons_never_echo_argv_or_output` | 8j / Decision 19. A missing program gives `could not be started`, and the reason has no `SENTINELARG` or `SECRET663ARG`. An exit 4 that printed `SECRET663OUT` leaves no `SECRET663OUT` in the reason. `printf SECRET663OUT` with `expect ["absent"]` is exactly `Failed { missing: ["absent"] }`. |
| `empty_argv_and_zero_timeout_are_errors_without_a_process` | 8k: an empty argv gives `empty`. `Duration::ZERO` gives `zero`, and the `touch` marker does not exist. |
| `non_utf8_output_still_matches` | 8l / Decision 18: `printf '\377alpha\376'` with `expect ["alpha"]` is `Ok`. |

8m, the existing `ports_test::run_probe_stub_never_reports_success`, is untouched. It stays green by design, as a
regression guard.

**What is not tested here, by design.** AC 9-14 are greps, gates and document checks that T-green runs. Proportionality: one
test per AC (AC 2 loops over its three errors), and nothing repeats a conformance case. AC 6 and 7 pin points the suite
leaves unpinned (Decisions 9 and 10).

## RED confirmation

`cargo test -p holler-cli --lib pane::profile_scope` gives `0 passed; 7 failed`. Each test fails on its feature assertion:

- `store_scope_passes_the_profile_scope_conformance_suite`: `assert_eq!(result, Ok(()))`. There are 15 `CaseFailure`s, each
  `not-implemented`. One example is `resolve(Alpha, None) failed with 'not-implemented'`. Case 11's is `the runs of the act
  ... expected 1, got 0`.
- `restore_failure_keeps_its_code_and_names_the_unrestored_edit`: `left: "not-implemented" right: "timeout"` (`answer:
  Err(NotImplemented)`).
- `restore_conflict_names_the_act_error_and_the_reconcile_step`: `left: "not-implemented" right: "profile-conflict"`.
- `first_write_timeout_says_the_edit_may_have_landed`: `left: "not-implemented" right: "timeout"`.
- `reconcile_step_single_quotes_the_profile_name`: `left: "" right: "to reconcile, run holler pane doctor --profile 'Demo
  Alpha' and then holler profile show 'Demo Alpha'"`.
- `set_of_a_spec_for_another_pane_is_usage_before_any_write`: `left: "not-implemented" right: "usage"`.
- `pane_store_fault_fails_a_remove_before_the_profile_write`: `left: Err(NotImplemented) right: Err(Unavailable { what:
  "pane store" })`.

`cargo test -p holler-pane --lib probe::tests` gives `0 passed; 12 failed`. Each test fails on the stub's
`Error("the probe runner is not implemented yet (story #663)")`:

- 8a, 8c, 8h, 8l: `assert_eq!`, `left: Error("the probe runner is not implemented yet (story #663)") right: Ok`.
- 8b: `left: Error(...) right: Failed { missing: ["gamma"] }`.
- 8d: `"exited with status 3" missing from the reason "the probe runner is not implemented yet (story #663)"`.
- 8e: `returned before the timeout: 14.08µs` (the lower bound).
- 8f and 8g: `the probe's child never wrote .../hlr-probe-663-<pid>-<n>/pid` (the existence assertion, after `run_probe`
  returned).
- 8i: `"more than 1 MiB" missing from the reason ...`.
- 8j: `"could not be started" missing from the reason ...`. The absence assertions hold on the stub too, by design.
- 8k: `"empty" missing from the reason ...`.

`cargo test -p holler-pane --test ports_test run_probe_stub_never_reports_success` gives `1 passed` (8m, green by design).

**Validity checks:**

- **No compile, setup or harness failure.** Both crates build, and every failure is a feature assertion.
- **Clean.** `cargo clippy -p holler-pane -p holler-cli --all-targets -- -D warnings` exits 0, and `bash scripts/lint.sh`
  exits 0. Both files are under 600 lines (`probe.rs` 305, `profile_scope.rs` 361), and
  `rustfmt --check --edition 2021` passes on both. `grep -c '#\[cfg(test)\]' probe.rs` prints `1`, and the module is the
  file's last item (AC 9's `sed` anchor).
- **Nothing left behind.** No `hlr-probe-663-*` directory and no `sleep 30` remained.
- **The tests are right, not just red.** As a throwaway check, reverted afterwards with no diff left, the scope tests were
  pointed at `FakeProfileScope`:
  - AC 1, AC 6 and AC 7 pass.
  - AC 2, 3 and 4 match the fake on the code and fail only on the message extensions of Decisions 5-7 (for example,
    `"may hold the edit" missing from "timed out: profile_store.cas_put"`).
  - AC 5 fails, because the stub's `reconcile_step` is `""`.

  That is exactly the fake-versus-real difference the brief documents (Decision 4's note). It shows that the fixture, the
  call-log expectations and the act-count logic are sound.

## Notes for F

- **8f/8g reap.** These tests depend on `kill` being on `PATH`. A lone `Child::kill` of the leader leaves `sleep 30`
  alive, and the poll then fails with the pid and its stat, as Decision 15 says.
- **8e asserts the lower bound exactly.** Do not return before the deadline.
- **Size.** `probe.rs`'s test module is about 270 lines, so F's production code there must stay under about 330 lines to keep
  the file under the 600-line warning. The brief estimates about 190.

## Ready for F

Confirmed: the RED is valid. F may implement against these tests.
