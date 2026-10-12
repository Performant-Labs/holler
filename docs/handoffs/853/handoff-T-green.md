# Handoff-T-green: #853 - `profile apply` creates column 1 first, and a real run with a refused step exits 3

**Date:** 2026-10-12
**Branch:** issue-853-implementation (on `07a576c`, F's implementation)
**Issue:** #853
**Handoff-F reviewed:** `docs/handoffs/853/handoff-F.md`
**Handoff-T-red:** `docs/handoffs/853/handoff-T-red.md`

## GREEN confirmation

F's handoff flags no test as wrong, and every test authored at RED passes as written:

- `cargo test -p holler-cli --test profile_verbs`: **118 passed, 0 failed** (RED had 105 passed and 13 failed).
- `cargo test -p holler-pane-testkit --test fake_herdr_test --test herdr_conformance_test`: 25 and 22 passed.
- `cargo test -p holler-adapter-herdr --test plan_splits_test`: 20 passed, the cross-check included.

**The tests fail when the behaviour is removed.** These were local edits to production code, each restored afterwards
(`git diff` against HEAD is empty for both files):

| Behaviour removed | Change | Result |
|---|---|---|
| The new order (A's warn 1), with F's tightened fake left in place | `cell_key` returns `(workspace, false, row, col)` | 8 fail: both `order.rs` tests, `rows.rs` row b (the 2x2 grid), `one_run_...`, `apply_twice_...`, both `dry_run.rs` order tests, `refused.rs` `a_refusal_after_the_act_began_...` |
| Exit 3 | `refused()` disabled in `run` | 8 fail: every `assert_refused` test (`rows.rs` row a, failing probe, per-pane usage, `one_run_...`; `failures.rs`; both acted-refusal tests in `refused.rs`; the `order.rs` limit) |
| The detail is carried only for acted steps (A's warn 3) | `.filter(\|(step, _)\| step.acted)` replaced with `.filter(\|_\| true)` | 6 fail: the `rest == ""` assertions for row a, the probe, the usage rule and the failed health, plus `one_run_...` and the C25 test |

The tightened fake's rule was already shown to be needed at RED: on the old fake, the cross-check failed at `r2c1` on
the grid `[2]`.

**Acceptance 3 on real Herdr 0.9.1, opted in.** Herdr 0.9.1 is in `~/.local/bin`, and this machine's default `herdr` is
0.8.2. The command:

```
PATH=$HOME/.local/bin:$PATH HOLLER_TEST_PANE_INTEGRATION=1 \
  cargo test -p holler-cli --test profile_apply_scenario -- --ignored --test-threads=1
```

| Test | Result |
|---|---|
| `grid::apply_builds_a_whole_2x2_grid_from_an_empty_workspace` | **ok**. It failed at RED exactly as the spike did. |
| `probe::a_failing_probe_refuses_the_create_and_the_relaunch_and_changes_nothing` | ok. Its real runs exit 3 and the steps are read from the dry run. |
| `scenario::the_apply_scenario_runs_in_order_on_scratch_instances` | ok after one test repair (below). Step 5's exit 3 and its one line pass on real state. |

No `hlr649`/`holler649` process was left afterwards.

**The test repair (a test fix, no production code changed).** On the first opted-in run, `scenario.rs:407` (step 4,
which #853 did not touch) expected the detail `its harness server on port N did not answer`. Since `b76f4d2` (#857,
already on `main` before this branch), the engine says `its harness at port N did not answer`
(`tx_apply/observe.rs:133`). So this opt-in assertion has been stale on `main` since #857, and it stopped the scenario
before step 5, the #853 change. I updated the expected wording to the engine's current text, with a comment, and the
whole scenario then passed in 21.8 s.

## Tier 1 results

| Check | Command | Expected | Actual | |
|---|---|---|---|---|
| Workspace tests (the pipeline's verify command) | `cargo test --workspace --no-fail-fast -- --skip roster_stays_accurate_under_concurrent_body_load` | all pass | 167 binaries, 2594 passed, 0 failed, 37 ignored, exit 0 (the same counts F reported) | PASS |
| Clippy | `cargo clippy --workspace --all-targets -- -D warnings` | clean | clean, run again after the test repair | PASS |
| Lint | `bash scripts/lint.sh` | exit 0 | exit 0. No changed file is near 900 lines; `scenario.rs` is 570. | PASS |
| Changelog | `bash scripts/changelog-check.sh` | ok | `changelog-check: ok` | PASS |
| Unused dependencies | `cargo machete` | none | none | PASS |
| Docs | `cargo test -p holler-cli --test docs_cli_test --test docs_pane_control_test --test cli_surface_test` | pass | 3, 18 and 3 passed | PASS |
| Canary | `cargo test -p holler-cli --test wire_selftest` | pass | 3 passed | PASS |
| `--help` | `./target/debug/holler profile apply --help` | the new order and exit 3 | both stated | PASS |

**A flake, not this change.** My first workspace run hit 3 failures in `holler-cli --test interrupt_test`, panicking at
`support/warmup.rs:136`:

- the hub logged `conn_liveness_expired` 6 ms after the body connected;
- clippy was running at the same time, which loaded the machine;
- the diff touches no hub, body, proto or test-support code.

Three back-to-back reruns of `interrupt_test` passed (11 of 11 each), and so did the clean workspace rerun above. It
does not block this story; it fits the repo's history of liveness and timing flakes under load.

## Tier 2 results

| Check | Method | Result |
|---|---|---|
| Coverage per criterion | Mapped each criterion to its tests (below) | PASS |
| Test quality and proportionality | Each test names one behaviour. The removal checks above show they fail for the right reason. They sit at the cheapest tier: a dry run on the rig for the order, the fake's own test plus an adapter dev-test for the rule, scratch only for real Herdr. No duplicate was added (see handoff-T-red's proportionality). `a_failed_step_beside_a_refused_one_still_exits_1` passes at RED by design: it guards the order of the decision. | PASS |
| Old order unpinned | `git grep` for `(workspace, row, col)`, "workspace, row and column order" and "in grid order" in `crates`, the ADRs and the guide | PASS. What remains is `fake_herdr_test.rs:570` (`snapshot_lists_panes_...`, left alone as required), `tx_apply.rs:358` and an ADR history line, both of which describe the old order as the old one. |
| Type safety | In `refused()`, `steps.len() - refusals.len()` cannot underflow because the refusals are a subset of the steps, and `check_split` uses `checked_sub` at the `u16` edge | PASS |
| Error handling | Exit 1 beats exit 3. A dry run exits 0. Every refused step names its pane and code, and acted steps carry their detail. | PASS |
| API contract | The JSON failure envelope has `data` null and `error.code` the first refused step's. The text form is one `error:` line on stderr with an empty stdout (`assert_refused`). | PASS |
| Security and secrets | The scratch scenario keeps its credential and tmux-env checks, and it passed. No asserted log or message holds a secret. | PASS |
| Protocol and goldens | Nothing on the wire changes and no golden is touched | N/A |
| Migration safety | No schema or record change | N/A |
| Browser or Playwright | None in this repo | N/A |
| Public repo | F flagged that my RED decisions entry named the operator's live Herdr sessions by their machine-derived names. I replaced them with a neutral description (it is my own entry). `grep` for account names and home paths across every changed file finds nothing. | PASS |

**Evidence appendix.** I appended a `## T (verify, GREEN)` section to `evidence.md` with two facts in unchanged code
that my tests rely on, each with an excerpt I copied myself from source:

- the stable `sort_by` on `cell_key` (`tx_apply.rs:349`), which keeps two specs of one cell in P's order;
- the take-over note's wording (`tx_apply/run.rs:123-129`).

## Acceptance criteria status

1. **The order, pure: PASS.** `order.rs` `steps_take_each_workspaces_column_1_by_row_then_the_rest_by_row_and_column`.
2. **The fix, on the fakes: PASS.**
   - `fake_herdr_test.rs` `split_only_mode_adds_a_row_only_below_a_last_row_of_one_pane`;
   - `plan_splits_test.rs` `the_split_only_fake_accepts_a_cell_exactly_when_the_planner_plans_it`;
   - `rows.rs` `row_b_missing_panes_are_created_by_splits_in_grid_order`, the 2x2 grid on `SplitOnly`, which fails with
     the old key.
3. **The fix, on the scratch set: PASS.** `grid.rs` `apply_builds_a_whole_2x2_grid_from_an_empty_workspace`, opted in
   on Herdr 0.9.1.
4. **The limit and the exit code: PASS.**
   - `order.rs` `a_row_below_a_row_of_two_live_panes_is_refused_and_the_run_exits_3`;
   - `refused.rs`: the acted refusal, the take-over note, and a failed step that still exits 1;
   - a reported step alone exits 0 (`rows.rs` row d), and a dry run exits 0 with the plan (`dry_run.rs`);
   - on real state: `probe.rs` and `scenario.rs` step 5.
5. **The old order unpinned: PASS.** Every test in the blast radius was updated and passes. `fake_herdr_test.rs`
   `snapshot_lists_panes_by_workspace_then_row_then_col` is untouched.
6. **The text: PASS.** The `--help` text shows the new order and exit 3. The module docs, ADR-0021, ADR-0003, the guide
   and the CHANGELOG Bug Fixes entry are in the diff, and the `docs_pane_control_test`, `docs_cli_test` and
   `changelog-check` checks pass.

## Blocking issues

None.

## Advisory notes

- **`refused()` relies on a refused step always carrying an error.** A refused step with `error: None` would be left
  out of the list and counted in `<M>`. F's evidence shows `halted` guarantees the error, so this is fine today.
- **"Workspaces keep their order" means workspace-name order.** The key sorts by the workspace string, as it did before
  #853, and `order.rs` asserts `main < side`. That is unchanged behaviour, not a regression. If "their order" was meant
  as P's order of first appearance, that would be a separate contract question.
- **The `interrupt_test` liveness flake** under machine load (above) may be worth an issue if it recurs in CI.
- **The stale `scenario.rs` step-4 wording** came from #857. The scenario is opt-in and CI never runs it, so any other
  stale wording in it only shows on an opted-in run. All three opt-in tests pass now.

T-green complete, no blocking issues. No UI surface — U is N/A, ready for S.
