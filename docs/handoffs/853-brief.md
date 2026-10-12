# [profile apply: creation order may hit grid-unreachable when a profile creates a whole multi-column grid (suspected, from reading, not run)](https://github.com/Performant-Labs/holler/issues/853)

Part of epic #633. **Operator decision (2026-10-12): fix first; it gates the release.** The issue text below is verbatim; the
spike comment follows it.

Found while preparing the brief for #667 (`docs/handoffs/667-brief.md`, decision 10). **This is from reading the code; nobody has run it.**

`profile apply` creates missing panes in `(workspace, row, column)` order (`crates/holler-pane/src/tx_apply.rs`), one cell at a time through `HerdrPort::ensure_pane`. The split planner (`crates/holler-adapter-herdr/src/plan.rs`, `plan_splits`) builds rows first, then each row's columns, and refuses "a row added below a row of more than one pane" as `grid-unreachable`. So for a profile that must create, from an empty workspace, a grid with more than one row and more than one column, apply would create `r1c2` before `r2c1`, and the planner would then refuse `r2c1` because row 1 already holds two panes.

Whether this is real depends on whether apply passes the whole target to the planner or one cell at a time, which the reading did not settle. A scratch run would settle it: an empty 2-row-by-2-column workspace and a profile of four panes.

It does not affect adopting panes that exist (the cutover's `fleet` profile), only creating a missing pane or a whole grid. If real, the fix is apply's order (rows before columns), the planner, or documentation of the limit.

AI assistance: drafted by an AI agent (Claude Code) from the #667 brief preparation; unverified until run.

Part of epic #633.

🤖 Generated with [Claude Code](https://claude.com/claude-code)

## The spike's result (comment on the issue, verbatim)

**Spike result, 2026-10-12 (scratch run on `main` at `b76f4d2`; Herdr 0.9.1):** the bug is **real**.

- **Empty 2x2 workspace, a profile of four panes, `holler profile apply`:** it created `r1c1` and `r1c2`, then refused `r2c1` and `r2c2` with `grid-unreachable` ("a row can be added only below a row of one pane, and row 1 has 2 panes"). A second apply gave the same refusals.
- **The workaround works:** launching `r1c1` and `r2c1` first, then `apply`, ended with all four panes at the right cells; a second apply adopted all four.
- **Cause:** `cell_key` in `crates/holler-pane/src/tx_apply.rs` sorts steps by `(workspace, row, col)`, so a row is filled before the next row exists. The planner is right to refuse; it cannot be fixed there.
- **Smallest fix:** change apply's order so a workspace's column 1 (by row) is created first, then the rest by `(row, col)`, the order the planner itself uses (`r1c1, r2c1, r1c2, r2c2`). Touches `cell_key`, its doc comment, and the places that state or pin "(workspace, row, col) order", plus the ADR and guide wording.
- **Side finding, NOT part of this story until the operator decides:** a run with refused steps exits **0**. Whether it should exit non-zero is a contract question for the amendment to raise; do not change it silently.

**Operator decision, 2026-10-12: fix this first.** This story gates the #633 release and is a prerequisite of #803 (the wizard then uses `profile apply` alone, not a column-1-first workaround). `rigor: in-session`; contract amendment first (the order is stated in ADR-0021 and the guide).

Filed by an AI agent (Claude Code) at the operator's direction.


## What the operator asked for

- Change apply's order so a workspace's column 1 (by row) is created before the rest, then by row and column:
  `r1c1`, `r2c1`, `r1c2`, `r2c2`. In `cell_key` (`crates/holler-pane/src/tx_apply.rs`) and in every place that states or pins
  the old order, with the ADR-0021 and guide wording.
- Contract amendment first (amend-first rule), `in-session` rigor, own worktree off `origin/main`.
- Tests: the spike's 2x2 case as a real scratch test (apply from an empty workspace creates all four), and a pure unit test of
  the order.
- **(Updated 2026-10-12, after the first cut of this brief.)** `profile apply` exits **non-zero when any step is refused**,
  part of this story, with the repo's existing refusal code (exit 3). The operator chose the one-line failure form for the
  output (below).
- Standing orders: park any new contract or scope choice, two-round limit, no live-state changes.

## Contract amendment (amend-first)

Committed before any code, in `docs/adr/ADR-0021.md`, `docs/adr/ADR-0003.md` and `docs/pane-control.md`:

- **ADR-0021 section 8, "Apply's step order and its exit code (#853)"** (a new pair of paragraphs after "Apply as built
  (#664)"), the changed sentences in "Apply as built", section 9's `profile apply` row, and **decisions 30 and 31**. The order is by workspace, then the workspace's column-1 cells by row, then every other
  cell by row and column. The key is the planner's own partition (`plan_splits` places column-1 cells first), so the apply order
  and the planner agree.
- **The guide** (`docs/pane-control.md`, "profile apply"): the step-order sentence and a new bullet, "The order of the steps".
- **What does not change:** the planner, `HerdrPort`, the step shape, every error code, `class_of`, every record, spec and wire
  field, and every other verb's exit codes. The order of `steps` in the output changes for a profile that has a column-1 cell below the first row (a dry run's
  plan included).
- **One limit stays and is documented:** a row cannot be added below a row that already holds two or more live panes (Herdr makes
  every pane by splitting one). Creating `r2c1` beside a live `r1c1` and `r1c2` is still a refused step (`grid-unreachable`). No
  live pane is moved to make room.

### The exit code (operator decision, 2026-10-12)

- **A real run that has a `refused` step and no `failed` one exits 3** (the repo's refusal code: ADR-0021 section 9, ADR 0003's
  exit 3; `import-blocked` and `profile-exists` already exit 3). A `failed` step still stops the run and exits 1. A
  `reported` step still exits 0. Refusals still do not stop the run: every step runs, and nothing is rolled back.
- **Output:** the run then prints only the failure, because the shared envelope has no `data` on a failure (`check_envelope`:
  `ok` is exactly "exit 0", `DataOnFailure`). JSON: the failure envelope, `error.code` the first refused step's code (step
  order), message `apply refused <N> step(s): <pane> (<code>), ...; <M> other step(s) ran`, then for each refused step that had begun its act `; <pane>: <detail>` (the detail ends with the reconcile step, once, and carries a take-over note; A's finding 1, decided in the amendment). Text: one `error:` line on stderr,
  nothing on stdout. The operator chose this over changing the envelope. A script that needs the steps as data runs
  `--dry-run` first (exit 0).
- **`--dry-run` is not changed** (exit 0 whatever it plans): the verbs' dry runs do not refuse; `pane import`'s dry run exits 0
  with problems and refuses only the real run.
- **Where it is recorded:** ADR-0021 section 8 ("Apply's step order and its exit code") and section 9's `profile apply` row,
  decision 31, ADR-0003's `profile apply` row, and the guide. **`cli-surface.txt` pins argument forms only (no exit codes) and
  does not change.** F adds the `CHANGELOG.md` entry (it links #853 and states both changes).
- **Code:** `crates/holler-cli/src/profile/apply.rs` (the exit decision and a `refused(&Applied)` beside `stopped`, and its
  module doc); the engine's result is unchanged.
- **Tests that assert exit 0 beside a refused step must change** (T finds them by running): at least
  `profile_verbs/apply/rows.rs` (`one_run_takes_every_row_in_grid_order` and its `refused`/`reported` pin) and the scratch
  scenario's refusals in `profile_apply_scenario/probe.rs` (`a_failing_check_refuses_the_create`, which reads the refused step's
  row from the data) and `scenario.rs`. A refused step is now read from the failure's message and from the live state, or from
  the engine's result in the fakes. A dry run's refused step keeps its row in `data` and exit 0.

## Acceptance, as it can be met

1. **The order, pure.** A unit test of the order: cells given in any order, in one workspace and in several, come out
   column-1-by-row first and then by row and column; workspaces keep their order; two specs of one cell keep P's order.
2. **The fix, on the fakes.** The decision (A's finding 2): the testkit's `FakeHerdr` does not plan with `plan_splits`; its
   `Placement::SplitOnly` (`herdr.rs:471-493`) is looser than the adapter's rule, and the apply rig uses
   `Placement::Absolute`. So **extend `SplitOnly` to ADR-0021 section 10's rule** (a new row only at column 1, below a last
   row of one pane; a new cell of an existing row only right of the row's last pane) and update its own tests; add a
   cross-check in `crates/holler-adapter-herdr/tests/` that the fake accepts a cell exactly when `plan_splits` plans one
   step for it (the testkit cannot depend on the adapter). Then `profile apply` of a four-pane 2x2 profile with no records,
   on a rig set to `SplitOnly`, creates all four and none is `grid-unreachable`; it fails (r2c1 refused) with the old order.
   Do not add a new double in the `holler-cli` tests or widen `holler-pane`'s API.
3. **The fix, on the scratch set.** A new test in `crates/holler-cli/tests/profile_apply_scenario/`, opt-in
   (`HOLLER_TEST_PANE_INTEGRATION=1`) and `#[ignore]`, modelled on the spike: an empty 2-row-by-2-column workspace and a profile
   of four panes; apply creates all four, the adapter lists them at `r1c1`, `r1c2`, `r2c1`, `r2c2`, and a second apply adopts all
   four. The rig's `WORKSPACES` gets a `2x2` workspace for it. The scratch needs a Herdr that speaks protocol 22 (0.9.1) first on
   `PATH`; on a machine where an older one comes first the opted-in run fails at connect, and the test says so.
4. **The remaining limit and the exit code.** On `SplitOnly`, with a live `r1c1` and `r1c2` and a spec `r2c1`, apply's `r2c1`
   step is `refused` with `grid-unreachable`, the run exits **3**, JSON is the failure envelope with `error.code`
   `grid-unreachable` and the one-line `apply refused 1 step(s): ...` message, text is one `error:` line, and the other steps
   still ran. Also: a `failed` step with a refused one still exits 1; a `reported` step alone exits 0; a dry run that plans a
   refusal exits 0 with the plan. **Acted refusal (A's finding 1):** a refused step whose launch had begun its act (a
   `verify_serving`-style refusal, as `rows.rs:544 a_refusal_after_the_act_began_names_the_reconcile_step` builds) exits 3 and
   its one line ends with that step's detail, which ends with the reconcile step, in both formats; a take-over note is carried
   the same way.
5. **The old order is unpinned everywhere.** Every test that pins `(workspace, row, col)` order or exit 0 beside a refused step
   (the list in "Blast radius") is updated, and each still proves what it proved before. **Not** `fake_herdr_test.rs:536`
   (`snapshot_lists_panes_by_workspace_then_row_then_col` pins the fake's snapshot listing, which matches the real adapter's and
   must not change).
6. **The text.** `tx_apply.rs`'s module doc and doc comments (`cell_key`'s own doc names `plan_splits` as the rule it mirrors; the
   key stays private inside `cell_key`, e.g. `(workspace, col != 1, row, col)`), `profile/apply.rs`'s module doc **and the
   `ProfileApply` clap doc at lines 34-51 (the `--help` text, which states both old rules)**, ADR-0021, ADR-0003, the guide and
   `CHANGELOG.md` (Bug Fixes, linking #853, both the order and the exit code) say the new behaviour.

## Pipeline

`rigor: in-session`, `uiSurface: false`. The PR closes #853 (this is the whole fix). It follows the repo's CI and the standing
orders: self-merge on green CI, an S pass and a clean PR-agent score.

## Blast radius and hot files

- **Production:** `crates/holler-pane/src/tx_apply.rs` (`cell_key` and its doc, the module doc at line 21, the doc comments at
  lines 103 and 331; no public order API) and `crates/holler-cli/src/profile/apply.rs` (the exit decision; `refused()` beside
  `stopped()`, reusing `reconcile_step` and each step's `acted`; the module doc and the `ProfileApply` clap doc). Exit codes
  come from `class_of` alone (no table in the verb). No consumer of apply's output exists on `main` (the wizard, #803, is parked).
- **Test kit (A's finding 2):** `crates/holler-pane-testkit/src/herdr.rs` (`Placement::SplitOnly`, `check_split`) and
  `crates/holler-pane-testkit/tests/fake_herdr_test.rs` (its SplitOnly tests, lines ~191-218), plus a cross-check test in
  `crates/holler-adapter-herdr/tests/`. `holler-pane-testkit` does not depend on the adapter and must not start to.
- **Tests that state or pin the old order or exit 0 beside a refused step** (T updates them; A's findings 3 and 4):
  `profile_verbs/apply/rows.rs` (`one_run_takes_every_row_in_grid_order`; `:182`, whose outcome order becomes r1c1, r2c1, r1c2;
  `:544 a_refusal_after_the_act_began_names_the_reconcile_step`, now exit 3), `profile_verbs/apply/rig.rs:462 EVERY_ROW` (becomes
  a, d, b, c, e, f) and its doc lines 422 and 461 (this file is on the parked #835 branch: touch only those lines),
  `profile_verbs/apply/dry_run.rs:20,63` (wording "in grid order"), and the comments at `profile_apply_scenario/probe.rs:133`
  and `scenario.rs:238` (their outcomes do not change: every scratch workspace there has one cell per column). **Not**
  `fake_herdr_test.rs:536`. The scratch rig's `WORKSPACES` doc (`pane_integration/scratch/mod.rs:97`, "each 2 rows by 1
  column") is updated for the new 2x2 entry; if the test side needs an extent, parse it from `WORKSPACES`.
- **New tests go in a new file** under `profile_verbs/apply/` (e.g. `order.rs`), registered in `profile_verbs/apply.rs`, not in
  `rows.rs` (594 lines) or `rig.rs` (775 lines). Acceptance 1 tests the key in an in-file `#[cfg(test)]` module of `tx_apply.rs`
  or through a dry run on the rig; it does not widen the API.
- **Docs:** `docs/adr/ADR-0021.md`, `docs/pane-control.md`, `CHANGELOG.md`.
- **Overlap with other branches:** `issue-835-implementation` (parked) edits `docs/adr/ADR-0021.md`, `docs/pane-control.md`,
  `CHANGELOG.md` and `crates/holler-cli/tests/profile_verbs/apply/rig.rs`. This story does not touch `rig.rs`'s #835 lines; the
  doc edits are at different paragraphs, so a merge should be mechanical. Whichever lands second merges main in.
- **Size guards:** `tx_apply.rs` and the apply test files are well under 900 lines; T checks with `scripts/lint.sh`.
