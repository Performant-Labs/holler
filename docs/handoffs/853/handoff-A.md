# Handoff-A: #853 - `profile apply` creates column 1 first, and a real run with a refused step exits 3  (up-front plan review)

**Date:** 2026-10-12
**Branch:** issue-853-implementation (at `1f4ab40`, the amend-first commit: ADR-0021, ADR-0003, `docs/pane-control.md`)
**Brief reviewed:** docs/handoffs/853-brief.md   **Reuse map:** none in the brief (automated run, no O survey). The brief's "Blast radius" and the double that Acceptance 2 names are the plan's reuse claims; the map below is A-derived.   **Wireframe:** N/A (no UI surface)
**Verdict:** BLOCK

## Summary

BLOCK, on two findings. Both are cheap to fix in the brief and the amendment now, and expensive once T has pinned tests.
The production plan is sound. `cell_key` in `tx_apply.rs` is the right place, and the only place, for the order, and the
proposed key is the planner's own partition (`plan.rs:98-100`). `refused()` beside `stopped()` in the verb leaves the engine
unchanged and free of I/O, and `class_of` makes every refused step's code exit 3. However:

1. The amendment's refused-run one-liner ("no other text") quietly drops step 6's reconcile step for a refused step whose
   act had begun. An unchanged sentence of the same ADR paragraph requires that step, and so does C25's test.
2. Acceptance 2 rests on a false premise. The test kit's `FakeHerdr` does not plan with `plan_splits`: its split-only rule
   is looser, and the apply rig uses absolute placement, so the old order passes on both. T cannot get RED, Acceptance 4's
   `grid-unreachable` cannot be produced on the fakes, and CI would keep no guard that depends on the order. Choosing which
   double to extend is an architecture decision, and the plan has to make it before T writes tests.

Everything else passes, with the warns below:

- The dry run stays at exit 0, as `pane import`'s does.
- No wire, golden file, record or spec changes.
- The ADR-0003 row follows that block's `#NNN: note` form.
- `docs_cli_test` and `docs_pane_control_test` pass on the amendment (run).
- No script or document consumes apply's output or exit code (searched).

## Findings

