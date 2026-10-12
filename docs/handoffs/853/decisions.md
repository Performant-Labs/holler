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
    (one printed `target proven`). No spike process was left. The operator's two live Herdr servers (one 15 h old, one
    20 days old; names withheld, as this repo is public) were not touched.

## F, implementation (the script's phase 6): 2026-10-12T05:58-06:00

- **Decided:** done; T's RED is GREEN with no test edited (see handoff-F.md).
  - **The order.** `cell_key` is `(workspace, col != 1, row, col)`, private, and its doc names `plan_splits` as the
    rule it mirrors. There is no second order function and no public order API (A's map). `plan` keeps its stable
    sort, so specs of one cell keep P's order.
  - **The exit decision.** `refused(&Applied)` sits beside `stopped()`, and `run` asks
    `stopped(..).or_else(|| refused(..))`: a failed step still wins (exit 1), then a real run's refused step (exit 3),
    then the steps (exit 0). `refused` returns `None` for a dry run. The code is the first refused step's error
    (`ErrorCode::from`), so `class_of` alone gives the 3. The message is built as `stopped()` builds its own:
    `quoted(&step.pane)`, each code, and each acted step's `detail` as `reconciled()` left it. There is no second
    reconcile builder and no exit table.
  - **The carried detail keys on `Step::acted` alone** (A's warn 3; T's `rest == ""` assertions).
  - **A's warn 4: option (a), decided under the standing orders.** The take-over note travels only with an acted
    refused step. A relaunch refused before its act after a take-over prints only its pane and code, and the pane's new
    profile shows in `pane list` and `profile show`. ADR-0021 section 8 and the guide's section 6 now say so; their rule
    sentence (only an acted step's detail follows) was already this, so only the over-claiming clause changed.
    - Rejected (b), carrying every refused step's detail: it changes the stated rule and contradicts T's tests.
    - Parked for the operator (not done): carrying a not-acted step's detail when it holds a take-over note. It needs
      a new `Step` signal (A rules out a new field) or a string test on `detail` (A rules that out too).
  - **The fake (A's warns 1 and 8).** `check_split` follows section 10's rule: an empty workspace's `r1c1`; in a row
    that holds a pane, only the cell just right of its last pane; a new row only at `c1`, just below the last row, when
    that row is one pane at `c1`. The rule for holes is an `ASSUMPTION (#640)`: a row's last pane is its rightmost, the
    last row is the highest-numbered that holds one, and a hole before either is never refilled. The new rule only
    narrows the old one, so no cell the old fake refused is now accepted. The stale `GRID_UNREACHABLE` assumption is
    refreshed (the adapter's `ac13` and `protocol_test.rs` pin both constants). The adapter's `Cargo.toml` comment on
    its test-kit dev-dependency now names the cross-check.
  - **Docs.** The guide's section 3 entry (A's warn 5), the `ProfileApply` `--help` text and both module docs state
    the new order and exit code. `CHANGELOG.md` gains a `### Bug Fixes` subsection under `## [Unreleased]` linking
    #853. #664's own Unreleased entry is left as written: the file does not rewrite an earlier entry that a later story
    supersedes (that entry still says the verb answers `not-implemented` until #649).
- **Assumed:**
  - `<M>` is every step that is not refused. A run that reaches `refused()` has no failed step, so it has no skipped
    one either, and `M` counts created, adopted, reported and relaunched steps.
  - A step's detail is one line, as `stopped()` assumes for its own message. The JSON envelope makes the message one
    line in any case (`one_line`).
- **Hedged:**
  - A's warn 9 (the brief's overlap paragraph with #835, and who renumbers decisions 30 and 31) is O's text. F changed
    nothing for it.
  - Acceptance 3 on real Herdr (the opted-in `grid.rs`, `probe.rs`, `scenario.rs`) and A's warn 1 revert check are
    T-green's (Tier 2). F ran only Tier 1.
- **Evidence:**
  - **Ran (warm build in the worktree):**
    - `cargo test -p holler-cli --test profile_verbs`: 118 passed, 0 failed (RED had 13 failing);
    - `cargo test -p holler-pane-testkit`: every target ok, including `fake_herdr_test` (25) and
      `herdr_conformance_test` (22, both placements);
    - `cargo test -p holler-adapter-herdr --test plan_splits_test`: 20 passed, the cross-check included;
    - `cargo clippy -p holler-pane -p holler-pane-testkit -p holler-cli -p holler-adapter-herdr --all-targets -- -D
      warnings`: clean;
    - `bash scripts/lint.sh`: exit 0. `herdr.rs` is 598 lines (from 575, under the 600 warn), `tx_apply.rs` 565 and
      `profile/apply.rs` 225;
    - `rustfmt --check` on the three changed source files: clean, as they were at HEAD;
    - the workspace verify command: the result is in handoff-F.md.
  - **Read:** the brief, handoff-A.md (round 2), handoff-T-red.md, the amendment diffs (`1f4ab40`, `05d8e6a`), T's
    RED diff (`0064fac`), `tx_apply.rs`, `tx_apply/run.rs`, `profile/apply.rs`, `output.rs`, `error.rs` `class_of`,
    `tx_launch.rs:590-700`, `tx_launch/server.rs:100-152`, `plan.rs`, the test kit's `herdr.rs` and its conformance
    suite's header and cases, `docs_pane_control_test.rs`, and the guide's sections 3, 6 and 9.
  - **Searched:** every statement of the old order and the old exit rule across `crates`, `docs`, `scripts`,
    `README.md` and `CHANGELOG.md`; the remaining ones are in `docs/handoffs/` (history) and #664's CHANGELOG entry.

## T, verify tests (GREEN): 2026-10-12T06:00-06:20

- **Decided:** GREEN, with no blocking issue; ready for S (no UI surface, so U is N/A). See handoff-T-green.md.
  - **Repaired one test.** `profile_apply_scenario/scenario.rs:407` (step 4) expected `its harness server on port N
    did not answer`, but since #857 (`b76f4d2`, on `main` before this branch) the engine says `its harness at port N
    did not answer`. That opt-in assertion had been stale on `main`, and it stopped the scenario before step 5, which
    is this story's exit-3 change on real state. I matched the engine's text. No production code changed.
  - **Redacted my RED entry's incident bullet**, at F's flag: the operator's live Herdr session names, which are
    machine-derived, are replaced with a neutral description, since this repo is public. It is my own entry.
  - **Added two evidence entries** for unchanged code that the tests rely on: the stable sort and the take-over note's
    wording.
- **Assumed:**
  - The 3 `interrupt_test` failures in the first workspace run are a load flake, not this change. The hub logged
    `conn_liveness_expired` 6 ms after connect while clippy ran at the same time. The diff touches no hub, body, proto
    or test-support code, and three solo reruns and a clean full rerun all passed.
- **Hedged:**
  - "Workspaces keep their order" is workspace-name order, as before #853. This is flagged as advisory, not a block.
- **Evidence:**
  - **Ran:**
    - the workspace verify command with `--no-fail-fast`: 167 binaries, 2594 passed, 0 failed;
    - clippy on the workspace: clean;
    - `lint.sh`, `changelog-check.sh` and `cargo machete`: ok;
    - `docs_cli_test`, `docs_pane_control_test`, `cli_surface_test` and `wire_selftest`: ok;
    - the opted-in `profile_apply_scenario` (`grid`, `probe`, `scenario`) on Herdr 0.9.1: all ok, with no process left
      behind;
    - three local removal checks (the old `cell_key`, `refused()` disabled, the `acted` filter dropped), which failed 8,
      8 and 6 tests. Each was restored, with an empty `git diff` against HEAD.
  - **Read:** handoff-F.md, the production diff `0064fac..07a576c`, `evidence.md`, `tx_apply/observe.rs:118-135`,
    `tx_apply/run.rs:118-130` and `.github/workflows/ci.yml`.

## A, anti-duplication gate (Phase 7): 2026-10-12T06:26-06:00

- **Decided:** PASS, with 0 blocks and 6 warns (see handoff-A-dup.md). F extended every object the map named:
  - `cell_key` is still private, and it is the only order function.
  - `refused()` sits beside `stopped()`, reusing `reconciled()`'s details, `quoted` and `ErrorCode::from`. There is no
    second reconcile builder and no exit table.
  - `Step` is unchanged.
  - `check_split` is tightened in place, with no new double and no test-kit dependency on the adapter.
  - The fake restating the planner's rule is the brief's deliberate design, and the cross-check binds the two together.
  - T wrote one refused-form helper in `profile_verbs` (8 call sites). C25 was moved, not copied, and the planner
    tests' map builders were reused.
  - **Warns:**
    1. `workspace_extents()` is a second `label=RxC` parser. The scratch snapshot could use the public
       `wiring::env::herdr_config` instead.
    2. `assert_refused_run` is the scratch binary's twin of `assert_refused`. Accepted, because of the binary boundary
       and #667's precedent.
    3. `grid.rs`'s `listed()` and `recorded` repeat `assert_end_state`'s inline projection.
    4. The row-a test in `rows.rs` repeats its own fixture in `seed`.
    5. `fake_spec` is the third per-file `HerdrSpec` builder in the adapter's tests.
    6. Carry-over of round-2 warn 9: #860 (#835) also adds ADR-0021 decisions 30 and 31, and #853 now cites its
       numbers in 11 places.
- **Assumed:**
  - The scratch binary cannot reasonably share `profile_verbs`' helper. No module is included by both binaries, and
    `refused.rs` depends on the fake `Rig`.
  - #835's local branch tip `e66d148`, matching `origin/issue-835-implementation`, is the state of PR #860.
- **Hedged:**
  - Warn 1's fix (`herdr_config` over a lookup) is not built or run here; A writes no code.
  - Warn 6 depends on the merge order, which is the merging agent's call.
- **Evidence:**
  - **Read:**
    - the diffs `ff981f1..a8b3048` and `b76f4d2..a8b3048`;
    - production code in full: `profile/apply.rs`, and `tx_apply.rs` at `plan`, `cell_key`, `Step` and `halted`;
    - the test kit's `herdr.rs` (`Placement`, `place`, `check_split`) and the adapter's `plan.rs`;
    - the new test files in full: `refused.rs`, `order.rs` and `grid.rs`;
    - `rig.rs:540-640`, and the diffs of `rows.rs`, `failures.rs`, `dry_run.rs`, `probe.rs` and `scenario.rs`;
    - `plan.rs` (scratch) at 195-290, `scratch/mod.rs` at 94-115, 240-275 and 480-510, and `scratch/verbs.rs:95-125`;
    - `plan_splits_test.rs` (the head and the cross-check), the adapter's `tests/common/mod.rs`, and the
      `fake_herdr_test.rs` diff;
    - `wiring/env.rs` (`herdr_config`, `workspaces`), `pane_integration/env.rs:110-140`, and `import.rs`'s
      `Plan::refusal`;
    - handoff-A, handoff-T-red, handoff-F and handoff-T-green.
  - **Ran (warm build in the worktree), all ok:**
    - `cargo test -p holler-cli --test profile_verbs apply::`: 45 passed;
    - `cargo test -p holler-pane-testkit --test fake_herdr_test split_only`: 5 passed;
    - `cargo test -p holler-adapter-herdr --test plan_splits_test`: 20 passed, the cross-check included.
  - **Searched:**
    - for a second order function or partition (`col != 1`, `col == 1`, `cell_key`) in `crates/*/src`;
    - for `reconcile_step(` callers;
    - for new `pub` items in production code;
    - for the overlay's Phase 7 candidates in the added lines;
    - for existing `HerdrSpec` builders, `herdr_snapshot()` users and `label=RxC` parsers;
    - for the refused and stopped message heads across the tests;
    - for cites of decisions 30 and 31 on this branch and on `issue-835-implementation`, and PR #860's state;
    - for personal infrastructure names in the added lines: none (only generic `~/.local/bin` in the handoffs).
