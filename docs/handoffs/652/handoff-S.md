# Handoff-S: #652 - The operator guide for `holler pane`

**Date:** 2026-10-11
**Branch:** `issue-652-implementation` (head `6960c9d`; one commit behind `origin/main`, `4366fb8`, a docs research file that touches nothing in this diff)
**Issue:** #652 (OPEN)   **Brief:** `docs/handoffs/652-brief.md`
**Handoffs reviewed:** `handoff-A.md` (round 2), `handoff-T-red.md`, `handoff-F.md`, `handoff-T-green.md`, `handoff-A-dup.md`, `evidence.md`
**Diff audited:** `git diff origin/main...HEAD`: `docs/pane-control.md` (new, 725 lines), `crates/holler-cli/tests/docs_pane_control_test.rs` (new, 513 lines), `crates/holler-cli/Cargo.toml` (one `[[test]]`), `docs/orchestrating.md`, `docs/README.md`, `scripts/pane-restart` (comment lines 3-4), `CHANGELOG.md`, and the run's handoffs.

## A precondition

Met. `handoff-A.md` (round 2, after round 1's BLOCK) is **PASS**: 0 block, 5 warn. `handoff-A-dup.md` is **PASS**: 0 block, 3 warn.

## T precondition

Met. `handoff-T-red.md`: RED is valid, with 16 of 16 failing on missing-content assertions and no compile or setup failures, plus a positive control. `handoff-T-green.md`: GREEN, with `docs_pane_control_test` 16 passed and `docs_cli_test` 3 passed. Each of nine mutations fails only its own acceptance. The workspace suite is all `ok`. There are no blocking issues. T's one repair, the `from_heading` end search, is confined to the test and was a real test bug that F reported.

## Acceptance criteria

| # | Criterion | Proving test / evidence | Status |
|---|---|---|---|
| 1 | Every `pane`/`profile` verb is named in a span or heading; PROPOSED line rule; no `holler roster --profile` | `every_verb_is_named_and_proposed_lines_say_so` (walks `Cli::command()`) | MET |
| 2 | Section 5 names every `FindingKind::ALL` code | `section_5_names_every_finding_kind` (T-green mutation: `stray-session` removed fails it) | MET |
| 3 | Section 9 has a classed row per `ALL_CODES` code, the exit rows, and the finding/code intersection in sections 5 and 9 | `section_9_has_a_classed_row_per_error_code` (cell built from `class_of`/`exit_code`; mutation: reclassing a row fails it) | MET |
| 4 | Section 4 names every `COLUMNS` name | `section_4_names_every_column` (whole-word match; mutation: `SYNC` renamed fails it) | MET |
| 5 | Section 7's `ok`/`refused` grid lines are true of `GridPos::parse` | `section_7_grid_examples_parse_as_stated` (calls the parser; mutation: `ok r2c2` fails it) | MET |
| 6 | Every `holler ...` command in the guide parses | existing `docs_cli_test` (scans `docs/**/*.md`; 3 passed in T-green) | MET |
| 7 | Section 4 marks the roster extension later | `section_4_marks_the_roster_extension_later` (guide line 379) | MET |
| 8 | Rollback and boundary in the guide, `orchestrating.md` and the head of `scripts/pane-restart` | `rollback_and_boundary_are_stated` (mutation: the old "removed when" sentence restored fails it) | MET |
| 9 | Section 10 names the replaced files, the launcher table, `source-missing` and `ADR-0021.md` | `section_10_names_the_replaced_files_and_the_launcher_table` | MET |
| 10 | `docs/README.md` links the guide under Guidance | `readme_indexes_the_guide_under_guidance` (after T's helper repair; mutation: the link moved to `## Testing` fails it, so the slice is bounded) | MET |
| 11 | Twelve numbered `##` headings in order | `guide_has_the_twelve_numbered_headings_in_order` | MET |
| 12 | No `~~~` fence and no `holler` line in an unscanned fence; not vacuous | `nothing_escapes_the_docs_test` | MET |
| 13 | Section 9 envelope examples are true envelopes, at least one ok and one failure | `section_9_envelope_examples_are_true_envelopes` (`check_envelope` with exit from `class_of`) | MET |
| 14 | Every `ImportFinding::ALL` code and `import-blocked` named in section 3 or 10 | `import_findings_are_named_in_section_3_or_10` | MET |
| 15 | The apply reference names its 8 outcomes, 6 actions, `detail` and two open codes | `profile_apply_reference_names_its_words` (mutation: `orchestrator-not-relaunched` removed fails it) | MET |
| 16 | The CHANGELOG `[Unreleased]` line links `/issues/652` and names `pane-control.md` | `changelog_unreleased_names_the_guide` (mutation: the line removed fails it) | MET |
| 17 | Audit content in sections 2, 5 and 10; section 3 headings computed from the clap tree | `audit_content_and_section_3_headings_are_present` (`heading_forms()` builds them from required args and `value_name`, plus import's runtime-required `--herdr-session`) | MET |

Each test reads its list from the code (`Cli::command()`, `FindingKind::ALL`, `ALL_CODES`/`class_of`, `COLUMNS`, `ImportFinding::ALL`, `GridPos::parse`, `check_envelope`). The two hand-kept lists, `PROPOSED_PROFILE_VERBS` and the apply words, carry the doc comments the brief requires. No test is a tautology: T-green's mutation table shows each test failing when its content is removed.

## Spec compliance

- **Decisions 1 to 8 and "Where this brief is the decision" 1 to 6: implemented as stated.**
  - The registry boundary (decision (a)) is in section 1 and repeated in sections 3, 5, 10 and 11 and in `orchestrating.md`.
  - The agent output is `, agent KEY` and JSON `agent` (line 58).
  - SYNC compares SHOWN with the session of record, and DRIVEN is not compared.
  - The cutover keeps the overnight watch and reads SYNC from `pane list`/`doctor`.
  - `orchestrating.md`'s section is rewritten, not deleted, and its heading drops "(interim)".
  - `scripts/pane-restart` changes a comment only.
- **A's warns were carried in.**
  - Warn 1: the computed heading forms `switch PANE SESSION` and `park --reason TEXT --release-when WHEN`.
  - Warn 2: section 10's closing paragraph is qualified (lines 686-689).
  - Warn 4: the apply-word constant names ADR-0021 section 9.
  - Warn 5: the test is 513 lines, so no split was needed.
- **Spot-checked against the code where no test pins the wording.**
  - The `not-fixable` mismatch has no bracket and names `relaunch` (`findings.rs:135-141`).
  - Three kinds name no command (`findings.rs:123-125`).
  - `profile list` prints name, slug, specs, live and generation (`profile/list.rs:20-25`).
  - `unpark` leaves an unparked pane as it is.
  - `pane list` is sorted by name.
  - `park` text is capped at 200 characters (`park.rs:62`).
  - All match.
- **The REFRESH AFTER MERGE of #649 lines (R1 to R3) were not re-cut. This is the brief-level defect behind this verdict.**
  - **What the brief requires.** It makes sections 2 and 4 depend on #649's *merged* code: "the build run re-cuts the excerpt named on that line from the merged code and patches the line before it writes the guide". "Depends on" names #649 as the one open dependency.
  - **What A said.** A's warn 3 asked to merge after #649 or to hold the PR, and said "S checks each REFRESH line against merged `wiring.rs`".
  - **The state now.** #649 is still **OPEN**, with no PR. `origin/main`'s `wiring.rs` is still the #670 stub, where every port answers `not-implemented`.
  - **What that means.** S cannot do the check the brief assigns, and the guide's present-tense claims in section 2 are false on `main` today. Those claims are lines 67-79: no hub gives `unavailable`; read verbs work without Herdr; `HOLLER_HERDR_SESSION`.
  - **F was open about it.** F disclosed this (handoff-F "Deviations" and "Known issues") and checked the claims against the #649 branch. So this is not a silent deviation.
- **I compared each claim with the #649 working tree (`crates/holler-cli/src/pane/wiring.rs` module doc and `docs/handoffs/649/`).**
  - No hub gives `unavailable` naming the hub: agrees (D2).
  - The five read verbs work with only the hub: agrees.
  - `HOLLER_HERDR_SESSION` and "the module doc lists the rest": agrees.
  - The `--herdr-session` flag is still required, with no default from the wiring: agrees (D5).
  - **Disagrees: "The two must name the same session, or the verb answers `unavailable`"** (guide line 79; restated for the import at lines 654-655). That is true for `launch`: the adapter refuses a name other than its configured session as `unavailable` (#649 D5, E24). It is **not** true for `pane import`. #649's own A review (W-Q, `docs/handoffs/649/decisions.md`) records: "import reports a `--herdr-session` that differs from `HOLLER_HERDR_SESSION` as `pane-unlabelled` for every pane". That means a dry run exits 0 with every pane a `pane-unlabelled` problem, and a real run is `import-blocked`. The sentence comes verbatim from the brief's section 2 (R2), so F implemented it faithfully. It is the step of the cutover procedure where an operator would most likely misread the output.

## Quality audit

- **Correctness and failure handling:** this is a docs story with no runtime code. The guide's error, exit and fail-closed statements match `class_of`, `FindingKind::remedy` and the verbs' modules where I checked them. The one inaccuracy is the import mismatch sentence above.
- **Build guards:**
  - The new test's `#![allow]` carries `// #149`.
  - The test has no `unwrap` or `panic` outside test code.
  - No touched file reaches 900 lines: guide 725, test 513, `orchestrating.md` 160, `scripts/pane-restart` 401.
  - There is no dead code. T-green records lint, clippy (`-D warnings`) and `cargo machete` as clean.
- **Protocol:** no wire change and no golden touched.
- **Tests:** unit tier, which fits a docs-conformance check. There are no sleeps and no cross-process behaviour. RED-first evidence is in T-red.
- **Documentation:**
  - `CHANGELOG.md` has an `## [Unreleased]` / `### Enhancements` entry linking #652 that says no behaviour changes.
  - `docs/README.md` indexes the guide under Guidance.
  - `orchestrating.md` is rewritten as the brief specified, and keeps the OpenCode usage, the notes and the Claude Code subsection.
- **Public-repository privacy:** I grepped the whole added diff, including the committed brief and handoffs, for tailnet/`ts.net`, hostnames, home paths, account names, e-mail, non-loopback IPs and key shapes. No hits. The examples use neutral names (`demo-alpha`, `/srv/demo`, loopback `127.0.0.1:8095`). `hj-c3r1`/`c3r1`/`c4r2` are the grid-style names already on `main`. The one `TOKEN=x` is a quoted ADR test example. T-green's `gitleaks` run is clean.
- **Commit and PR hygiene:**
  - All subjects are Conventional Commits, and every commit carries a `Co-Authored-By` trailer.
  - The script's phase commits carry no session link. That is the pipeline's standard form, so it is not flagged here.
  - No PR exists yet. The AI disclosure is added to the PR body after the script opens it (CLAUDE.md), so it is not auditable at this phase.

## Scope check

The diff is exactly the Blast radius: the guide, the new test target and its `[[test]]` entry, one section of `orchestrating.md`, one README line, the comment on `scripts/pane-restart` lines 3-4 (the brief allows "one comment line (or two)"), and the CHANGELOG entry. No source file outside tests changed. There is no over-delivery. The one shortfall is the under-delivery of the #649 re-cut, explained above.

## Verdict

**ADVISORY-HOLD**

**The defect.** The brief has an ordering defect and one wrong sentence, and F implemented both faithfully.

1. **The #649 dependency is unmet, so the brief's own REFRESH step cannot run.**
   - Sections 2 and 4 state #649's contract as present fact. #649 is still OPEN, and on `main` every pane verb answers `not-implemented`.
   - The brief requires R1 to R3 to be re-cut "from the merged code ... before it writes the guide". A's warn 3 asked to merge after #649 or hold the PR.
   - Merging now, which the script's auto-merge would do on a PASS, publishes an operator guide whose section 2 is false on `main`. It also leaves no gate that re-checks it when #649 lands, because no test reads #649's code.
2. **The brief's R2 sentence is wrong for `pane import`.** "The two must name the same session, or the verb answers `unavailable`" (guide lines 76-79, echoed at 654-655) holds for `launch`. #649's own A review (W-Q) records that `pane import` with a mismatched `--herdr-session` reports every pane as `pane-unlabelled`, so a real run is `import-blocked`.

**Conventions violated.** The brief's own "Depends on" and REFRESH-AFTER-MERGE rule. The pipeline rule that the guide states what is on `main` (brief, paragraph 2).

**Proposed fix (operator's call).**

- **Recommended.**
  1. Hold the PR (do not auto-merge) until #649 merges.
  2. Merge `origin/main` into the branch, re-cut R1 to R3 from the merged `wiring.rs`, and patch the brief.
  3. Have F correct section 2 (lines 76-79) and section 10, step 1 (lines 654-655). `launch`, and `profile apply` when it creates, answer `unavailable` for a session other than the configured one. `pane import` reports every pane as `pane-unlabelled` (a problem, so a real run is `import-blocked`).
  4. Have F drop section 10's second known fact (lines 628-629), which is stale once #649 merges.
  5. Re-run `docs_pane_control_test` and `docs_cli_test`.
- **Alternative, if the operator wants the guide on `main` before #649.** F qualifies each #649 statement in sections 2 and 4 as "once the hub wiring (#649) has merged" and fixes the import sentence now. A follow-up then removes the qualifiers after #649.

Everything else passes: all 17 acceptances are met by tests that fail when their content is removed, the scope is exact, and quality and privacy are clean.

## Advisory notes (non-blocking)

1. **A-dup warn 1.** Section 3's `profile apply` subsection (lines 332-337) restates section 6's reference. It could be cut to the exit-code sentence and a pointer.
2. **A-dup warn 2.** `SCANNED_TAGS` (test line 43) mirrors `docs_cli_test`'s tag list by hand. The follow-up is to move it to `tests/fixture/mod.rs`.
3. **The import-exclusion follow-up of A's warn 2 now exists as #835** ("holler pane import leaves panes Holler does not drive alone, and --adopt-claude-code adopts them"). When the guide is next touched, section 10's first fact ("No mechanism excludes a pane from the import yet") and section 12 can name #835.
4. **#649's A review (W-N b) calls "every verb that reaches Herdr" too broad.** The guide's line 76 keeps that phrase but immediately lists the three verbs, so it reads correctly. Re-check it at the re-cut.
5. **The brief's "Pipeline" line says `rigor: second-opinion`.** No handoff records an outside-model gate (`deepseek-v4-pro`) at the brief or diff gate. O should confirm that gate ran, or record the rigor this run actually used.
6. **The branch is one commit behind `origin/main`** (`4366fb8`, no overlap). Merge before the PR.