| # | Severity | Plan element | Drift dimension | Finding | Suggested fix |
|---|---|---|---|---|---|
| 1 | block | The amendment's "A refused step now exits 3" (ADR-0021 section 8 at line 740, decision 31), the guide's "A refused run prints only the refusal" (`pane-control.md:710`) and the brief's "The exit code": a message "naming every refused pane with its code and no other text" (ADR-0021:750) | ADRs; cross-cutting (error boundary: the reconcile step) | On a real run with a refused step, the one-line failure is the run's only output (a failure envelope has no `data`), so no refused step's `detail` is printed. The plan's own contract puts step 6's reconcile step in that `detail` when the step's launch or relaunch had begun its act (`Step::acted`). This case is reachable: `run.rs:60` (`tx_launch::launch(..)?`) carries `TxFailure.acted` through `Halt` into a `Refused` step, for example on a `verify_serving` refusal. Several rules still require that step: ADR-0021:720-722, unchanged by the amendment ("carries step 6's reconcile step, a `refused` step at the end of its `detail`"); ADR-0021:648 ("Every failure once the act has started ends with the reconcile step"); and the guide (`pane-control.md:720`, unchanged). C25's test `rows.rs:544` (`a_refusal_after_the_act_began_names_the_reconcile_step`, run: passes on this branch) requires the step in both formats, so that the run "must not read as if nothing ran". Every other acted path prints it: `pane launch`'s `failure_body` (`launch.rs:471-478`) does so for its exit-3 refusals too, and apply's own `stopped` carries it in the failed step's detail. The take-over notes that a refused step carries are lost the same way (`run.rs:124-133`, `run.rs:203-212`, e.g. "is in no profile now ... run apply again"); ADR-0021:705-707 says the operator learns that state "as the step's error says". The brief does not list `rows.rs:544` among the tests that change and gives no rule for it, so T must either delete C25's guard or contradict the amendment. | Amend before any test is written. (a) When any refused step has `acted`, the message ends once with `"; "` and `reconcile_step(Some(P))`, appended only when absent, as `failure_body` does. Build it with the existing builder, never a second one. (b) Decide once whether a refused step's note is carried. Recommended: yes, after that pane's code. (c) Reword ADR-0021:720-722 and `pane-control.md:720` to say where an acted refusal's reconcile step is printed now. (d) In the brief, list `rows.rs:544` as a test that changes, and add an acceptance item: for a real run with an acted refusal, exit 3 and the one line ends with the reconcile step, in JSON `error.message` and in the text `error:` line; row a's refusal, which ran nothing, adds none. |
| 2 | block | Acceptance 2 ("on the testkit's fake Herdr (which plans with the real `plan_splits`) ... It fails with the old order"), Acceptance 4's `grid-unreachable` on the fakes, and the Blast radius ("Nothing else ... changes") | pattern consistency (the test double against the adapter); reuse (which object to extend) | `FakeHerdr` does not use `plan_splits`. `holler-pane-testkit` depends only on `holler-pane` and `serde_json` (its `Cargo.toml`) and has its own `GRID_UNREACHABLE` (`herdr.rs:52`). Its default, `Placement::Absolute`, places any free cell, and that is what the apply rig uses (`rig.rs:199-200`: 3x3, no `set_placement`). Its `Placement::SplitOnly` (`check_split`, `herdr.rs:471-493`) accepts any free cell that has a pane to its left or above it. That is looser than the adapter's planner (ADR-0021 section 10; `plan.rs:168-205`: a new row only at column 1, below a last row that holds one pane). So the old order builds the whole 2x2 under both placements. Two tests show it (run on this branch): `rows.rs:182` (`row_b_missing_panes_are_created_by_splits_in_grid_order`) passes with `r1c1, r1c2, r2c1` all `created` on `SplitOnly`, which is exactly the sequence real Herdr refuses; and `fake_herdr_test.rs:205-217` passes while making `r1c2, r2c2, r2c1` after `r1c1`. Consequences: (a) Acceptance 2 cannot be RED. (b) Acceptance 4's case (live `r1c1` and `r1c2`, spec `r2c1`) is not refused on the fakes. (c) Acceptance 3 is opt-in and `#[ignore]`, so CI would have no test that depends on the order, and the gap between fake and adapter that hid this bug would stay. Fixing it means editing shared test infrastructure outside the blast radius, or adding a double: that extend-vs-new choice belongs to Phase 3, not to T in the middle of RED. | **Extend** `FakeHerdr`'s `Placement::SplitOnly` to the adapter's rule (`plan.rs`, `Widths::step_to`): `r1c1` starts an empty workspace; a new cell in an existing row goes only right of that row's last pane; a new row starts only at column 1, directly below the last row, and that row must hold one pane. Update the fake's doc and its `ASSUMPTION (#640)` wording, and update `fake_herdr_test.rs::split_only_mode_refuses_an_absolute_placement`. The conformance suite uses only column-1 cells and is written to hold against real Herdr, so it should keep passing. Add a cross-check in `crates/holler-adapter-herdr/tests/`; the adapter dev-depends on the test kit, so it can see both. Over small grids, for every free cell, the fake's `SplitOnly` must accept exactly when `plan_splits` plans one step for a one-cell target. Add `crates/holler-pane-testkit/src/herdr.rs` and `crates/holler-pane-testkit/tests/fake_herdr_test.rs` to the blast radius. The tests for Acceptance 2 and 4 call `rig.herdr.set_placement(Placement::SplitOnly)`. **Rejected:** (a) a new `HerdrPort` in the `holler-cli` tests that wraps the planner: it is a parallel fake, and `rig.rs`, at 775 lines, is the #835 overlap file. (b) A test-kit dependency on the adapter: it reverses the test kit's adapter-agnostic layering and forms a cycle with the adapter's dev-dependency. (c) A fault-scripted refusal for Acceptance 2: it cannot show that the result depends on the order, though it is acceptable for exit-code mechanics alone. |
| 3 | warn | The Blast radius list of "tests that state or pin the old order", and Acceptance 5 | scope | `fake_herdr_test.rs:536` is `snapshot_lists_panes_by_workspace_then_row_then_col`. It pins the order in which `FakeHerdr::snapshot()` lists panes, which matches the real adapter's snapshot (`adapter.rs:25-27`, "by row and then column"). It has nothing to do with apply's step order, and Acceptance 5 would push T to change it. It must not change. The list also misses places that do change. **`rig.rs:462` `EVERY_ROW`:** `every_row`'s cells `r1c1, r1c2, r1c3, r2c1, r2c2, r2c3` become `a, d, b, c, e, f`, and its doc lines `rig.rs:422,461` change with it (`rig.rs` is the #835 overlap file). **`rows.rs:182`:** its outcome order becomes `r1c1, r2c1, r1c2`. **`dry_run.rs:20,63`:** the wording "in grid order". `probe.rs:133` and `scenario.rs:238` change only in their comments: every scratch workspace is 2x1, so both orders agree there. | Strike `fake_herdr_test.rs:536` from the list and from Acceptance 5. Add the lines above, naming `rig.rs`'s lines explicitly because of #835. |
| 4 | warn | Acceptance 6 and the production list ("`profile/apply.rs`'s module doc") | docs | `ProfileApply`'s clap doc (`holler-cli/src/profile/apply.rs:34-51`) is the verb's `--help` text, and it states both old rules. Line 36 says "in workspace, row and column order", and line 43 says "A refused or reported step does not change the exit code". The brief names only the module doc. No help golden file pins this text (searched). | Add the clap doc to Acceptance 6 and to the production list. Add `cell_key`'s own doc comment (`tx_apply.rs:349`) beside lines 21, 103 and 331. |
| 5 | warn | `cell_key` alongside the planner | abstraction / single rule | `holler-adapter-herdr` depends on `holler-pane`, so the apply order and the planner's partition (`plan.rs:98-100`) cannot share code, and the fake's rule (finding 2) is a third copy. Keeping the planner unchanged is right. | Keep the key inside `cell_key` (e.g. `(workspace, col != 1, row, col)`) and add no public order API to `holler-pane`. In `cell_key`'s doc comment, name `plan_splits` as the rule it mirrors, and let the comment at `plan.rs:98` name apply in return. The cross-check of finding 2, together with Acceptances 1 and 2, ties the three copies together. |
| 6 | warn | Acceptance 1 ("a pure unit test of the order") | test seam / API surface | `cell_key` and `plan` are private. | Do not widen the API of `holler-pane` to test them. Use either an in-file `#[cfg(test)]` module in `tx_apply.rs` (precedent: `probe.rs`, `unmanaged.rs`; the file has 555 lines) or the public contract: `apply` with `dry_run: true` on the rig, with no spec recorded, so every step is a `planned` create in key order. "Workspaces keep their order" means today's order by workspace name, the first element of `cell_key`, not P's order; T pins that reading. |
| 7 | warn | Where the new apply tests go | size and structure | `rows.rs` has 594 lines and `rig.rs` has 775. The overlay flags about 800 lines, and lint fails a file at 900. | Put the new Acceptance 2 and 4 tests and the exit-3 cases in a new file under `profile_verbs/apply/`, registered in `profile_verbs/apply.rs` (e.g. `order.rs` or `refused.rs`), not in `rows.rs` or `rig.rs`. Limit the `rig.rs` changes to `EVERY_ROW` and its doc lines. |
| 8 | warn | Acceptance 3: "the rig's `WORKSPACES` gets a `2x2` workspace" | shared test infrastructure | `WORKSPACES` (`pane_integration/scratch/mod.rs:98`) is in #649's scratch module, which `pane_integration` and `profile_apply_scenario` both include whole. Its doc says "each 2 rows by 1 column", and `herdr_snapshot()` (`:483`) hardcodes `Extent { rows: 2, cols: 1 }` for every label. Reading back a 2x2 still works, because the adapter's `snapshot` reads the grid tree and not the extent (`adapter.rs:389-405`), and the binary gets 2x2 from `HOLLER_HERDR_WORKSPACES`. | Update the doc comment. If the test side ever needs a workspace's extent, parse it from `WORKSPACES` rather than add a second hardcoded value. |

