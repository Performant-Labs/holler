# Handoff-A-dup: Phase 7 - `profile apply` creates column 1 first, and a real run with a refused step exits 3  (anti-duplication gate, cycle 2)

**Date:** 2026-10-12, 07:33 MDT
**Branch:** issue-853-implementation
**Diff base:** `c1f7deb` (cycle 1's A-dup PASS)   **Diff head:** `c39699c` (cycle 2's T-green)
**Reuse map:** `docs/handoffs/853/handoff-A.md`, "Reuse & Analogous-Feature map" (unchanged; F's cycle-2 handoff names no new object)
**Verdict:** PASS

This replaces cycle 1's handoff, which is in git at `c1f7deb`. This cycle's own edits are measured against the clean merge
tree `c2941f8` (`git merge-tree --write-tree 21b29a0 4e691d9`), so #835's merged content is kept apart from F's and T's
edits.

## Summary

PASS: 0 blocks, 6 warns (1 new, 5 carried over unchanged). Cycle 1's warn 6 is resolved. This cycle built no object, so
it has nothing to duplicate, and the merge brought in nothing that #853 now duplicates.

- **This cycle's own edits are text only.** On top of the clean merge tree:
  - F renumbered 6 cites: `profile/apply.rs:12` and `:150`, ADR-0021 `:742`, `:744` and `:860`, and `CHANGELOG.md:615`.
    F also renumbered the decisions `30.` and `31.` to `32.` and `33.`, removed the blank line the merge left in the
    list, and applied S's advisory-2 sentence at ADR-0021 `:713`.
  - T renumbered 7 test comments: `order.rs` three times, `refused.rs` three times, and the scenario's `plan.rs` once.
  - No function, type, helper, flag or test was added or renamed.
- **#853's code is byte-identical to what cycle 1 passed.** `git diff c1f7deb HEAD` over `profile/`, `tx_apply.rs`,
  `tx_apply/`, the test kit and the adapter shows only the 2 doc-comment lines in `profile/apply.rs`. `cell_key` is
  still the only order function, and it is still private. `refused()` still sits beside `stopped()`, and `check_split`
  is still tightened in place.
- **The merge (#835, `4e691d9`) and #853 do not overlap in code.**
  - Both touch only four docs, and in different paragraphs (I compared the hunks). #835's merged version does not touch
    `profile_verbs/apply/rig.rs`, which the brief expected from the parked branch.
  - #835's new test helpers serve `pane import` and `pane list`: import's harness detection in
    `pane_verbs/import/harness.rs`, the `LostChain` host wrapper, `run_with_host`, and `pane_integration/observe.rs`'s
    `list_over_the_chain`, `unmanaged` and `text_row`. None repeats one of #853's (`assert_refused`, `Refusal`,
    `assert_carries`, `spec_in`, `assert_refused_run`, `listed`, `workspace_extents`, `split_only_fake`, `grids_up_to`,
    `fake_spec`), and none could replace one.
  - #835 adds no Herdr-config builder, no refusal-form helper and no exit decision. Its only CLI change near this code
    refuses `--adopt-claude-code` as `usage`, through the existing `not_implemented_message`.
  - #835 made `HerdrPort::observe_pane` required and left `ensure_pane` alone, the method #853's docs name.
- **Against the new `main`, the PR diff is #853's alone.** It covers the same 23 files outside `docs/handoffs/`, with
  the same per-file line counts as before the merge, except ADR-0021: 58 lines added instead of 59 (the blank line).
- **No drift in rework.**
  - ADR-0021's decisions run 1 to 33, each once: 30 and 31 are #835's, 32 and 33 are #853's.
  - Its section headings did not move, so every section cite points where it did before the merge.
  - The guide's `profile apply` reference and #835's cutover text do not contradict each other.
  - The CHANGELOG has one `[Unreleased]`, with #835 under Enhancements and #853 under Bug Fixes.
  - No wire, golden or `holler-proto` change.
  - No touched file is over 800 lines. The largest is `rig.rs` at 776, and this cycle does not touch it.
  - None of the overlay's Phase 7 candidates appears in the PR's added lines, and this cycle's added lines hold no
    personal infrastructure name.

## Findings

| # | Severity | File:line | Finding | Suggested fix |
|---|---|---|---|---|
| 1 | warn | `docs/adr/ADR-0021.md:765` | New in this review, but older than this cycle: the amendment (`1f4ab40`) cites "(section 14)" for "`pane import` exits 0 on a dry run and refuses only its real run". Section 14 is "Unmanaged panes". The rule is in section 13, "Migration from the launcher's files" (`:1088-1093`), and section 9's `pane import` row holds the codes (`:854`). Every other "section 14" cite in the ADR means unmanaged panes or `observe_pane`. The merge did not cause it: the headings are the same on `b76f4d2` and on the merged tree. A's two rounds, cycle 1's A-dup and S all passed it. It is a cross-reference slip, not duplication or drift. | Change "section 14" to "section 13" at `:765`. No test or script pins this text. It is one word in a doc, so it can ride the PR, or go to a follow-up if S's "nothing else has changed" is held to strictly. |
| 2 | warn | `crates/holler-cli/tests/pane_integration/scratch/mod.rs:102` | Cycle 1's warn 1, unchanged: `workspace_extents()` is a second `label=RxC` parser beside the public `wiring::env::herdr_config`. #835 touched neither file. | As in cycle 1: an optional fold, or a follow-up. |
| 3 | warn | `crates/holler-cli/tests/profile_apply_scenario/plan.rs:203` | Cycle 1's warn 2, unchanged: `assert_refused_run` is the scratch binary's twin of `assert_refused`. Accepted, because of the binary boundary and #667's precedent. Its doc comment now cites decision 33. | No action. |
| 4 | warn | `crates/holler-cli/tests/profile_apply_scenario/grid.rs:30,76-86` | Cycle 1's warn 3, unchanged: `listed()` repeats `assert_end_state`'s projection. | Optional: a `Scratch::herdr_cells` accessor. |
| 5 | warn | `crates/holler-cli/tests/profile_verbs/apply/rows.rs:189-194` | Cycle 1's warn 4, unchanged: the row-a test's `seed` repeats its own fixture. | Optional fold. |
| 6 | warn | `crates/holler-adapter-herdr/tests/plan_splits_test.rs:394` | Cycle 1's warn 5, unchanged: `fake_spec` is the third per-file `HerdrSpec` builder. #835 added none. | No action now. |

**Resolved:** cycle 1's warn 6, the decision numbers that collided with #835's. The list reads 1 to 33, each once.
All 13 of S's cites name 32 or 33, and S's grep finds nothing.

No duplication. The cycle's edits are text, and the merge is clean.

## Notes for F

None required: the verdict is PASS.
- Warn 1 is a one-word doc fix. Whoever runs the next cycle or the PR can take it.
- Warns 2 to 6 carry over as cycle 1 left them. S's advisory 5 already lists warns 2, 4 and 5 as follow-ups.
