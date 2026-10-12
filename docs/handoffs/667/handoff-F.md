# Handoff-F: Phase 6 (script numbering; the pipeline doc's Phase 5) - #667 profile apply on scratch Herdr, tmux and OpenCode

**Date:** 2026-10-11
**Branch:** issue-667-implementation (worktree `.claude/worktrees/0667-apply-scenario`, head `6739845` before this phase)
**Issue:** #667

## What was done

This story has no production code (brief decision 13). A's W-1 (binding) gives F two documentation edits. That is #652's precedent, where F wrote the guide and its CHANGELOG entry. Nothing under `crates/` was touched.

- `CHANGELOG.md`: one new entry, first under `## [Unreleased]` / `### Enhancements`. It uses decision 14's text unchanged and wraps at about 120 columns, like the entries next to it. It names:
  - the epic #633 and the story #667, each linked;
  - the opt-in run line of decision 4: `HOLLER_TEST_PANE_INTEGRATION=1 cargo test -p holler-cli --test profile_apply_scenario -- --ignored --test-threads=1`;
  - the reuse of #649's scratch set, whose helpers moved into one place;
  - "Tests only: no behaviour change."
- `docs/pane-control.md` section 12 ("Later"): deleted the one bullet `- the real-adapter scenario for \`profile apply\` (#667);` (was line 792), because the scenario now exists. The other six bullets and the list's punctuation are unchanged: each line still ends with `;` and the last with `.`.

## Design decisions

- **Where the entry goes:** first under `### Enhancements`. The section is newest first: #652's entry, the latest one added, is at the top today, with #649's below it.
- **The text:** decision 14's proposed text, word for word. A's W-1 says to use "decision 14's text".
  - *Considered:* linking the bare `#649` and adding a sentence about the new `--herdr-session` guard.
  - *Rejected:* the brief decided the text. Bare `#NNN` references already appear in this section (`(#829)`, `(#654)`, `(#648)`). The guard is test-side and changes nothing for an operator.
- **The guide:** only the one bullet. A's nit on `docs/testing.md` (it documents #640's opt-in run but not #649's or this one) is left to #666 or a docs follow-up, as A says. ADR-0021 lines 1030-1031 already describe the scenario (brief E60), so it needs no edit.

## Reuse / extend-vs-new

Nothing new was built. The CHANGELOG entry follows #649's own mention of its opt-in scenario (brief E72). The guide edit is a deletion only.

## Architecture notes for A

None. Two documentation files changed. No code, manifest, layer, dependency, schema or contract changed. `archChanged: false`.

## Deviations from spec / wireframe

None. There is no wireframe (no UI surface).

## Tier 1 self-check (incl. tests now GREEN)

| Command | Result |
|---|---|
| `bash scripts/changelog-check.sh` | `changelog-check: ok`, exit 0 |
| `cargo test -p holler-cli --test docs_pane_control_test --test docs_cli_test --test profile_apply_scenario --test pane_integration` | `docs_cli_test`: `ok. 3 passed; 0 failed; 0 ignored`; `docs_pane_control_test`: `ok. 16 passed; 0 failed; 0 ignored`; `pane_integration`: `ok. 68 passed; 0 failed; 4 ignored ... finished in 119.04s`; `profile_apply_scenario`: `ok. 5 passed; 0 failed; 2 ignored` |
| `bash scripts/lint.sh` | exit 0. Its 57 `warn:` lines are pre-existing file-size warnings, and none names a file of this story. The golden-diff summary printed nothing. |
| `cargo clippy -p holler-cli --all-targets -- -D warnings` | exit 0 |

**What these results cover:**
- T's pins are GREEN: `profile_apply_scenario` 5 of 5, and the 2 new guard pins inside `pane_integration`'s 68. The counts match `handoff-T-red.md`'s restored state.
- The two edited files are read by one test only, `docs_pane_control_test`. No test reads section 12 of the guide: no `section(&doc, 12)` call exists. A search of `crates/`, `plugins/` and `scripts/` for `CHANGELOG` and `pane-control` finds only `docs_pane_control_test.rs`, `scripts/changelog-check.sh` and comments.

**Not run here:**
- The opted-in scenarios. They are T-green's (A1's two consecutive runs, and A16's mutants). T-red ran them once, and this phase's edits cannot reach them.
- The full workspace suite. The script's orchestrator green check runs it after this phase.

## Evidence appendix

`docs/handoffs/667/evidence.md` has two entries:
- what `scripts/changelog-check.sh` requires;
- the `docs_pane_control_test` CHANGELOG pin, which the new entry, placed above the #652 line in the same `[Unreleased]` section, keeps satisfied.

## Tests that look wrong (for T)

None.

## Known issues

None of F's.
- Decision 10's creation-order hazard (the brief's Question 1) and Question 3 (naming) stay the operator's. A recorded them in `handoff-A.md`, "Notes for O", item 4.
- W-9 (record on #667 that the brief supersedes the issue's harness, dependency and blast-radius text) is O's or the operator's.

## Files changed

- `CHANGELOG.md`
- `docs/pane-control.md`

(Pipeline files of this phase: `docs/handoffs/667/handoff-F.md`, `docs/handoffs/667/evidence.md`, and an appended entry in `docs/handoffs/667/decisions.md`.)
