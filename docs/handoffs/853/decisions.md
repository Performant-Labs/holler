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

## A, up-front plan review, round 2 (fresh run on the amended brief `05d8e6a`): 2026-10-12T05:17-06:00

- **Decided:** PASS, with 0 blocks and 9 warns (see handoff-A.md, which replaces round 1's; round 1's is in git at `eb29e7f`).
  - **Round 1's blocks are resolved.**
    - Block 1: an acted refused step's `detail`, which ends with the reconcile step, now travels in the one line. The ADR,
      the guide and Acceptance 4 say so, and `rows.rs:544` is listed as a test that changes.
    - Block 2: `SplitOnly` is extended to section 10's rule, with a cross-check against `plan_splits`; no new double.
  - **Warns:**
    1. The fake's change is F's, because `holler-pane-testkit/src` is a production path. T carries Acceptance 2's RED with
       the cross-check, the fake's tests and `rows.rs:182`. The apply 2x2 test passes at Phase 4 and is shown to depend
       on the order at Phase 6, by reverting `cell_key`.
    2. Acceptance 1's in-file `#[cfg(test)]` option is not open to T. This corrects my round-1 warn 6.
    3. A create's `grid-unreachable` is an acted refusal, so its one line carries the detail.
    4. A take-over note on a relaunch refused before its act is not carried, though the ADR says it is.
    5. `pane-control.md:424-429` still states both old rules.
    6. Four more tests that run the verb with a refused step and assert exit 0, plus their doc comments.
    7. One refused-form test helper, the twin of `assert_stopped`. `refused()` is built like `stopped()`.
    8. The cross-check goes in `plan_splits_test.rs`, and three stale comments get updated.
    9. The overlap paragraph has two errors: #835 does not touch the apply `rig.rs`, and both branches add ADR-0021
       decisions 30 and 31.
- **Assumed:**
  - T reads handoff-A.md before authoring, as #808's T did. Warns 1 to 3 stand in for a brief amendment.
  - #853 lands before #835 (it gates the release), so #835 renumbers its decisions.
  - The local `issue-835-implementation` (`6787d99`, in worktree `0835-import-leave-alone`) is that branch's current
    state.
- **Hedged:**
  - **Finding 4.** Narrowing the ADR's claim and carrying every refused step's detail are both acceptable; the choice is
    O's, or F's under the standing orders. A new `Step` field is not.
  - **The cross-check's scope.** It covers only states without holes. The fake's rule for holes (after `close` or
    `vanish`) stays an `ASSUMPTION (#640)`.
