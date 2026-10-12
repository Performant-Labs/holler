# Handoff-T-red: Phase 4 - #647 the reconcile engine and `holler pane doctor`

**Date:** 2026-10-09
**Branch:** issue-647-implementation (worktree `.claude/worktrees/0647-reconcile-doctor`, head `540491b`)
**Brief / wireframe reviewed:** `docs/handoffs/647-brief.md` (as amended in `a98258a`); wireframe N/A (no UI surface)

## A precondition

Confirmed: A returned PASS on the plan (`docs/handoffs/647/handoff-A.md`, 0 blocks, 7 warns).

## Scaffold landed for RED (brief "Test plan")

The brief's test plan says T lands the types first, so that RED is an assertion failure and not a compile error. These
are declarations only, with no behaviour:

- `crates/holler-pane/src/findings.rs`: `FindingKind` (12 kinds, `ALL`, `code()`), `FixState`, `FixError` and
  `Finding`, exactly as in Decision 10. They use the serde derive the brief specifies. A's W-5(a) suggests a
  hand-written `Serialize` from `code()`; that is F's call, and AC 25 holds either way.
- `crates/holler-pane/src/reconcile.rs`: `ReconcileRequest`, `Report`, `ScopeSummary`, `HerdrSummary`, `HostSummary`,
  `PaneSummary` and `ObservedHealth`, as in Decision 10. `reconcile()` returns `Err(PaneError::NotImplemented)`.
- `crates/holler-cli/src/pane/doctor.rs`: the `PaneDoctor` args of Decision 11 (`[PANE]`, `--fix`, `ProfileOpt`).
  `run` still returns `emit_error(.., not_implemented(STORY))`.

F replaces all three bodies. No test pins the spelling of `ObservedHealth`'s values or the text summary line, so F is
free to act on A's W-5(b).

## Tests authored

Unless a row says otherwise, every test is in the `pane_verbs` target, under `crates/holler-cli/tests/pane_verbs/`.
The tiers:

- **engine:** calls `holler_pane::reconcile::reconcile` directly over the six fakes, with `now_ms = NOW`;
- **verb:** runs `holler pane doctor` in-process through `verb_harness::run_verb_with`;
- **unit:** a pure-vocabulary test.

The rig is `doctor/rig.rs`, built as Decision 12 describes:

- one Herdr pane, tmux session, server (port 48100+i), session and TUI per seeded pane;
- a `sample_pane` record naming them;
- `FakeProfileScope` over `Arc`s of the two stores.

A fresh rig is a healthy fleet. Setup goes through the ports, so tests that assert on calls read
`Rig::calls_since(mark)`.

