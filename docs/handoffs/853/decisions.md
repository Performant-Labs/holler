# Decision journal: #853, `profile apply` creates a workspace's column 1 first; a real run with a refused step exits 3

Run handoff directory `docs/handoffs/853` · branch `issue-853-implementation` · rigor in-session · uiSurface false
(D and U are N/A). Append-only; every phase adds its own entry.

## A, up-front plan review: 2026-10-12T04:54-06:00

- **Decided:** BLOCK, on 2 blocks and 6 warns (see handoff-A.md).
  - **Block 1.** The amendment's refused-run one-liner ("no other text") drops step 6's reconcile step for a refused step
    whose act had begun. That contradicts ADR-0021:720-722 and 648, both unchanged, and C25's test `rows.rs:544`.
  - **Block 2.** Acceptance 2's premise is false. `FakeHerdr` does not plan with `plan_splits`: the rig uses `Absolute`,
    and `SplitOnly` accepts any free cell with a pane to its left or above it. So the old order passes on the fakes, T
    cannot get RED, and Acceptance 4's `grid-unreachable` cannot be produced there.
  - **Recommended for block 2:** extend `FakeHerdr`'s `SplitOnly` to ADR-0021 section 10's rule, plus a cross-check against
    `plan_splits` in `holler-adapter-herdr/tests`.
  - **Rejected for block 2:** a new planner-wrapping double in the `holler-cli` tests, and a test-kit dependency on the
    adapter.
  - **The production placement passes:**
    - the order belongs in `cell_key`, and nowhere else;
    - `refused()` goes beside `stopped()`, with the engine's result unchanged;
    - `class_of` gives exit 3 for every refused step's code;
    - the dry run stays at exit 0.
  - With no O survey in this automated run, A wrote the Reuse map in handoff-A.md. It is the baseline for the Phase 7 gate.
- **Assumed:**
  - **The reconcile step.** The operator's "one-line failure form" decision covers the output's shape (one line, no
    envelope change), not the removal of step 6's reconcile step. Restoring that step inside the one line keeps the
    operator's choice. If the operator meant to drop it, decision 31 must say so and C25 must be retired explicitly.
  - **The fake's rule.** Tightening `SplitOnly` keeps the Herdr conformance suite green, because its cases use only
    column-1 cells (from the suite's doc and case names; not run against a tightened fake).
- **Hedged:**
  - **Notes on refused steps.** Whether a refused step's take-over note is carried in the refused-run message is left to
    the amendment. Recommended: yes.
  - **Which double.** The choice between extending the fake (recommended) and another double is the amendment's; A rejects
    only the parallel options.
  - **Seam for Acceptance 1.** It is T's choice: an in-file `#[cfg(test)]` module, or a dry run on the rig.
- **Evidence:**
  - **Read:**
    - the brief, and the amendment diff (`git show 1f4ab40`);
    - `tx_apply.rs`, `tx_apply/run.rs`, `holler-cli/src/profile/apply.rs` and `output.rs`;
    - `pane/launch.rs:440-479` and `pane/profile_scope.rs:1-56`;
    - `plan.rs` and `adapter.rs` (the header, `ensure`, `snapshot`);
    - `holler-pane-testkit/src/herdr.rs` and its `Cargo.toml`, plus the conformance suite's header;
    - `rows.rs`, `rig.rs:415-475` and `fake_herdr_test.rs:185-250,480-568`;
    - `profile_apply_scenario/main.rs` and `probe.rs:110-150`;
    - `pane_integration/scratch/mod.rs:90-100,475-500`;
    - ADR-0021 sections 8 and 9 (lines 484-533, 640-760, 838-849, 902, 930-935) and the ADR-0003 verb block.
  - **Ran on this branch (warm build):**
    - `apply::rows::row_b_missing_panes_are_created_by_splits_in_grid_order`: ok. `r1c1, r1c2, r2c1` are all `created` on
      `SplitOnly`.
    - `split_only_mode_refuses_an_absolute_placement`: ok. The fake accepts `r1c2, r2c2, r2c1` after `r1c1`.
    - `apply::rows::a_refusal_after_the_act_began_names_the_reconcile_step`: ok. C25 holds today, at exit 0.
    - `docs_cli_test` (3 tests) and `docs_pane_control_test` (18): ok on the amendment.
  - **Searched:**
    - every statement of the old order and the old exit rule (`grep` over `crates`, `docs`, `scripts` and `CHANGELOG.md`);
    - consumers of `profile apply` in `scripts/` and `docs/`: none;
    - golden files holding apply output: none.