## Reuse & Analogous-Feature map (A-derived; the brief has none)

- **Relevant code mapped:**
  - `crates/holler-pane/src/tx_apply.rs`: `plan` and `cell_key` (331-353); `until_failed` (434-449); `Planned::halted`, which sets `acted` (518-539).
  - `crates/holler-pane/src/tx_apply/run.rs`: `create` and `relaunch` (51-136); the take-over notes (124-133, 203-212).
  - `crates/holler-cli/src/profile/apply.rs`: `run`, `reconciled` and `stopped` (68-134).
  - `crates/holler-adapter-herdr/src/plan.rs`: `plan_splits` (75-106); `below` and `right_of` (150-205).
  - `crates/holler-adapter-herdr/src/adapter.rs`: one cell per `ensure_pane` (222-249).
  - `crates/holler-pane-testkit/src/herdr.rs`: `Placement` (83-94) and `place`/`check_split` (440-493).
  - `crates/holler-cli/src/pane/launch.rs`: `failure_body` (470-478).
  - `crates/holler-cli/src/pane/profile_scope.rs`: `reconcile_step` (49).
  - `crates/holler-cli/src/output.rs`: `exit_code` through `class_of` (336-340).
- **Closest analogous features:**
  - For the order: the planner's own partition in `plan_splits`.
  - For the exit form: `stopped` in the same verb.
  - For a run-level exit-3 refusal: `pane import`'s `import-blocked`. It is decided in the engine before any write, whereas apply's is decided in the verb after the run, which is consistent with `stopped`.
  - For the reconcile step on an acted failure or refusal: `failure_body`.