| Test | Pins | Tier, and why |
|---|---|---|
| `doctor.rs::healthy_fleet_reports_nothing_and_records_the_observation` | baseline, no false positives: a healthy fleet gives no findings; the first pass writes `health: Healthy`, `shown` and `at = now_ms` (Decision 6) | engine: needs the clock seam |
| `doctor.rs::incident_tui_on_new_empty_session_while_hub_drives_the_old_one` | AC 1 | engine: needs the clock seam and the stored record |
| `doctor.rs::incident_three_day_wedged_server` | AC 2: exactly `[server-wedged]`, relaunch, health `Unhealthy("server-wedged")` | engine |
| `doctor.rs::incident_bare_unregistered_harness_in_the_orchestrators_pane` | AC 3: exactly `[tui-foreign-session]`, relaunch, `not-fixable` | engine |
| `doctor.rs::unregistered_herdr_pane_is_reported` | AC 3: `grid`, `herdr_pane`, pane and remedy null; none in a named-pane run | engine |
| `doctor.rs::incident_registered_pane_whose_process_died` | AC 4: `[server-down]`, relaunch, health `Unhealthy("server-down")` | engine |
| `doctor.rs::tmux_session_gone_is_reported`, `herdr_pane_gone_is_reported` | AC 4 | engine |
| `doctor.rs::incident_stray_ping_session` | AC 5: one stray, pane and remedy null; two servers on a shared directory give one finding with `ports == [48100, 48101]` | engine |
| `doctor.rs::incident_deleted_session_under_a_tui` | AC 6: `session-of-record-missing` (reset) plus a `not-fixable` mismatch (relaunch), and nothing else | engine |
| `doctor.rs::fix_selects_the_session_of_record_and_never_changes_it` | AC 7: `fixed`, remedy null, TUI on S1, record unchanged apart from three fields, one `SelectSession`, no harness writer, no Herdr send, host `Ps` only | engine: needs the call logs |
| `doctor.rs::second_fix_run_reports_nothing_new` | AC 8: subset of the first run, no mismatch, no `CasPut` | engine |
| `doctor.rs::fix_never_guesses_a_session` | AC 9: both halves (record `None`; S1 deleted) | engine |
| `doctor.rs::fix_skips_the_orchestrator_unless_named` | AC 10 | engine |
| `doctor.rs::fix_failure_is_reported` | AC 11: `failed`, `fix_error.code` `unavailable`, relaunch, exit 0 | verb: the AC pins the exit code |
| `doctor/read_only.rs::doctor_without_fix_makes_only_read_calls` | AC 12, on one fleet with all nine incident kinds; also asserts each kind is reported, so the read-only checks are not vacuous | engine |
| `doctor/read_only.rs::unchanged_observation_writes_nothing` | AC 13 | engine |
| `doctor/read_only.rs::record_write_conflict_is_reported_not_retried` | Decision 6 and R-6: a `generation-conflict` on the write is one `observe-failed` with remedy `holler pane doctor demo-c1r1`, and exactly one `CasPut` | engine. **Not a numbered AC**; it covers Tier 2's concurrent-write edge case |
| `doctor/surface.rs::doctor_profile_reports_only_its_panes` | AC 14 (a) and (b): only P's panes; no unregistered pane; Q's own-directory strays not reported | engine |
| `doctor/surface.rs::another_profiles_session_of_record_is_not_a_stray` | AC 14 (c): shared directory. A precondition asserts that P's server lists Q's session | engine |
| `doctor/surface.rs::doctor_named_pane_scopes` | AC 15: scoping, plus the four refusals in both formats (`pane-not-found` 3, `pane-not-in-profile` 3, `profile-not-found` 3, `usage` 2) | engine for scoping; verb for refusals |
| `doctor/surface.rs::doctor_envelope_and_exit_codes_match_across_formats` | AC 16: the rigs of ACs 1, 2, 4 and 14 (the refusals go through the same `both_formats` helper in AC 15) | verb |
| `doctor/surface.rs::doctor_store_failure_is_an_error` | AC 17 | verb |
| `doctor/surface.rs::finding_prints_rowcol` | AC 18 | verb (text and JSON) |
| `doctor/surface.rs::json_data_shape_is_pinned` | AC 19 | verb (JSON) |
| `doctor/surface.rs::herdr_version_shown_and_unsupported_reported` | AC 20: hosts distinct and sorted by name | verb (JSON) |
| `doctor/surface.rs::observe_failure_is_a_finding_not_an_abort` | AC 21. Not vacuous: a vanished Herdr pane and an unregistered one are planted, so a guess would show | verb (JSON) |
| `doctor/surface.rs::doctor_output_holds_no_secret` | AC 22: four world states, each run with and without `--fix`, in both formats and on both streams. Asserts the command sentinel and the env name are both absent | verb |
| `doctor/surface.rs::text_output_escapes_control_characters` | AC 23: no ESC byte; the host stays on one line; JSON keeps the raw value | verb |
| `doctor/surface.rs::observation_runs_concurrently` | AC 24: a `HealthGate` harness wrapper (condvar, at most 2 s) must record 4 calls in flight at once | engine. An invariant, not a duration |
| `doctor/surface.rs::doctor_remedies_parse` | AC 26: the set of doctor-form remedies is exactly `holler pane doctor`, `... demo-c1r1`, `... demo-c1r1 --fix` and `... demo-c2r1 --fix`, and each parses through `verb_harness::parse::try_parse` (A's W-6(c)). Also checks that a `host.ps` failure gives an `observe-failed` that names `host.ps` | engine |
| `crates/holler-pane/tests/findings_test.rs::finding_kind_codes_are_stable` | AC 25 | unit. Moved here per A's W-6(a): it needs no fake, and it mirrors `error_test.rs` |

Fixture and stub edits (AC 27):

- `tests/fixtures/cli-surface.txt`: the `# #647` group is now Decision 11's five lines.
- `tests/pane_verbs/process/stub.rs`: `("pane", "doctor", 647)` is deleted. Its `// #647` line stays.

The old stub case in `doctor.rs` is replaced.

## RED confirmation

```
$ cargo test -p holler-cli --test pane_verbs doctor
test result: FAILED. 0 passed; 31 failed; 0 ignored; 0 measured; 63 filtered out
```

Each failure is on the missing behaviour. None is a compile, setup or harness failure: every rig built, and every
world mutation succeeded before the assertion that failed.

- **21 engine tests** fail at `doctor/rig.rs:278`. The message is
  `reconcile must complete the pass and report: not implemented`: `reconcile()` returns `Err` where a `Report` is
  expected.
- **`observation_runs_concurrently`** fails at `doctor/surface.rs:534` with
  `four health checks in flight at once; left: 0, right: 4`. It reads the invariant before it unwraps the result.
- **7 verb tests** fail on their first exit-code or envelope assertion:
  - `fix_failure_is_reported`, `herdr_version_shown_and_unsupported_reported`, `json_data_shape_is_pinned` and
    `observe_failure_is_a_finding_not_an_abort`: `the pass completes: exit 0`, left `1` right `0`, envelope
    `not-implemented`.
  - `finding_prints_rowcol` and `text_output_escapes_control_characters`: left `1` right `0`, with
    `err: "error: not implemented (story #647)"`.
  - `doctor_envelope_and_exit_codes_match_across_formats`: left `1` right `0`.
- **`doctor_store_failure_is_an_error`** fails with left `"not-implemented"`, right `"unavailable"`.
- **`doctor_output_holds_no_secret`** fails its non-vacuity guard, `the runs reported the pane`.

These pass already, as the brief's test plan expects. They pin vocabulary and parsing:

- `cargo test -p holler-pane --test findings_test`: 1 passed;
- `cargo test -p holler-cli --test cli_surface_test --test docs_cli_test --test pane_cli_process`: 3, 3 and 34 passed;
- the rest of `pane_verbs`: 63 passed.

Other checks at RED:

- `cargo clippy -p holler-cli -p holler-pane --all-targets -- -D warnings` is clean.
- `bash scripts/lint.sh` exits 0, with no warning for a touched file. The largest new file is `surface.rs`, at 586 lines.
- `rustfmt --check --edition 2021` passes on the touched `.rs` files.
- `gitleaks detect --no-git` finds no leaks in the new test directory.

## Notes for F and O

- **Rig choice (A's W-6(b)).** No #643 or #644 rig is on `main`, so `doctor/rig.rs` is the first. Per A, the
  Forward-compat should say that #645 and #646 reuse it.
- **AC 33.** The diff adds `crates/holler-pane/tests/findings_test.rs`, `doctor/rig.rs`, `doctor/surface.rs` and
  `doctor/read_only.rs`. The brief's Files list names `doctor.rs` "and its submodules" but not the `holler-pane` test
  file. O should add it to Files and AC 33, which is what A's W-6(a) asks.
- **Strings the tests expect, from Decision 3:**
  - `holler pane relaunch <pane>`;
  - `holler pane reset <pane>`;
  - `holler pane doctor <pane> --fix`;
  - `holler pane doctor <pane>`, or `holler pane doctor` when no pane applies.

  An `observe-failed` message names the op as `<port>.<method>` (`herdr.snapshot`, `herdr.version`, `host.ps`).
- **What the tests do not pin.** They leave F free on these:
  - which session a finding carries, apart from AC 1 (the mismatch carries S2, the shown one) and AC 3 (the foreign
    session);
  - the order of findings;
  - `ObservedHealth`'s value spellings;
  - the text-mode summary line.

## Ready for F

Confirmed: the RED is valid. F may implement against these tests.
