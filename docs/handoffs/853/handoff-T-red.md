# Handoff-T-red: #853 - `profile apply` creates column 1 first, and a real run with a refused step exits 3

**Date:** 2026-10-12
**Branch:** issue-853-implementation (on `ff981f1`, A's round-2 PASS)
**Brief / wireframe reviewed:** `docs/handoffs/853-brief.md`, `docs/handoffs/853/handoff-A.md` (round 2), the amendment in
`docs/adr/ADR-0021.md` section 8 and `docs/pane-control.md` section 6. No wireframe (no UI surface).

## A precondition

Confirmed: A returned PASS on the plan (round 2, 0 blocks, 9 warns). I followed warns 1 to 3 and 6 to 8, which A
addressed to T:

- I edited nothing under `crates/*/src`.
- Acceptance 1 is tested through a dry run on the rig.
- The refused-form assertions use the forms A worked out.
- I wrote one refused-form helper.
- The cross-check is in `plan_splits_test.rs`.

## Tests authored

All new and changed tests are under `crates/*/tests/`. Production code and the test kit's `src/` are untouched.

### New

| Test | Pins | Tier | Why this tier |
|---|---|---|---|
| `profile_verbs/apply/order.rs` `steps_take_each_workspaces_column_1_by_row_then_the_rest_by_row_and_column` | **Acceptance 1**, the order. P lists the specs scrambled across two workspaces (`main`, `side`). The steps come out as each workspace's column-1 cells by row, then its other cells by row and column, with workspaces in their own order. Two specs of one cell (`dup-b` listed before `dup-a`) keep P's order. | engine on the rig, dry run | `cell_key` is private, and T cannot write in `holler-pane/src` (A warn 2). A dry run calls no `ensure_pane`, so any workspace works. |
| `profile_verbs/apply/order.rs` `a_row_below_a_row_of_two_live_panes_is_refused_and_the_run_exits_3` | **Acceptance 4, the limit that stays.** On `SplitOnly`, with a live r1c1 and r1c2, the create of `pane-n` at r2c1 is `refused` (`grid-unreachable`). Nothing is recorded and no Herdr pane is made or moved, and the other two steps still run, in the order p, n, q. The verb exits 3 with `error.code` `grid-unreachable`, and its one line carries n's detail, which ends with the reconcile step (an acted refusal, A warn 3). | engine plus verb on the rig | This is where the fake's rule and the verb's exit code meet. |
| `profile_verbs/apply/refused.rs` `assert_refused` and `Refusal`, plus `assert_carries` | The one check of the refused form (A warn 7), the twin of `assert_stopped`. JSON: exit 3, `data` null, `error.code` the first refused step's, and the message starting with exactly `apply refused <N> step(s): "<pane>" (<code>), ...; <M> other step(s) ran`. Text: exit 3, empty stdout, and exactly one `error: <same message>` line. It returns the rest of the message. `assert_carries` checks that the rest is `; "<pane>": <engine detail>`, ending with the reconcile step, once. | helper | It is defined once, `pub(crate)`, and built on `envelope_of`. |
| `refused.rs` `a_refusal_after_the_act_began_carries_its_detail_and_the_reconcile_step` | **Acceptance 4, acted refusal (A's finding 1).** C25, moved here from `rows.rs:544` and rewritten for exit 3. It covers a `create` and a row-f `relaunch` refused by `verify_serving`. The engine's order is a, z, x. The verb exits 3 coded `pane-in-other-profile` (the first refused step) and carries x's detail with the reconcile step. Row a adds nothing beyond its pane and code. | engine plus verb | Same as before; it now needs the helper. |
| `refused.rs` `an_acted_refusal_after_a_take_over_carries_its_note` | **Acceptance 4, the take-over note.** `pane-x` of Q is taken over, then its relaunch act is refused. The engine's detail holds `joined "demo" before the relaunch`, and the verb's one line carries that detail and the reconcile step. A's warn 4 case (a relaunch refused before its act) is left to F and O, as A said, and is not tested. | engine plus verb | |
| `refused.rs` `a_failed_step_beside_a_refused_one_still_exits_1` | **Acceptance 4.** A `failed` step beside a refused one still exits 1 with the failure alone (`assert_stopped`). | engine plus verb | It guards the decision's order. **It passes at RED by design** (exit 1 holds today). |
| `holler-adapter-herdr/tests/plan_splits_test.rs` `the_split_only_fake_accepts_a_cell_exactly_when_the_planner_plans_it` | **Acceptance 2, the cross-check (A warn 8).** It covers every grid with no hole, up to 3 rows of up to 3 panes, plus the empty workspace. For each free cell of a 3-by-3 extent and the ring outside it, the fake's `ensure_pane` (fresh fake per probe) is accepted exactly when `plan_splits` plans one step, and otherwise refused with the planner's code. | adapter dev-test | The test kit cannot depend on the adapter; the adapter already dev-depends on the test kit, as `ac13` does. |
| `holler-pane-testkit/tests/fake_herdr_test.rs` `split_only_mode_adds_a_row_only_below_a_last_row_of_one_pane` | **Acceptance 2, the fake's own rule.** With r1c1 and r1c2 placed, r2c1 and r2c2 are both `grid-unreachable`, the message names the cell, and nothing is created. Below a row of two, r3c1 is refused, and a cell right of its row's last pane is accepted. | the fake's own test | It pins the fake's rule without the adapter. |
| `profile_apply_scenario/grid.rs` `apply_builds_a_whole_2x2_grid_from_an_empty_workspace` (`#[ignore]`, opt-in) | **Acceptance 3.** The spike as a scratch test. `hlr649sq` (2x2, new in `WORKSPACES`) starts empty. A profile of four spec-only panes, listed in reverse, is applied. All four are created, in step order g11, g21, g12, g22, and Herdr's snapshot lists each at its recorded cell. A second apply adopts all four and makes nothing. The run ends with the end state, the tmux env and the credential checks. | scratch (real Herdr 0.9.1, tmux, OpenCode) | The brief asks for it; only real Herdr shows the nesting. |

### Changed (the old order, or exit 0 beside a refused step; each still proves what it proved)

- **`rig.rs` `EVERY_ROW`** becomes a, d, b, c, e, f. Its doc lines 422 and 461 are updated, and the file is now 776
  lines.
- **`rows.rs`:**
  - `one_run_takes_every_row_in_grid_order`:
    - The engine's order is the new one.
    - The live verb now asserts the refused form: rows a and e, `ran: 4`, nothing carried.
    - The JSON step order and C22's action strings and keys come from `--dry-run`, which exits 0 with them. The action
      strings are now `refuse, report, create, adopt, refuse, relaunch`.
  - `row_a_...`, `a_failing_probe_...` and `a_per_pane_usage_...` now use `assert_refused`, and each asserts that
    nothing is carried (none acted).
  - `row_b_missing_panes_are_created_by_splits_in_grid_order` (Acceptance 2 on the fakes) is now the whole 2x2 grid, in
    the order r1c1, r2c1, r1c2, r2c2.
  - `apply_twice_...`: the first run's order becomes c, b, n, f. The brief's blast radius missed this one.
  - The C25 test moved to `refused.rs`.
  - The module doc is updated. The file is now 556 lines.
- **`failures.rs` `a_failed_health_for_a_pane_row_a_refuses_is_not_a_failure`:** exit 3 via `assert_refused`, and its doc
  is updated.
- **`dry_run.rs`:** the wording "in grid order" in two docs; the dry run's exit 0 beside refused steps is noted.
- **`fake_herdr_test.rs` `split_only_mode_refuses_an_absolute_placement`:** the closing sequence becomes rows first
  (r2c1, r1c2, r2c2). It passes on both the loose and the tightened fake.
- **`plan_splits_test.rs` `ac12`:** comment only. The sequence it names is the one the fake accepted before #853.
- **The scratch scenario:**
  - `plan.rs` gains `assert_refused_run`, the reader of a refused run's failure, since its steps are no longer data.
  - `probe.rs` (i) and (ii) read the order and rows from a dry run, then assert the real run exits 3 with
    `[(Y, probe-failed)]` (ran 2) and `[(Z, probe-failed), (Y, probe-failed)]` (ran 1). Nothing is carried, since the
    probes acted nothing.
  - `scenario.rs` step 5 is split into `step_5_refused`, to stay under clippy's cognitive-complexity limit. It reads A1
    and A2's refusals from the dry run, then the exit-3 one line. The comment at line 238 is updated.
  - `pane_integration/scratch/mod.rs`: `WORKSPACES` gains `hlr649sq=2x2`, with a new `workspace_extents()`.
    `herdr_snapshot` uses each workspace's own extent, and its connect message names the protocol-22 need.

**Not touched:** `fake_herdr_test.rs:536` `snapshot_lists_panes_by_workspace_then_row_then_col`, as the brief requires.

### Proportionality

Before adding anything, I checked what existing tests already cover:

- **"A reported step alone exits 0"** is already pinned by `row_d_...` (`data_of` asserts exit 0).
- **"A dry run that plans a refusal exits 0 with the plan"** is already pinned by
  `dry_run.rs a_dry_run_prints_its_plan_as_the_steps_object` (every_row has refused rows a and e).
- **The 2x2 grid on the fakes** extends `rows.rs:182` rather than adding a second split-only test.

I added no new test for any of these.

## RED confirmation

`cargo test -p holler-cli --test profile_verbs`: **105 passed, 13 failed**. Each failure is an assertion about the
missing behaviour:

| Test | Fails at | Why it is the right reason |
|---|---|---|
| `order::steps_take_each_...` | `order.rs:56` | left `[main-r1c1, main-r1c2, main-r1c3, main-r2c1, main-r2c2, dup-b, dup-a, side-r1c1, side-r1c2, side-r2c1]`, right `[main-r1c1, main-r2c1, dup-b, dup-a, main-r1c2, main-r1c3, main-r2c2, side-r1c1, side-r2c1, side-r1c2]`: the old `(workspace, row, col)` key |
| `order::a_row_below_a_row_of_two_...` | `order.rs:106` | left `[(pane-p, Adopted), (pane-q, Adopted), (pane-n, Created)]`: the old order, and the loose fake creates r2c1 below a row of two |
| `rows::row_b_missing_panes_...` | `rows.rs:223` | left r1c1, r1c2, r2c1, r2c2 (all `Created`), right r1c1, r2c1, r1c2, r2c2 |
| `rows::one_run_takes_every_row_in_grid_order` | `rows.rs:35` | left a, b, c, d, e, f, right a, d, b, c, e, f |
| `rows::apply_twice_...` | `rows.rs:501` | left c, n, f, b, right c, b, n, f |
| `dry_run::a_dry_run_calls_only_its_reads_...` | `dry_run.rs:32` | old order against `EVERY_ROW` |
| `dry_run::a_dry_run_prints_its_plan_...` | `dry_run.rs:78` | old order in the JSON `steps` |
| `refused::a_refusal_after_the_act_began_...` | `refused.rs:138` | left a, x, z, right a, z, x (the order; the exit-3 part follows) |
| `refused::an_acted_refusal_after_a_take_over_...` | `refused.rs:43` | `a refused step exits 3`: left 0, right 3. The engine's detail already holds the take-over note, so the precondition passed. |
| `rows::row_a_...`, `rows::a_failing_probe_...`, `rows::a_per_pane_usage_...`, `failures::a_failed_health_for_a_pane_row_a_refuses_...` | `refused.rs:43` | `a refused step exits 3`: left 0, right 3. The output is the old `ok:true` envelope with the steps. |

The other two RED runs:

- **`cargo test -p holler-pane-testkit --test fake_herdr_test split_only`:** 4 passed, 1 failed.
  `split_only_mode_adds_a_row_only_below_a_last_row_of_one_pane` fails at `fake_herdr_test.rs:233` with
  `r2c1: expected grid-unreachable, got Ok(HerdrPane { ... grid: r2c1 ... })`. The fake still accepts any cell with a
  pane above it.
- **`cargo test -p holler-adapter-herdr --test plan_splits_test`:** 19 passed, 1 failed. The cross-check fails with
  `r2c1 on the grid [2]: the planner answered Err(Refused { code: "grid-unreachable", ... "place rows before columns" }),
  the fake Ok(HerdrPane { ... r2c1 ... })`.

**Acceptance 3, run for real.** I ran it opted in with Herdr 0.9.1 first on PATH:

```
PATH=$HOME/.local/bin:$PATH HOLLER_TEST_PANE_INTEGRATION=1 cargo test -p holler-cli --test profile_apply_scenario grid:: -- --ignored --test-threads=1
```

This machine's default `herdr` is 0.8.2 (linuxbrew), and the 0.9.1 build is in `~/.local/bin`. The test failed
exactly as the spike did:

- g11 and g12 were `Created`;
- g21 and g22 were `Refused` with `grid-unreachable`, detail "r2c1 cannot be reached: a row can be added only below a
  row of one pane, and row 1 has 2 panes ...; place rows before columns; to reconcile, run holler pane doctor
  --profile 'hlr649-grid' ...";
- the assertion: left `(Create, Refused)`, right `(Create, Created)`.

The scratch set tore itself down: no `holler649`/`hlr649` process was left. The detail ending with the reconcile step
confirms A's warn 3 on real Herdr: a create's `grid-unreachable` is an acted refusal.

**Not run at RED:** the opted-in `probe` and `scenario` tests, which take several minutes. They build and are listed as
ignored. With today's code they would stop at their exit-3 assertions (`json(.., 3)`).

**These pass at RED, as expected:**

- `a_failed_step_beside_a_refused_one_still_exits_1`, a guard on behaviour that does not change.
- `fake_herdr_test` `split_only_mode_refuses_an_absolute_placement`, which passes on both fakes.

**At Phase 6:**

- Per A warn 1, I will show that the 2x2 and order tests depend on the order by reverting `cell_key` locally on F's
  tightened fake.
- I will run `probe` and `scenario` opted in.

**Other checks:**

- `cargo clippy -p holler-cli -p holler-pane-testkit -p holler-adapter-herdr --all-targets -- -D warnings`: clean.
- `bash scripts/lint.sh`: exit 0, and no changed file is near 900 lines; the largest is `rig.rs` at 776.
- `pane_claude` (43 ok), `pane_integration` (69 ok) and the test kit's other targets, including `herdr_conformance_test`
  (22 ok): unaffected.
- Not run: `cargo fmt --check`. CI does not run it, the workspace is not fmt-clean on this toolchain, and I kept the
  surrounding 100-column style.

## Ready for F

RED is valid. F may implement against these tests:

1. **`cell_key`:** the key `(workspace, col != 1, row, col)`, private.
2. **`profile/apply.rs`:** `refused()` beside `stopped()`, giving exit 3 and the one-line message described below.
3. **The test kit's `check_split`:** section 10's rule, plus its docs and `ASSUMPTION` (A warns 1 and 8).

The message, as the tests pin it:

- **The head**, exactly `apply refused <N> step(s): <quoted pane> (<code>), ...; <M> other step(s) ran`. `<M>` counts
  every step that is not refused.
- **For each acted refused step, in step order:** `; <quoted pane>: <detail as reconciled() leaves it>`.
- **`error.code`** is the first refused step's code.
- **The text form** is `error: <same message>` on stderr, with nothing on stdout.

Two notes for F:

- The fake's refusal message must name the cell (`r2c1`).
- The new `assert_carries` checks `rest.starts_with("; \"<pane>\": <engine detail>")`, so the carried detail must begin
  with the engine's `detail` unchanged.
