# Handoff-T-green: Phase 7 - #647 the reconcile engine and `holler pane doctor`

**Date:** 2026-10-09
**Branch:** issue-647-implementation (worktree `.claude/worktrees/0647-reconcile-doctor`, head `9500955`)
**Issue:** #647
**Handoff-F reviewed:** `docs/handoffs/647/handoff-F.md`
**Handoff-T-red:** `docs/handoffs/647/handoff-T-red.md`

## GREEN confirmation

F's commit `9500955` touches no test file (`git diff --stat 8d1b199..HEAD` lists only production code, ADRs, the
CHANGELOG and handoffs). F listed no tests under "Tests that look wrong (for T)".

```
$ cargo test -p holler-cli --test pane_verbs doctor
test result: ok. 32 passed; 0 failed; 0 ignored; 0 measured; 63 filtered out     (31 from RED + 1 added below)
$ cargo test -p holler-cli --test pane_verbs
test result: ok. 95 passed; 0 failed
$ cargo test -p holler-pane
every target ok; findings_test 1 passed
$ for i in $(seq 20); do cargo test -p holler-cli --test pane_verbs observation_runs_concurrently; done
20/20 passed
```

**Spot-check (do the tests fail when the behaviour is removed?).** I mutated F's code one behaviour at a time, ran the
test that should catch each mutation, and restored the file:

| Mutation | Test that must catch it | Result |
|---|---|---|
| M1 the orchestrator is fixed without being named | `fix_skips_the_orchestrator_unless_named` | FAILED (caught) |
| M2 the record is always written | `unchanged_observation_writes_nothing` | FAILED (caught) |
| M3 strays are judged against the scope, not every record | `another_profiles_session_of_record_is_not_a_stray` | FAILED (caught) |
| M4 the host name is printed unquoted in text | `text_output_escapes_control_characters` | FAILED (caught) |
| M5 pane chains run one after another | `observation_runs_concurrently` | FAILED (caught) |
| M6 `fixed` without seeing the session of record afterwards | the 31 doctor tests | **passed: a gap** |
| M7 unregistered Herdr panes are reported in scoped runs | `doctor_profile_reports_only_its_panes` | FAILED (caught) |

**M6 is a hole in my suite, not a defect in F's code.** Decision 5 says "`shown_session(pane_id)`; equal to S is
`fixed`, anything else `failed`", and no test covered a select that is acknowledged but switches nothing. I added one:

- `doctor.rs::acknowledged_select_that_switches_nothing_is_not_fixed` (engine tier).
  - The setup closes the pane's TUI and sets the test kit's `Quirk::SelectAckedWithoutTui`, the spike's real behaviour.
  - It asserts:
    - a `SelectSession` was made;
    - the mismatch is `failed` with `fix_error.code == "shown-driven-mismatch"`;
    - the remedy is `holler pane relaunch demo-c1r1`;
    - no TUI appeared;
    - `last_observed.shown` stays `None`.
  - It passes on F's code and fails under M6 (`left: Fixed, right: Failed`).

The production files match `HEAD` again (`git diff --quiet -- crates/holler-pane/src crates/holler-cli/src`). After I
restored them, I touched them and rebuilt, so the runs above are on clean, unmutated builds. The restored copies kept
their older mtimes, so without the touch cargo would have reused the mutated builds.

## Tier 1 results

These ran from `ci.yml` on `9500955`, before any mutation, with `HOLLER_STATE_DIR` set to a fresh scratch directory.
F found that `logging_test` sees this machine's live hub otherwise.

