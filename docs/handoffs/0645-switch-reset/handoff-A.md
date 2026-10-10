# Handoff-A: Phase 3 - #645 part 2, `--first` on reset and the activity refusals  (up-front plan review)

**Date:** 2026-10-10
**Branch:** `issue-0645-switch-reset` (at `3d95aec`, main tip)
**Brief reviewed:** `docs/handoffs/0645-switch-reset/brief.md`   **Reuse map:** `docs/handoffs/0645-switch-reset/survey.md` (Reuse & Analogous-Feature map)   **Wireframe:** N/A (no UI surface)
**Verdict:** PASS

## Summary

PASS. The plan extends exactly the objects the Reuse map names — the shared `execute` path
(`crates/holler-cli/src/pane/switch.rs`), `SwitchRequest`/`plan()` in `crates/holler-pane/src/tx_switch.rs`,
reset's own clap args (`crates/holler-cli/src/pane/reset.rs`), and the two default-armed `HarnessPort`
seam methods the operator's carve-out allows — and I verified against the tree that it breaks no
in-tree impl and needs no file outside the brief's F-boundary. The ordering (prompt after the record
write) is the issue's own order and the only one consistent with invariant I2; the activity check slots
into `plan()` after `check_health` gated to `Target::Fresh` with a session of record, leaving switch
untouched. One `warn`: the brief's F-boundary omits ADR-0021's switch/reset rows, which part 1's own
commit (`bff4dbe`) wrote and which this delta makes stale — a boundary gap against the run's own
precedent, trivially fixed by O. No block findings.

## Verified against the tree (the six review questions)

1. **The carve-out's extent — confirmed.** Every in-tree `impl HarnessPort` compiles untouched with two
   default-armed methods; there are exactly nine: `OpenCodeHarness`
   (`crates/holler-adapter-opencode/src/lib.rs:239`), `FakeHarness`
   (`crates/holler-pane-testkit/src/harness.rs:281`), `Unwired`
   (`crates/holler-cli/src/pane/wiring.rs:169`), `Mutant`
   (`crates/holler-pane-testkit/tests/harness_conformance_test.rs:145`), `Recording`
   (`crates/holler-adapter-opencode/tests/real_opencode/rig.rs:191`), `TestHarness`
   (`crates/holler-pane/tests/ports_test.rs:216`), `HookedHarness`
   (`crates/holler-cli/tests/pane_verbs/launch/rig.rs:645`), `WriterInSelect`
   (`crates/holler-cli/tests/pane_verbs/switch.rs:478`), `HealthGate`
   (`crates/holler-cli/tests/pane_verbs/doctor/surface.rs:471`). Nothing enumerates the trait's method
   list: the testkit's `HarnessOp` (`holler-pane-testkit/src/harness.rs:33-42`) is hand-maintained with
   no compiler tie to the trait, and the conformance suite is a fixed case list. Both signatures
   (`send_prompt(port, session, text) -> Result<(), PaneError>`, `session_activity(port, session) ->
   Result<Activity, PaneError>`) are object-safe (`&self`, concrete returns), so
   `assert_send_sync::<dyn HarnessPort>` (`ports_test.rs:270`) and the `Ports` bundle
   (`ports.rs:230-238`) are unaffected. The first-message pass-through reaches the engine without
   leaving the boundary: `PaneReset` grows `--first` (reset.rs, ruling 2), reset's `run()` passes it to
   the shared `execute` (switch.rs:72-87, the map's named extension point), `SwitchRequest` grows
   `first: Option<String>` (tx_switch.rs:92-99, derives survive), switch's `run()` passes `None`
   (switch.rs:48-58). No file outside the F-list is needed for the code.
2. **Ordering — sound; queuing before record would be wrong.** The issue's own words ("create a fresh
   session … switch the TUI to it, record it, **then** queue the first message") fix the order, and
   independently: I2 (`ADR-0021.md:202`) makes `session_of_record` the only session a pane's TUI shows
   and the hub drives, so queueing before `cas_put` would drive a not-yet-of-record session; and a
   `cas_put` failure *after* a queued prompt would add a fourth failure shape (a prompt sitting in an
   unrecorded session that the message must disclose) on top of the existing
   `acted`/`created` machinery (tx_switch.rs:110-151). Prompt-after-record leaves every existing
   failure mode untouched and adds exactly one terminal state. The failure semantics (record succeeded,
   prompt failed → honest failure, no reconcile step — brief.md:22-25) are right and achievable in
   boundary, with the constraint in finding N-2 below.
3. **The activity refusals — right shape, right slot.** `plan()` (tx_switch.rs:191-204) slots the check
   after `check_health` (a live server is the thing queried; a dead one must surface as
   `server-unhealthy`, not an activity error) and after `refuse_orchestrator` (so the orchestrator-pane
   refusal keeps its zero-port-call property, `switch.rs` AC 6 test at `tests/pane_verbs/switch.rs:385`),
   gated to `Target::Fresh` **and** `session_of_record.is_some()`. That gating is what makes "a pane
   with NO session of record must not be refused" true by construction (doctor's remedy,
   `tests/pane_verbs/reset.rs:110-123`) and leaves switch (`Target::Existing`) path-untouched. Existing
   op-sequence assertions stay green: `FakeHarness` does not override the seam, so the engine's
   `session_activity` call runs the trait default and never enters the fake's call log —
   `reset.rs:71-77`'s `[Health, CreateSession, SelectSession, ShownSession]` still holds. The
   **default=idle permissive seam is the right call**: distinguishing `unknown` from `idle` would make
   every port that has not wired the seam — including the real adapter until #642 part 3 and the CLI's
   `Unwired` — refuse, changing existing behavior and breaking doctor's remedy, exactly what the
   carve-out forbids. The permissive cost (a busy pane resets unrefused on an unwired port) is
   documented and expires when #642 part 3 lands.
