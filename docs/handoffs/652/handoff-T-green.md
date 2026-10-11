# Handoff-T-green: #652 - The operator guide for `holler pane`

**Date:** 2026-10-11
**Branch:** `issue-652-implementation`
**Issue:** #652
**Handoff-F reviewed:** `docs/handoffs/652/handoff-F.md`
**Handoff-T-red:** `docs/handoffs/652/handoff-T-red.md`

## Test repair (F's "Tests that look wrong")

F was right. `from_heading` searched for the end of the slice from offset 0 of `rest`, so a `start` that begins with
`\n## ` (only the README test passes one, `"\n## Guidance"`) matched itself and returned the empty string.
`readme_indexes_the_guide_under_guidance` could never pass. The fix is in the helper, not the call: the end search now
skips the first byte (`rest[1..].find("\n## ").map_or(rest.len(), |e| e + 1)`). The other two callers
(`"### \`holler profile apply"` and `"## [Unreleased]"`) do not start with a newline, so their slices are unchanged.
No production file was touched.

## GREEN confirmation

```text
cargo test -p holler-cli --test docs_pane_control_test --test docs_cli_test
docs_cli_test:          test result: ok. 3 passed; 0 failed
docs_pane_control_test: test result: ok. 16 passed; 0 failed
```

**The tests fail when the behaviour is removed.** Each mutation below was applied to F's docs, run, then reverted with
`git checkout`. Every one left 15 passed and 1 failed, and the failing test was the one for that acceptance:

| Mutation | Fails |
|---|---|
| `pane-control.md` line removed from `docs/README.md` | acceptance 10 |
| README link moved from Guidance to `## Testing` | acceptance 10 ("Guidance does not link pane-control.md"), so the repaired slice is bounded |
| `/issues/652` line removed from CHANGELOG | acceptance 16 |
| `SYNC` renamed in the guide | acceptance 4 |
| `` `stray-session` `` removed | acceptance 2 |
| `pane-not-found` row reclassed as `failure (1)` | acceptance 3 |
| `ok r2c1` changed to `ok r2c2` | acceptance 5 |
| `removed when #644 and #653 have merged` put back in `orchestrating.md` | acceptance 8 |
| `` `orchestrator-not-relaunched` `` removed | acceptance 15 |

## Tier 1 results

| Check | Expected | Actual | Result |
|---|---|---|---|
| `bash scripts/lint.sh` | exit 0 | exit 0 (two existing warns on unrelated files) | PASS |
| `bash scripts/changelog-check.sh` | ok | `changelog-check: ok` | PASS |
| `cargo clippy --workspace --all-targets -- -D warnings` | clean | clean | PASS |
| `cargo test -p holler-cli --test wire_selftest` | ok | 3 passed | PASS |
| `cargo test -p holler-cli --test docs_cli_test` | ok | 3 passed | PASS |
| `cargo machete` | no unused deps | none found | PASS |
| `bash scripts/tests/pane-restart-test.sh` | exit 0, no `not ok` | exit 0, 0 `not ok` | PASS |
| `bash scripts/test-hooks.sh` | exit 0 | exit 0 | PASS |
| `cargo test --workspace --no-fail-fast -- --skip roster_stays_accurate_under_concurrent_body_load` (the configured unit command) | all ok | exit 0; 152 `test result: ok`, no FAILED | PASS |

**Cross-check with F.** F reported `docs_cli_test` 3 passed, `docs_pane_control_test` 15/1 (the test bug), lint,
changelog-check and pane-restart-test passing, and the workspace with one failing target (the same test). The results
here match. The difference is the repaired test, which now passes.

## Tier 2 results

| Check | Method | Result |
|---|---|---|
| Coverage | Acceptances 1-5 and 7-17 each have a test; 6 is the existing `docs_cli_test` (passing) | PASS |
| Test quality | Each test names its acceptance and failed alone in the mutation runs. All tests are unit tier (files and the code's tables, no binary). Lists are computed from `FindingKind::ALL`, `ALL_CODES`, `class_of`, `COLUMNS`, `ImportFinding::ALL`, `GridPos::parse`, `check_envelope` and the clap tree. The apply words are the only hand-kept list, and the evidence appendix backs them | PASS |
| Proportion | One 513-line target for 16 acceptances, no duplication of `docs_cli_test`'s extraction | PASS |
| Lint limits | Test file 513 lines (< 600 warn, < 900 fail); `#![allow]` carries `// #149` | PASS |
| Secrets | `gitleaks detect --no-git` on the guide: no leaks; no key-shaped strings in the guide or `orchestrating.md` | PASS |
| Protocol / goldens | No protocol-visible change; no golden touched | N/A |
| Behaviour | Production diff is docs, a comment in `scripts/pane-restart` (lines 3-4) and CHANGELOG; no source change | PASS |
| Evidence appendix | T added three entries to `evidence.md`: the `tx_apply` `as_str` words behind `APPLY_ACTIONS`/`APPLY_OUTCOMES`, `pane import`'s runtime-required `--herdr-session`, and `profile-drift` being a code but not yet a kind | done |

## Acceptance criteria status

| # | Status | Test |
|---|---|---|
| 1 | PASS | `every_verb_is_named_and_proposed_lines_say_so` |
| 2 | PASS | `section_5_names_every_finding_kind` |
| 3 | PASS | `section_9_has_a_classed_row_per_error_code` |
| 4 | PASS | `section_4_names_every_column` |
| 5 | PASS | `section_7_grid_examples_parse_as_stated` |
| 6 | PASS | `docs_cli_test::every_documented_holler_command_parses` |
| 7 | PASS | `section_4_marks_the_roster_extension_later` |
| 8 | PASS | `rollback_and_boundary_are_stated` |
| 9 | PASS | `section_10_names_the_replaced_files_and_the_launcher_table` |
| 10 | PASS | `readme_indexes_the_guide_under_guidance` (after the helper repair) |
| 11 | PASS | `guide_has_the_twelve_numbered_headings_in_order` |
| 12 | PASS | `nothing_escapes_the_docs_test` |
| 13 | PASS | `section_9_envelope_examples_are_true_envelopes` |
| 14 | PASS | `import_findings_are_named_in_section_3_or_10` |
| 15 | PASS | `profile_apply_reference_names_its_words` |
| 16 | PASS | `changelog_unreleased_names_the_guide` |
| 17 | PASS | `audit_content_and_section_3_headings_are_present` |

## Blocking issues

None.

## Advisory notes

- **#649 is still OPEN** (checked with `gh issue view 649` this phase). Sections 2 and 4 state #649's contract. I compared
  them with the #649 branch's `crates/holler-cli/src/pane/wiring.rs` module doc (lines 22-38): no hub is `unavailable`
  naming the hub, the five read verbs work with only the hub, and `HOLLER_HERDR_SESSION` is the session variable. They
  agree. No test reads #649's code, so the suite cannot catch drift if #649 changes before it merges. S, or a check
  after #649 merges, re-reads that module doc. Section 10's second known fact goes stale once #649 merges.
- `cargo fmt --all -- --check` reports drift across the whole workspace (no `rustfmt.toml` in the repo, and CI has no
  fmt step), so it is not a signal for this diff.
- The branch is 1 commit behind `origin/main` (`4366fb8`, a docs research file for #830). It does not touch any file
  in this diff.
