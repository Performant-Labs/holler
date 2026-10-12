# Handoff-T-green: #853, cycle 2 - S's REWORK: `origin/main` merged, #853's ADR-0021 decisions renumbered to 32 and 33

**Date:** 2026-10-12, 07:27 MDT
**Branch:** `issue-853-implementation` at `0330fa0`. This is F's phase commit. It concludes the merge of `origin/main`
(`4e691d9`, #835) as a two-parent commit (`21b29a0`, `4e691d9`). `origin/main` is an ancestor of `HEAD`, and it has not
moved since.
**Issue:** #853
**Handoff-F reviewed:** `docs/handoffs/853/handoff-F.md` (cycle 2)
**Handoff-T-red:** `docs/handoffs/853/handoff-T-red.md`

This replaces cycle 1's T-green handoff, which is in git at `a8b3048`. Its removal checks, its opted-in results on
Herdr 0.9.1 and its test repair all still hold: since then, #853's production code has changed only in two doc comments
(below).

## What T changed (test comments only, no production code)

F flagged 7 test comments that still cited #853's old decision numbers (S's item 1 (c)). I renumbered them:

- `crates/holler-cli/tests/profile_verbs/apply/order.rs:2`, `:28` and `:95`: decision 30 becomes 32.
- `crates/holler-cli/tests/profile_verbs/apply/refused.rs:2`, `:31` and `:222`: decision 31 becomes 33.
- `crates/holler-cli/tests/profile_apply_scenario/plan.rs:198`: decision 31 becomes 33.

The diff is 7 lines in 3 files, all comments. No assertion and no test name changed.

**S's two done-checks now hold:**

- `git grep -n -i -E "decision(s)? (30|31)\b|\(30 and 31\)" -- crates docs/adr docs/pane-control.md CHANGELOG.md`
  finds nothing (exit 1).
- ADR-0021's "Decisions taken" reads 1 to 33, each once and in order. Decisions 30 and 31 are #835's (`:1584`, `:1587`),
  and 32 and 33 are #853's (`:1591`, `:1597`).
- Every #853 cite now names 32 or 33: `profile/apply.rs:12` and `:150`, ADR-0021 `:742`, `:744` and `:860`, and the 7
  test comments.

## GREEN confirmation

These ran on the merged tree, with my comment edits on top.

- `cargo test --workspace --no-fail-fast -- --skip roster_stays_accurate_under_concurrent_body_load` (the verify
  command): **167 binaries, 2,606 passed, 0 failed, 37 ignored, exit 0.** The load average was about 3.9.
- By binary:
  - `profile_verbs` 118;
  - `fake_herdr_test` 25, `herdr_conformance_test` 22, `plan_splits_test` 20;
  - `profile_apply_scenario` 5 passed and 3 ignored (opt-in).

**The tests still fail when the behaviour is removed.** I checked by diff, not by re-running the reverts:

- `git diff c1f7deb HEAD` over `profile/apply.rs`, `tx_apply.rs`, `tx_apply/` and the test kit's `herdr.rs` is two doc
  comment lines (decision 31 becomes 33).
- So the code that cycle 1's three removal checks reverted (`cell_key`, `refused()` and the `acted` filter) is
  byte-identical, and those results stand.
- The other 18 crate files that changed since `c1f7deb` all come from #835's merge.
- Against `main`, the PR's crate diff names the same files as it did before the merge.

## Tier 1 results

| Check | Command | Expected | Actual | |
|---|---|---|---|---|
| Workspace tests | the verify command above | all pass | 2,606 passed, 0 failed, exit 0 | PASS |
| Clippy | `cargo clippy --workspace --all-targets -- -D warnings` | clean | exit 0 | PASS |
| Lint | `bash scripts/lint.sh` | exit 0 | exit 0. The edited test files are 131, 245 and 386 lines. | PASS |
| Changelog | `bash scripts/changelog-check.sh` | ok | `changelog-check: ok` | PASS |
| Unused dependencies | `cargo machete` | none | none | PASS |
| Docs (S's item d) | `docs_pane_control_test`, `docs_cli_test`, `cli_surface_test` (in the workspace run) | pass | 19 (#835 added one), 3, 3 | PASS |
| Canary | `wire_selftest` (in the workspace run) | pass | 3 | PASS |
| Whitespace | `git diff --check` | clean | clean | PASS |

**Cross-check against F.** F's two full runs each had one flake: the `profile_rename_faults_test` inode assertion
(#665) and `remote_admin_test` (#847). Both are in code that neither #853 nor the merge touches. My run had neither, and
I got 2,606 passed where F got 2,605 plus 1 failure. That is F's total, so the results agree.

## Tier 2 results

| Check | Method | Result |
|---|---|---|
| Coverage per criterion | Unchanged since cycle 1. Each of the brief's 6 criteria maps to a test (below). | PASS |
| Test quality | The change is to comments only. Every test still names its behaviour, and the comments now cite the right decisions. | PASS |
| Old cites gone | S's grep (above) | PASS |
| Type safety, error handling, API contract | No code changed in #853's paths this cycle | PASS (unchanged) |
| Protocol and goldens | Nothing on the wire changed | N/A |
| Migration safety | No schema or record change | N/A |
| Security and secrets | No asserted log or message changed | PASS |
| Browser or Playwright | None in this repo | N/A |
| Evidence appendix | My edits rely on no new fact in unchanged code. F's cycle-2 entry covers decisions 30 and 31 belonging to #835. | Nothing to add |

## Acceptance criteria status

1. **The order, pure: PASS.** `order.rs` `steps_take_each_workspaces_column_1_by_row_then_the_rest_by_row_and_column`.
2. **The fix on the fakes: PASS.**
   - `fake_herdr_test.rs` `split_only_mode_adds_a_row_only_below_a_last_row_of_one_pane`;
   - `plan_splits_test.rs` `the_split_only_fake_accepts_a_cell_exactly_when_the_planner_plans_it`;
   - `rows.rs` row b, the 2x2 grid on `SplitOnly`.
3. **The fix on the scratch set: PASS.** `grid.rs` `apply_builds_a_whole_2x2_grid_from_an_empty_workspace`. It is opt-in,
   and it passed on Herdr 0.9.1 in cycle 1. I did not re-run it this cycle, because the code it exercises did not change.
4. **The limit and the exit code: PASS.**
   - `order.rs` `a_row_below_a_row_of_two_live_panes_is_refused_and_the_run_exits_3`;
   - `refused.rs`'s three tests;
   - `rows.rs` row d and `dry_run.rs`.
5. **The old order unpinned: PASS.** Unchanged since cycle 1.
6. **The text: PASS.** It now holds on the merged tree too. Every #853 cite names 32 or 33, and the ADR list holds each
   number once (S's item 1).

## Blocking issues

None.

## Advisory notes

- **rustfmt is not a gate here.** `rustfmt --check` flags three pre-existing long lines in `refused.rs` (`:47-48`, `:81`,
  `:192`), but those lines are not ones I edited. Across the workspace, `cargo fmt --check` flags 242 files, and neither
  CI nor `lint.sh` runs rustfmt. I left the lines alone, because S asked that nothing change beyond item 1.
- I did not take S's optional test-side advisories 5 and 6: A-dup's folds, the `pane_claude/mixed.rs` exit-3 assertion,
  and the `..._in_grid_order` renames. S's REWORK says the next S passes once item 1 is done and nothing else has changed.
  They stay open as follow-ups.
- **Advisory 1 (privacy in unpushed history)** is still the operator's call. A rewrite would now have to replay the
  merge.

T-green complete, no blocking issues. No UI surface — U is N/A, ready for S.
