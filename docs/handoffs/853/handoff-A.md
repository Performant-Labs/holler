# Handoff-A: #853 - `profile apply` creates column 1 first, and a real run with a refused step exits 3  (up-front plan review, round 2)

**Date:** 2026-10-12
**Branch:** issue-853-implementation (at `05d8e6a`: amendment round 2 on top of the amend-first commit `1f4ab40`; round 1's BLOCK is `eb29e7f`)
**Brief reviewed:** docs/handoffs/853-brief.md (round 2)   **Reuse map:** none from O (automated run). The A-derived map below carries over from round 1, with corrections.   **Wireframe:** N/A (no UI surface)
**Verdict:** PASS

## Summary

PASS, with 0 blocks and 9 warns. Both of round 1's blocks are resolved.

- **Block 1: the reconcile step.** It is fixed in four places. The one line now carries each acted refused step's
  `detail`, and `reconciled()` already ends that detail with the reconcile step:
  - ADR-0021 section 8 (lines 751-755), with lines 720-723 reworded;
  - the guide (`pane-control.md:712-714` and `721-722`);
  - Acceptance 4;
  - `rows.rs:544`, which is now listed as a test that changes.
- **Block 2: the double.** The brief now:
  - extends `FakeHerdr`'s `Placement::SplitOnly` to ADR-0021 section 10's rule;
  - cross-checks the fake against `plan_splits` in the adapter's tests;
  - runs Acceptances 2 and 4 on `SplitOnly`;
  - forbids a new double.

  I checked that this is feasible:
  - The adapter dev-depends on the test kit.
  - `plan` and `layout` are public modules.
  - `GridMap::default()` and `plan_splits_test.rs`'s `map_of` build every state the cross-check needs.

  Tightening the fake breaks only `rows.rs:182` and `fake_herdr_test.rs:191-218`. The conformance suite's split-only run
  uses column 1 only, so it keeps passing.

The production plan still passes:

- The order lives in `cell_key` and nowhere else.
- `refused()` goes beside `stopped()`.
- Exit 3 comes from `class_of`. In `halted`, every refused step's code is refusal-class by construction.
- The dry run is unchanged.

The warns are for T and F. Warns 1 to 3 decide how T writes RED and Acceptance 4's assertions, so T should read them
before authoring.

## Findings

| # | Severity | Plan element | Drift dimension | Finding | Suggested fix |
|---|---|---|---|---|---|
| 1 | warn | Acceptance 2 ("extend `SplitOnly` ... it fails (r2c1 refused) with the old order"); the Blast radius's "Test kit" | phase ownership; test seam | `crates/holler-pane-testkit/src/herdr.rs` is a production path (`crates/*/src/**` in `.aftersight/pipeline.config.json`). T writes only under `crates/*/tests/**`: in #646 the permission layer denied T an edit in `src/` (`docs/handoffs/0646-implementation/handoff-T-red.md:14-22`), and #808 left the test kit's `src/` to F (`docs/handoffs/808/handoff-T-red.md:16-17`). So F tightens `check_split`. At Phase 4 the fake is still loose, and the old order builds the whole grid on it. Run on this branch: `rows.rs:182` passes today, with r1c1, r1c2 and r2c1 all `created` on `SplitOnly`. The apply-level 2x2 test therefore passes at RED. | **T:** these tests carry Acceptance 2's RED, and each fails today for the right reason: the cross-check (the fake against `plan_splits`), the fake's updated `SplitOnly` tests, and `rows.rs:182` with its new outcome order. In handoff-T-red, record that the apply 2x2 test passes at Phase 4 on the loose fake. At Phase 6, show that it fails with the old order by reverting `cell_key` locally on the tightened fake. Do not build a double, and do not edit the test kit's `src/`, to force RED. **F:** tighten `check_split`, and update its docs (finding 8). |
| 2 | warn | Acceptance 1's seam ("an in-file `#[cfg(test)]` module of `tx_apply.rs` or ... a dry run on the rig") | test seam | This corrects my round-1 warn 6: the in-file option is not open. T cannot write in `crates/holler-pane/src/` (finding 1), and F writes no tests. A file in `holler-pane`'s own `tests/` would not work either, because that crate has no test-kit dev-dependency. | Test the order through the public contract, in the new `order.rs`. Run the engine on the rig with `dry_run: true` and no records, so that every step is a `planned` create in key order, or read the verb's `--dry-run` JSON. Specs in several workspaces work in a dry run, which never calls `ensure_pane`. |
| 3 | warn | Acceptance 4's main case ("the one-line `apply refused 1 step(s): ...` message") and ADR-0021:755 ("A refused step that acted nothing ... adds nothing beyond its pane and code") | cross-cutting (error boundary) | The launch engine decides which refusals count as acted; the code alone does not. `TxFailure.acted` is set to `entered` (`tx_launch.rs:605-616`), and the act calls `ensure_pane` first (`tx_launch.rs:680`). The checks before the act do not test reachability: `pane-exists` (`tx_launch.rs:225`), and `observe` (`408-423`) with `refuse_occupied` and `refuse_running`. So a create's `grid-unreachable` is an **acted** refusal. In Acceptance 4's main case, the one line ends with `; <pane>: <grid-unreachable message>; <reconcile step>`. The other refusals print only the pane and the code: the pre-act ones (`probe-failed`, `pane-exists`, `grid-occupied`, `port-in-use`, and a launch `usage` coded `spec-not-launchable`) and the planning ones (rows a and e, `harness-change-refused`). | **T:** assert these forms. **F:** key the carried detail on `Step::acted` alone. This also confirms the design: a dry run runs no probe and no split, so act-time refusals never appear in its plan, and the one line is the only place their message shows. |
| 4 | warn | ADR-0021:751-755 and `pane-control.md:712-714`: the carried detail keeps "a take-over note ... not lost" | ADRs (a claim the rule does not meet) | The rule keys on `acted`, but a take-over note is attached on its own terms. On any relaunch error, `relaunch()` (`run.rs:123-133`) adds the note "left profile Q and joined P before the relaunch, and stays in P". Suppose the leave and the join land and the relaunch is then refused before its act: `probe-failed` or `port-in-use` from `observe`, or `spec-not-launchable` from the relaunch rules. That step has `acted: false`. Its one line prints only `<pane> (<code>)`, so the move is never reported. ADR-0021:705-707, unchanged, says the operator learns what the take-over did "as the step's error says". The note of a failed join is safe: `cas_put` answers only failure-class codes (`ports.rs:95-106`), so that note rides on a `failed` step, which `stopped` prints. No existing test covers the missing case. | Decide once (O, or F under the standing orders, recorded in decisions.md). Either option works: (a) reword ADR-0021:752-755 and the guide to "a take-over note on such a step", and add that a refused step that acted nothing prints only its pane and code (the move shows in `profile show`); or (b) carry every refused step's detail. Either way `Step` stays as it is: no new engine field, and no string test on `detail`. |
| 5 | warn | The amend-first guide edit | docs | Section 3's verb entry, `### holler profile apply NAME` (`pane-control.md:424-429`), still states both old rules: "A `refused` or `reported` step does not stop the run or change the exit code", and "... 1 for a failed step, and 0 otherwise". The amendment changed only section 6. No docs test pins this sentence: `docs_pane_control_test.rs:471-482` checks only the vocabulary. | F rewrites lines 426-429 to match section 6 (a real run with a refused step exits 3; a dry run exits 0), under Acceptance 6's "the guide". |
| 6 | warn | "Tests that assert exit 0 beside a refused step must change ... at least ..." | scope (blast radius) | More tests than the brief names run the verb with a refused step and assert exit 0, or read `data`. **In the apply tests:** `rows.rs:175` (row a); `rows.rs:290` (`probe-failed`; run today, passes at exit 0); `rows.rs:440` (`spec-not-launchable`); `failures.rs:252` (row a with a failed health call). Their doc comments need the same change: `rows.rs:23-25, 158, 253, 407, 541` and `failures.rs:226`. **In the scratch scenario:** `scenario.rs:448-455` (step 5) reads its refusals from `data`, and so does `probe.rs:36`'s `apply()`. Tests that only call the engine (`observe.rs`, `takeover.rs:129`) do not change. | T updates each one. It keeps its engine assertions and moves its verb assertion to exit 3 and the one line, using finding 7's helper. |
| 7 | warn | `refused()` "beside `stopped()`", and the tests of the refused form | pattern consistency; anti-duplication | The failed form already has one test helper. `Stop` and `assert_stopped` (`rig.rs:571-626`) check exit 1, `data` null, the code, the message's prefix and tail, one `error:` line on stderr and an empty stdout, and the apply tests share them. About ten tests (finding 6's, plus the new ones) will check the refused form. | **T:** write one twin helper, defined once and `pub(crate)`, built on `envelope_of`; for example `Refusal` and `assert_refused`. It checks exit 3, `data` null, `error.code`, the prefix `apply refused `, the `; <M> other step(s) ran` part and the text form. `rig.rs` is at 775 lines, so the new file is the better home. Copying `assert_stopped`'s body into each test would be a Phase 7 rejection. **F:** build the message as `stopped()` does: `quoted(&step.pane)`, each `detail` as `reconciled()` left it, and `ErrorCode::from(error)`. Return `None` for a dry run. Add no second reconcile builder and no exit table. |
| 8 | warn | "add a cross-check in `crates/holler-adapter-herdr/tests/`"; the fake's docs | placement; docs | The natural home is `plan_splits_test.rs`. It already holds the two fake-against-adapter checks, `ac12_the_fakes_sequence_stops_where_real_herdr_nests` and `ac13_the_refusal_code_is_the_test_kits`, and the private map builders `rows_tree`, `map_of` and `target`. `ac12`'s comment (`plan_splits_test.rs:333-335`) describes the fake's old sequence (r1c1, r1c2, r2c2, r2c1), which the tightened fake refuses at r2c2. A `GridMap` cannot represent a fake with holes: the fake keeps panes in place after `close` or `vanish`, while real Herdr reflows (`herdr.rs:325-328`). | **T:** put the cross-check in `plan_splits_test.rs`. A new file is fine only if `rows_tree` and `map_of` move to `tests/common/mod.rs`; never copy them. Cover only states without holes, built by successive splits: use `GridMap::default()` for the empty workspace, a fresh fake for each probe, and some `grid-out-of-range` cells. Reword `ac12`'s comment. **F:** update the docs of `Placement::SplitOnly` (`herdr.rs:89-93`) and of `check_split` (`471-473`), and state the rule for holes in an `ASSUMPTION (#640)` comment. Refresh the stale `GRID_UNREACHABLE` assumption (`herdr.rs:43-48`): its "No verb or adapter raises or names them yet" has been false since #640's `ac13`. |
| 9 | warn | "Overlap with other branches" | coexistence | Two corrections. **(a)** #835 does not touch `crates/holler-cli/tests/profile_verbs/apply/rig.rs`. Its branch (`6787d99`, from merge base `b76f4d2`) changes `pane_verbs/import/rig.rs`, as its brief says (`835-brief.md:71`). My round 1 repeated the mix-up. The "touch only those lines" limit is therefore moot; the file's size is the only remaining reason to keep `rig.rs` edits small. **(b)** The ADR merge is not mechanical. #835 also appends decisions **30** and **31** ("`pane import` leaves Claude Code alone" and "`observe_pane` is required"), at the same point in ADR-0021's "Decisions taken". The other shared edits merge cleanly: section 9 rows six lines apart, different guide paragraphs, and separate CHANGELOG entries. #831 and #832 also touch these files, but their changes are already on `main` (#857, #840). | Correct the paragraph. Whichever branch lands second renumbers its pair of decisions. #853's two numbers are cited in the text, at ADR-0021:739, 741 and 854; #835's are not cited anywhere. #853 gates the release, so the natural order is that it lands first and keeps 30 and 31. |