- **Extend-vs-new:**
  - EXTEND `cell_key`.
  - EXTEND the verb with `refused()` beside `stopped()`, reusing `reconcile_step` and each step's `acted`.
  - EXTEND `FakeHerdr`'s `SplitOnly`.
  - NEW: one cross-check test in `holler-adapter-herdr/tests`, and the new apply test file(s).
  - **Parallel paths, which the Phase 7 gate would reject:**
    - a second order function, or a public order API in `holler-pane`;
    - a new Herdr double or decorator in the `holler-cli` tests;
    - a second reconcile-step builder;
    - an exit-code table in the verb (exit codes come from `class_of` alone).

## Notes for O

On the automated path a BLOCK stops the run as `needs-amendment`. Amend the following, commit, and start a **fresh** run (not
`resumeFromRunId`):

1. **Contract (finding 1).** Edit the ADR-0021 section 8 paragraph "A refused step now exits 3" (line 740 on; the message
   at 750), decision 31, the guide bullet at `pane-control.md:710` and the brief's "The exit code".
   - Replace "naming every refused pane with its code and no other text" with a rule that keeps step 6: when any refused
     step had begun its act, the one line ends once with `; ` and the reconcile step of P (`reconcile_step(Some(P))`,
     appended only when absent).
   - State whether a refused step's note is carried.
   - Reword ADR-0021:720-722 and `pane-control.md:720` to match.
   - In the brief, add `crates/holler-cli/tests/profile_verbs/apply/rows.rs:544` to the tests that change, and add an
     acceptance item for the acted refusal at exit 3 in both formats.
2. **The double (finding 2).** In the brief's Acceptance 2, Acceptance 4 and Blast radius:
   - Replace "the testkit's fake Herdr (which plans with the real `plan_splits`)" with the decision: extend `FakeHerdr`'s
     `Placement::SplitOnly` to ADR-0021 section 10's rule, with a cross-check against `plan_splits` in
     `crates/holler-adapter-herdr/tests/`.
   - Add `crates/holler-pane-testkit/src/herdr.rs` and `crates/holler-pane-testkit/tests/fake_herdr_test.rs` to the blast
     radius.
   - State that the tests for Acceptance 2 and 4 run on `SplitOnly`. Acceptance 2 is then RED on the old order.
3. **List corrections (findings 3 and 4)**, in the same edit: strike `fake_herdr_test.rs:536`; add `rig.rs:422,461,462`,
   `rows.rs:182`, `dry_run.rs:20,63` and the `ProfileApply` clap doc at `profile/apply.rs:34-51`.

Findings 5 to 8 are for T and F, and need no amendment.

## Patterns referenced

- `crates/holler-adapter-herdr/src/plan.rs` and ADR-0021 section 10 (#640): the split rule, and one cell per `ensure_pane` (`adapter.rs:222-249`).
- `crates/holler-cli/src/pane/launch.rs:470-478` and `crates/holler-cli/src/pane/profile_scope.rs:49`: the reconcile step after an acted failure, exit 3 included (ADR-0021 section 8, step 6, and lines 648 and 720-722).
- `crates/holler-cli/src/profile/apply.rs:110-134`: `stopped`, the verb's existing one-line failure form.
- `crates/holler-pane-testkit/src/herdr.rs:83-94,471-493` and `crates/holler-pane-testkit/src/conformance/herdr.rs:23-27`: the fake's placements, and the conformance suite's split-only premise.
- `crates/holler-cli/tests/profile_verbs/apply/rows.rs:182,544` and `crates/holler-pane-testkit/tests/fake_herdr_test.rs:191-218,530-568`: the baselines run on this branch.
