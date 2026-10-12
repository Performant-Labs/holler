[S]

# Handoff-S: #853 - `profile apply` creates column 1 first, and a real run with a refused step exits 3 (spec audit, cycle 2)

**Date:** 2026-10-12, 07:52 MDT
**Branch:** `issue-853-implementation` at `6100914` (A-dup's cycle-2 PASS).
- `HEAD` holds `origin/main` through F's merge commit `0330fa0`, whose parents are `21b29a0` and `4e691d9`.
- `origin/main` is still `4e691d9` (#835's PR #860). I checked with `git ls-remote` at the start and at the end of this
  audit.
- `origin/issue-853-implementation` is `05d8e6a`, an ancestor of `HEAD`, so the PR phase's push is a fast-forward.
- No PR exists: `gh pr list --head issue-853-implementation --state all` is empty.

**Issue:** [#853](https://github.com/Performant-Labs/holler/issues/853), OPEN, read live at 07:36 MDT. The body and its
two comments (the spike result, and the operator's exit-code decision of 04:12 MDT) are the ones the brief quotes.
Nothing is new since.

**Brief:** `docs/handoffs/853-brief.md`, round 2 (`05d8e6a`).

This replaces cycle 1's S handoff, which is in git at `21b29a0`.

**Handoffs reviewed:**
- cycle 1's `handoff-S.md`: REWORK, one item;
- `handoff-A.md`: round 2, PASS;
- `handoff-T-red.md`;
- `handoff-F.md`: cycle 2;
- `handoff-T-green.md`: cycle 2, PASS;
- `handoff-A-dup.md`: cycle 2, PASS;
- `decisions.md` and `evidence.md`.

**Diff audited:**
- `git diff origin/main...HEAD`, merge base `4e691d9`. Outside `docs/handoffs/` it is 23 files, with 1,089 lines added
  and 243 removed. I read all of it again on the merged tree:
  - the production diff: `tx_apply.rs`, `profile/apply.rs`, the test kit's `herdr.rs` and the adapter's `Cargo.toml`;
  - `order.rs`, `refused.rs` and `grid.rs` in full, and every other test diff;
  - the ADR-0021, ADR-0003, guide and CHANGELOG diffs.
- This cycle's own edits, kept apart from #835's merged content: `git diff c2941f8 HEAD`, where `c2941f8` is
  `git merge-tree --write-tree 21b29a0 4e691d9`. Outside the handoffs that is 6 files, with 16 lines added and 17
  removed: the renumbering, one blank line, and the advisory-2 sentence.

## A precondition

Met.
- `handoff-A.md` round 2 is PASS, with 0 blocks and 9 warns.
- The anti-duplication gate passed again in cycle 2. `handoff-A-dup.md` has 0 blocks and 6 warns, one of them new
  (Advisory 1).

## T precondition

Met. `handoff-T-green.md` (cycle 2) has no blocking issue, and RED then GREEN is recorded.

- **RED** (`handoff-T-red.md`):
  - `profile_verbs`: 105 passed and 13 failed, each on the missing behaviour;
  - the fake's new split-only test and the adapter's cross-check also failed;
  - the opted-in `grid.rs` failed on Herdr 0.9.1 exactly as the spike did.
- **GREEN on the merged tree:**
  - the verify command: 2,606 passed, 0 failed, 37 ignored;
  - clippy `-D warnings`, `lint.sh`, `changelog-check.sh` and `cargo machete`: clean;
  - `docs_pane_control_test` 19, `docs_cli_test` 3 and `cli_surface_test` 3.
- **The removal checks still stand.** In cycle 1 the old key failed 8 tests, `refused()` disabled failed 8, and the
  `acted` filter dropped failed 6. I checked that the code they reverted has not changed: `git diff c1f7deb HEAD` over
  #853's code and tests is the 2 doc-comment lines in `profile/apply.rs` and the 7 test comments.
- `HEAD` differs from T-green's `c39699c` only in `decisions.md` and `handoff-A-dup.md`.
- The opted-in scratch tests (`grid`, `probe`, `scenario`) passed on Herdr 0.9.1 in cycle 1. They were not re-run, and
  the code they exercise is byte-identical.

## Acceptance criteria

| # | Criterion (the brief's "Acceptance, as it can be met") | Proving test or evidence | Status |
|---|---|---|---|
| 1 | The order, pure. Cells in any order, in one workspace and in several, come out column 1 by row, then by row and column. Workspaces keep their order, and two specs of one cell keep P's order. | `profile_verbs/apply/order.rs` `steps_take_each_workspaces_column_1_by_row_then_the_rest_by_row_and_column`: a dry run on the rig, with P scrambled across `main` and `side`, and `dup-b` listed before `dup-a`. It asserts the step order the engine answers, not the key. It failed with the old key (removal check). | Met |
| 2 | The fix on the fakes. `SplitOnly` follows section 10's rule; the fake's own tests are updated; there is a cross-check against `plan_splits`; a four-pane 2x2 apply on `SplitOnly` creates all four and fails with the old order. No new double, and no wider `holler-pane` API. | The test kit's `check_split` (`herdr.rs:475`). `fake_herdr_test.rs:226` `split_only_mode_adds_a_row_only_below_a_last_row_of_one_pane`, plus the updated `split_only_mode_refuses_an_absolute_placement`. `plan_splits_test.rs:409` `the_split_only_fake_accepts_a_cell_exactly_when_the_planner_plans_it`: every grid with no hole up to 3 by 3, and every free cell of a 4-by-4 window, the out-of-range ring included. `rows.rs` `row_b_missing_panes_are_created_by_splits_in_grid_order`: the 2x2 on `SplitOnly`, which fails with the old key. No new double. `cell_key`, `last_row` and `end_of` are private, and the test kit's `Cargo.toml` is unchanged. | Met |
| 3 | The fix on the scratch set. Opt-in and `#[ignore]`; an empty 2x2 workspace; all four created and listed at their cells; a second apply adopts all four; `2x2` in `WORKSPACES`; the protocol-22 message. | `profile_apply_scenario/grid.rs` `apply_builds_a_whole_2x2_grid_from_an_empty_workspace`. `WORKSPACES` gains `hlr649sq=2x2`. `herdr_snapshot`'s connect message names protocol 22, and it is the test's first adapter connect: `Scratch::start` starts the scratch Herdr but makes no connect. It failed at RED as the spike did, and passed at GREEN. | Met |
| 4 | The limit and the exit code. `r2c1` below a live two-pane row is refused `grid-unreachable` and the run exits 3, with the failure envelope, its code and the one-line message; text is one `error:` line, and the others ran. A failed step beside a refused one exits 1. A reported step alone exits 0. A dry run that plans a refusal exits 0 with its plan. An acted refusal carries its detail, ending with the reconcile step, in both formats, and a take-over note is carried the same way. | `order.rs` `a_row_below_a_row_of_two_live_panes_is_refused_and_the_run_exits_3`. `refused.rs` `a_failed_step_beside_a_refused_one_still_exits_1`. `rows.rs` `row_d_a_healthy_pane_that_differs_is_reported_and_never_moved` (`data_of` asserts exit 0). `dry_run.rs` `a_dry_run_prints_its_plan_as_the_steps_object` (rows a and e refused, exit 0 in JSON and in text). `refused.rs` `a_refusal_after_the_act_began_carries_its_detail_and_the_reconcile_step` (a create and a relaunch) and `an_acted_refusal_after_a_take_over_carries_its_note`. `assert_refused` checks the JSON and the text form on fresh rigs. On real state: `probe.rs` (i) and (ii), and `scenario.rs` step 5. | Met |
| 5 | The old order is unpinned everywhere, each test still proves what it proved, and `fake_herdr_test.rs`'s snapshot test is left alone. | The rig's `EVERY_ROW`; in `rows.rs`, the every-row, row a, probe, usage, apply-twice and row b cases; `failures.rs` entry 33; the `dry_run.rs` docs; `probe.rs`; `scenario.rs`; `ac12`'s comment. `snapshot_lists_panes_by_workspace_then_row_then_col` is untouched. The every-row case still pins every JSON fact it pinned on `main`: the step order, pane-e's `refused` and its code, the action strings and the step keys. It now reads them from the dry run. | Met |
| 6 | The text. `tx_apply.rs`'s docs and `cell_key`'s doc, which names `plan_splits`, with the key private; `profile/apply.rs`'s module doc and the `ProfileApply` clap doc; ADR-0021, ADR-0003, the guide, and a `CHANGELOG.md` Bug Fixes entry that links #853 and states both changes. | All are in the diff, on the merged tree: `tx_apply.rs`'s module doc, the docs of `Applied.steps` and `plan`, and `cell_key`'s doc; the module doc and the clap doc of `profile/apply.rs`; ADR-0021 sections 8 and 9, with decisions 32 and 33; ADR-0003's row; the guide's sections 3 and 6; one `### Bug Fixes` under `[Unreleased]`, linking #633 and #853, with both changes. Every #853 cite names 32 or 33. | Met. Two small text slips do not block: Advisories 1 and 2. |

## Spec compliance

- **Cycle 1's REWORK item is done**, in all four parts.
  - **(a) The merge.** `0330fa0` is a two-parent merge of `4e691d9`, not a rebase. `05d8e6a` stays an ancestor, so the
    push is a fast-forward.
  - **(b) The list.** ADR-0021's "Decisions taken" runs 1 to 33, each number once and in order. Decisions 30 and 31 are
    #835's (`:1584`, `:1587`), and 32 and 33 are #853's (`:1591`, `:1597`).
  - **(c) The cites.** All 13 name 32 or 33:
    - ADR-0021 `:742`, `:744` and `:860`, and `CHANGELOG.md:615`;
    - `profile/apply.rs:12` and `:150`;
    - `order.rs:2`, `:28` and `:95`, `refused.rs:2`, `:31` and `:222`, and the scenario's `plan.rs:198`.

    Cycle 1's done-check grep finds nothing (exit 1). I also swept the PR's added lines for a bare 30 or 31. The only
    hit is `rows.rs:2`'s "brief C14 entries ... 30 and 32", which names #664's brief entries, as it does on `main`.
  - **(d) Re-verified on the merged tree** by T-green (see the T precondition).
- **The cycle's two edits beyond the item are accepted.**
  - F removed the blank line the merge left inside the list, so the list is tight again, as it is on `main`.
  - ADR-0021 `:713` now reads "0 for a dry run that does not fail, or a real run with no failed or refused step". This
    is cycle 1's suggested wording, and it matches the code: `stopped()` has no dry-run guard, so a failing dry run exits
    1, and `refused()` returns `None` for a dry run.
- **The production code is as the brief decided.** I re-read it on the merged tree.
  - **The key.** `cell_key` (`tx_apply.rs:360`) is `(workspace, col != 1, row, col)` and private. It is the planner's
    own partition (`plan.rs:98-100`). The stable `sort_by` (`:349`) keeps two specs of one cell in P's order.
  - **The exit decision.** `run` asks `stopped(..).or_else(|| refused(..))` (`profile/apply.rs:94`), so the exit is 1,
    else 3, else 0.
    - The code is the first refused step's, so `class_of` gives the 3. There is no exit table.
    - `halted` (`tx_apply.rs:528`) is the only place that builds an `Outcome::Refused`, and it always sets `error`. So
      `refused()`'s `filter_map` drops no refused step, and `ran` cannot underflow.
  - **The carried detail** keys on `Step::acted` alone. An acted step's detail already ends with the reconcile step,
    because `reconciled()` runs first.
  - **The fake.** `check_split` follows section 10's rule for every grid with no hole. Its rule for holes is an
    `ASSUMPTION (#640)`.
- **What does not change does not change:**
  - the planner, `HerdrPort`, the step shape, the wire, the records, every code and `class_of`;
  - the dry run;
  - `cli-surface.txt`, which pins argument forms only (I checked);
  - the protocol goldens.
- **The brief against the issue.** This is unchanged from cycle 1.
  - The brief's ruling that `cli-surface.txt` does not change is right.
  - The operator's "`ok` reflecting the failure" is met. The failure envelope has `ok` false, and its message names
    every refused step with its code. Decision 33 records the operator's choice of this one-line form over an envelope
    change.
  - The operator's dry-run condition is met: "only if the amendment finds the verbs' dry runs already do; otherwise it
    is left as is and noted". The dry run stays at exit 0, and ADR-0021, the guide and decision 33 note it.
- **A's warn 4, option (a).** This is F's choice under the standing orders, recorded in decisions.md and accepted in
  cycle 1. Its follow-up stays parked (Advisory 4).
- **The merge brings no semantic conflict.**
  - #835 shares only four docs with this branch, in different paragraphs, and adds no test that runs `profile apply`.
  - T-green's full run covers every non-ignored test that runs the verb. That includes `pane_claude/mixed.rs`
    `apply_never_changes_a_live_panes_harness`, which still finds its code in the failure envelope.
  - The one ignored test in that file, `say_delivers_...` (blocked by #839 and #850), has no refused step.

## Quality audit

- **Correctness and failure handling.**
  - Cycle 1's findings still hold on the merged tree. The refusals are a subset of the steps; an acted refused step
    always has a detail; and `checked_sub` avoids a `u16` wrap.
  - The fake's rule equals the planner's single-cell rule for every grid with no hole, and the cross-check proves it
    up to 3 by 3.
  - No error path, lock or write path changed. Refusals still do not stop the run, and nothing is rolled back.
- **Build guards.**
  - Nothing added under `crates/*/src` uses `unwrap`, `expect`, `panic!`, `unreachable!`, `todo!` or `#[allow`.
  - Every touched file is under 900 lines. The largest are `rig.rs` (776), the test kit's `herdr.rs` (598),
    `scenario.rs` (570) and `tx_apply.rs` (565).
  - Clippy `-D warnings` and `cargo machete` are clean (T-green), so there is no dead code or unused dependency.
- **Protocol.** Nothing changes: no wire field, no golden file and no error-code table entry. `docs/protocol/v2.md`
  needs no change.
- **Tests.**
  - Each test sits at the cheapest tier that shows its behaviour: a dry run on the rig for the order, the fake's own
    test and an adapter dev-test for the rule, and scratch only for real Herdr.
  - No sleep is added: a grep of the added lines finds no `sleep` and no `Duration`.
  - There is one refused-form helper per test binary: `assert_refused`, and the scratch binary's `assert_refused_run`.
  - RED-first evidence is in `handoff-T-red.md`.
- **Documentation.**
  - `CHANGELOG.md`'s `[Unreleased]` has one `### Bug Fixes` (#853's), with #835's entries under `### Enhancements`.
    I re-ran `changelog-check.sh`: it prints `changelog-check: ok`.
  - The guide, ADR-0021 and ADR-0003 are updated. There is no new log event, flag or protocol field.
  - Two one-line text slips remain, and neither blocks: a wrong section cite in ADR-0021 (Advisory 1), and a loose
    sentence in the `--help` text (Advisory 2).
- **Public-repository privacy.**
  - The PR diff is clean. I grepped its 2,874 added lines for host, machine, account and tailnet names, home paths,
    e-mail addresses, non-loopback IPs, Herdr session names and secret patterns. The only hit is prose: cycle 1's own
    description of this check.
  - The three commits since cycle 1 add nothing private. The merge's hits are prose about the privacy rule in #835's
    handoffs, which are byte-identical to `main`.
  - The unpushed history is Advisory 3, where I correct cycle 1's premise.
- **Commit hygiene.**
  - All 12 first-parent branch commits have Conventional subjects and a `Co-Authored-By` trailer.
  - None has a session link. That is the script's phase-commit form, and #835's squash on `main` has the same form.
  - The PR does not exist yet (Advisory 5).

## Scope check

F delivered exactly the brief's scope, as in cycle 1: three small production edits (the key, the exit decision and the
fake's rule), their docs, and a comment in the adapter's `Cargo.toml`.

- Cycle 2 adds only the merge of `main` and text edits. It changes no behaviour.
- **Over-delivery:** none. The advisory-2 sentence was cycle 1's offer, in cycle 1's words.
- **Under-delivery:** none.
- The one test edit outside the brief is still the necessary one: T-green's `scenario.rs:407` wording repair, stale on
  `main` since #857.

## Verdict

**PASS.**
- All six criteria are met on the merged tree. Each is proved by a test that asserts behaviour and fails without the
  change.
- The branch is spec-compliant, and cycle 1's one REWORK item is done in full.
- The quality audit finds two one-line text slips and a note on the unpushed history. None blocks.

**Why the section cite (Advisory 1) is not REWORK:**
- It is a slip in the amendment's own text (O's `1f4ab40`), which A passed in both rounds.
- The sentence's claim is right; only its pointer is wrong. No test or script reads it.
- Cycle 1 closed its list, and a third F cycle for one word is out of proportion.

## Advisory notes (non-blocking)

1. **ADR-0021 `:765` cites the wrong section.** This is A-dup's cycle-2 warn 1, which I confirmed.
   - The sentence is "`pane import`, exits 0 on a dry run and refuses only its real run (section 14)". Section 14 is
     "Unmanaged panes".
   - On both `b76f4d2` and the merged tree, the rule is in section 13, "Migration from the launcher's files": its order,
     items 1 and 2 (`:1088-1093`). The codes are in section 9's `pane import` row (`:854`).
   - The fix is one word: "(section 14)" becomes "(section 13)". No test or script pins this text.
   - It can ride the next story that amends ADR-0021 section 8, or a docs-only follow-up. If the operator wants it in
     this PR, it is a one-word docs commit before the merge, and CI re-runs.
2. **The `--help` text over-claims a dry run's exit code.** The `ProfileApply` clap doc
   (`crates/holler-cli/src/profile/apply.rs:54`) says "--dry-run prints the plan, exits 0 and changes nothing".
   - A dry run with a failed step exits 1 and prints no plan. That is by design: `stopped()` has no dry-run guard
     (ADR-0021 `:713-715`).
   - "Prints the plan" was already loose on `main`; #853 added "exits 0". This is the same slip as cycle 1's advisory 2,
     which F fixed in the ADR.
   - Suggested: "--dry-run prints the plan and changes nothing; it exits 0 unless a step fails."
   - Nothing pins the sentence (`git grep` finds only the source line). Optional: a follow-up, or the verb's next edit.
3. **Privacy in the unpushed history: cycle 1's premise, corrected.** The PR phase's push publishes the branch's
   commits. In `decisions.md`, T-red's "Incident, no harm" bullet named two of the operator's live Herdr sessions.
   T-red's `0064fac` added the names, F's `07a576c` kept them, and T-green's `a8b3048` removed them. All three commits
   show them, and GitHub keeps a PR's commits after a squash merge and branch deletion.
   - Cycle 1 judged that this discloses nothing new because "the same name is on `main` today". That holds for one of
     the two names only. I checked each by masked comparison, and I copy neither here.
     - **The first name** is the machine's short hostname itself. It is already on `main`, in
       `docs/handoffs/655-herdr-move/decisions.md` (from #834).
     - **The second name** is not in `main`'s tree, `main`'s history, the pushed tip `05d8e6a`, or issues #655 and #853.
       It is a word common in the repo (it appears in 366 files on `main`) joined to that same hostname.
   - So the push adds no identifying fact that `main` does not already show, and I do not hold the run for it.
   - The script's PR phase pushes right after this gate, with no stop in between. Keeping the PR's commit list clean
     would mean stopping the run and having the main session rewrite the unpushed commits: a
     `git rebase --rebase-merges` from `05d8e6a` that edits `0064fac`'s bullet to T-green's wording, after which the
     push stays a fast-forward. No pipeline role rewrites history, so this cannot be a REWORK item. I judge it not worth
     a stop.
   - The exposure that matters is the hostname already on `main`. Removing it, as #796 did for other files, is a
     separate follow-up.
4. **Carried over from cycle 1, still open, none required:**
   - **#664's own CHANGELOG entry** (`CHANGELOG.md:145-162` on the merged tree) still says "in workspace, row and column
     order" and "A refused or reported step leaves the exit code at 0". The repo keeps `[Unreleased]` entries as
     written, and #853's entry supersedes both. Reconcile them at release.
   - **A's warn 4, parked for the operator.** A relaunch refused before its act after `--take-over` prints only its
     pane and code. The ADR and the guide say so.
   - **A-dup's optional folds:**
     - `workspace_extents()` against `wiring::env::herdr_config`;
     - `grid.rs`'s `listed()` against `assert_end_state`;
     - the row-a `seed` repeat in `rows.rs`;
     - `fake_spec`, the third per-file `HerdrSpec` builder.
   - **`pane_claude/mixed.rs` `apply_never_changes_a_live_panes_harness`.** Its comment "the pane's step names the
     refusal" is out of date: the run now exits 3, and the code is in the one line. Optionally, assert exit 3 there.
     Also optional: rename `one_run_takes_every_row_in_grid_order` and `row_b_..._in_grid_order`.
   - **Size.** The test kit's `herdr.rs` is 598 lines, 2 under `lint.sh`'s 600-line warning.
5. **For the merging agent.**
   - Once the script opens the PR, add the AI disclosure that `CONTRIBUTING.md` requires to the PR body with
     `gh pr edit`, per the repo's `CLAUDE.md`.
   - CI runs the workspace suite. F saw two flakes from `main` there, in code neither side touches:
     - `profile_rename_faults_test`'s inode assertion (#665's test). No issue exists yet; F left filing one to the main
       session.
     - `remote_admin_test` (#847).

     If either fails CI, re-run it. Neither needs a code change.