## Reuse & Analogous-Feature map (A-derived; the baseline for the Phase 7 gate)

- **Relevant code mapped:**
  - `crates/holler-pane/src/tx_apply.rs`: `plan` and `cell_key` (331-353), `until_failed` (434-449), and `Planned::halted`,
    which sets `acted` (515-539).
  - `crates/holler-pane/src/tx_apply/run.rs`: `create` and `relaunch` (51-136), and the take-over notes (123-133, 184-214).
  - `crates/holler-pane/src/tx_launch.rs`: what runs before the act (225, 408-423) and what is acted (605-616, 680).
  - `crates/holler-cli/src/profile/apply.rs`: `run`, `reconciled` and `stopped` (68-134), and the clap doc (34-51).
  - `crates/holler-adapter-herdr/src/plan.rs`: `plan_splits` (75-106), `right_of` and `below` (150-205).
  - `crates/holler-pane-testkit/src/herdr.rs`: `Placement` (83-94), `place` and `check_split` (440-493).
  - `crates/holler-cli/tests/profile_verbs/apply/rig.rs`: `envelope_of`, `data_of`, `Stop` and `assert_stopped` (551-626).
  - `crates/holler-adapter-herdr/tests/plan_splits_test.rs`: `map_of` (34-36), `ac12` and `ac13` (332-352).
