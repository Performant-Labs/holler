# Handoff-T-red: Phase 4 - #645a `pane switch` and `pane reset` (RED)

**Date:** 2026-10-09
**Branch:** issue-645-implementation (worktree `.claude/worktrees/0645-switch-reset`, head `bb52f26`)
**Brief / wireframe reviewed:** `docs/handoffs/645-brief.md` (as amended in `8cf4f00`); no wireframe (no UI surface)

## A precondition

Confirmed: A returned PASS on the plan (round 2, `docs/handoffs/645/handoff-A.md`, commit `bb52f26`).

## What RED changed (brief "Test plan", RED first)

| File | Change |
|---|---|
| `crates/holler-cli/tests/pane_verbs/doctor.rs:10` | `mod rig;` -> `pub(crate) mod rig;` (Decision 16; nothing else in the file) |
| `crates/holler-cli/tests/pane_verbs/switch.rs` | replaced: the shared runner (`Case`, `both`, `both_with`, `failed`, `data`, `with_s2`), the `WriterInSelect` wrapper, ACs 1-13 and 23 |
| `crates/holler-cli/tests/pane_verbs/reset.rs` | replaced: ACs 14-19, using `crate::switch::{...}` and `crate::doctor::rig::{...}` |
| `crates/holler-pane/src/tx_switch.rs` | the brief's public API, with bodies that answer `not-implemented` (no `todo!()`) |
| `crates/holler-cli/src/pane/switch.rs`, `reset.rs` | the real `Args` structs (`PANE`, `SESSION`, `--as-operator`, `ProfileOpt`). `run` still emits `not_implemented(645)` |
| `crates/holler-cli/tests/pane_verbs/process/stub.rs` | AC 20: the two `645` entries deleted, `// #645` kept |
| `crates/holler-cli/tests/fixtures/cli-surface.txt` | AC 21: the `# #645` block, exactly as the brief gives it |
| `docs/adr/ADR-0003.md:51-52` | AC 22: `holler pane switch PANE SESSION [--as-operator] [--profile NAME]` and `holler pane reset PANE [--as-operator] [--profile NAME]`, with `#645` in column 67 like the other rows |

**F's work:**
- Fill `tx_switch.rs`.
- Replace both `run` bodies, and add `Verb` and `emit_outcome` to `switch.rs`. They were left out, not stubbed, because `dead_code` is denied and nothing would call them yet.
- The ADR-0021 edits (Decision 17) and the CHANGELOG (ACs 24 and 26).

## Tests authored

Every test runs in the `pane_verbs` target (`crates/holler-cli/tests/pane_verbs/main.rs`, which already declares
`mod switch; mod reset;`, so no `[[test]]` entry is needed). They are in-process, on the doctor rig's fakes.

**The tier.** This is the cheapest tier that pins the ACs, because every AC is stated at the verb: exit code, envelope,
text line, and calls made. A unit test of the engine alone would duplicate these and could not see the CLI's usage path,
its output or the format parity.

**The shared runner.** `both` / `both_with` (switch.rs) builds a fresh rig per format, captures every seeded pane's record
and TUI, marks the call logs, runs the verb, and asserts these for **every** run of every test:
- the same exit code in text and JSON;
- `check_envelope(out, code)` is `Ok`, and `err` is empty in JSON;
- text writes one line, to `out` on success or `error: ...` to `err` on failure, and the other stream is empty;
- `calls.herdr` and `calls.host` are empty (I4).