4. **Rulings compliance — confirmed.** Ruling 2: `--first` lives in reset.rs's own `Args`
   (reset.rs:21-31); the shared flag groups (`pane/args.rs` `ProfileOpt`) untouched. Ruling 3: codes as
   `RefusalCode::from_static` consts in the engine/port's own files; the `PaneError` enum is closed
   (`error.rs:39-68`, `CODE_COUNT` 22) and the closed set expresses everything — the two activity
   refusals and the port default's "prompt-unsupported" travel as `PaneError::Refused`, and a prompt
   failure propagates the port's own error, whose class `class_of` (`error.rs:266-303`) already decides
   per ADR-0021 §9 (open code → 3; `timeout`/`unavailable` → 1). ADR-0003 row
   (`docs/adr/ADR-0003.md:52`) and `cli-surface.txt` lines (`tests/fixtures/cli-surface.txt:128-133`)
   get this verb's rows only, part 1's precedent. Ruling 4: rustfmt on new content only.
5. **RED policy — honestly RED, with one granularity note.** None of the four case families can pass on
   main: `--first` is rejected by clap today (exit 2, and `tests/pane_verbs/switch.rs:648` pins its
   absence from `--help`), the engine never calls `session_activity` so a faked-busy pane resets
   successfully, and there is no prompt step to fault. See N-4: the RED will be compile-level.
