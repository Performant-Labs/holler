# Handoff-A-dup: #835 - `holler pane import` leaves Claude Code panes alone, with `--adopt-claude-code`  (anti-duplication gate, second pass: after S's REWORK)

**Date:** 2026-10-11 (20:46 MDT)
**Branch:** `issue-835-implementation`
**Diff base:** `d6ae539` (`origin/main` after `git fetch` at 20:48 MDT, and the merge base)   **Diff head:** `d2922a9`
**This cycle's diff:** `git show --remerge-diff e6d5fa0`, which is F's edits beyond the automatic merge of `ccc53a0` and
`d6ae539`. T-green's `d2922a9` changed handoffs only.
**Reuse map:** `docs/handoffs/835/handoff-A.md` (pass 4: "Reuse map" and its list of parallel paths to flag)
**Verdict:** PASS

This handoff replaces the first A-dup's (PASS, at `e5e35ee`).

## Summary

PASS: 0 `block`, 2 `warn`. This cycle has no code. F reworded prose and doc comments in place (S's REWORK 1) and merged
`main` (#851). Neither adds a parallel path, and neither changes an object of the map.

- **The cycle is prose only.** It touches five files outside `docs/handoffs`:
  - `CHANGELOG.md`;
  - the `PaneImport` doc, which is the help (`holler-cli/src/pane/import.rs`);
  - two `holler-pane` doc comments (`import.rs`, `import/observe.rs`);
  - the guide.

  In the three Rust files only comment lines changed: a filter of the diff for lines that are not comments finds none.
  Each edit replaces a sentence in place. There is no new section, table, helper or test, and no file outside this
  story's diff.
- **The merge is clean, and no file of `main` was edited.** `git merge-tree --write-tree ccc53a0 d6ae539` gives `e77e7a0`
  with no conflict, and `git diff e77e7a0 e6d5fa0` is exactly the remerge diff: F's 5 prose files and 2 handoffs. #851
  and #835 share three docs only, in separate places:
  - `CHANGELOG.md` has one `[Unreleased]` section, with both entries;
  - ADR-0003 keeps #851's header and version line beside #835's `pane import` row;
  - #851's 4 guide lines sit before section 10's fact.
- **The rest of the feature is the one the first A-dup passed.** `b488a33..ccc53a0` changed handoffs only, and the 31
  other feature files are byte-identical to `b488a33`. So the first A-dup's object-by-object table (at `e5e35ee`) stands.
- **The new warn is about the records.** The rework made the user-facing limits sentence name a case that the
  contract's limits do not. The second warn is the first A-dup's, carried unchanged.

## Findings

| # | Severity | File:line | Finding | Suggested fix |
|---|---|---|---|---|
| 1 | warn | `docs/adr/ADR-0021.md:973-976`; `crates/holler-pane/src/pane.rs:153-156` | **New, from this rework: the contract and its user docs now list the rule's limits differently.**<br>One limits sentence, with three cases, is now in the help (`pane/import.rs:73-76`), the guide's section 10 (`pane-control.md:705-707`) and the CHANGELOG (`:86-88`):<ul><li>`claude` has exited;</li><li>it is not in the foreground ("a wrapper script runs it, or it is suspended");</li><li>it was started under another name.</li></ul>ADR-0021 section 13's "The rule's limits" and `is_claude_code_command`'s doc keep two: exited, and "another `argv[0]` (a wrapper, or `node` ...)". So a suspended `claude` is named in the user docs only, and "a wrapper" sits under a different case in each. The guide links section 13 as "the contract", so a reader who follows the link finds the shorter list.<br>This is not a contradiction. Both statements are true, and the ADR's definition of the read (`:953`, the foreground process of each live pane) implies the missing case. F's design decision 4 records leaving the ADR and `pane.rs` as they are, on S's reading that they are exact. | Optional, and not needed for this PR; O decides. If another cycle runs, or as a follow-up, add one clause to ADR-0021's "The rule's limits" bullet and to `is_claude_code_command`'s doc. For example: "nor is one whose `claude` is not the pane's foreground process (a wrapper script that stays in the foreground, or a suspended job)". Then every place names the same cases. |
| 2 | warn | `crates/holler-cli/tests/pane_verbs/import/rig.rs:258` | **Carried from the first A-dup, unchanged by this cycle (test-only).** `ImportRig::run_with_host` copies `run_with_store` (`:237`) line for line and changes only which port it replaces. A smaller twin: `text_row` (`tests/pane_integration/observe.rs:279`) is the import rig's `text_line` (`rig.rs:528`) in another test binary. | Optional, T's, and not needed for this PR: one private rig method that takes the port overrides, called by both; `text_line` could move to `verb_harness`, which both test binaries include through `#[path]`. |

No duplication of production code. The extension is clean, and the rework added no drift in code.

## This cycle, against the map

| The map's item | In this cycle | Parallel path? |
|---|---|---|
| **The guide (rework row).** Section 3's subsection and sections 1, 10 and 12, edited in place. The map flags a second table of the import's findings, a guide section that restates ADR-0021 section 13, and any change to `main`'s files beyond the named sentences. | **Edited in place:** section 1 (`:29-31`), section 3's `harness-claude-code` row (`:299`) and section 10's fact (`:700-707`).<ul><li>The findings table still has one row per code.</li><li>Section 10 still links section 13 as the contract, and adds no port detail.</li><li>#851's guide lines are untouched.</li></ul>The fact gains no line: 12 before and after. | none |
| **The rule.** One constant and one predicate in `pane.rs`, and no comparison with `"claude"` anywhere else. | **Untouched.** A grep finds one production caller, `observe.rs:131`. Every other `"claude"` in `src` is a protocol harness id. The module doc of `import.rs` links the predicate and does not restate it. | none |
| **The observation.** The chain in `observe()`. | `read_tmux`'s doc only (`observe.rs:114-116`). The chain is unchanged. | none |
| **The switch.** Step 1 of `Input::read`. | The `PaneImport` doc (the help) only. `Input::read` is unchanged. | none |
| **Everything else in the map:** the port read, the adapter, the forwards, the fake and its pins, the tests. | Unchanged since `b488a33`. | none |

**Beyond the map:**

- **Hub and protocol.** No hub, body, protocol or golden file is in this story's diff. #851's code (`build_info` and the
  version string) comes from `main` through the merge. It is not this story's, and it is not in `git diff
  origin/main...HEAD`.
- **File sizes.** The three Rust files are 353, 220 and 244 lines. The guide is 821 lines: 817 from this branch, plus
  #851's 4 (S's advisory 1, enforced by no check). `lint.sh` passed at T-green.
- **Names.** The cycle's added lines have no personal host, account or network name (grep).

## Notes for F

None: the verdict is PASS.

## For S and the operator (outside this gate)

1. **The rule's limits are now written in five places, and no test pins any of them** (by S's choice): the help, the
   guide's section 10, the CHANGELOG, ADR-0021 section 13 and `pane.rs`. A later change to the rule, such as #831's,
   updates the four standing ones; the CHANGELOG entry is historical. This adds to the first A-dup's note 2, the
   "refused until #831" prose.
2. **F merged `main` in its own phase commit** (`e6d5fa0`), outside S's REWORK list (F's design decision 6). I checked it:
   no conflict, and no file of `main` was edited. The run's agent should name #851 (PR #854) in the PR's coexistence
   notes, as F's known issue 3 says.

## Patterns referenced

- `docs/handoffs/835/handoff-A.md` (pass 4): the reuse map and its list of parallel paths.
- `docs/handoffs/835/handoff-S.md` (REWORK 1) and `handoff-F.md` (cycle 3, design decisions 4 to 6).
- The automatic merge tree `e77e7a0` (`git merge-tree --write-tree ccc53a0 d6ae539`), compared with `e6d5fa0`.
- ADR-0021 section 13 (`:950-986`), `crates/holler-pane/src/pane.rs:140-159`, `crates/holler-pane/src/ports.rs:340-348`
  and `crates/holler-pane/src/import/observe.rs:1-132`.
- The first A-dup, at `e5e35ee`.
