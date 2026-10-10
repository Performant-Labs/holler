# Handoff-F: Phase 5 - Implement (#645 part 2, `--first` on reset and the activity refusals)

**Date:** 2026-10-10
**Branch / worktree:** `issue-0645-switch-reset` in `<run-worktree>` (on top of T's RED commit `560a6c0`)
**Contract:** `handoff-T-red.md` (9 items: 7 new cases, the flipped `help_names_the_arguments` pin, runner inheritance)
**Attempt:** 2. This attempt re-runs F over the tree attempt 1 left.

## About attempt 2

Attempt 1 implemented the whole contract. The stage refused it for one reason only: it tried
to write `crates/holler-cli/tests/fixtures/cli-surface.txt`, which is under `tests/**`, and F
is denied that directory. Since then O has moved that fixture row to T's boundary in
`brief.md` (journaled in `decisions.md`, about 11:05 AM).

In this attempt I did the following:

- I made **no production edits**. The working tree already held attempt 1's
  implementation, and I found nothing to change.
- I **wrote nothing under `tests/**`**.
- I re-reviewed the whole production diff against each test assertion.
- I re-ran the suites listed below.
- I rewrote this handoff.

The implementation, recorded once more as the contract's record:

## What I changed

All edits are inside the F-boundary in `brief.md`. Nothing is committed; O commits.

- **`crates/holler-pane/src/ports.rs`**: the carve-out and nothing else (N-1).
  - Two default-armed `HarnessPort` methods:
    - `send_prompt(&self, port: u16, session: &str, text: &str) -> Result<(), PaneError>`.
      Its default refuses with `PaneError::Refused { code: PROMPT_UNSUPPORTED, .. }`.
    - `session_activity(&self, port: u16, session: &str) -> Result<Activity, PaneError>`.
      Its default answers `Ok(Activity::Idle)`.
  - `pub const PROMPT_UNSUPPORTED: RefusalCode = RefusalCode::from_static("prompt-unsupported")`.
  - `pub enum Activity { Idle, Busy, HoldingQuestion }`. The enum is closed, derives
    `Debug, Clone, Copy, PartialEq, Eq`, and has no Serde. There is no root re-export.
  - One module-doc sentence records the amendment.
  - **Satisfies** `seam_defaults_are_permissive`. It also makes the wrapper doubles compile,
    which clears all six RED errors. Every existing `HarnessPort` impl compiles unchanged:
    the testkit `FakeHarness`/`Mutant`, the adapter's `OpenCodeHarness`/`Recording`, and
    `ports_test`'s `TestHarness`.
- **`crates/holler-pane/src/tx_switch.rs`** (the engine):
  - **Codes (ruling 3):** `SESSION_BUSY` (`session-busy`) and `SESSION_HOLDS_QUESTION`
    (`session-holds-question`).
  - **Plan gate:** `plan()` now matches on `existing`.
    - Switch (`Some`) keeps `check_listed`/`check_unclaimed` unchanged.
    - Reset (`None`, which is exactly `Target::Fresh`) runs the new `check_idle`. It runs
      after `refuse_orchestrator` and `check_health`, and before any write.
    - `check_idle` returns `Ok` **without a port call** when `session_of_record` is `None`
      (N-5).
    - Otherwise it makes one `session_activity(record.harness.port, session_of_record)` query:
      - `Idle` passes.
      - `Busy` and `HoldingQuestion` are refused with a message that names the session and
        the pane.
      - A query that fails is passed on as it is.
    - **Satisfies** `reset_refuses_a_busy_or_questioning_conversation`,
      `reset_skips_the_activity_gate_without_a_session_of_record` and
      `switch_is_unaffected_by_the_activity_seam`.
  - **Prompt step:**
    - The survey's name: `SwitchRequest.first: Option<String>`.
    - Only after `cas_put` succeeds, `send_prompt(record.harness.port, &target, text)` runs.
      The target is by then the session of record, so the prompt cannot reach the old
      session or another pane (I2).
    - **Satisfies** `reset_with_first_queues_the_message_to_the_new_session_only`, including
      its `then_of_record` ordering pin and its `[Health, CreateSession, SelectSession,
      ShownSession]` op sequence. Also satisfies `reset_without_first_queues_no_prompt`.
  - **N-2's third decoration:**
    - New field `SwitchFailure.unprompted: Option<String>`.
    - A prompt failure sets only that field (`acted: false`, `created: None`).
    - `message()` then appends `; session "<id>" was created, shown and recorded, but the
      first message did not land`.
    - It appends no reconcile step and no "is not recorded" clause.
    - The error is still the port's own, so `class_of` decides the exit.
    - **Satisfies** `reset_first_prompt_failure_after_the_record_is_honest`.
  - **Module docs:**
    - New step 5, Prompt.
    - The failure paragraph covers the prompt failure.
    - "At most seven port calls" is now **eight**. I checked the count against the code:
      - A switch makes 7 calls: read, `health`, `list_sessions`, `pane_store.list`,
        `select_session`, `shown_session`, `cas_put`.
      - A reset with a record and `--first` makes 8: read, `health`, `session_activity`,
        `create_session`, `select_session`, `shown_session`, `cas_put`, `send_prompt`.
    - Part 1's "not here" deferral note is removed.
- **`crates/holler-cli/src/pane/reset.rs`**:
  - `PaneReset` gains `#[arg(long, value_name = "TEXT")] first: Option<String>` (ruling 2),
    which is passed to `execute`.
  - The module doc names `--first`, the two refusals and the prompt-failure behavior.
  - **Satisfies** the flipped `help_names_the_arguments` pin. It is also the clap half of
    every `--first` case.
- **`crates/holler-cli/src/pane/switch.rs`**:
  - `execute` and `request` take `first: Option<String>`.
  - Switch passes `None`; reset passes its flag through.
  - `execute` now has 7 parameters, which is within clippy's limit.

Docs (W-1, N-3; F's rows):

- **`docs/adr/ADR-0003.md`**: reset's row is now `holler pane reset PANE [--as-operator]
  [--profile NAME] [--first TEXT]   #645`. It is still one row, so `docs_rows` stays green.
- **`docs/adr/ADR-0021.md`**, the three spots in W-1:
  - **§8 "Switch and reset as built"** now covers:
    - the activity refusals (reset only, skipped when the pane has no session of record);
    - the prompt step after the record;
    - the honest prompt-failure message with no reconcile step;
    - "at most eight port calls";
    - the park-state sentence.
  - **§8, new paragraph "The prompt and activity seam":**
    - It records the two default-armed methods, `Activity`, and both defaults.
    - **N-3:** it explains why the default is `prompt-unsupported` (a refusal) and not
      `not-implemented`.
    - It records who replaces what: #642 part 3 the real methods, #684 the fake's recording.
    - It records two departures from the old 645b proposal, so that #646 and #649 build on
      the seam:
      - the prompt goes from the verb to `HarnessPort::send_prompt`, not through the hub's
        prompt path;
      - the activity read is on the session of record, not on DRIVEN.
  - **§9 codes row:** the PROPOSED (645b) clause is replaced with the as-built open codes:
    `session-busy`, `session-holds-question` and `prompt-unsupported`.
  - **Deferred list:** the PROPOSED item is rewritten as decided.

## Self-check

All runs below went through the worktree's `target` symlink. I confirmed it still points at
the run-private `<cache>/tmp/opencode/0645-target` and that `CARGO_TARGET_DIR` is unset, so
the shared, stale cache was not involved. Cargo ran one invocation at a time.

| Command | Result |
|---|---|
| `cargo test -p holler-cli --test pane_verbs` | **221 passed, 0 failed**. All 8 contract cases are `ok`: `reset_with_first_…`, `reset_without_first_…`, `reset_first_prompt_failure_…`, `reset_refuses_a_busy_or_questioning_…`, `reset_skips_the_activity_gate_…`, `switch_is_unaffected_…`, `seam_defaults_are_permissive`, `help_names_the_arguments`. |
| `cargo test -p holler-pane` | All green: lib 12/0, all 13 integration binaries (including `ports_test` 6/0), doctests 3/0. |
| `cargo test -p holler-cli --test pane_cli_process --test cli_surface_test --test docs_cli_test` | 35 + 3 + 3, all green. |
| `cargo test --workspace -- --skip roster_stays_accurate_under_concurrent_body_load` (the brief's full command) | **exit 0**: 140 result lines, 1759 passed, 0 failed. |
| `cargo clippy -p holler-pane -p holler-cli -p holler-pane-testkit -p holler-adapter-opencode --all-targets -- -D warnings` | Clean, exit 0. This also checks every `HarnessPort` implementor. |
| `rustfmt --edition 2021 --check` on the four touched `.rs` files | Clean. Ruling 4: no other file was reformatted. |
| `git diff --check`; grep of the diff for `unsafe` and `#[allow` | Clean; nothing added. |

**Test count.** `git grep -c '#[test]'` over `tests/pane_verbs/`, excluding `process/` (the
separate `pane_cli_process` binary), gives **214 at base `3d95aec`** and **221 at HEAD**. The
difference is exactly the 7 new cases, and the 221 matches the run.

T's handoff gives a base of "219 passed". That number came from the stale shared cache
(`decisions.md`, odd thing 3). It is not a suite defect; it only matters when reading counts.

This is a self-check only. **The authoritative GREEN is the plugin's T-green run.**

## Tests I think are wrong (if any)

None. Every assertion matches the brief and A's notes as written, and no change was made to
fit a test.

## Items for T / O (not test defects)

1. **The `cli-surface.txt` row for `--first` is now T's** (brief, T boundary). Under `# #645`,
   after `pane reset | demo-c1r1 --as-operator`, the proposed line is:
   `pane reset | demo-c1r1 --first "ship the fix" --format=json`
   - The same argv parses and runs in the green incident case.
   - No test enforces flag coverage, so the suite is green without the row. The fixture's
     header asks for every flag at least once, though. This is AC 7's `cli-surface.txt`
     half, which F cannot reach.
2. **ADR-0021 §9 class table, line 534**, still says "#645's and #646's, planned" about the
   open codes. That line is outside the three W-1 spots, so I left it. If O wants it updated
   at merge, the fix is a one-word edit ("#645's, merged; #646's, planned").

## Notes for A-dup / the operator (decisions taken, within the brief)

- **Park state and `--first`:**
  - Under the carve-out, `reset --first` calls `HarnessPort::send_prompt` directly. It does
    not go through the hub's prompt gate (#646), so a parked pane's reset still queues its
    first message.
  - No AC or test asks for a park check, so I added none.
  - The ADR seam paragraph records this gap for #646 and #649. A refusal on parked panes
    would be a follow-up.
- **`--first ""`** is not validated by the verb; the text goes to the port as typed. Nothing
  pins this either way. I4 is not at stake, because the harness API is not a TUI. The real
  adapter (#642 part 3) is where an empty prompt should be judged.
- **Success output is unchanged with `--first`**: the same text line and the same
  `{verb, pane, previous}` data. Exit 0 means the prompt was queued; a prompt that did not
  land always makes the run fail.
- **Message wording.** Only the brief's phrase is pinned:
  - busy: `session "<id>" of <pane> is running a turn; reset it once it ends`
  - question: `session "<id>" of <pane> holds a question; answer it first`
  - prompt failure: `<port error>; session "<id>" was created, shown and recorded, but the
    first message did not land`
- **Engine scope of `first`:** the engine would queue a first message after any target it
  records. Only reset ever sets it (switch passes `None`), and the docs describe it as
  reset's.

## Ready for T(green)

Implemented against the RED contract, entirely inside F's boundary, with nothing written
under `tests/**`. Locally, the six compile errors are gone, all 7 new cases and the flipped
help pin pass, and the brief's full workspace command exits 0. The plugin should now run the
suite: GREEN advances the loop; RED means I rework.
