# Handoff-F: #652 - The operator guide for `holler pane`

**Date:** 2026-10-11
**Branch:** `issue-652-implementation`
**Issue:** #652

## What was done

- `docs/pane-control.md` (new): the operator guide, twelve numbered `##` sections in the brief's order (the rule I1 and
  the registry boundary; what a pane runs, the agent, what a verb needs; one `###` subsection per verb with the heading
  forms T computed from the clap tree; the `pane list` columns and SYNC; doctor and every finding kind; profiles and the
  `profile apply` reference; grid notation with the parsed example block; probe, argv, layout and Herdr version; the
  envelope, exit codes and one classed row per closed error code; the cutover procedure with the launcher table and the
  replaced files; the rollback; later).
- `docs/orchestrating.md`: the section "Restarting a pane on another model (interim)" is rewritten as "Restarting a pane
  on another model": it opens with `holler pane relaunch PANE --model PROVIDER/ID` in a `bash` fence and links the guide,
  names `scripts/pane-restart` as the fallback until the 7-day sign-off (replacing the "Until ... land" opening and the
  "removed when #644 and #653 have merged" sentence), keeps the OpenCode usage and notes, says a pane restarted with the
  script leaves the registry's control, and keeps the Claude Code subsection with a lead sentence that such a pane is
  outside the registry and the sign-off does not retire its path.
- `docs/README.md`: one "Guidance" line linking `pane-control.md`.
- `scripts/pane-restart`: line 3 (the stop-gap sentence) becomes two comment lines: the fallback for
  `holler pane relaunch --model` until the 7-day sign-off of #654, and the Claude Code path (#784) has no replacement.
  Comment only.
- `CHANGELOG.md`: one `## [Unreleased]` / `### Enhancements` entry linking #652 and naming the guide, the rewritten
  section and the retained fallback, and saying no behaviour changes.

## Design decisions

- **Every list is read from the code at writing time**, not from the brief: the verbs and flags from `holler ... --help`
  on the built binary, the finding kinds from `FindingKind::ALL`, the import findings and their levels from
  `ImportFinding::ALL`/`level()`, the error codes and classes from `ALL_CODES`/`class_of` (ADR-0021 section 9's table),
  the columns from `COLUMNS`.
- **Section 3 headings use T's computed forms** (A warn 1): `holler pane switch PANE SESSION` and
  `holler pane park --reason TEXT --release-when WHEN`.
- **Section 10's closing paragraph is qualified** (A warn 2): "A pane whose files name another harness gets no record",
  with a pointer to the first known fact (Claude Code panes whose files name `opencode`). The two known facts are stated
  before step 1 as facts, and the guide invents no exclusion procedure.
- **Bare `holler pane import` / `holler pane relaunch` spans are avoided**: `docs_cli_test` rejects them as missing a
  required argument, so prose says "the import (`pane import`)" and the orchestrating.md sentence carries the full form.
- **The `profile apply` reference lives in section 6** under a `###` heading, with a short summary in section 3's apply
  subsection; acceptance 15 reads both.
- **The import findings are a table in section 3** (code, level, meaning), so section 10 can refer to it.
- **Examples use neutral names** (`demo-alpha`, `demo-mo`, Herdr session `demo`, `/srv/demo`, `/srv/fleet-state`,
  loopback port 8095); the existing `orchestrating.md` examples (`c3r1`, `hj-c3r1`) are kept as they were.

## Reuse / extend-vs-new

The brief's plan: a new page indexed under the existing "Guidance" heading of `docs/README.md`, and the one section of
`docs/orchestrating.md` whose deletion condition is obsolete rewritten in place (not a parallel page). The guide links
ADR-0021 sections 9 and 13 instead of restating their per-verb and replacement tables, and points to the module doc of
`wiring.rs` for the environment instead of copying it.

## Architecture notes for A

None: docs, one comment line in a script and a CHANGELOG entry. No source, no dependency, no contract change.

## Deviations from spec / wireframe

- The brief lists `holler pane switch PANE` and `holler pane park PANE --reason ...` as heading forms; the guide uses the
  computed forms, as A warn 1 and T's handoff direct.
- Sections 2 and 4 state #649's contract (no hub is `unavailable`; read verbs work without Herdr; `HOLLER_HERDR_SESSION`
  and the module doc of `wiring.rs`; DRIVEN prints `-`) while #649 is still OPEN. A warn 3 asked for these to wait for
  #649's merge; this run reached F first. I checked them against the #649 branch's `wiring.rs` module doc (T-green PASS
  at `dfb395c`), which agrees, and kept the wording to what that doc and R1 to R3 both say (the guide names only
  `HOLLER_HERDR_SESSION`, not the fallbacks, which differ between the issue body and the branch). See Known issues.

