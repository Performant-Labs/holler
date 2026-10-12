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

---

# Handoff-S (re-audit): #652 - after the ADVISORY-HOLD, with #649 and #832 merged

**Date:** 2026-10-11
**Branch:** `issue-652-implementation` (head `0b32125`). `origin/main` (local ref `7fc00f1`) is one commit ahead with `docs/handoffs/655-herdr-move/decisions.md` only, which does not touch this diff.
**Issue:** #652 (OPEN)   **Brief:** `docs/handoffs/652-brief.md` at `e9eb05c` (re-cut R1 to R3 after #649 merged as `2d217dd`)
**Handoffs reviewed:** `handoff-A.md` (round 2), `handoff-T-red.md`, `handoff-F.md` (rework section), `handoff-T-green.md` (rework section), `handoff-A-dup.md` (cycle 2), `evidence.md`, `decisions.md`
**Diff audited:** `git diff origin/main...HEAD`
- `docs/pane-control.md`: new, 794 lines.
- `crates/holler-cli/tests/docs_pane_control_test.rs`: new, 523 lines.
- `crates/holler-cli/Cargo.toml`: one `[[test]]`.
- `docs/orchestrating.md`, `docs/README.md` and `CHANGELOG.md`.
- `scripts/pane-restart`: comment lines 3-4.
- The handoffs.

## A precondition

Met. `handoff-A.md` (round 2) is **PASS** (0 block, 5 warn). `handoff-A-dup.md` cycle 2 is **PASS** (0 block, 4 warn, none required).

## T precondition

Met.
- `handoff-T-red.md`: a valid RED, 16 of 16 failing on missing content.
- `handoff-T-green.md` (rework): GREEN, with `docs_pane_control_test` 16 passed and `docs_cli_test` 3 passed. Three new mutations each fail only acceptance 17. Lint, clippy, machete, `pane-restart-test.sh`, `test-hooks.sh` and gitleaks are all clean.
- The one workspace failure is `remote_admin_test`, the known flake #847. It is OPEN, and it reproduced 1 time in 5 when run alone. It is in Rust code that this docs-only diff does not touch.
- There are no blocking issues.

## The previous verdict's items

| Item of the ADVISORY-HOLD | Resolution | Status |
|---|---|---|
| 1. The #649 dependency was unmet, so R1 to R3 could not be re-cut | #649 merged (`2d217dd`) and was merged into the branch (`edbb863`). The brief re-cut R1 to R3 (`e9eb05c`). F rewrote section 2 from the merged `wiring.rs` module doc. I checked it against `crates/holler-cli/src/pane/wiring.rs:22-38`: no hub gives `unavailable` naming the hub, and no adapter is touched; the read verbs need only the hub; a bad variable fails at its port's first use; `HOLLER_HERDR_SESSION` is read and `HERDR_SESSION` is not; unset gives `usage`. All agree. The guide points to the module doc and does not copy the table. | RESOLVED |
| 2. The import mismatch sentence was wrong | Section 2 (lines 97-102) and section 10, step 1 (lines 715-717) now say: `launch` answers `unavailable` naming both sessions (`holler-adapter-herdr/src/adapter.rs:180-188`, `extent_of`); `pane import` reports every pane `pane-unlabelled`, because the snapshot carries only the adapter's session (`adapter.rs:262-264`) and the join filters by the flag's session (`import/assess.rs:163-171`); a dry run exits 0 and a real run is `import-blocked`. Acceptance 17 now pins `pane-unlabelled` in sections 2 and 10. | RESOLVED |
| Section 10's stale second fact | Dropped. The first fact names #835 (OPEN, the import exclusion). | RESOLVED |

## Acceptance criteria

