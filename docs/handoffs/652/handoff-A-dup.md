# Handoff-A-dup: #652 - The operator guide for `holler pane`  (anti-duplication gate, cycle 2)

**Date:** 2026-10-11
**Branch:** `issue-652-implementation`
**Diff base:** `edbb863` (the merge of origin/main with #649; cycle 1 was `5d366b9..3d318df`, PASS)   **Diff head:** `b35ab04`
**Reuse map:** `docs/handoffs/652-brief.md` (Blast radius, "Files named by this brief", R1 to R3 as re-cut in `e9eb05c`) and `docs/handoffs/652/handoff-A.md`
**Verdict:** PASS

## Summary

PASS: 0 `block`, 4 `warn`. This cycle is F's rework after S's ADVISORY-HOLD and the #649 and #832 merges (`d698473`) and
T's test extension (`b35ab04`). It edits the same five files in place and adds no file, page, section or helper: the
#649 statements are re-cut into the existing section 2 and 4 text, #832's unmanaged panes are added to the sections that
already describe panes outside the registry, and the #829/#839/#835 caveats are each stated once in the guide and
referenced elsewhere by section number. The guide still points to the `wiring.rs` module doc for the environment list
instead of copying it. The test extension reuses the file's own `ticked`/`section` helpers and reads `pane-unmanaged`
from `holler_pane::unmanaged::PANE_UNMANAGED`. Cycle 1's warn 3 (#649 statements written from an open branch) is
resolved; warns 1 and 2 stand unchanged.

## Findings

| # | Severity | File:line | Finding | Suggested fix |
|---|---|---|---|---|
| 1 | warn | `docs/pane-control.md:359-377` | Carried from cycle 1: the `holler profile apply NAME` subsection of section 3 restates section 6's reference (step shape, all six actions and eight outcomes, the open codes, the exit codes). Two copies of one contract in one page. | Cut it to purpose, flags, example and a pointer to section 6 (also buys back lines, see 3). |
| 2 | warn | `crates/holler-cli/tests/docs_pane_control_test.rs:44` | Carried from cycle 1: `SCANNED_TAGS` hand-mirrors `docs_cli_test.rs`'s fence-tag list, so acceptance 12 silently goes stale if that list changes. | Follow-up: share the list through `tests/fixture/mod.rs` (outside this story's Blast radius). |
| 3 | warn | `docs/pane-control.md` (794 lines) | The guide grew from 725 to 794 lines this cycle, at the overlay's "about 800" flag line and 106 lines under `scripts/lint.sh`'s 900 hard fail. #665, #829, #835 and #839, all named in section 12, will each add text to it. | Not blocking (lint exits 0). The next story that adds a section should name where it goes or split a section (e.g. section 9's code table) into its own page; finding 1's cut is the cheapest headroom. |
| 4 | warn | `crates/holler-cli/tests/docs_pane_control_test.rs:500-509` | The new assertions type `` `pane-unlabelled` `` as a literal although `ImportFinding::PaneUnlabelled.code()` exists, while the same test reads `pane-unmanaged` from `PANE_UNMANAGED`. Consistent with the literals acceptance 17 already carried (`already-imported`, `source-missing`), so a tidy-up, not drift. Separately, the guide's unmanaged-table columns (section 4) mirror the private `UNMANAGED_COLUMNS` (`pane/list.rs:175`) and are unpinned, as T noted. | Use `ticked(ImportFinding::PaneUnlabelled.code())`; pin the unmanaged columns once `UNMANAGED_COLUMNS` is public (a later story). |

No duplication; extension is clean. No new near-copy of the docs-test extraction, the section slicer, or any
`holler-pane` table. The new unmanaged-pane text is documentation of #832's code on `main` (`holler_pane::unmanaged`,
`pane/list.rs`, `pane/doctor.rs`), not a second definition of it. The rework goes beyond the brief's text (#829, #832, the
`reset --first` caveat), which F journalled and justified by the brief's "state what is on `main`" rule; that is a scope
question for S, not architectural drift. `docs/orchestrating.md` states the #829 caveat in two sentences and links the
guide's section 2 rather than restating it.

## Notes for F

PASS: none required. Findings 1, 3 and 4 are optional tidy-ups within this story's files; finding 2 is a follow-up
outside its Blast radius.