| Check | Command | Expected | Actual | Result |
|---|---|---|---|---|
| Lint | `bash scripts/lint.sh` | exit 0 | exit 0 (re-run after the test edit: 0) | PASS |
| Changelog | `bash scripts/changelog-check.sh` | `ok` | exit 0 | PASS |
| Clippy | `cargo clippy --workspace --all-targets -- -D warnings` | clean | clean (re-run after the test edit: clean) | PASS |
| Workspace tests | `cargo test --workspace --no-fail-fast -- --skip roster_stays_accurate_under_concurrent_body_load` (CI's form) | 0 failed | 126 targets, 1414 passed, 0 failed, 5 ignored | PASS |
| Docs CLI | `cargo test -p holler-cli --test docs_cli_test` | pass | exit 0 | PASS |
| Wire canary | `cargo test -p holler-cli --test wire_selftest` | pass | exit 0 | PASS |
| Unused deps | `cargo machete` | none | exit 0 | PASS |
| Hook tests | `bash scripts/test-hooks.sh` | pass | exit 0 | PASS |

Cross-check against F:

- F's isolated workspace run had 1415 passed. The one-test difference is CI's `--skip`, which F did not pass.
- Every other command F reported matches.
- F's note that AC 31's grep is vacuous in this worktree is correct, and I used the relative form below.

There is no server or API to smoke-test. The verb is covered in-process by the `verb` tier tests.

## Tier 2 results

| Check | Method | Result |
|---|---|---|
| Coverage | each AC 1-26 has a named test (table below). Decision 5's post-act check, which no AC covered, now has one | PASS |
| Test quality | each test names one behaviour and sits at the engine tier unless it pins an exit code or output (verb tier). 6 of 7 mutations were caught; the 7th gap is closed. No redundant test found | PASS |
| Type safety | no `unsafe` (AC 32 grep empty); clippy `-D warnings` clean; the one `#[allow]` (`findings_test.rs`) carries `// #647` | PASS |
| Error paths | refusals in both formats (AC 15, 16); store failure exit 1 (AC 17); a failed port call is a finding, not an abort (AC 21, AC 26's `host.ps`); select failure (AC 11); acknowledged but ineffective select (new) | PASS |
| Data integrity | the write is skipped when nothing changed (AC 13); a CAS conflict is one `observe-failed` with no retry (`record_write_conflict_is_reported_not_retried`); the record is unchanged outside three fields (AC 7) | PASS |
| Concurrency | `observation_runs_concurrently` is an invariant (4 in flight), not a duration; 20/20 | PASS |
| API contract | JSON shape pinned (AC 19); exit codes match across formats (AC 16); vocabulary codes (AC 25) | PASS |
| Security | no secret in either stream, with or without `--fix` (AC 22); no ESC byte in text (AC 23, and M4 is caught); remedies carry only constant words and a `PaneName` (evidence entry) | PASS |
| Migration | none (no schema or protocol change, no goldens) | N/A |
| Browser / Playwright | none in this repo | N/A |

Quality gates AC 27-33:

- **AC 27:** the ADR-0003 grep prints `1`; `cli_surface_test`, `docs_cli_test` and `pane_cli_process` pass (in the
  workspace run).
- **AC 28:** `Deferred to #647` count is `0`; `Decided (#647)` is at line 472. The ADR-0021 diff is exactly Decision 1's
  three edits (+8/-3).
- **AC 29:** `changelog-check: ok`.
- **AC 30:** clippy, the workspace tests and lint all pass. The largest touched test file is `surface.rs` at 586 lines;
  `doctor.rs` is now about 500.
- **AC 31:** `rustfmt --check --edition 2021` passes on every touched `.rs` file, `doctor.rs` included after the new
  test. The relative-path `cargo fmt --all --check` grep prints nothing.
- **AC 32:** the `unsafe` grep is empty, the manifest and lock diff is 0 lines, and machete passes.
- **AC 33:** see the advisory notes. Every path outside the pipeline's `docs/handoffs/647*` is in Files, or is a named
  fallback (`reconcile/observe.rs`, `doctor/rig.rs`) or a `doctor.rs` submodule (`read_only.rs`, `surface.rs`). The
  exception is `crates/holler-pane/tests/findings_test.rs`.

Evidence appendix:

- F's 11 entries cover the engine's reliance on unchanged code.
- I appended 3 entries for unchanged test-kit behaviour the tests rely on: `FakeHarness::select_session`, the fake
  Herdr's unsupported `version()`, and `FakeProfileScope::resolve`'s refusals. The excerpts were copied from source.

## Acceptance criteria status

| AC | Status | Backing test |
|---|---|---|
| 1 | PASS | `incident_tui_on_new_empty_session_while_hub_drives_the_old_one` |
| 2 | PASS | `incident_three_day_wedged_server` |
| 3 | PASS | `incident_bare_unregistered_harness_in_the_orchestrators_pane`, `unregistered_herdr_pane_is_reported` |
| 4 | PASS | `incident_registered_pane_whose_process_died`, `tmux_session_gone_is_reported`, `herdr_pane_gone_is_reported` |
| 5 | PASS | `incident_stray_ping_session` |
| 6 | PASS | `incident_deleted_session_under_a_tui` |
| 7 | PASS | `fix_selects_the_session_of_record_and_never_changes_it` (+ `acknowledged_select_that_switches_nothing_is_not_fixed` for Decision 5's other half) |
| 8 | PASS | `second_fix_run_reports_nothing_new` |
| 9 | PASS | `fix_never_guesses_a_session` |
| 10 | PASS | `fix_skips_the_orchestrator_unless_named` |
| 11 | PASS | `fix_failure_is_reported` |
| 12 | PASS | `read_only::doctor_without_fix_makes_only_read_calls` |
| 13 | PASS | `read_only::unchanged_observation_writes_nothing` |
| 14 | PASS | `surface::doctor_profile_reports_only_its_panes`, `surface::another_profiles_session_of_record_is_not_a_stray` |
| 15 | PASS | `surface::doctor_named_pane_scopes` |
| 16 | PASS | `surface::doctor_envelope_and_exit_codes_match_across_formats` |
| 17 | PASS | `surface::doctor_store_failure_is_an_error` |
| 18 | PASS | `surface::finding_prints_rowcol` |
| 19 | PASS | `surface::json_data_shape_is_pinned` |
| 20 | PASS | `surface::herdr_version_shown_and_unsupported_reported` |
| 21 | PASS | `surface::observe_failure_is_a_finding_not_an_abort` |
| 22 | PASS | `surface::doctor_output_holds_no_secret` |
| 23 | PASS | `surface::text_output_escapes_control_characters` |
| 24 | PASS | `surface::observation_runs_concurrently` (20/20) |
| 25 | PASS | `holler-pane/tests/findings_test.rs::finding_kind_codes_are_stable` |
| 26 | PASS | `surface::doctor_remedies_parse` |
| 27-33 | PASS (33 with the advisory below) | gate commands above |

## Blocking issues

None.

## Advisory notes

- **AC 33 / Files:** `crates/holler-pane/tests/findings_test.rs` is still not in the brief's Files list. It is T's file,
  placed there per A's W-6(a) and flagged to O at RED. It is a brief bookkeeping item for O or S, not something F must
  change.
- **The deviation on dedup by the whole finding** (F's handoff) is needed for AC 21, and AC 21's test pins the two
  pane-less `observe-failed` findings, so that deviation is covered by a test.
- **C-8 stands.** The relaunch and reset remedies name verbs that are still stubs until #644 and #645 merge, as the
  brief says. AC 26 parse-tests only the doctor-form remedies.
- **Mutation testing in this repo:** restore a mutated file with `git checkout` or `touch`, not `mv` of a copy. A copy
  keeps the older mtime, and cargo's fingerprint then reuses the mutated build.

T-green complete, no blocking issues. No UI surface — U is N/A, ready for S.