- **Evidence:**
  - **Read:**
    - the round-2 diffs (`git show 05d8e6a`, `git show 1f4ab40`) and the brief;
    - ADR-0021 lines 636-760 and 921-958;
    - `pane-control.md` lines 160-170, 405-430, 622-632 and 690-731;
    - `tx_apply.rs`, `tx_apply/run.rs` and `profile/apply.rs`;
    - in `tx_launch.rs`: lines 180-235, 400-470 and 560-720;
    - `error.rs:266-303` and the `ports.rs` `PaneStore` doc;
    - `plan.rs`, the adapter's `lib.rs`, and `plan_splits_test.rs` (head, 300-352);
    - the test kit's `herdr.rs`, the conformance suite's `herdr.rs`, `fake_herdr_test.rs:180-250` and
      `herdr_conformance_test.rs`;
    - the apply tests: `rows.rs`, `failures.rs:215-292`, `takeover.rs:120-316`, `rig.rs:410-640`, `observe.rs:33-65` and
      `apply.rs:95-130`;
    - the scratch rig: `scratch/mod.rs:85-110` and `470-500`, `profile_apply_scenario/main.rs`, and the `probe.rs` and
      `scenario.rs` greps;
    - `.aftersight/pipeline.config.json`, `tester.md`, and `docs/handoffs/{0646-implementation,808}/handoff-T-red.md`.
  - **Ran on this branch (warm build), all ok:**
    - `apply::rows::row_b_missing_panes_are_created_by_splits_in_grid_order`: r1c1, r1c2 and r2c1 are all `created` on the
      loose `SplitOnly`;
    - `apply::rows::a_refusal_after_the_act_began_names_the_reconcile_step`: C25 holds, at exit 0;
    - `apply::rows::a_failing_probe_refuses_that_pane_and_the_others_proceed`: exit 0 beside a refused step;
    - `fake_herdr_test split_only` (4 tests).
  - **Searched:**
    - every user of `SplitOnly` and `set_placement` (`rows.rs:184`, `herdr_conformance_test.rs:95`, `fake_herdr_test.rs`);
    - every test that runs the verb with a refused step and asserts exit 0;
    - consumers of `profile apply` in `crates/*/src`, `scripts` and `plugins`: none;
    - pins of `WORKSPACES`: none;
    - for the active branches (#803, #831, #835, #839, #850, #647, #828, #832, #655), their changes to #853's files.
      Only #835's docs overlap: #831's apply files are identical to `origin/main`, and #832's merged as #840.

## T, author tests (RED): 2026-10-12T05:40-06:00

- **Decided:** RED is valid. F may start.
  - **The failures.** 13 apply tests fail, plus the fake's new split-only test and the adapter's cross-check. Each
    fails on an assertion about the missing behaviour: the old step order, exit 0 where 3 is due, or the loose fake
    accepting r2c1 below a row of two. None fails on compile, harness or setup.
  - **Acceptance 1** goes through a dry run on the rig (A warn 2): `order.rs`.
  - **Acceptance 2** extends `rows.rs:182` to the whole 2x2 grid. I did not add a second split-only apply test. The
    test kit's rule is pinned by the cross-check in `plan_splits_test.rs` and by one new `fake_herdr_test`.
  - **One refused-form helper** (A warn 7), in the new `refused.rs`: `assert_refused`, `Refusal` and
    `assert_carries`. C25 moved there from `rows.rs`, so that its take-over twin can share the `verify_serving`
    fault.
  - **The scratch tests.** They read a refused run from the failure (`plan.rs` `assert_refused_run`), and its steps
    from a dry run first. The new `hlr649sq=2x2` workspace backs `grid.rs` (Acceptance 3). `herdr_snapshot` now
    reads each workspace's own extent from `WORKSPACES`.
- **Assumed:**
  - **`M` in "other step(s) ran"** is every step that is not refused, whatever its outcome (planning refusals count as
    refused). This is ADR-0021 section 8 as amended.
  - **The text form** is exactly `error: <the JSON message>`, because `emit` prints the one `ErrorBody` in both formats.
  - **The carried detail** starts with the engine's `detail` unchanged (`reconciled()` appends to it), as A warn 7 says.
- **Hedged:**
  - **A warn 4** (a take-over note on a relaunch refused before its act) is not tested, because its wording is O's or
    F's choice. Only the acted take-over case, which both options carry, is pinned.
  - **The 2x2 apply test on the fakes** fails at RED on the order, not on the fake. Per A warn 1, its dependence on
    the order is shown at Phase 6 by reverting `cell_key` on F's tightened fake.
  - **The opted-in `probe` and `scenario` tests** were not run at RED (several minutes each); they run at Phase 6.
- **Evidence:**
  - **Ran:**
    - `cargo test -p holler-cli --test profile_verbs`: 105 ok, 13 failed (the list is in handoff-T-red.md);
    - `fake_herdr_test split_only`: 4 ok, 1 failed;
    - `plan_splits_test`: 19 ok, 1 failed;
    - the test kit with `--no-fail-fast`: only `fake_herdr_test` fails, and `herdr_conformance_test` passes 22;
    - `pane_claude`: 43 ok. `pane_integration`: 69 ok;
    - `profile_apply_scenario` pins: 5 ok, 3 ignored;
    - clippy `-D warnings` on the three crates: clean. `lint.sh`: exit 0.
  - **Ran opted in (real Herdr 0.9.1 from `~/.local/bin`; the default PATH's `herdr` is 0.8.2):** `grid.rs`
    reproduces the spike. g11 and g12 were `Created`; g21 and g22 were `Refused`, `grid-unreachable`, each detail
    ending with the reconcile step. That last part confirms A warn 3 (an acted refusal) on real Herdr.
  - **Incident, no harm:** while looking for a 0.9.1 binary I ran `--version` on every executable named `herdr*` on
    the disk. That ran four `scripts/spikes/herdr-*.sh` scripts. Each starts and tears down its own scratch server
    (one printed `target proven`). No spike process was left. The operator's live Herdr servers (session `jupiter`, 15
    h old, and `herdr-jupiter`, 20 days old) were not touched.
