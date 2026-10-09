# Handoff-A: Phase 3 - #644 `pane launch` and `relaunch`  (up-front plan review)

**Date:** 2026-10-09
**Branch:** issue-644-implementation (worktree `.claude/worktrees/0644-launch-relaunch`, head `e7064e9`)
**Brief reviewed:** `docs/handoffs/644-brief.md` (as amended in `e7064e9`)   **Reuse map:** the brief's "Reuse map (extend, do not duplicate)", lines 1878-1892 (there is no separate `survey.md`)   **Wireframe:** N/A (no UI surface)
**Verdict:** BLOCK

## Summary

BLOCK, on four findings. Each one changes the binding API or an acceptance criterion, so they must be settled before T
writes RED tests. The overall shape is right: a pure engine in `holler-pane` over `Ports` (ADR-0021 section 5), thin verbs
that reuse `SpecFlags::validate`, `output::emit` and `class_of`, the I8 order through `edit_spec` with the record write
inside the act (as the test kit's scope documents), open codes declared where they are raised, and no frozen file touched.
The four blocks:

1. `tx_launch::spec_of_pane` duplicates the Pane-to-spec mapping. ADR-0021 section 3 puts that mapping in
   `profile_snapshot.rs`, and #662a is writing it there right now as `spec_from_pane`.
2. Relaunch's "same directory, move only with `--grid`" rules are checked only in the CLI. The engine, which `profile apply`
   (#664) will also call, can then record a false directory or leave a Herdr pane behind.
3. The record written at R fills in `last_observed.driven` without observing it. The frozen contract says that field is
   never inferred.
4. `launch.rs` gets a second reconcile-step function, beside the one #663 is building for #644 to call. Under the real
   scope this would also print the step twice.

## Findings

| # | Severity | Plan element | Drift dimension | Finding | Suggested fix |
|---|---|---|---|---|---|
| 1 | block | `pub fn spec_of_pane(&Pane) -> ProfileSpec` in `tx_launch.rs` (API line 1483; Reuse map line 1891; relaunch base, line 1630) | pattern consistency (parallel path); ADR | A second public copy of the Pane-to-spec mapping. ADR-0021 section 3 and issue #662 put it in `profile_snapshot.rs`. #662a's in-flight plan defines `profile_snapshot::spec_from_pane` there with the same mapping, plus `FIXED_PORT_POLICY_PREFIX` and `fixed_port_policy`, and expects #644 to use them. | Drop `spec_of_pane`. Relaunch's base becomes `profile_snapshot::spec_from_pane`, and `port_of_policy` parses with `FIXED_PORT_POLICY_PREFIX`. The MO either sequences #662a first or moves #662a's exact API into this blast radius. |
| 2 | block | relaunch CLI step 3 (lines 1632-1640); `RelaunchRequest.move_grid` (line 1506); B3 and B10 (lines 1655, 1659) | layering (choke point) | The cwd and cell rules are checked only in `relaunch.rs`, and the engine trusts a `move_grid` flag the caller computes. Any other caller (#664 `apply`, named in Forward-compat) can make the engine record a cwd tmux does not have, or create a pane at a new cell and leave the old pane open and unrecorded. | Replace `move_grid` with `grid_given`. The engine works out the move itself and, as its first plan step, refuses a cwd change or a cell change made without `--grid`. The `usage` text stays the same, so AC 19 does not change. Also check `spec.pane == name` there. |
| 3 | block | the record written at R (line 1621); AC 1 (line 1706), AC 20 (line 1817) | cross-cutting (I6, the record contract); ADR | `last_observed.driven = Some(sid)` is set without being observed: no port can see the hub's DRIVEN before #649. The frozen `LastObserved` (`pane.rs:179-180`) and ADR-0021 section 1 (line 45) say "never inferred". #647's plan leaves `driven` as stored, so the inferred value would stay. | Launch writes `driven: None`, and relaunch keeps the stored value. `shown` stays what O1 observed. Amend AC 1 and AC 20, and add a C-14. |
| 4 | block | `launch::reconcile_step` (line 1550); `emit_outcome` appends it whenever `acted` (decision 15, line 1948); AC 16h and 16j | duplication; layering | #663's in-flight plan makes `profile_scope::reconcile_step` the one shared copy for #644 and #646. It also adds the step to the scope's own `profile-conflict`, restore and timeout errors. #644 plans a second function with its own quoting and always appends it, so under the real scope the user sees two steps naming two doctor forms. The fake scope does not add the step, so #644's tests cannot see this. | One function, one owner and one doctor form, agreed with #663 before T pins AC 16h and 16j. The verb appends the step only to errors that do not already carry it (or the scope stops adding it). |
| 5 | warn | `TxOptions.now_ms` default (line 1489); "`std::time` only" (line 1452) | duplication | `holler_proto::clock::now_millis` already does exactly this (0 on a clock error). #207 folded 5+ hand-written copies into it, and `holler-pane` already depends on `holler-proto`. | Name it as the default in the API and in the Reuse map. |
| 6 | warn | `pub(crate) mod rig` in `tests/pane_verbs/launch.rs` | duplication (test helpers) | #643 (`crate::list::Rig`) and #647 (`doctor/rig.rs`) plan rigs over the same seven fakes in the same test target. Nothing on main sets a pattern yet. | If `crate::list::Rig` is on main when T starts, build on its fakes and add only the linked host and the hooks. Otherwise, say in Forward-compat which rig #645 and #646 reuse. |
| 7 | warn | relaunch's B10 runs before R (lines 1659-1660) | transaction order | If closing the old pane fails after the move, R is skipped and P is restored. The record then keeps the old pane id and cell while the new pane holds the TUI. | Write R first, then close the old pane best-effort and fail loudly if that fails. Or state why not. |
| 8 | warn | decision 12 (line 1935); the #647 row of Forward-compat (line 1983) | cross-story contract | Launch refuses any unrecorded occupant and expects #647 to "offer the way out". #647's plan reports `unregistered-herdr-pane` with no remedy. After a crash mid-launch (AC 7), no Holler verb frees the cell. | Correct the row and file a follow-up for O to assign. |
| 9 | warn | CLI step 2 reads P for the base, and `edit_spec` reads P again later | concurrency | Between the two reads (the probe alone can take 5 s), another writer's change to the same pane's entry is overwritten with no conflict reported, because `SpecEdit::Set` carries no expected generation. | Add it to Risks, and file a follow-up with #663 for an expected generation on the edit. |
| 10 | warn | decision 20 (the ADR-0021 edits) | ADR consistency; merge hygiene | (a) #647 and #662 are editing the adjacent "Deferred" lines and section 3 at the same time. (b) Section 9 writes a story's own codes as "open (#N)", and `unavailable` and `timeout` are already common to every verb. (c) The section 12 "Decided (#644)" note picks an option the operator did not list. | (a) Mark items decided in place, and cite #662's `fixed:<port>` sentence instead of restating it. (b) Follow the row convention. (c) Mark the note PROPOSED, or record the operator's confirmation. |

### Finding detail (the evidence behind each row)

**1. A second home for the Pane-to-spec mapping.**

- ADR-0021 section 3, lines 149-154: "The mapping lives in `holler-pane/src/profile_snapshot.rs` (#662), which the
  migration (#650) reuses." Issue #662's scope says the same. The stub's own module doc (`profile_snapshot.rs:1-4`) reads
  "Snapshot of a live pane as a `ProfileSpec`".
- #662 is in wave 3 beside #644, and its run is live (branch `issue-662-implementation` at `5b47d82`, scoped to 662a).
  Its brief (`docs/handoffs/662-brief.md:1440-1455` on that branch) makes these binding:
  - `spec_from_pane(pane: &Pane) -> ProfileSpec`, with exactly the field mapping of `spec_of_pane`, `fixed:<port>` included;
  - `FIXED_PORT_POLICY_PREFIX = "fixed:"`;
  - `fixed_port_policy(port)`.
- That brief names #644 as the reader of the prefix (lines 1936 and 1949), and it does not know `spec_of_pane` exists. So
  the justification in this brief ("#662 should call or move this one rather than write a second") does not hold.
- What happens if both ship:
  - Two public copies of one mapping in one crate.
  - The `fixed:<port>` grammar split between `tx_launch::port_of_policy` (the parser) and `profile_snapshot` (the prefix
    and the formatter).
  - `profile_diff::diff_spec` compares through `spec_from_pane` (662-brief lines 1558-1562), so any later difference
    between the two copies shows up as false drift in `profile show` and `apply` for every pane #644 launches.
  - Decision 20 leaves the ADR's sentence about where the mapping lives untrue.

**2. Relaunch's invariants checked at one call site.**

- The two rules protect the record and the layout:
  - "Relaunch cannot change a pane's directory": tmux keeps its cwd. `FakeHost::ensure_session` on an existing session
    "changes nothing, neither the cwd nor the processes" (brief F-12).
  - "Relaunch moves a pane only with `--grid`" (ADR-0021 section 10, line 431).
- `move_grid` can be worked out from `spec` and `record`: the CLI makes it true exactly when the effective cell differs.
  As a separate field, all it adds is a way to pass a request that contradicts itself.
- Over the engine as specified, two things can go wrong:
  - `move_grid: false` with a different cell: B3 creates a pane at the new cell, B10 is skipped, and the old Herdr pane is
    left open with no record naming it.
  - A different cwd: R records a cwd the tmux session does not have.
- This matters because of who else calls the engine. Forward-compat (line 1981) has #664 call `tx_launch::relaunch` with a
  complete spec. `--spec-only` lets P's cwd and cell drift from the live pane on purpose (decision 11, AC 16g), and those
  drifted specs are what `apply` would pass.
- The codebase's pattern is that the shared transaction enforces its own rules:
  - the scope's own guards before any write (`holler-pane-testkit/src/profile_scope.rs:49-53`);
  - the registry's membership check inside its compare-and-swap (ADR-0021 "Decisions taken", item 2);
  - this brief's own engine re-check of `spec_only` (line 1499) and of `herdr_session` (launch step 2).
- `usage` keeps the CLI's exit codes. If O wants `apply` to be able to match a code, an open refusal code is an option.

**3. An inferred DRIVEN in a stored record.**

- `pane.rs:179-180`: "What reconcile last observed about which session the pane shows and which the hub drives. Written by
  reconcile, never inferred." ADR-0021 line 45 says the same, with I6.
- How DRIVEN follows `session_of_record` is deferred to #649 and #654 (ADR-0021 lines 452-453).
- `shown` is observed (O1), so writing it is fine. `driven` is copied from `sid`, so the record would claim the hub drives
  a session nobody checked. That is the condition the epic's SHOWN-versus-DRIVEN check exists to expose (#633).
- #647's plan (`issue-647-implementation` at `878a5b9`, 647-brief lines 888-889, 937, 1051-1053) keeps to the contract:
  DRIVEN cannot be observed before #649, so reconcile compares SHOWN with `session_of_record` and leaves
  `last_observed.driven` as stored. #644 would then be the field's only writer, and its unobserved value would stay.
- Writing `driven` would need an amend-first change to the frozen `LastObserved` contract.

**4. Two reconcile steps.**

- ADR-0021 section 8 step 6 and section 12 make the step a concern of every verb that acts: launch, relaunch and close, and
  any verb that times out.
- #663's plan (`issue-663-implementation` at `aaf8fb5`, 663-brief lines 1520-1548) has two parts:
  - Decision 8: `pub fn reconcile_step(profile: &ProfileName)` in `holler-cli/src/pane/profile_scope.rs`, "pub because the
    spec-editing verbs (#644, #646) print the same step ... one copy".
  - Decisions 5 to 7: the scope adds that step to its own `profile-conflict`, restore-failure and first-write-timeout errors.
- With #644's `emit_outcome` appending its own step to every acted failure, a real `profile-conflict` would read "... to
  reconcile, run holler pane doctor --profile 'P' and then holler profile show 'P'; to reconcile, run: holler pane doctor
  demo-c1r1, holler profile show 'P'".
- `FakeProfileScope` adds no step, so #644's tests stay green and #649 is where this first shows.
- #645 and #646 would also have to import from `pane::launch`, a verb module, or copy it.
- #647 gives `pane doctor` a `[PANE]` positional (647-brief line 27), so `holler pane doctor <pane>` will parse. The choice
  between the two doctor forms is the only open point.

**5 to 10.** Evidence for the warns:

- **5:** `crates/holler-proto/src/clock.rs:33-40`.
- **6:** 643-brief lines 1081-1086 (`issue-643-implementation` at `2e7f221`) and 647-brief lines 1015-1020. #647's rig also
  hand-writes the launch sequence that `tx_launch::launch` will provide. That is a note for O, not for this story.
- **8:** 647-brief line 904: remedy "none (no holler verb adopts a pane; `pane import` is #650's)".
- **9:** brief F-7: `edit_spec` reads P itself; CLI step 2 reads it earlier, at lines 1570 and 1630.
- **10:**
  - (a) ADR-0021 lines 523-532. #647 deletes line 524 (647-brief line 888), and #662 rewrites line 525 and section 3
    (662-brief lines 1766-1768).
  - (b) ADR-0021 lines 331-339: the common codes, and the "open (#645)" row form.
  - (c) ADR-0021 lines 462-468 ("with an amendment to the contract first"), and the ADR's PROPOSED convention (line 3).

## Notes for O

Amend the brief as follows, then start a **fresh** run. A `resumeFromRunId` would replay this verdict.

1. **Finding 1, the mapping.**
   - Remove `spec_of_pane` from the API sheet, the Reuse map and Forward-compat.
   - Relaunch's base is `holler_pane::profile_snapshot::spec_from_pane(&record)`.
   - `port_of_policy` stays in `tx_launch.rs` as the grammar's parser, but builds on
     `profile_snapshot::FIXED_PORT_POLICY_PREFIX`. No second `"fixed:"` literal.
   - The MO decides the order:
     - (a) #662a merges first (its `profile_snapshot.rs` is about 70 lines), and #644 starts from that `origin/main`; or
     - (b) #644 goes first and fills `profile_snapshot.rs` with #662a's exact `spec_from_pane`, `fixed_port_policy` and
       `FIXED_PORT_POLICY_PREFIX` (signatures and docs as in 662-brief lines 1440-1455). Its blast radius widens to that
       file, and the MO amends #662a's brief to reuse them.
   - Decision 20 must leave ADR-0021 section 3's sentence about where the mapping lives true. The `port_policy` "Deferred"
     item is #662's to close.
2. **Finding 2, relaunch's rules.**
   - Change the request to `RelaunchRequest { record, spec, grid_given: bool, profile, spec_only }`.
   - Add a first plan step to `tx_launch::relaunch`, with no port call and skipped with `spec_only`:
     - `spec.pane == record.name`, else `usage`;
     - an unchanged `host.cwd`, else `usage` with the same message;
     - a cell change only when `grid_given`, else `usage` with the same message naming both positions.
   - The move is "the cell differs". The CLI passes `grid_given = --grid was given` and keeps its parsing.
   - Do the same `spec.pane == name` check in `launch`.
   - AC 19 stays as written. Optionally add one engine-level case, such as
     `relaunch_engine_refuses_a_cell_change_without_grid`.
3. **Finding 3, DRIVEN.**
   - The record at R gets `last_observed { shown: Some(sid), driven: None, at: now_ms() }` for launch, and the stored
     `record.last_observed.driven` for relaunch.
   - Amend AC 1 and AC 20 to match.
   - Add C-14: the issue's "confirm SHOWN equals DRIVEN" is checked as SHOWN == `session_of_record` (I2) until #649 wires
     DRIVEN. This is the reading #647 uses.
4. **Finding 4, the reconcile step.** Settle with #663, whose brief is not yet plan-reviewed:
   - One function. The recommendation is #663's `profile_scope::reconcile_step`, taking `(pane: &PaneName, profile:
     Option<&ProfileName>)`.
   - One doctor form.
   - One rule for which layer appends it: the verb adds the step only to errors that do not already carry it, or the scope
     stops adding it and every verb appends.
   - Rewrite decision 15, the `launch.rs` API lines, and AC 16h and 16j to the single agreed text. #644 adds no second
     quoting helper.
   - If #663 cannot change, the brief says exactly which text #644 appends and why there is no duplicate.
5. **The warns** are small edits:
   - Name `holler_proto::clock::now_millis` (finding 5).
   - One sentence on the rig (finding 6).
   - Order R before B10, or justify (finding 7).
   - Fix the #647 row of Forward-compat and add the follow-ups for findings 8 and 9.
   - Follow the ADR edit rules of finding 10. For 10(c), ask the operator, or have F write the note as PROPOSED.

## Patterns referenced

- `docs/adr/ADR-0021.md`: section 1 (line 45), section 3 (lines 149-154), section 5 (lines 176-192), section 8 (lines
  285-305), section 10 (lines 429-432), section 12 (lines 455-468).
- `crates/holler-pane/src/pane.rs:179-190` (`LastObserved`) and `crates/holler-pane/src/profile_snapshot.rs:1-4`.
- `crates/holler-pane-testkit/src/profile_scope.rs:31-55`: the scope's own guards before any write, and "recording the pane
  ... is the verb's, inside its act".
- `crates/holler-proto/src/clock.rs:1-40` (`now_millis`, #207).
- The in-flight sibling briefs, read from their branches: `issue-662-implementation` (`5b47d82`), `issue-663-implementation`
  (`aaf8fb5`), `issue-647-implementation` (`878a5b9`) and `issue-643-implementation` (`2e7f221`), each at
  `docs/handoffs/<N>-brief.md`.