| Test | AC | What it pins |
|---|---|---|
| `switch::switch_moves_the_tui_and_the_record_together` | 1, 2 | exit 0, and the TUI shows S2. The record is the previous one with exactly four fields set and the generation +1. `last_observed.driven` is pre-set to `ses_driven` so that "kept" actually bites, and `at` falls inside the run's clock window. Harness calls `[Health, ListSessions, SelectSession, ShownSession]`, store calls `[Get, List, CasPut]`. The exact text line. JSON `verb`, `previous`, `pane.session_of_record`, and `data.pane` equal to the stored record |
| `switch::switch_to_a_deleted_session_changes_nothing` | 3 | 3 `session-not-found`, no `SelectSession`, records and TUIs unchanged, the TUI still on S1 |
| `switch::switch_to_another_panes_session_is_refused` | 4 | 3 `session-of-other-pane`, the message names `demo-c2r1`, no `SelectSession`, both panes unchanged (the rig's shared data directory makes P's server list Q's session) |
| `switch::switch_refuses_an_unhealthy_server` | 5 | killed, and frozen: 3 `server-unhealthy`, the message contains `run holler pane relaunch demo-c1r1`, harness calls exactly `[Health]`, unchanged. A `Health` timeout fault is 1 `timeout` |
| `switch::switch_refuses_the_orchestrators_pane_unless_as_operator` | 6 | 3 `orchestrator-pane` naming `--as-operator`, harness calls empty, unchanged. With `--as-operator`, exit 0 recorded as AC 1 |
| `switch::switch_mismatch_after_select_records_nothing` | 7 | `close_tui` with `SelectAckedWithoutTui` on: 1 `unavailable`, the message contains `its home screen` and ends with `; to reconcile, run holler pane doctor demo-c1r1 --fix`, unchanged |
| `switch::switch_select_failure_names_the_reconcile_step` | 8 | a `SelectSession` timeout fault: 1 `timeout`, ends with the reconcile step, unchanged |
| `switch::switch_record_conflict_after_the_act` | 9 | `WriterInSelect` (a `HarnessPort` that calls `concurrent_put` with P's record at `hold: Drained`, then delegates): 1 `generation-conflict`, ends with the reconcile step. The stored record is the other writer's (S1, `Drained`, generation +1), and the TUI shows S2 |
| `switch::switch_in_a_profile` | 10 | `--profile demo` on P: exit 0, recorded. Q outside it: 3 `pane-not-in-profile`, harness calls empty, unchanged. `--profile nope`: 3 `profile-not-found`, unchanged |
| `switch::switch_unknown_pane` | 11 | `demo-c9r9`: 3 `pane-not-found`, harness calls empty |
| `switch::switch_usage` | 12 | `BAD_NAME`, and SESSION `"ses x"`, `""`, `"ses_\u{1b}[31m"` and 65 `a`s: 2 `usage`, with no pane store, profile store, harness or probe call. No raw ESC in either message. (Checked: clap accepts each of these argvs, so they reach the verb.) |
| `switch::switch_to_the_current_session_is_idempotent` | 13 | `switch P S1`: exit 0, recorded as AC 1 with generation +1, `previous == S1`, and the exact text line |
| `switch::help_names_the_arguments` | 23 | `switch --help` has `PANE`, `SESSION` and `--as-operator`; `reset --help` has `PANE` and `--as-operator`, and no `--first` |
| `reset::reset_creates_a_fresh_session_and_switches_to_it` | 14, 2 | exit 0. The new id is not S1, is listed by P's server, and is recorded as AC 1. Harness calls `[Health, CreateSession, SelectSession, ShownSession]`, store calls `[Get, CasPut]`. The exact text line. JSON `verb == "reset"`, `previous == S1`, and `data.pane` equal to the stored record |
| `reset::reset_is_doctors_remedy_for_no_session_of_record` | 15 (a) | `session_of_record = None`: exit 0, `previous` null, text ends `(was none)`. A later doctor pass has no `no-session-of-record`, `session-of-record-missing` or `shown-driven-mismatch` for P |
| `reset::reset_is_doctors_remedy_for_a_deleted_session_of_record` | 15 (b) | after `delete_session(S1)`, doctor's own remedy string is `holler pane reset demo-c1r1`. It parses with `try_parse`, it **is** the argv that is run, it exits 0, and the doctor pass after it is clean of the three kinds |
| `reset::reset_leaves_the_old_session_as_a_stray` | 16 | after a reset, a doctor pass reports exactly `["stray-session"]`, for S1 |
| `reset::reset_refusals_create_nothing` | 17 | four cases: orchestrator (3 `orchestrator-pane`), killed server (3 `server-unhealthy`), Q outside `--profile demo` (3 `pane-not-in-profile`), and `demo-c9r9` (3 `pane-not-found`). For each: no `CreateSession`, the session list unchanged, and everything unchanged. The list is read through Q's live server, which shares the data directory, so the killed-server case is also checked |
| `reset::reset_failure_after_create_names_the_unrecorded_session` | 18 | an `Unavailable` fault on `SelectSession`: 1 `unavailable`. The message contains `session "<new>" was created and is not recorded` and ends with the reconcile step. Nothing is changed. The new id is on the server, and doctor lists it as a stray |
| `reset::reset_mismatch_records_nothing` | 19 | `close_tui` with the quirk: 1 `unavailable`, the message has `"<new>"`, `its home screen` and the reconcile step, and nothing is changed |

ACs 20-22 are pinned by the existing suites: `pane_cli_process` (`stub.rs`, `docs_rows.rs`, `flags.rs`), `cli_surface_test`
and `docs_cli_test`. ACs 24-26 are F's, and T checks them at GREEN.

## RED confirmation

`cargo test -p holler-cli --test pane_verbs` returns `test result: FAILED. 94 passed; 19 failed`. The 94 that pass are
every pre-existing test, all of doctor's included, after the `pub(crate)` edit, plus `help_names_the_arguments`.

Each of the 19 fails **after** its setup and after `both`'s format-parity, envelope and I4 checks have passed. It fails
on the verb's answer, which is still `{"code":"not-implemented","message":"not implemented (story #645)"}` with exit 1:

| Assertion (switch.rs) | Tests | Failing output |
|---|---|---|
| `failed`, line 207: the exit code | the exit-2 and exit-3 cases: ACs 3, 4, 5, 6, 10's refusals, 11, 12 and 17 | `left: 1, right: 3` (or `right: 2` for `switch_usage`), e.g. `exit 3 (session-not-found): Outcome { code: 1, out: "{...\"not-implemented\"...}" }` |
| `failed`, line 212: the error code | the exit-1 cases: ACs 7, 8, 9, 18 and 19 | `left: "not-implemented", right: "unavailable"` (or `"timeout"`, or `"generation-conflict"`) |
| `data`, line 221: success | ACs 1, 13, 14, 15 (a), 15 (b), 16, and the first run of 6 and 10 | `the run succeeds: Outcome { code: 1, ... }`, `left: 1, right: 0` |

Not one failure is a compile error, a missing `[[test]]`, a setup panic or a timeout.

**Why `help_names_the_arguments` passes in RED.** The brief's test plan puts the real `Args` structs in RED, because the
new positionals are what make the fixture lines and the ADR rows parse. So the help test pins a surface that RED itself
lands. On `origin/main` (`e327569`) the stub `PaneSwitch` has no positional at all (`value_name` count 0), so this test
fails there. The surface suites stay green for the same reason: `pane_cli_process` 34/34, `cli_surface_test` 3/3,
`docs_cli_test` 3/3.

**Hygiene, already on the RED tree:**
- `cargo clippy --workspace --all-targets -- -D warnings` is clean.
- `rustfmt --check --edition 2021` passes on the five new or replaced files and on `doctor.rs`.
- `bash scripts/lint.sh` exits 0. `switch.rs` is 599 lines and `reset.rs` 267, both under 900.
- No `Cargo.toml` changed.

## Notes for F

- The tests pin the brief's exact strings:
  - `switched demo-c1r1 to session "<id>" (was "<id>")` and `reset demo-c1r1 to a new session "<id>" (was ...)`, with
    `(was none)` when there was none;
  - `; session "<id>" was created and is not recorded`;
  - `; to reconcile, run holler pane doctor demo-c1r1 --fix` at the very end of the message;
  - `its home screen`;
  - `run holler pane relaunch demo-c1r1`;
  - `--as-operator` in the orchestrator refusal;
  - `demo-c2r1` in the session-of-other-pane refusal.
- `data.pane` must serialize equal to the stored record (`serde_json::to_value(&stored)`).
- The call sequences in AC 2 are exact. P1 without `--profile` is one `get`, and reset must not call `list` or
  `list_sessions`.
- On a refusal, a killed or frozen server must stop at `[Health]`. The orchestrator, profile and unknown-pane refusals must
  reach no harness method.

## Ready for F

Confirmed: RED is valid. F may implement against these tests.