| # | Criterion | Proving test / evidence | Status |
|---|---|---|---|
| 1 | Every verb named with its required args, the PROPOSED line rule, no `holler roster --profile` | `every_verb_is_named_and_proposed_lines_say_so` (walks `Cli::command()`) | MET |
| 2 | Section 5 names every `FindingKind::ALL` code | `section_5_names_every_finding_kind`. It went RED on `unmanaged-stalled` after the #832 merge and is GREEN after F's row, which shows it reads the live table. | MET |
| 3 | Section 9 has a classed row per `ALL_CODES` code, the exit rows, and the finding/code intersection | `section_9_has_a_classed_row_per_error_code` (class cell built from `class_of`/`exit_code`) | MET |
| 4 | Section 4 names every `COLUMNS` name | `section_4_names_every_column` | MET |
| 5 | Section 7's grid examples are true of `GridPos::parse` | `section_7_grid_examples_parse_as_stated` | MET |
| 6 | Every `holler ...` command parses | `docs_cli_test` (3 passed) | MET |
| 7 | Roster extension marked later | `section_4_marks_the_roster_extension_later` (guide line 432) | MET |
| 8 | Rollback and boundary in the guide, `orchestrating.md` and the script head | `rollback_and_boundary_are_stated` | MET |
| 9 | Section 10 names the replaced files, the launcher table, `source-missing`, `ADR-0021.md` | `section_10_names_the_replaced_files_and_the_launcher_table` | MET |
| 10 | README links the guide under Guidance | `readme_indexes_the_guide_under_guidance` | MET |
| 11 | Twelve numbered `##` headings in order | `guide_has_the_twelve_numbered_headings_in_order` | MET |
| 12 | No `~~~` and no `holler` line in an unscanned fence | `nothing_escapes_the_docs_test` | MET |
| 13 | Section 9 envelopes are true envelopes | `section_9_envelope_examples_are_true_envelopes` (`check_envelope`) | MET |
| 14 | Every `ImportFinding::ALL` code and `import-blocked` named in section 3 or 10 | `import_findings_are_named_in_section_3_or_10` | MET |
| 15 | The `profile apply` words | `profile_apply_reference_names_its_words` | MET |
| 16 | CHANGELOG `[Unreleased]` line links `/issues/652` and names `pane-control.md` | `changelog_unreleased_names_the_guide` | MET |
| 17 | Audit content in sections 2, 5 and 10; section 3 headings computed from the clap tree; plus, this round, `pane-unlabelled` in sections 2 and 10 and `pane-unmanaged` (from `PANE_UNMANAGED`) in section 9 | `audit_content_and_section_3_headings_are_present` (T-green mutation table: each removal fails it alone) | MET |

## Spec compliance

- **The brief's decisions 1 to 8 and "Where this brief is the decision" 1 to 6 are implemented as stated.** This is unchanged from the first audit:
  - the registry boundary;
  - `, agent KEY`;
  - SYNC against the session of record;
  - the overnight watch;
  - the section rewritten, not deleted;
  - a comment-only script change.
- **The re-cut R1 to R3 are implemented as the patched brief states them.** See the table above.
  - R3 (DRIVEN prints `-`) is in section 4 at lines 422-424.
  - That text names #839 as the story that makes DRIVEN follow the session of record. `gh issue view 839` shows it OPEN, with that title.
- **Deviations from the brief's text, all disclosed by F and journalled in `decisions.md`.**
  - **#832, merged mid-run.** Section 1 of the brief says an acting verb answers `pane-not-found` for a pane outside the registry. On `main`, `pane/mod.rs:60-90` (`acting_pane` plus `refuse_if_unmanaged`) refuses an unmanaged pane with `pane-unmanaged`, for exactly `relaunch`, `switch`, `reset`, `park`, `unpark` and `close`. The guide's lines 31-33 match the code. The brief's own rule that the guide states what is on `main` governs here. The merge had also turned acceptance 2 RED.
  - **#829.** The model, effort, environment and ceilings are recorded but not applied. ADR-0021 says so, and `gh issue view 829` is OPEN with that title.
  - **#839.** The `reset --first` caveat; the issue absorbs the closed #828.
  - Each of these is a correction toward the code on `main`, not a new feature, and none silently changes a brief decision.

## Quality audit