## Tier 1 self-check (incl. tests now GREEN)

```text
cargo test -p holler-cli --test docs_cli_test --test docs_pane_control_test
docs_cli_test:          test result: ok. 3 passed; 0 failed
docs_pane_control_test: test result: FAILED. 15 passed; 1 failed
  readme_indexes_the_guide_under_guidance: "docs/README.md has no `## Guidance` section"  (a test bug, see below)
bash scripts/changelog-check.sh          changelog-check: ok
bash scripts/lint.sh                     exit 0
bash scripts/tests/pane-restart-test.sh  exit 0, 0 "not ok" lines
cargo test --workspace --no-fail-fast -- --skip roster_stays_accurate_under_concurrent_body_load
  151 test binaries `ok`; 1 target failed: docs_pane_control_test, only readme_indexes_the_guide_under_guidance
```

## Evidence appendix

`docs/handoffs/652/evidence.md` (the session check of the Herdr adapter, `(default)` in `profile show`, the launch text
line, the doctor finding line and remedy, the PROPOSED stubs).

## Tests that look wrong (for T)

- **`readme_indexes_the_guide_under_guidance` cannot pass.** It calls `from_heading(&readme, "\n## Guidance")`, and
  `from_heading` (`docs_pane_control_test.rs:69-73`) slices `rest = &doc[at..]` and ends at `rest.find("\n## ")`, which
  finds the start's own leading `\n## ` at offset 0, so it always returns the empty string and the first assertion fails
  ("docs/README.md has no `## Guidance` section") however the README reads. `docs/README.md` has `## Guidance` (line 34)
  and the new line `- [**pane-control.md**](pane-control.md) ...` under it. Fix in the test: search for the end after the
  start (for example `rest[1..].find("\n## ").map_or(rest.len(), |e| e + 1)`), or call it with `"## Guidance"`. The CHANGELOG
  test passes because its start, `"## [Unreleased]"`, has no leading newline. (T's RED handoff quotes the second
  assertion's message for this test; the run here shows the first one.)

## Known issues

- **#649 is OPEN.** The statements of sections 2 and 4 that come from #649 (R1 to R3) are written from the #649 branch,
  not from merged code. S (or the build after #649 merges) checks them against the merged `wiring.rs` module doc: no hub
  is `unavailable` (exit 1) naming the hub; `pane list`/`get`/`watch` and `profile list`/`show` work with no Herdr;
  `HOLLER_HERDR_SESSION` is the Herdr session variable; DRIVEN prints `-`. Section 10's second known fact ("the dry run
  cannot run on the live fleet until #649 has merged") becomes stale once it merges and should then be removed.
- A follow-up for an import exclusion of Claude Code panes whose files name `opencode` (A warn 2) is O's to file; the
  guide states the fact only.

## Files changed

- `docs/pane-control.md` (new)
- `docs/orchestrating.md`
- `docs/README.md`
- `scripts/pane-restart` (comment, line 3)
- `CHANGELOG.md`

---

# Handoff-F (rework): #652 - after S ADVISORY-HOLD, with #649 and #832 merged

**Date:** 2026-10-11
**Branch:** `issue-652-implementation` (head `e9eb05c` before this round; `origin/main` has nothing beyond it)
**Issue:** #652

## What was done

- `docs/pane-control.md`:
  - **Section 2, R1 to R3 re-cut from the merged `wiring.rs`** (S's verdict item 1): the hub is the only precondition and
    no hub is `unavailable` touching no adapter; a missing or malformed variable fails at its port's first use;
    `HERDR_SESSION` is not read and an unset `HOLLER_HERDR_SESSION` is `usage`; `--herdr-session` is not defaulted.
  - **The mismatch sentence fixed** (S's verdict item 2, the patched brief's R2): `launch` answers `unavailable` (apply's
    create step fails and stops the run); `pane import` reports every pane `pane-unlabelled`, a dry run exits 0, a real
    run is `import-blocked`. Repeated at section 10, step 1, as how to read such a dry run.
  - **Section 10's second fact dropped** (stale since #649); the first fact names #835 as the follow-up.
  - **#832 (merged into the branch at `8dc1aca`) documented**: it had turned `section_5_names_every_finding_kind` RED
    (`unmanaged-stalled`) and made section 1's "a verb that names such a pane answers `pane-not-found`" false. Sections 1,
    3 (`list`, `get PANE` by label or Herdr pane id, `pane-unmanaged` for the six acting verbs, doctor's
    `--unmanaged-stalled-after`), 4 (the second table, the JSON keys, IDLE `-` against Herdr 0.8.2, the grid-tab limit),
    5 (the `unmanaged-stalled` row and paragraph; "four name none"), 9 (`pane-unmanaged` among the open codes) and 10.
  - **#829 stated**: the model, effort, env names and ceilings are recorded, not applied (section 2, `relaunch`, step 5,
    section 11's way back, section 12).
  - **`reset --first` caveat** (ADR-0021 lines 365-367): do not use it on a parked or held pane; #839 owns the fix. Also
    in `park` and step 5. DRIVEN's follow-up named as #839 in section 4.
- `docs/orchestrating.md`: the relaunch paragraph no longer says the server starts "on the new model"; it says the model
  is recorded and not applied until #829, so the fallback is what puts a pane on another model meanwhile.
- `CHANGELOG.md`: the #652 line says `--model` is recorded and not yet applied (#829).
- `docs/handoffs/652/evidence.md`, `decisions.md`: the rework's facts and decisions.

## Design decisions

- **#832 documented, not deferred.** The brief predates its merge, but the brief's rule is that the guide states what is
  on `main`, and the merge made the docs test RED; leaving it would have shipped a failing test and a false boundary.
- **#829 stated although the brief does not name it.** Without it the guide and `orchestrating.md` told the operator a
  relaunch changes the model the server runs, which ADR-0021 lines 604-605 say it does not. The orchestrating.md section
  keeps its heading and its lead command (the docs test and the brief require it) and gains the caveat.
- The `wiring.rs` variable table is still not copied (brief); only `HOLLER_HERDR_SESSION` and its two behaviours are named.

## Reuse / extend-vs-new

Unchanged: the same page, sections and files are edited in place; ADR-0021 sections 8, 9, 13 and 14 and the `wiring.rs`
module doc are pointed to, not restated.

## Architecture notes for A

None: docs, CHANGELOG and handoffs only.

## Deviations from spec / wireframe

- The brief's section 1 text ("A verb that names it as an existing pane ... answers `pane-not-found`") is superseded by
  #832 as merged: an acting verb answers `pane-unmanaged` for an unmanaged pane, `pane-not-found` otherwise.
- #829, #839 and the `reset --first` caveat are added beyond the brief's text, for the reason above.

## Tier 1 self-check (incl. tests now GREEN)

```text
before this round (main merged in): docs_pane_control_test 15 passed, 1 FAILED
  section_5_names_every_finding_kind: section 5 (doctor) does not name these finding kinds: `unmanaged-stalled`
cargo test -p holler-cli --test docs_cli_test --test docs_pane_control_test
  docs_cli_test:          test result: ok. 3 passed; 0 failed
  docs_pane_control_test: test result: ok. 16 passed; 0 failed
bash scripts/changelog-check.sh   changelog-check: ok
bash scripts/lint.sh              exit 0 (guide 794 lines, orchestrating.md 163; under the 850 decompose mark)
gitleaks detect --no-git          docs/pane-control.md, docs/orchestrating.md: no leaks found
```

The full workspace suite was not re-run: this round changes only Markdown and the CHANGELOG, which only the two docs
tests and `changelog-check.sh` read.

## Evidence appendix

`docs/handoffs/652/evidence.md`, section "F rework (after S ADVISORY-HOLD; #649 and #832 merged)".

## Tests that look wrong (for T)

None. (T may want a test that pins the `pane-unmanaged` open code in section 9 and the import mismatch sentence; none of
the 16 reads them today.)

## Known issues

- Section 4's IDLE and section 5's `unmanaged-stalled` describe a field Herdr 0.8.2 does not fill; the guide says so.
- A-dup warns 1 and 2 (apply subsection restating section 6; `SCANNED_TAGS` hand-mirrored) are unchanged.
- The brief's "Pipeline" rigor question (S advisory note 5) is O's.

## Files changed

- `docs/pane-control.md`
- `docs/orchestrating.md`
- `CHANGELOG.md`
