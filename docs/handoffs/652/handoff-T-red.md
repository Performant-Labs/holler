# Handoff-T-red: #652 - The operator guide for `holler pane`

**Date:** 2026-10-11
**Branch:** `issue-652-implementation`
**Brief / wireframe reviewed:** `docs/handoffs/652-brief.md` (acceptances 1 to 17, "How to write the guide so `docs_cli_test` passes"); `docs/handoffs/652/handoff-A.md`; wireframe N/A (no UI surface)

## A precondition

Confirmed: A returned PASS on the plan (round 2, 0 block, 5 warn; `handoff-A.md`, commit `5115f41`).

## Tests authored

One new target, `crates/holler-cli/tests/docs_pane_control_test.rs` (511 lines, under the 600 lint warning, so no
directory target was needed, A's warn 5). It is declared as `[[test]] name = "docs_pane_control_test"` in
`crates/holler-cli/Cargo.toml` after `docs_cli_test`. All tests are **unit tier**: they read files and the code's
public tables in process (no binary, no hub). That is the cheapest tier that can check a doc against the clap tree and
`ALL_CODES`. Each test collects its failures and asserts once. A file that does not exist yet reads as empty, so each
test lists its own missing items and does not stop at a read panic.

| Test | Acceptance | Pins |
|---|---|---|
| `every_verb_is_named_and_proposed_lines_say_so` | 1 | Every `pane`/`profile` subcommand of `Cli::command()` (`help` filtered) is named by a span or heading `` `holler NS VERB`` followed by a space or a backtick. The PROPOSED line rule for `profile rename/export/import` and `profile-drift`: every line naming one says `PROPOSED`, and at least one such line exists. No `holler roster --profile`. |
| `section_5_names_every_finding_kind` | 2 | Each `FindingKind::ALL[i].code()` appears in backticks in section 5. |
| `section_9_has_a_classed_row_per_error_code` | 3 | Each code of `ALL_CODES` has a row in section 9 that starts `` | `code` | `` and contains `usage (2)`, `refusal (3)` or `failure (1)`. The number comes from `class_of(code).exit_code()`. There are exit rows `| 0 |` and one per `ErrorClass` exit code. The intersection of `FindingKind::ALL` and `ALL_CODES` is computed, not typed (today `herdr-version-unsupported`), and appears in backticks in sections 5 and 9. |
| `section_4_names_every_column` | 4 | Each name of `holler_cli::pane::list::COLUMNS` appears in section 4 as a whole word. |
| `section_7_grid_examples_parse_as_stated` | 5 | Section 7's `text` fence: each `ok <form>` parses with `GridPos::parse`, and each `refused <code> <form>` fails with that code. Required lines: `ok r2c1`, `ok c1r2`, `ok 2,1`, `ok R2C1` (each must be row 2, column 1) and `refused grid-ambiguous hj-c1r2`. There must be one true refused line per grid code. |
| (acceptance 6) | 6 | Not a new test. The existing `docs_cli_test` already scans `docs/**/*.md`. |
| `section_4_marks_the_roster_extension_later` | 7 | Section 4 has a line containing `holler roster`, `#648` and `later`. |
| `rollback_and_boundary_are_stated` | 8 | Guide: `scripts/pane-restart`, `until the 7-day sign-off`, `harness-not-opencode`, and a line with both `Claude Code` and `scripts/pane-restart`. Section 11: `holler pane relaunch`, `--fix`. Section 1: `Claude Code`. `docs/orchestrating.md`: `holler pane relaunch`, `scripts/pane-restart`, `Claude Code`, and no `removed when #644 and #653 have merged`. First ten lines of `scripts/pane-restart`: `until the 7-day sign-off`, and no `A stop-gap`. |
| `section_10_names_the_replaced_files_and_the_launcher_table` | 9 | Section 10 names `sessions.toml`, `last-session`, `owned-sessions`, `pane.env`, `launcher.toml`, `--launcher`, `source-missing`, `ADR-0021.md`. |
| `readme_indexes_the_guide_under_guidance` | 10 | `docs/README.md`'s `## Guidance` section contains `](pane-control.md)`. |
| `guide_has_the_twelve_numbered_headings_in_order` | 11 | The guide's `## ` lines are exactly `## 1. ` to `## 12. `, in order. The titles are not pinned. |
| `nothing_escapes_the_docs_test` | 12 | No `~~~` line. No fence outside `""/bash/sh/shell/console/text` holds a line starting `holler `. At least one scanned fence holds a `holler ` command, so the check is never vacuous. |
| `section_9_envelope_examples_are_true_envelopes` | 13 | Each `json` fence in section 9 that starts `{"schema_version"` passes `check_envelope(block, exit)`, where `exit` is 0 for success and `class_of(error.code).exit_code()` for a failure. There must be at least one success example and one failure example. |
| `import_findings_are_named_in_section_3_or_10` | 14 | Each `ImportFinding::ALL` code and `IMPORT_BLOCKED.as_str()` appears in backticks in section 3 or section 10. |
| `profile_apply_reference_names_its_words` | 15 | The apply reference names, in backticks, the 8 outcome words, the 6 action words, `detail`, `orchestrator-not-relaunched` and `spec-not-launchable`. The reference is the `` ### `holler profile apply `` subsection of section 3, which runs to the next `## `, plus section 6. These are hand-kept constants, and their doc names ADR-0021 section 9's apply row. |
| `changelog_unreleased_names_the_guide` | 16 | A line between `## [Unreleased]` and the next `## [` links `/issues/652` and contains `pane-control.md`. |
| `audit_content_and_section_3_headings_are_present` | 17 | Section 2: `agent-key-invalid`, `agent-unusable`, `--herdr-session`. Section 5: `not-fixable`. Section 10: `profile-exists`, `already-imported`, `--herdr-session`, `launcher.toml`, `source-missing`. Section 3: a heading line starting `` ### `FORM` `` for each built verb. FORM is computed from the clap tree: required positionals first, then required flags in declaration order. The placeholder is the first `value_name`, else the id upper-cased. `--herdr-session NAME` is added for `pane import`. |

**For F, the computed heading forms (authoritative over the brief's list, as A's warn 1 says):**

```text
### `holler pane list`
### `holler pane get PANE`
### `holler pane watch`
### `holler pane launch PANE`
### `holler pane relaunch PANE`
### `holler pane switch PANE SESSION`
### `holler pane reset PANE`
### `holler pane park --reason TEXT --release-when WHEN`
### `holler pane unpark`
### `holler pane close`
### `holler pane doctor`
### `holler pane import --from STATE_ROOT --herdr-session NAME`
### `holler profile create NAME`
### `holler profile delete NAME`
### `holler profile list`
### `holler profile show NAME`
### `holler profile apply NAME`
```

Each heading line must **start** with exactly that text. A suffix after the closing backtick is fine, for example
`` ### `holler profile apply NAME` (#664) ``. An extra token inside the backticks is not, for example
`` `holler pane list [PANE]` ``. Put the apply words (acceptance 15) in backticks in the `holler profile apply` subsection
of section 3 or in section 6. Put the class cell in each error row in the form `refusal (3)`.

## RED confirmation

`cargo test -p holler-cli --test docs_pane_control_test` on the branch head with only the test staged:

```text
test result: FAILED. 0 passed; 16 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.00s
```

Every failure is an assertion that names missing content. None is a compile, import or setup error. Excerpts:

- `section_5_names_every_finding_kind`: "section 5 (doctor) does not name these finding kinds: `herdr-version-unsupported` ... `stray-session`" (all 12).
- `section_4_names_every_column`: "PANE POS PROFILE PROJECT HEALTH SHOWN DRIVEN SYNC HOLD".
- `section_7_grid_examples_parse_as_stated`: "no `ok r2c1` line ... no `refused grid-ambiguous hj-c1r2` line ... no true `refused grid-out-of-range ...` line".
- `rollback_and_boundary_are_stated`: lists the guide items, plus "docs/orchestrating.md still says `removed when #644 and #653 have merged`", "scripts/pane-restart head: `until the 7-day sign-off`" and "scripts/pane-restart head still says `A stop-gap`". (`orchestrating.md` already contains `holler pane relaunch`, `scripts/pane-restart` and `Claude Code`, so those three items are correctly absent from the failure list.)
- `changelog_unreleased_names_the_guide`: "CHANGELOG.md's [Unreleased] has no line linking /issues/652 and naming pane-control.md".
- `readme_indexes_the_guide_under_guidance`: "docs/README.md's Guidance does not link pane-control.md".
- `every_verb_is_named_and_proposed_lines_say_so`: there is no span for any of the 20 verbs, and no PROPOSED line for any of the four PROPOSED words.
- `audit_content_and_section_3_headings_are_present`: the 9 section words and the 17 computed headings listed above.
- `guide_has_the_twelve_numbered_headings_in_order`, `nothing_escapes_the_docs_test` ("no scanned fence holds a `holler ` command example"), `section_9_*`, `import_findings_*`, `profile_apply_*`, `section_10_*` and `section_4_marks_*` each fail on their own missing-content assertion.

**Positive control (the tests can go GREEN).** I wrote a scratch guide temporarily, then deleted it; it was never
staged. It had the twelve headings, a grid block (`ok r2c1`, `ok c1r2`, `ok 2,1`, `ok R2C1`,
`refused grid-ambiguous hj-c1r2`, `refused grid-out-of-range r0c1`), exit rows, three classed code rows, one ok
envelope and one failure envelope (`pane-not-found`), a `bash` fence with `holler pane list`, and the heading
`` ### `holler pane switch PANE SESSION` ``. With it, tests 5, 11, 12 and 13 passed. Test 3 then listed only the codes
with no row: the `usage`, `timeout` and `grid-ambiguous` rows matched, so the `class_of` cell format is right. Test 17
no longer listed the `switch` heading.

**Tier 1 for the test alone:**
- `cargo clippy -p holler-cli --test docs_pane_control_test -- -D warnings`: clean.
- `bash scripts/lint.sh`: exit 0. The file has 511 lines, and its `#![allow]` carries `// #149`.

## Ready for F

RED is valid. F may implement against these tests. F writes `docs/pane-control.md`, the `docs/orchestrating.md`
section, the `docs/README.md` line, the comment on `scripts/pane-restart` line 3 and the CHANGELOG entry. F must not
edit this test; if a test is wrong, T repairs it. `docs_cli_test` must keep passing on the new guide (acceptance 6).
Per A's warn 3, sections 2 and 4 describe #649's contract. No test here reads #649's code, so the RED does not depend on
it.
