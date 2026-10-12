# Handoff-A-dup: Phase 7 - `profile apply` creates column 1 first, and a real run with a refused step exits 3  (anti-duplication gate)

**Date:** 2026-10-12
**Branch:** issue-853-implementation
**Diff base:** `ff981f1` (A's round-2 PASS; merge base with `main` is `b76f4d2`)   **Diff head:** `a8b3048` (T-green)
**Reuse map:** `docs/handoffs/853/handoff-A.md`, "Reuse & Analogous-Feature map" (A-derived; no O survey in this automated run)
**Verdict:** PASS

## Summary

PASS: 0 blocks, 6 warns. F extended every object the map named, and built no parallel path.

- **The order.** `cell_key` is still the only place the order lives, and it stays private. Its key is now
  `(workspace, col != 1, row, col)`. There is no second order function and no public order API.
- **The exit decision.** `refused()` sits beside `stopped()`. It reuses the details `reconciled()` already built, plus
  `quoted` and `ErrorCode::from`. `reconcile_step` is still called only from `reconciled()`. The verb holds no exit
  table, and the 3 comes from `class_of`. `tx_apply::Step` gets no new field.
- **The fake.** `check_split` is tightened in place, using two private helpers on the fake's private `Workspace`
  (`last_row`, `end_of`). There is no new double. The test kit's `Cargo.toml` is unchanged, and the adapter's
  `Cargo.toml` change is a comment only.
  - The fake's rule restates the planner's rule on purpose: the brief's Acceptance 2 asks for it, and ADR-0021
    section 5 bars the test kit from depending on the adapter.
  - `the_split_only_fake_accepts_a_cell_exactly_when_the_planner_plans_it` ties the two together over every grid with
    no hole, up to 3 by 3. That makes the copy a reviewed decision, not drift.
- **T's tests.**
  - There is one refused-form helper in `profile_verbs`: `assert_refused`, `Refusal` and `assert_carries` in
    `refused.rs`. It is `pub(crate)`, built on `envelope_of`, and called from 8 tests.
  - C25 was moved out of `rows.rs`, not copied.
  - The cross-check reuses `plan_splits_test.rs`'s `map_of` and `target`. `order.rs`'s `spec_in` builds on `new_spec`.
- **No drift in rework.**
  - No new public item in production code.
  - No wire, protocol or golden-file change.
  - ADR-0021, ADR-0003 and the guide change in the same branch.
  - Every touched source file is under 800 lines. The largest is `rig.rs` at 776, and only 4 of its lines changed.
  - None of the overlay's Phase 7 candidates (token store, `Lockout`, `Roster`, `log`, `Hub`/`Body`/`mint_token`/
    `join`/`wait_for`/`StateDir`) is touched or copied.

The warns are small repeats on the test side, plus one merge-order carry-over (warn 6), which the merging agent needs
to know about.

## Findings

| # | Severity | File:line | Finding | Suggested fix |
|---|---|---|---|---|
| 1 | warn | `crates/holler-cli/tests/pane_integration/scratch/mod.rs:102` | `workspace_extents()` is a second parser of the `label=RxC` grammar. The rig passes the same `WORKSPACES` constant to the binary as `HOLLER_HERDR_WORKSPACES` (`:263`), and the binary parses it with `holler_cli::pane::wiring::env::herdr_config`. That function is public and already called from tests (`pane_integration/env.rs:118`), and it returns the same `HerdrConfig` type. So `herdr_snapshot` (`:498`) builds its Herdr config one way and the binary under test builds it another. This is not a block: the brief said "parse it from `WORKSPACES`", and #649's code already parsed the labels locally. | Build `herdr_snapshot`'s config with `env::herdr_config`, over a lookup that answers the scratch session, socket and `WORKSPACES`, then drop `workspace_extents()`. This can be a quick in-PR fold or a follow-up. |
| 2 | warn | `crates/holler-cli/tests/profile_apply_scenario/plan.rs:203` | `assert_refused_run` restates the JSON half of `assert_refused` (`profile_verbs/apply/refused.rs:37`): exit 3, `data` null, the first refused step's code, and the head `apply refused <N> step(s): ...; <M> other step(s) ran`. The map called for one refused-form helper, and there are now two, one per test binary. Accepted: the `profile_apply_scenario` binary includes only `support`, `pane_integration/out.rs` and `scratch` by `#[path]`, and `refused.rs` depends on the fake `Rig`. #667 set this pattern: `assert_plan_agrees_with_show` mirrors the fakes' `a_dry_run_and_profile_show_agree` in the same `plan.rs`. Each binary defines its helper once, and no test copies it. | No action. If the head form changes, update both helpers. |
| 3 | warn | `crates/holler-cli/tests/profile_apply_scenario/grid.rs:30,76-86` | `listed()` and the `recorded` list repeat the projection that `Scratch::assert_end_state` computes inline (`scratch/verbs.rs:105-120`): Herdr's snapshot as `(pane id, cell)` for some workspaces, and the records read the same way. The equality at `:86` is the same claim `assert_end_state` makes at `:98`. This is not a parallel path: the test needs the reading at three points (empty, then after each apply), and `assert_end_state` does not expose it. | Optional: add a `Scratch::herdr_cells(workspaces)` accessor and use it in both places. |
| 4 | warn | `crates/holler-cli/tests/profile_verbs/apply/rows.rs:189-194` | In the row-a test (`:174`), the `seed` closure repeats the fixture already built at `:175-177`. The sibling `a_failing_probe_...` (`:286-301`) defines `seed` once and then calls `let rig = seed()`. | Fold it the same way, and read `pane-a`'s record from the seeded rig. |
| 5 | warn | `crates/holler-adapter-herdr/tests/plan_splits_test.rs:394` | `fake_spec` is the third `HerdrSpec` builder written per file in the adapter's tests (the others are `adapter_test.rs:62` and `adapter_messages_test.rs:32`). `tests/common/mod.rs` has `cell` but no spec builder. Writing it per file is the crate's existing pattern. | No action now. Later, a `common::spec(workspace, row, col)` could replace all three. |
| 6 | warn | `docs/adr/ADR-0021.md:1538,1544` | Carried over from handoff-A warn 9, with a larger scope. #835's PR #860 is open, has passed S and is mergeable, and it also adds ADR-0021 decisions **30** and **31**. #853 now cites its two numbers in 11 places: ADR-0021 `:739`, `:741` and `:857`, `CHANGELOG.md:599`, `profile/apply.rs:12,150`, `order.rs:2,28,95`, `refused.rs:2` and `plan.rs:198`. #835 cites its numbers nowhere outside the list itself. | Land #853 first (it gates the release) and have #835 renumber its two headings. If #860 merges first, renumber all 11 cites here and re-run the `docs_*` tests. |

No duplication of production code, and the extensions are clean.

## Notes for F

None required: the verdict is PASS. Warns 1, 3 and 4 are optional test-side folds, which T can make before the PR or
file as follow-ups. Warn 6 is for whoever merges.
