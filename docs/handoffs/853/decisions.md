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

## S, spec audit: 2026-10-12T06:45-06:00 (MDT)

- **Decided:** REWORK (production), one item. See `handoff-S.md`.
  - **On the branch, all is met.** Each of the six acceptance criteria has a proving test that asserts behaviour, and
    T-green's removal checks show they fail without the change. The branch is spec-compliant, and the quality audit is
    clean.
  - **The item: `origin/main` moved during the run.**
    - PR #860 (#835) merged at 06:29 MDT as `4e691d9`, three minutes after A-dup's entry.
    - Its ADR-0021 decisions 30 and 31 collide with #853's. The merge is textually clean (`git merge-tree` exits 0), so
      the PR would show as mergeable and CI would pass, but ADR-0021 would hold 30, 31, 30 and 31.
    - The fix:
      - merge `origin/main` (not a rebase: `05d8e6a` is already on origin, and the push must fast-forward);
      - renumber #853's decisions to 32 and 33;
      - update 13 cites. A-dup counted 11; `refused.rs:31` and `:222` also cite "Decision 31". Two of the 13 are in the
        `profile/apply.rs` doc comments.
      - re-run the docs tests and the verify command on the merged tree.
  - **Accepted:**
    - F's option (a) for A's warn 4: A allowed it, and F recorded it here.
    - The brief's ruling that `cli-surface.txt` does not change: the fixture holds no exit codes (checked).
    - T-green's `scenario.rs:407` wording repair: stale on `main` since #857, and needed for the scenario to reach step 5.
    - F leaving #664's CHANGELOG entry as written: the repo keeps `[Unreleased]` entries as written.
- **Assumed:**
  - T-green's GREEN on `07a576c` holds for `c1f7deb`, which adds handoffs only. I re-ran no build or test, as the role
    says.
  - A textually clean merge has no semantic conflict beyond the numbering. #860 shares only four docs files with this
    branch. T-green re-verifies on the merged tree.
