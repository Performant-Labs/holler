[S]

# Handoff-S: #853 - `profile apply` creates column 1 first, and a real run with a refused step exits 3 (spec audit)

**Date:** 2026-10-12, 06:45 MDT
**Branch:** `issue-853-implementation` at `c1f7deb` (A-dup's PASS).
- `origin/issue-853-implementation` is at `05d8e6a`, so the five phase commits from `ff981f1` on are not pushed.
- No PR exists: `gh pr list --head issue-853-implementation --state all` is empty.

**`origin/main` moved during the run.** PR #860 (#835) merged at 06:29 MDT as `4e691d9`, three minutes after A-dup's
entry. It is not an ancestor of `HEAD`, whose base is `b76f4d2`. `git merge-tree --write-tree HEAD origin/main` merges
with no conflict (`CHANGELOG.md`, ADR-0003, ADR-0021 and the guide auto-merge), but the result is wrong. See the Verdict.

**Issue:** [#853](https://github.com/Performant-Labs/holler/issues/853), OPEN, read live at 06:38 MDT. The body and its two
comments (the spike result, and the operator's exit-code decision of 04:12 MDT) are the ones the brief quotes and
follows. Nothing is new since.

**Brief:** `docs/handoffs/853-brief.md`, round 2 (`05d8e6a`).

**Handoffs reviewed:**
- `handoff-A.md`: round 2, PASS;
- `handoff-T-red.md`;
- `handoff-F.md`;
- `handoff-T-green.md`: PASS;
- `handoff-A-dup.md`: PASS;
- `decisions.md` and `evidence.md`.

**Diff audited:** `git diff origin/main...HEAD` (merge base `b76f4d2`). Outside `docs/handoffs/` it is 23 files, with 1,090
lines added and 243 removed.
- Production, read in full: `tx_apply.rs` (`plan`, `cell_key`, `choose`, `Planned::dry` and `halted`), `tx_apply/run.rs`
  (`step`), `profile/apply.rs`, the test kit's `herdr.rs` (`place`, `check_split`, `last_row`, `end_of`), and the adapter's
  `Cargo.toml`.
- Tests: `order.rs`, `refused.rs` and `grid.rs` in full. Every changed test file by its diff, plus `rows.rs:1-260` and
  `dry_run.rs:60-110` in full.
- Docs: the ADR-0021, ADR-0003, guide and CHANGELOG diffs.
- Read for checking: the planner (`plan.rs:75-216`) and `scratch/mod.rs` (`start`, `herdr_snapshot`).

## A precondition

Met. `handoff-A.md` round 2 is PASS, with 0 blocks and 9 warns. The anti-duplication gate (`handoff-A-dup.md`) is also
PASS, with 0 blocks and 6 warns.

## T precondition

Met. `handoff-T-green.md` has no blocking issue, and RED then GREEN is recorded.

- **RED** (`handoff-T-red.md`):
  - `profile_verbs`: 105 passed and 13 failed. Each failure is an assertion about the missing behaviour: the old order,
    exit 0 where 3 is due, or the loose fake accepting `r2c1`.
  - The fake's new split-only test and the adapter's cross-check also failed.
  - The opted-in `grid.rs` failed on Herdr 0.9.1 exactly as the spike did.
- **GREEN:**
  - `profile_verbs`: 118 passed, 0 failed;
  - the test kit: `fake_herdr_test` 25 and `herdr_conformance_test` 22;
  - `plan_splits_test`: 20;
  - the workspace verify command: 2,594 passed, 0 failed.
- **Removal checks.** Three local reverts each made tests fail: the old key failed 8, `refused()` disabled failed 8, and
  the `acted` filter dropped failed 6.
- **Opted in.** `grid`, `probe` and `scenario` passed on Herdr 0.9.1.

## Acceptance criteria

| # | Criterion (the brief's "Acceptance, as it can be met") | Proving test or evidence | Status |
|---|---|---|---|
| 1 | The order, pure. Cells in any order, in one workspace and in several, come out column 1 by row, then by row and column. Workspaces keep their order, and two specs of one cell keep P's order. | `profile_verbs/apply/order.rs` `steps_take_each_workspaces_column_1_by_row_then_the_rest_by_row_and_column`: a dry run on the rig, with P scrambled across `main` and `side`, and `dup-b` listed before `dup-a`. It asserts the step order the engine answers, not the key. It fails with the old key (T-green's removal check). | Met |
| 2 | The fix on the fakes. `SplitOnly` follows section 10's rule; the fake's own tests are updated; there is a cross-check against `plan_splits`; a four-pane 2x2 apply on `SplitOnly` creates all four and fails with the old order. No new double, and no wider `holler-pane` API. | `holler-pane-testkit/src/herdr.rs` `check_split`. `fake_herdr_test.rs` `split_only_mode_adds_a_row_only_below_a_last_row_of_one_pane`, plus the updated `split_only_mode_refuses_an_absolute_placement`. `plan_splits_test.rs` `the_split_only_fake_accepts_a_cell_exactly_when_the_planner_plans_it`: 40 grids with no hole, every free cell of the 4-by-4 window, including the out-of-range ring. `rows.rs` `row_b_missing_panes_are_created_by_splits_in_grid_order`: the 2x2 on `SplitOnly`, which fails with the old key. No new double; `cell_key` stays private. | Met |
| 3 | The fix on the scratch set. Opt-in and `#[ignore]`; an empty 2x2 workspace; all four created and listed at their cells; a second apply adopts all four; `2x2` in `WORKSPACES`; the protocol-22 message. | `profile_apply_scenario/grid.rs` `apply_builds_a_whole_2x2_grid_from_an_empty_workspace`. `WORKSPACES` gains `hlr649sq=2x2`. `herdr_snapshot`'s connect message names protocol 22, and it is `grid.rs`'s first adapter connect. It failed at RED as the spike did and passed at GREEN. | Met |
| 4 | The limit and the exit code. `r2c1` below a live two-pane row is refused `grid-unreachable` and the run exits 3, with the failure envelope, its code and the one-line message; text is one `error:` line, and the others ran. A failed step beside a refused one exits 1. A reported step alone exits 0. A dry run that plans a refusal exits 0 with its plan. An acted refusal carries its detail, ending with the reconcile step, in both formats, and a take-over note is carried the same way. | `order.rs` `a_row_below_a_row_of_two_live_panes_is_refused_and_the_run_exits_3`. `refused.rs` `a_failed_step_beside_a_refused_one_still_exits_1`. `rows.rs` `row_d_a_healthy_pane_that_differs_is_reported_and_never_moved` (`data_of` asserts exit 0). `dry_run.rs` `a_dry_run_prints_its_plan_as_the_steps_object` (rows a and e refused, exit 0 in both formats). `refused.rs` `a_refusal_after_the_act_began_carries_its_detail_and_the_reconcile_step` (a create and a relaunch) and `an_acted_refusal_after_a_take_over_carries_its_note`. `assert_refused` checks the JSON and the text form on fresh rigs. On real state: `probe.rs` (i) and (ii), and `scenario.rs` step 5. | Met |
| 5 | The old order is unpinned everywhere, each test still proves what it proved, and `fake_herdr_test.rs`'s snapshot test is left alone. | The rig's `EVERY_ROW`; in `rows.rs`, the every-row, row a, probe, usage, apply-twice and row b cases; `failures.rs` entry 33; the `dry_run.rs` docs; `probe.rs`; `scenario.rs`; `ac12`'s comment. `snapshot_lists_panes_by_workspace_then_row_then_col` is untouched. My grep finds the old order only where the text calls it the old one (`tx_apply.rs:358`, ADR-0021 decision 30's text). | Met |
| 6 | The text. `tx_apply.rs`'s docs and `cell_key`'s doc, which names `plan_splits`, with the key private; `profile/apply.rs`'s module doc and the `ProfileApply` clap doc; ADR-0021, ADR-0003, the guide, and a `CHANGELOG.md` Bug Fixes entry that links #853 and states both changes. | All are in the diff. The guide's section 3 entry is updated too (A's warn 5). | Met on the branch. The decision numbers break on the merged tree: Verdict item 1. |

## Spec compliance

- **The order.** `cell_key` is `(workspace, col != 1, row, col)`. It stays private, and it is the planner's own partition
  (`plan.rs:98-100`). The sort is still the stable `sort_by`. There is no second order function.
- **The exit decision.**
  - `run` asks `stopped(..).or_else(|| refused(..))`, so the exit is 1, else 3, else 0.
  - `refused()` returns `None` for a dry run.
  - The code is the first refused step's, so `class_of` gives the 3.
  - I checked the premise in `halted` (`tx_apply.rs:528-549`). Every `refused` step carries an error, and that error is
    refusal-class. This holds for planning refusals too: `Row::Refuse` goes through `Planned::dry`, which calls `halted`
    (`run.rs:36`). So `refused()`'s `filter_map` drops no refused step.
- **The message** is the one ADR-0021 section 8 and the brief state. The carried detail keys on `Step::acted` alone. An
  acted step's detail already ends with the reconcile step, because `reconciled()` runs first.
- **What does not change does not change.** That covers the dry run, the step shape, the wire, the records, every code,
  `class_of`, the planner, `HerdrPort` and `cli-surface.txt`.
- **A's warn 4, option (a).** F narrowed the over-claiming take-over-note clause in ADR-0021 and the guide. A allowed
  either option "under the standing orders", and F recorded the choice in decisions.md, so this is not a silent
  deviation. Its follow-up is parked (Advisory 4).
- **The brief against the issue.**
  - The operator's comment lists `cli-surface.txt` among the places to record the exit code. The brief rules it out,
    because the fixture pins argument forms only. I checked: it holds no exit codes. The ruling is reasoned, recorded and
    right.
  - The comment's "the `--format=json` envelope reports the same refused steps with `ok` reflecting the failure the way
    the other verbs do" is met. The failure envelope has `ok` false, and its message names every refused step with its
    code. Decision 31 records the operator's choice of this one-line form over an envelope change.
- **One gap, from `main` moving.** Decisions 30 and 31 now collide with #835's on `main`. Verdict item 1.

## Quality audit

- **Correctness and failure handling.**
  - `steps.len() - refusals.len()` cannot underflow, because the refusals are a subset of the steps.
  - An acted refused step always has a detail, because `reconciled()` sets one.
  - The fake's rule equals the planner's single-cell rule for every grid with no hole:
    - a row that holds a pane grows only just right of its last pane;
    - a new row starts only at column 1, just below the last row, when that row is one pane.
  - The cross-check proves the fake and the planner agree up to 3 by 3. The rule for holes is an `ASSUMPTION (#640)`.
  - `checked_sub` avoids a `u16` wrap.
  - No error path, lock or write path changed.
- **Build guards.**
  - No `unwrap`, `expect`, `panic` or `#[allow` is added under `crates/*/src`.
  - Every touched file is under 900 lines. The largest are `rig.rs` (776), the test kit's `herdr.rs` (598), `scenario.rs`
    (570) and `tx_apply.rs` (565).
  - Clippy `-D warnings` is clean on the workspace (T-green).
- **Protocol.** Nothing changes: no wire field, no golden file, and no error-code table entry.
- **Tests.**
  - Each sits at the cheapest tier that shows its behaviour: a dry run on the rig for the order, the fake's own test and an
    adapter dev-test for the rule, and scratch only for real Herdr.
  - No sleep is added.
  - There is one refused-form helper per test binary (`assert_refused`, and the scratch binary's `assert_refused_run`).
    A-dup accepted both.
  - RED-first evidence is in `handoff-T-red.md`.
- **Documentation.**
  - `CHANGELOG.md` has one `### Bug Fixes` under `## [Unreleased]`, linking #853 and #633, with both changes and the
    behaviour change in bold.
  - The guide's sections 3 and 6 are updated, as are ADR-0021 sections 8 and 9 and its decisions, and ADR-0003's row.
  - There is no new log event, flag or protocol field.
- **Public-repository privacy.**
  - `git diff origin/main...HEAD` is clean. I grepped it for private host, machine and account names, tailnet names,
    home-directory paths, non-loopback IPs, e-mail addresses and secrets. The only hits are generic: `~/.local/bin` and
    the word "linuxbrew" in the handoffs.
  - The unpushed branch history is a different matter: Advisory 1.
- **Commit hygiene.**
  - All 8 branch commits have Conventional subjects and a `Co-Authored-By` trailer. None has a session link, which is
    the script's own phase-commit form; #835's squash on `main` has the same form.
  - The PR does not exist yet: Advisory 7.

## Scope check

F delivered exactly the brief's scope: three small production edits (the key, the exit decision, the fake's rule), their
docs, and a comment in the adapter's `Cargo.toml`. Under-delivery: none. Over-delivery: none.

One test edit is outside the brief, and it was necessary and explained. In `scenario.rs:407` (step 4), T-green matched
the engine's wording, which has been stale on `main` since #857 (`tx_apply/observe.rs:133`, checked). Without it the
opted-in scenario stops before step 5, which is this story's exit-3 change on real state. `herdr_snapshot`'s
per-workspace extent is the brief's own "parse it from `WORKSPACES`".

## Verdict

**REWORK** (production), one item. The list is final. The next S passes once this item is done and nothing else has
changed.

1. **Merge `origin/main` and renumber #853's two ADR-0021 decisions from 30 and 31 to 32 and 33.**
   - **Why.** A's round-2 warn 9 and A-dup's warn 6 set the rule, and the brief's overlap paragraph backs it: whichever
     of #835 and #853 lands second merges `main` in and renumbers its pair. #835 landed first.
     - `main`'s ADR-0021 now has decision 30 ("`pane import` leaves a pane whose tmux session runs Claude Code alone")
       and decision 31 ("`HerdrPort::observe_pane` is required").
     - The merge is textually clean, so GitHub would show the PR as mergeable and CI would pass. But the squash would
       leave two decisions numbered 30 and two numbered 31 (lines 1584-1598 of the merged file).
     - Every #853 cite would then point at #835's decisions. That includes two doc comments in production code:
       `profile/apply.rs:12` would send a reader to "`observe_pane` is required".
     - `main` cites #835's numbers nowhere outside the list, so #853 is the one to renumber.
   - **(a) Merge.** Run `git merge origin/main` on `issue-853-implementation`. Use a merge, not a rebase:
     `origin/issue-853-implementation` already holds `1f4ab40..05d8e6a`, and the PR phase's plain `git push` must stay a
     fast-forward.
   - **(b) The list.** In the merged ADR-0021, change #853's entry `30.` to `32.` and its `31.` to `33.`. They are at
     lines 1538 and 1544 on `c1f7deb`, and 1592 and 1598 after the merge. The list then reads 29, 30, 31 (both #835's),
     32, 33, each once.
   - **(c) Every cite.** The line on `c1f7deb` comes first; the line after the merge follows where it moves.
     - `docs/adr/ADR-0021.md:739` (`:742`): "decision 30" becomes 32.
     - `docs/adr/ADR-0021.md:741` (`:744`): "decision 31" becomes 33.
     - `docs/adr/ADR-0021.md:857` (`:860`), section 9's `profile apply` row: "decision 31" becomes 33.
     - `CHANGELOG.md:599` (`:615`): "(30 and 31)" becomes "(32 and 33)".
     - `crates/holler-cli/src/profile/apply.rs:12` and `:150`: "decision 31" becomes 33.
     - **Tests, comments only.** These are T-green's; F flags them:
       - `crates/holler-cli/tests/profile_verbs/apply/order.rs:2`, `:28` and `:95`: 30 becomes 32;
       - `crates/holler-cli/tests/profile_verbs/apply/refused.rs:2`, `:31` and `:222`: 31 becomes 33;
       - `crates/holler-cli/tests/profile_apply_scenario/plan.rs:198`: 31 becomes 33.

     That is 13 cites. A-dup counted 11 and missed `refused.rs:31` ("Decision 31, at the verb") and `:222` ("Decision
     31's order").

     The item is done when two checks hold. First,
     `git grep -n -i -E "decision(s)? (30|31)\b|\(30 and 31\)" -- crates docs/adr docs/pane-control.md CHANGELOG.md`
     finds nothing that cites #853's decisions. Second, ADR-0021's list holds each number once. The run's handoffs (the
     brief, A, T, F, A-dup, evidence) keep their history as written; decisions.md records the renumbering.
   - **(d) Re-verify on the merged tree.** Run:
     - the workspace verify command;
     - clippy `-D warnings`, `scripts/lint.sh` and `scripts/changelog-check.sh`;
     - `docs_pane_control_test` (#835 added a case to it), `docs_cli_test` and `cli_surface_test`.

     #835 also changed `HerdrPort` (`ports.rs`) and the import path. The merge changes none of #853's code or test
     files, but the merged tree is what lands.

`reworkKind: production`: the item touches `crates/holler-cli/src/profile/apply.rs` and needs the merge. The test
comments are T-green's.

## Advisory notes (non-blocking)

1. **Privacy, in unpushed history.** This is for the operator, before the PR phase pushes.
   - T-red's commit `0064fac` added the operator's two machine-derived Herdr session names to this run's
     `decisions.md`, in T-red's "Incident, no harm" bullet. F's `07a576c` still carries them.
   - T-green's `a8b3048` replaced them with a neutral description, so the PR's diff and the squash that lands on `main`
     are clean.
   - The PR phase's `git push -u origin issue-853-implementation` publishes those commits in the PR's commit list, and
     GitHub keeps a PR's commits after a squash merge and branch deletion.
   - This discloses nothing new: the same name is on `main` today, in `docs/handoffs/655-herdr-move/decisions.md` (from
     #834), and in issue #655.
   - If the operator wants the PR history clean anyway, rewrite only the unpushed commits, before item 1's merge. Edit
     `0064fac`'s bullet to T-green's wording, so that the later commits replay with no change there. The push then stays a
     fast-forward from `05d8e6a`.
   - Removing the name from `main`, as #796 did for other files, is a separate follow-up.
2. **One sentence in ADR-0021 (optional; F is in the file for item 1).** Line 710 reads "0 for a run with no failed step
   and no refused step, which prints its steps (... the plan for a dry run)".
   - It leaves out a dry run that plans a refused step, which exits 0. The same section's "A `--dry-run` is not changed"
     paragraph says so, and so do the guide and the code.
   - Suggested: "0 for a dry run that does not fail, or a real run with no failed or refused step, which prints its
     steps".
   - This is the amendment's text, which A passed, and not F's.
3. **#664's own CHANGELOG entry** (`CHANGELOG.md:131-146`) still says "in workspace, row and column order" and "A refused
   or reported step leaves the exit code at 0".
   - F left it as written, which follows the repo's practice of keeping `[Unreleased]` entries as written. #835's S ruled
     the same way.
   - #853's Bug Fixes entry supersedes both statements. Reconcile them when the release moves `[Unreleased]` under a
     version.
4. **Parked for the operator** (A's warn 4). A relaunch refused before its act after `--take-over` prints only its pane
   and code. The ADR and the guide now say that `pane list` and `profile show` show the move. Carrying the note would
   need a new `Step` signal, which A ruled out.
5. **A-dup's optional folds are not done.** None is required:
   - warn 1: `workspace_extents()` against `wiring::env::herdr_config`;
   - warn 3: `grid.rs`'s `listed()` against `assert_end_state`;
   - warn 4: the row-a `seed` repeat in `rows.rs`.
6. **`pane_claude/mixed.rs` `apply_never_changes_a_live_panes_harness`** runs apply with a `harness-change-refused` step
   and checks only that the output names the code.
   - That run now exits 3, and the code comes from the one-line refusal. The test passes and never pinned exit 0, so it
     was outside the blast radius.
   - Its comment "the pane's step names the refusal" is out of date. Optional: assert exit 3 there.
   - Also optional: rename `one_run_takes_every_row_in_grid_order` and `row_b_..._in_grid_order`, whose "grid order" now
     means apply's order.
7. **PR hygiene, for the merging agent.** Per the repo's `CLAUDE.md`, once the script opens the PR, add the AI
   disclosure that `CONTRIBUTING.md` requires, with `gh pr edit`.
8. **Size.** The test kit's `herdr.rs` is 598 lines, 2 under `lint.sh`'s 600-line warning, so the next edit there will
   warn.
