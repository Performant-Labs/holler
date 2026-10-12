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

---

# Handoff-T-green (rework): #652 - after F's rework (S ADVISORY-HOLD; #649 and #832 merged)

**Date:** 2026-10-11
**Branch:** `issue-652-implementation` (head `d698473`; `origin/main` is one commit ahead, `7fc00f1`, a #655 handoff file only)
**Issue:** #652
**Handoff-F reviewed:** `docs/handoffs/652/handoff-F.md` (rework section)
**Handoff-T-red:** `docs/handoffs/652/handoff-T-red.md`

## Test change this round

F flagged no wrong test. F noted that none of the 16 tests read the two facts S's verdict turned on, so I extended
acceptance 17's test (`audit_content_and_section_3_headings_are_present`), the one that pins content the brief requires
in named sections:

- sections 2 and 10 name `` `pane-unlabelled` ``. This is the brief's re-cut R2: `pane import` with a `--herdr-session`
  other than the adapter's reports every pane `pane-unlabelled`. Section 3's import findings table does not count,
  because the test reads sections 2 and 10 only;
- section 9 names `pane-unmanaged` (#832's open code) among the open codes. The test reads it from
  `holler_pane::unmanaged::PANE_UNMANAGED`, not as a hand-typed string.

No new test function was added. `UNMANAGED_COLUMNS` (`list.rs:175`) is private, so the unmanaged table's columns are
not pinned. Making it `pub` would be a production change, so it is an advisory note below. The test file is now 523 lines.
No production file was touched.

## GREEN confirmation

```text
cargo test -p holler-cli --test docs_pane_control_test --test docs_cli_test
docs_cli_test:          test result: ok. 3 passed; 0 failed
docs_pane_control_test: test result: ok. 16 passed; 0 failed
```

**Mutations of the new assertions.** Each one was applied to the guide, run, then reverted with `git checkout`. Each left
15 passed and 1 failed, and the one failure was acceptance 17 naming the missing item:

| Mutation | Fails with |
|---|---|
| `` `pane-unlabelled` `` removed from section 2 (lines 100 and 102) | `section 2: pane-unlabelled` |
| `` `pane-unlabelled` `` removed from section 10, step 1 (line 716) | `section 10: pane-unlabelled` |
| `pane-unmanaged` dropped from section 9's open codes (line 672) | `section 9: pane-unmanaged` |

Removing only line 100 still passed, because line 102 of section 2 names the code as well. So the assertion is about
the section, not about one line. Last round's mutation table still applies to the other 15 tests. F's rework also turned
`section_5_names_every_finding_kind` back to GREEN: it had gone RED on `unmanaged-stalled` after the #832 merge.

## Claims of the rework checked against the code

| Guide claim | Code | Agrees |
|---|---|---|
| No hub: `unavailable` naming it, no adapter touched; read verbs need only the hub; a bad variable fails at its port's first use | `wiring.rs:22-27` | yes |
| `HOLLER_HERDR_SESSION`; `HERDR_SESSION` not read; unset is `usage` | `wiring.rs:33` | yes |
| `launch` in another session: `unavailable` naming both | `adapter.rs:180-188` (`extent_of`) | yes |
| `pane import` mismatch: every pane `pane-unlabelled`; a dry run exits 0; a real run is `import-blocked` | `adapter.rs:262-264` (snapshot stamped with the adapter's session), `assess.rs:163-171` (filter by `--herdr-session`), `import.rs:18-20` | yes |
| `pane-unmanaged` is a refusal, exit 3 | `unmanaged.rs:34-36` | yes |
| Unmanaged table columns PANE-ID, LABEL, POS, PROJECT, STATE, IDLE | `list.rs:175` | yes |
| `reset --first` gate: #839 (absorbs #828) | `gh issue view 839`: OPEN, title says it absorbs #828 | yes |

## Tier 1 results

| Check | Expected | Actual | Result |
|---|---|---|---|
| `bash scripts/lint.sh` | exit 0 | exit 0 | PASS |
| `bash scripts/changelog-check.sh` | ok | `changelog-check: ok` | PASS |
| `cargo clippy --workspace --all-targets -- -D warnings` | clean | clean | PASS |
| `cargo test -p holler-cli --test wire_selftest` | ok | 3 passed | PASS |
| `cargo test -p holler-cli --test docs_cli_test` | ok | 3 passed | PASS |
| `cargo machete` | no unused deps | none found | PASS |
| `bash scripts/tests/pane-restart-test.sh` | exit 0, no `not ok` | exit 0, 0 `not ok` | PASS |
| `bash scripts/test-hooks.sh` | exit 0 | exit 0 | PASS |
| `gitleaks detect --no-git` on `docs/pane-control.md` | no leaks | no leaks found | PASS |
| `cargo test --workspace --no-fail-fast -- --skip roster_stays_accurate_under_concurrent_body_load` | all ok | 151 targets `ok`; 1 FAILED: `remote_admin_test::remote_admin_traffic_never_moves_the_bodys_last_seen` (`last_seen` moved by 1 s) | PASS (known flake) |

**The one workspace failure is the known flake #847** ("fails under the loaded workspace run", OPEN). I re-ran
`remote_admin_test` five times by itself: it passed 4 times and failed once with the same 1-second `last_seen` drift. The
diff touches no Rust source (`git diff origin/main...HEAD --stat`: docs, the docs test, its `[[test]]`, a comment
in `scripts/pane-restart`, CHANGELOG), so it cannot cause this failure.

**Cross-check with F.** F reported `docs_cli_test` 3 passed, `docs_pane_control_test` 16 passed, changelog-check ok and
lint exit 0. F did not re-run the workspace. My results match F's.

## Tier 2 results

| Check | Method | Result |
|---|---|---|
| Coverage | Acceptances 1-5 and 7-17 each have a test; 6 is `docs_cli_test`. S's two verdict items are now pinned (above) | PASS |
| Test quality | The extension reads its code from `PANE_UNMANAGED`, asserts per section, and fails alone under each mutation | PASS |
| Proportion | No new test function; three words added to an existing acceptance's table | PASS |
| Lint limits | Test file 523 lines; `#![allow]` still carries `// #149` | PASS |
| Secrets | gitleaks clean; the guide has no key-shaped strings | PASS |
| Protocol / goldens | No protocol-visible change | N/A |
| Evidence appendix | Three T entries added to `evidence.md` (`assess.rs:163-171`, `import.rs:18-20`, `unmanaged.rs:34-36`) | done |

## Acceptance criteria status

All 17 PASS, with the same tests as the table above. Acceptance 17 now also backs the re-cut R2 (sections 2 and 10) and
section 9's `pane-unmanaged`.

## Blocking issues

None.

## Advisory notes

- `UNMANAGED_COLUMNS` is private (`list.rs:175`), so the guide's second table is the only list of its columns that no test
  pins. A follow-up could make it `pub`, as `COLUMNS` is, and have `section_4_names_every_column` read both.
- The branch is one commit behind `origin/main` (`7fc00f1`: `docs/handoffs/655-herdr-move/decisions.md`). That commit
  does not touch any file in this diff.
- #847 (flake) will turn up in CI's workspace run from time to time. It is not this story's.