- **Hedged:**
  - **Why REWORK, and not PASS with a note for the merger.** Nothing automatic catches the duplicate numbers. The fix
    touches a production file, so it should pass the gates. #835's S required a merge of `origin/main` through a REWORK
    the same way.
  - **The unpushed history.** T-red's commit added the operator's machine-derived Herdr session names, and T-green
    redacted them in the tree. They stay in the unpushed commits, so this is an advisory for the operator before the
    push, not a REWORK item.
    - Nothing new is disclosed: the same name is already public on `main` (#834's handoff) and in issue #655.
    - #835's S treated its own unpushed-history item the same way.
- **Evidence:**
  - **Live:**
    - `git fetch` at the start of the audit, then the shared `origin/main` at `4e691d9`, confirmed with
      `git ls-remote origin` (the branch on origin is `05d8e6a`);
    - `gh pr view 860`: merged at 06:29 MDT;
    - `gh issue view 853`: the body and two comments, nothing new since 04:12 MDT;
    - `gh pr list --head issue-853-implementation`: none;
    - `git merge-tree --write-tree HEAD origin/main`: clean, and the merged ADR-0021 lists 29, 30, 31, 30, 31.
  - **Read:**
    - the brief, every handoff, this journal and `evidence.md`;
    - the production diff in full;
    - `order.rs`, `refused.rs` and `grid.rs` in full, and the other test diffs;
    - the planner (`plan.rs:75-216`);
    - `tx_apply.rs` (`plan`, `choose`, `dry`, `halted`) and `run.rs` (`step`);
    - `scratch/mod.rs` (`start`, `herdr_snapshot`);
    - the ADR, guide and CHANGELOG diffs;
    - `CONTRIBUTING.md` and `ci.yml`;
    - the release's workflow script: S's routing and the PR phase's push;
    - #835's `handoff-S.md` and journal, as precedent.
  - **Greps:**
    - the old order and the old exit rule, over `crates`, `docs` and `CHANGELOG.md`;
    - the cites of decisions 30 and 31 on the branch, on `main` and in the merged tree;
    - `unwrap`, `expect`, `panic` and `#[allow` in the added `src` lines;
    - `wc -l` of every touched file;
    - private names, IPs, home paths and secrets, in the diff and in each branch commit;
    - consumers of `profile apply` in `src`, `scripts` and `plugins`: none;
    - other tests that run `profile apply` (`pane_claude/mixed.rs`).

## F, S's REWORK (the script's phase 6, cycle 2): merge `origin/main`, renumber #853's decisions to 32 and 33: 2026-10-12T06:47-06:00 (MDT)

- **Decided:** S's item 1 is done for production code and docs (`handoff-F.md`, which replaces cycle 1's handoff at
  `07a576c`). `archChanged: false`.
  - **The merge.** I ran `git merge --no-ff --no-commit origin/main`. `origin/main` was at `4e691d9`, #835's PR #860,
    merged at 06:29 MDT.
    - It was clean. Git auto-merged `CHANGELOG.md`, ADR-0003, ADR-0021 and the guide.
    - The staged tree equalled `git merge-tree --write-tree 21b29a0 4e691d9` (`c2941f8`).
    - F does not commit. `MERGE_HEAD` stays for the script's phase commit, which concludes the merge as a two-parent
      commit, as #835's `526bbd3` was.
    - It is not a rebase: `05d8e6a` is already on origin, and the PR phase's push must fast-forward.
  - **The list.** #853's `30.` and `31.` are now `32.` and `33.`, and the list reads 1 to 33, each once and in order.
    The blank line the merge left between #835's 31 and #853's pair is gone, so the list is tight again, as on `main`.
  - **The cites in production code and docs (6 of S's 13):**
    - ADR-0021 `:742` (now 32), `:744` (33) and `:860`, section 9's row (33);
    - `CHANGELOG.md:615` ("(32 and 33)");
    - `profile/apply.rs:12` and `:150` (33).
  - **The other 7 are test comments, which are T-green's,** and handoff-F lists them for T. They are `order.rs:2,28,95`
    (30 becomes 32), `refused.rs:2,31,222` (31 becomes 33) and `plan.rs:198` (31 becomes 33). F edits no test.
  - **S's advisory 2: done.** ADR-0021 `:713` now reads "0 for a dry run that does not fail, or a real run with no
    failed or refused step, which prints its steps", in S's words. It now agrees with the section's own dry-run
    paragraph, the guide and the code.
  - **Not taken, by owner:** advisory 1, the history rewrite (the operator's; F rewrites no history); 3 (#664's entry
    stays as written); 4 (parked); 5 and 6 (T's); 7 (the merging agent's); 8 (no edit to the test kit's `herdr.rs`).
- **Assumed:**
  - The phase commit (`git add -A`, then `git commit -m ... --allow-empty`) concludes the merge in progress, as it did
    for #835. The hooks allow it:
    - `commit-msg` accepts the script's Conventional subject;
    - `prepare-commit-msg` adds the trailer for a `-m` message;
    - gitleaks scans the staged merge, which is `main`'s reviewed content plus this cycle's text.
  - `origin/main` stays at `4e691d9` until that commit. I fetched before the merge and again at 07:06 MDT, and
    `git ls-remote` agreed. If it moves, the next phase merges again.
- **Hedged:**
  - **Advisory 2 and the blank line go past item 1's letter.** S offered advisory 2 while F was in the file, and the
    text is S's own. The blank line sits inside the list S named. Nothing else outside the item changed.
  - **Advisory 1 is now harder.** With the merge staged on top of `0064fac`, rewriting that commit means replaying the
    merge (`git rebase --rebase-merges` from `05d8e6a`) or redoing it. S judged that it discloses nothing new.
  - **A flake on `main`, not this change.** The first workspace run failed 1 test of 2,606:
    `holler-hub` `profile_rename_faults_test::a_failed_profile_save_puts_the_pane_file_back_and_the_same_rename_then_succeeds`
    (#665), at its `assert_ne!` on the pane file's inode (`:250`).
    - Saving the file back writes a new file, and ext4 can hand it the inode number the first replace just freed.
      Run alone, the test failed 1 time in 20.
    - `crates/holler-hub` is byte-identical to `main` and to `21b29a0`.
    - No issue exists for it (searched). Filing one is for the main session, since it is an outward action.
    - The rerun's one failure is #847's open load flake (`remote_admin_test`). It passed 6 times in 6 alone.
- **Evidence:**
  - **Ran on the merged tree (warm build):**
    - the workspace verify command with `--no-fail-fast`: 167 binaries, 2,605 passed, 1 failed (the flake above), 37
      ignored;
    - the rerun: 2,605 passed and 1 failed, a different test this time. It was `remote_admin_test::
      remote_admin_traffic_never_moves_the_bodys_last_seen`, the load flake open as #847 (`last_seen` moved 1 s in its 2 s
      window). The inode test passed. Every test passed in at least one of the two full runs;
    - `profile_rename_faults_test` alone, 20 times: 19 passed, 1 failed;
    - `remote_admin_test` alone, 6 times: 6 passed. The file, `holler-hub`, `holler-body` and `holler-proto` are
      identical to `main` and to `21b29a0`;
    - from the first run: `docs_pane_control_test` 19 (18 before #835's case), `docs_cli_test` 3, `cli_surface_test` 3,
      `profile_verbs` 118, `fake_herdr_test` 25, `herdr_conformance_test` 22, `plan_splits_test` 20, `pane_verbs` 347,
      `wire_selftest` 3;
    - `cargo clippy --workspace --all-targets -- -D warnings`: exit 0, with `holler-cli` re-checked by touching its
      sources;
    - `bash scripts/lint.sh`: exit 0, with every warning on a file this cycle does not touch;
    - `bash scripts/changelog-check.sh`: ok. `cargo machete`: clean. `git diff --check` and `--cached --check`: clean;
    - `rustfmt --edition 2021 --check` on `profile/apply.rs`: clean.
  - **Checks:**
    - S's grep finds only the 7 test comments.
    - The decision list holds 1 to 33, each once and in order.
    - Against `main`, the PR diff covers the same 23 files outside `docs/handoffs/` as before the merge, with the same
      line counts. The only differences are this cycle's edits.
  - **Read:**
    - `handoff-S.md`, `handoff-A-dup.md`, `handoff-T-green.md`, this journal, `evidence.md` and the brief;
    - #835's journal on `main`, for the `--no-commit` merge precedent;
    - the script's `buildPhaseCommitTaskPrompt`, and its routing after an S REWORK (F, then T-green);
    - `.githooks/{pre-commit,commit-msg,prepare-commit-msg}`;
    - the merged ADR-0021 list and cites, and the guide's diff against `main`.

## T, verify tests (GREEN), cycle 2, after S's REWORK: 2026-10-12T07:15-06:00 (MDT)

- **Decided:** PASS. I renumbered the 7 test comments that F flagged under S's item 1 (c): `order.rs:2`, `:28` and `:95`
  (30 becomes 32); `refused.rs:2`, `:31` and `:222` (31 becomes 33); and `profile_apply_scenario/plan.rs:198` (31
  becomes 33). They are comments only, with no assertion or test name changed. S's two done-checks now hold, and the
  merged tree is GREEN.
- **Assumed:**
  - Cycle 1's removal checks still stand without a re-run. The code they reverted (`cell_key`, `refused()`, the `acted`
    filter, and the test kit's `check_split`) is byte-identical between `c1f7deb` and `0330fa0`. The only change in
    those files is two doc-comment lines in `profile/apply.rs`.
  - The opt-in scratch set (`grid`, `probe`, `scenario`) did not need a re-run on Herdr 0.9.1, for the same reason.
- **Hedged:**
  - I took none of S's optional test-side advisories 5 and 6, and I did not reformat `refused.rs`'s pre-existing long
    lines. S's REWORK says the next S passes once item 1 is done and nothing else has changed. rustfmt is not a gate:
    neither CI nor `lint.sh` runs it, and `cargo fmt --check` flags 242 files across the workspace.
- **Evidence:**
  - `git grep -n -i -E "decision(s)? (30|31)\b|\(30 and 31\)" -- crates docs/adr docs/pane-control.md CHANGELOG.md`:
    no match.
  - ADR-0021's decision list reads 1 to 33, each once.
  - `HEAD` `0330fa0` has the parents `21b29a0` and `4e691d9`, and `origin/main` is still `4e691d9`.
  - The verify command with `--no-fail-fast`: 167 binaries, 2,606 passed, 0 failed, 37 ignored. That includes
    `profile_verbs` 118, `docs_pane_control_test` 19, `docs_cli_test` 3, `cli_surface_test` 3, `wire_selftest` 3,
    `fake_herdr_test` 25, `herdr_conformance_test` 22 and `plan_splits_test` 20.
  - Clippy `-D warnings`: exit 0. `lint.sh`: exit 0. `changelog-check`: ok. `cargo machete`: clean.
    `git diff --check`: clean.

## A, anti-duplication gate (Phase 7), cycle 2, after S's REWORK: 2026-10-12T07:33-06:00 (MDT)

- **Decided:** PASS, with 0 blocks and 6 warns (see handoff-A-dup.md, which replaces cycle 1's).
  - This cycle's own edits are text only. F renumbered 6 cites and the two decisions, and applied S's advisory-2
    sentence. T renumbered 7 test comments. Nothing was added, so nothing is duplicated.
  - #853's code is byte-identical to cycle 1's, apart from 2 doc-comment lines.
  - The merge of #835 overlaps #853 only in four docs, at different paragraphs. #835's test helpers (import's harness
    detection, `LostChain`, `run_with_host`, the `pane list` chain pin) repeat none of #853's, and #835 adds no config
    builder, refusal helper or exit decision.
  - Against the new `main`, the PR diff is the same 23 files with the same line counts, except ADR-0021 (one line
    fewer).
  - Cycle 1's warn 6 (decision numbers) is resolved.
  - **Warns:**
    1. New, but from the amendment (`1f4ab40`), not this cycle: ADR-0021 `:765` cites "section 14" for `pane import`'s
       dry-run rule, which is in section 13. It is a one-word doc fix, and no test pins it.
    2. to 6. Cycle 1's warns 1 to 5, unchanged. #835 touched none of their files.
- **Assumed:**
  - The clean merge tree `c2941f8` (`git merge-tree --write-tree 21b29a0 4e691d9`) is the baseline for the merge's
    content. Diffed against `0330fa0`, it differs only in `CHANGELOG.md`, `profile/apply.rs`, ADR-0021 and three
    handoff files.
  - `origin/main` is still `4e691d9`. I checked with `git ls-remote`, not a fetch.
- **Hedged:**
  - I ran no tests. No code changed this cycle, and T-green's full run on the merged tree (2,606 passed) covers it.
  - Warn 1's fix is a doc edit that S's REWORK did not ask for. Whether it rides this PR is the next phase's call.
- **Evidence:**
  - **Read:**
    - handoff-S, handoff-F and handoff-T-green (cycle 2), cycle 1's handoff-A-dup, and the brief;
    - `profile/apply.rs` in full, and `tx_apply.rs:338-363`;
    - ADR-0021 `:706-770` and `:1580-1610`, and its headings on the merged tree and on `b76f4d2`;
    - the guide's `profile apply` reference (`:683-745`) and the merged cutover steps;
    - #835's diffs of `pane/import.rs`, `pane_integration/observe.rs`, `ports.rs` (the method lines) and ADR-0003, and
      its guide hunks.
  - **Ran (read-only git and one check):**
    - `git merge-tree --write-tree 21b29a0 4e691d9`, which gives `c2941f8`;
    - `git diff c2941f8 0330fa0`, `git diff 0330fa0 c39699c`, and `git diff c1f7deb HEAD` over #853's code;
    - `git diff --numstat`, before the merge against after;
    - `git ls-remote origin refs/heads/main`;
    - `scripts/changelog-check.sh`: ok.
  - **Searched:**
    - for new `fn`, `struct` and `const` definitions in both branches' test diffs;
    - for Herdr-config builders and `label=RxC` parsers on the merged tree;
    - for old-order phrases, and for "refused ... exit 0" statements, on the merged tree;
    - with S's decision-number grep (no match), and for duplicate numbers in ADR-0021's list (none);
    - for every "section 14" cite in ADR-0021, and for tests that pin `:765`'s text (none);
    - for the overlay's Phase 7 candidates in the PR's added crate lines (none);
    - for private-name patterns in this cycle's added lines (none).