6. **Duplication checkpoint (for a-dup).** The `WriterInSelect` precedent (`switch.rs:472-504`) is a
   ~30-line decorator: delegate the eight existing methods to `inner`, override the one method under
   test, side-effect only. The new wrappers must stay that shape — record-only overrides of
   `send_prompt`/`session_activity` over `FakeHarness`, injected through the rig's existing
   `ports_with(&dyn HarnessPort)` (`doctor/rig.rs:160`), no session/TUI state of their own. A wrapper
   that starts modelling behaviour (its own sessions map, its own TUI) is a second `FakeHarness` and
   blocks at a-dup. Note the wrappers' seam calls are invisible to the rig's call log (the default
   bypasses `FakeHarness`'s logging), so the wrapper's own recording is the assertion surface for
   prompts — by design, not a gap.

## Findings

| # | Severity | Plan element | Drift dimension | Finding | Suggested fix |
|---|---|---|---|---|---|
| W-1 | warn | F-boundary, brief.md:50-53 (docs) | pattern consistency | The brief's F-boundary names ADR-0003 + cli-surface.txt but omits ADR-0021's switch/reset rows, which this delta makes stale: the codes-table row still says "**PROPOSED** (645b): a pane that is not idle or holds a question" (`docs/adr/ADR-0021.md:453`); the deferred item still says "pending the operator … after #649 points DRIVEN at `session_of_record`" (`ADR-0021.md:669-671`); and §8's as-built paragraph now misstates the verbs ("a run makes at most seven port calls", "they send no prompt", "a refusal derived from pane state belongs at `send_prompt`", `ADR-0021.md:376-378`). Part 1's own commit `bff4dbe` wrote that section and those markers, so the established pattern is that this story maintains them. As written, the PR would complete #645 while the ADR of record still describes the verbs as not sending prompts. | O adds ADR-0021's three switch/reset spots (the :453 row, the :669-671 item, the §8 paragraph's stale sentences) to F's doc boundary, mirroring part 1's precedent — or O amends the ADR itself at merge. Either way the as-built text must record the reset-only scoping of the activity refusals and the send_prompt seam. |
| N-1 | note | "two default-armed methods ONLY" (brief.md:50, decisions.md:35-36) | abstraction level | The carve-out's extent necessarily includes the seam methods' own signature types: the small closed `Activity` answer enum and the default's `RefusalCode` const must live in `ports.rs` too. Strictly read, "two methods ONLY" excludes them and F could misplace them (a new file, or `error.rs`, which is Never). | Read the carve-out as "the two methods and their minimal signature types, all in `ports.rs`". `Activity`: closed, minimal derives, no Serde (it never travels the wire). |
| N-2 | note | AC 3, brief.md:22-25 | pattern consistency | The prompt-failure-after-record cannot reuse `SwitchFailure`'s existing decorations: `created: Some` would append "session X was created and is not recorded" — false after a successful `cas_put` — and `acted: true` would append the reconcile step, which AC 3 forbids (tx_switch.rs:137-150). | `SwitchFailure` (in-boundary, tx_switch.rs) grows a third decoration (e.g. a prompt-failed marker) so `message()` states "created, shown and recorded, but the first message did not land" and appends neither the not-recorded clause nor the reconcile step. The reported code stays the port's error; exit per `class_of`. |
| N-3 | note | AC 5 default, brief.md:30-31; survey.md:36-37 | pattern consistency | The `send_prompt` default's stable open refusal ("prompt-unsupported") departs the repo's convention for unlanded methods — `not-implemented` (ADR-0021.md:445; `Unwired` answers it for every method, wiring.rs:169-200). The departure is deliberate and better here: the reset itself succeeded, so exit 3 "understood and declined" is honest, and the stable code is what #642 part 3 replaces. | Record the choice and the code name in the ADR amendment (W-1's edit) so #642 part 3 knows exactly what it is replacing and why it is not `not-implemented`. |
| N-4 | note | RED policy, brief.md:41-46 | (for T) | The wrapper doubles must override the not-yet-existing trait methods, so the RED run fails at compile and takes the whole `pane_verbs` binary with it — every existing green test reads RED too. Honest (the API genuinely does not exist) but coarse. Also `tests/pane_verbs/switch.rs:648` (`assert!(!reset.contains("--first"))`, part 1's AC 23 pin) must be flipped by T — it is in T's boundary. | T journals the compile-level RED with evidence as the brief requires, flips the :648 pin and extends the help assertions to name `--first`, and keeps the new cases in the two boundary test files. |
| N-5 | note | AC 4, brief.md:26-29 | (for T) | The "no session of record is not refused" case is only proven if it passes while the port *would* report busy — otherwise it passes vacuously through the idle default even if the engine wrongly queried activity. | T's no-session-of-record case uses a wrapper whose `session_activity` answers busy for **any** query, and asserts the run still succeeds with no `session_activity` call recorded — proving the skip is by `None`, not by the default. |

No block findings. The plan's deliberate departures from ADR-0021's PROPOSED wording — the prompt goes
verb→`HarnessPort::send_prompt` rather than "the hub's one prompt path", activity is read on the
session of record rather than the DRIVEN session, and the work does not wait for #649
(`ADR-0021.md:669-671`) — are the operator's carve-out decision (decisions.md:23-34), reviewed and
recorded, which makes them the desired path, not drift. They must be written into the ADR amendment
(W-1) so #646 (`say`/`interrupt`/`answer`) and #649 do not grow a divergent second prompt path or a
second activity vantage unaware of this seam.

## Notes for O

- Act on W-1 before F runs: one line in the brief's F-boundary (or an O-owned ADR edit at merge). N-1
  is a reading the brief should state once so F does not misplace the `Activity` enum.
- The engine's "at most seven port calls" doc line (tx_switch.rs:40) becomes at most eight with
  `--first`; F updates it with the module docs (in boundary) — listed here so it is not missed.

## Patterns referenced

- `crates/holler-pane/src/tx_switch.rs` — the engine under extension (plan/act/observe/record, `SwitchFailure`
  decorations, the :40-42 deferral note this run discharges).
- `crates/holler-cli/src/pane/switch.rs` + `reset.rs` — the shared execute path and the verb-owns-its-flags split.
- `crates/holler-cli/tests/pane_verbs/switch.rs:472-541` — the `WriterInSelect` wrapper-double pattern (`both_with`
  + `rig.ports_with`), the a-dup baseline for the new wrappers.
- `crates/holler-pane/src/error.rs` — the closed code set, `RefusalCode::from_static`, `class_of` (ADR-0021 §9).
- `docs/adr/ADR-0021.md` §8 "Switch and reset as built", §9 exit table (:453, :669-671, :376-378) — the as-built
  record this delta must amend (W-1).
