# Handoff-A-dup: #652 - The operator guide for `holler pane`  (anti-duplication gate)

**Date:** 2026-10-11
**Branch:** `issue-652-implementation`
**Diff base:** `5d366b9` (origin/main merge base; F's own diff `8e0fbc1..3d318df`)   **Diff head:** `3d318df`
**Reuse map:** `docs/handoffs/652-brief.md` (Blast radius, "Files named by this brief", evidence E/R/C) and `docs/handoffs/652/handoff-A.md`
**Verdict:** PASS

## Summary

PASS: 0 `block`, 3 `warn`. F extended what the brief named and built no parallel path: the guide is one new page indexed
under the existing "Guidance" heading of `docs/README.md`; the obsolete "(interim)" section of `docs/orchestrating.md`
is rewritten in place, not copied to a second page; `scripts/pane-restart` changes only its line-3 comment; the CHANGELOG
gets one `[Unreleased]` entry. The guide links ADR-0021 section 9 (per-verb codes) and section 13 (the replacement table)
and the module doc of `wiring.rs` (the environment list) instead of restating them, as the brief required; both ADR anchors
resolve. The new test target follows `docs_cli_test`'s `[[test]]` cadence and reads every list from the code's tables. The
warns are one in-page duplicate of the `profile apply` contract, one hand-mirrored fence-tag list in the test, and the
carried-over #649 ordering.

## Findings

| # | Severity | File:line | Finding | Suggested fix |
|---|---|---|---|---|
| 1 | warn | `docs/pane-control.md:332-337` | The `holler profile apply NAME` subsection of section 3 restates the reference of section 6 (lines 456-497): the step shape, all six action and eight outcome words, the open codes and the exit codes. Two copies of one contract in one page can drift; acceptance 15 reads section 3's subsection and section 6 concatenated, so it passes with the words in section 6 alone. The brief's section 3 asks for purpose, flags, an example and exit codes per verb, and points to "The `profile apply` reference" for the rest. | Cut lines 332-337 to the exit-code sentence and a pointer to section 6's reference (a later edit; not blocking). |
| 2 | warn | `crates/holler-cli/tests/docs_pane_control_test.rs:43` | `SCANNED_TAGS` (and the `fences()` parser) hand-mirror the tag list of `docs_cli_test::extract` (`docs_cli_test.rs:101`). Acceptance 12 ("nothing escapes the docs test") is only true while the two lists agree; a tag added to or dropped from `docs_cli_test` leaves this test checking the old set with no failure. `workspace_root()` is another per-file copy too, but that is the established pattern (`docs_cli_test.rs:29`, `dev_scripts_test.rs:32`, the wizard tests), so it is not flagged. | Follow-up: move the scanned-tag list into the shared `tests/fixture/mod.rs` (already used by `docs_cli_test` for `split_args`) and have both targets read it. `fixture/mod.rs` is outside this story's Blast radius, so not in this change. |
| 3 | warn | `docs/pane-control.md:67-79`, `:369-371`, `:628-629` | Carried over from Phase 3 warn 3: the #649 statements (no hub gives `unavailable`; read verbs need no Herdr; `HOLLER_HERDR_SESSION` and the `wiring.rs` module doc; DRIVEN prints `-`; "the dry run cannot run until #649 has merged") are written from the open #649 branch, and no test reads #649's code. Not duplication; re-flagged so it is not lost. | S checks them against merged `wiring.rs`; after #649 merges, re-cut R1 to R3 and drop section 10's second known fact. |

No duplication of the analogous objects: no near-copy of the docs-test extraction (the new test delegates command parsing
to `docs_cli_test`, as the brief says), of the `docs_errors_test` slicer beyond its documented precedent, or of any
`holler-pane` table; the error-code, finding-kind, import-finding and column tables in the guide are required by
acceptances 2 to 4 and 14 and are pinned to `ALL_CODES`/`class_of`, `FindingKind::ALL`, `ImportFinding::ALL` and
`COLUMNS`. T-green's repair of `from_heading` (`docs_pane_control_test.rs:68-76`) stays inside the new file. Files are
within size limits (guide 725 lines, test 513).

## Notes for F

PASS: none required. Findings 1 and 2 are optional tidy-ups; finding 3 is S's check.