- **Closest analogous features:**
  - The order: the planner's own partition in `plan_splits` (`plan.rs:98-100`).
  - The refused run's output: `stopped` in the same verb.
  - The test of that output: `assert_stopped`.
  - The fake against the adapter: `ac13`, together with the conformance suite's split-only run.
- **Extend-vs-new:**
  - EXTEND `cell_key`, keeping it private. The key `(workspace, col != 1, row, col)` mirrors `plan_splits`.
  - EXTEND the verb with `refused()` beside `stopped()`, reusing `reconciled()`'s details, `quoted` and `ErrorCode::from`.
  - EXTEND `FakeHerdr`'s `Placement::SplitOnly` (`check_split`). This is F's change.
  - NEW: one cross-check in `plan_splits_test.rs`, and one new apply test file with one refused-form helper.
- **Parallel paths, which the Phase 7 gate rejects:**
  - a second order function, or a public order API in `holler-pane`;
  - a new Herdr double or decorator in the `holler-cli` tests, or a test-kit dependency on the adapter;
  - a second reconcile-step builder, or an exit-code table in the verb;
  - a new field on `tx_apply::Step` (finding 4);
  - copies of `assert_stopped` in each test, or copies of `plan_splits_test.rs`'s map builders.

## Notes for O

None required: the verdict is PASS.

- **Findings 4 and 9** fix wording and coordinate with #835. O may fold them into the brief, or leave them to F and S.
- **Findings 1 to 3 and 5 to 8** are for T and F, as written.

## Patterns referenced

- `crates/holler-adapter-herdr/src/plan.rs:75-106,150-205` and ADR-0021 section 10: the split rule that the fake copies.
- `crates/holler-pane/src/tx_launch.rs:225,408-423,605-616,680`: whether a launch refusal comes before the act or is acted.
- `crates/holler-cli/src/profile/apply.rs:89-134`: `reconciled` and `stopped`, the verb's one-line failure form.
- `crates/holler-cli/tests/profile_verbs/apply/rig.rs:551-626`: `envelope_of`, `data_of`, `Stop` and `assert_stopped`.
- `crates/holler-adapter-herdr/tests/plan_splits_test.rs:332-352` and
  `crates/holler-pane-testkit/tests/herdr_conformance_test.rs:91-101`: the existing checks of the fake against the
  adapter, and the conformance suite's split-only run.