- **Correctness:** this story is docs only. I spot-checked the claims that are new this round against the code: `wiring.rs` module doc, `adapter.rs` `extent_of` and `pane`, `assess.rs` `labelled`, `unmanaged.rs` (`PANE_UNMANAGED`, observed and never stored, and Herdr 0.8.2 reporting no last-screen-change time), `pane/mod.rs` `acting_pane`, and `park.rs` (an unpark of a pane that is not parked leaves it as it is). All agree with the guide.
- **Build guards:**
  - The test file's `#![allow]` carries `// #149`.
  - There is no production `unwrap`, `expect` or `panic`.
  - The lint size gate covers `crates/**/*.rs` only. The test file is 523 lines, under 900.
  - The guide is 794 lines. It is not gated, but A-dup warn 3 notes it is near the overlay's 800-line mark.
- **Protocol:** no wire change and no golden touched.
- **Tests:** unit tier, which fits a docs-conformance test. There are no sleeps. The lists are read from the code's own tables (`Cli::command()`, `FindingKind::ALL`, `ALL_CODES`/`class_of`, `COLUMNS`, `ImportFinding::ALL`, `GridPos::parse`, `check_envelope`, `PANE_UNMANAGED`). RED-first evidence is in T-red, and mutation evidence is in both T-green rounds.
- **Documentation:**
  - The CHANGELOG `[Unreleased]` / `### Enhancements` entry links #652 and states the #829 caveat and that no behaviour changes.
  - The README indexes the guide.
  - `orchestrating.md` leads with `holler pane relaunch`, keeps the fallback and the Claude Code subsection, and no longer has the "removed when" sentence.
- **Public-repository privacy:**
  - I grepped the added lines of the guide, test, CHANGELOG, README, `orchestrating.md`, script and Cargo.toml for tailnet or `ts.net`, host and account names, home paths, e-mail, private IPv4 ranges and key shapes. There were no hits.
  - The examples use `demo-*`, `/srv/...` and loopback `127.0.0.1:8095`.
  - T-green's gitleaks run is clean.
- **Commit hygiene:**
  - Every subject is a Conventional Commit, and every commit carries a `Co-Authored-By` trailer.
  - The script's phase commits carry no session link. That is the pipeline's standard form, so it is not flagged.
  - No PR exists yet. CLAUDE.md has the run add the AI disclosure after the script opens the PR.

## Scope check

The files are exactly the Blast radius: the guide, the test and its `[[test]]`, one section of `orchestrating.md`, one README line, the script comment and the CHANGELOG. No source file outside tests changed.

The content beyond the brief's text (#832's unmanaged panes, the #829 and #839 caveats) is the minimum needed to keep the guide true of `main` and the docs test green after mid-run merges, and F explained it. This is not over-delivery. Nothing the brief asks for is missing.

## Verdict

**PASS**

Both items of the earlier ADVISORY-HOLD are resolved against merged code. All 17 acceptances are proven by tests that read the code's own tables and fail when their content is removed. The scope is the Blast radius, and quality and privacy are clean.

## Advisory notes (non-blocking)

1. **A-dup warn 1, still open.** Section 3's `profile apply` subsection (lines 359-376) restates section 6's reference. Cutting it to a pointer would also buy line headroom (A-dup warn 3: the guide is 794 lines).
2. **A-dup warn 2, still open.** `SCANNED_TAGS` hand-mirrors `docs_cli_test`'s fence tags. The follow-up is to share them via `tests/fixture/mod.rs`.
3. **A-dup warn 4.** Acceptance 17 types `` `pane-unlabelled` `` where `ImportFinding::PaneUnlabelled.code()` exists. Separately, the unmanaged table's columns are unpinned because `UNMANAGED_COLUMNS` (`pane/list.rs:175`) is private. Both belong in a later story.
4. **Rigor.** The brief's "Pipeline" line says `rigor: second-opinion`. No handoff records the outside-model (`deepseek-v4-pro`) gate. O should confirm it ran, or record the rigor the run used (carried from the first audit).
5. **Before the PR.** Merge `origin/main` (`7fc00f1`, no overlap). After the PR opens, add the AI disclosure per `CONTRIBUTING.md`. CI's workspace run may hit flake #847, which does not come from this story.
