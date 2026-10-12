# Handoff-T-green: #835 - `holler pane import` leaves Claude Code panes alone, with `--adopt-claude-code` (cycle 4: after F cycle 5, the merge of `origin/main` after #857)

**Date:** 2026-10-12 (05:12 MDT)
**Branch:** `issue-835-implementation` at `59d8d59`. That is F cycle 5's two-parent commit: `cea1c17` (brief round 5) and
`origin/main` `b76f4d2` (PR #857). `git fetch` at about 04:50 MDT shows `origin/main` still at `b76f4d2`, and
`git merge-tree --write-tree HEAD origin/main` exits 0, so the branch is the PR's merge result.
**Issue:** #835
**Handoff-F reviewed:** `docs/handoffs/835/handoff-F.md` (cycle 5)
**Handoff-T-red:** `docs/handoffs/835/handoff-T-red.md` (cycle 2)

**Verdict: PASS.** This handoff replaces cycle 3's PASS (at `e6d5fa0`). This cycle is S's cycle-3 REWORK: the merge of
`main` after #857, plus prose.

## Tests repaired in this phase

None. F's "Tests that look wrong (for T)" says none, and I found none. No test was authored this cycle.

## Merge resolutions in test files (checked)

S's item 1e asked T-green to confirm GREEN on the merged tree because test files changed in the merge.

- `crates/holler-cli/tests/pane_integration/wired.rs`, `crates/holler-pane/tests/ports_test.rs` and
  `crates/holler-pane/tests/claude_code_rule_test.rs` are byte-identical to `origin/main`'s (`git diff --quiet origin/main
  HEAD -- <file>`). So is `crates/holler-pane/src/unmanaged.rs`. All four leave the PR's diff.
- `crates/holler-cli/tests/docs_pane_control_test.rs` differs from `main`'s only by this story's
  `section_3_names_every_flag_of_pane_import`. `main`'s `built_lines_marked_proposed` and its two tests are kept. The file
  runs 19 tests, all passing.
- `git show --remerge-diff 59d8d59` shows conflict-marker removal and, as code, only `observe.rs`'s merged `use` lines.
  `pane.rs` differs from `main` only by the three-limit doc paragraph of `is_claude_code_command`. There is one constant
  and one predicate.

## GREEN confirmation

```
$ cargo test --workspace --no-fail-fast -- --skip roster_stays_accurate_under_concurrent_body_load   (CI's step)
exit 0. 167 result groups: 2600 passed, 0 failed, 36 ignored

$ cargo test -p holler-cli --test docs_cli_test     -> 3 passed
$ cargo test -p holler-cli --test wire_selftest     -> 3 passed
```

Every test from T-red cycles 1 and 2 runs in the workspace pass and passes, including `pane_verbs`
(`import::harness::*`, `import::the_adrs_name_the_switch_and_the_two_new_codes`), `pane_integration`,
`docs_pane_control_test`, `claude_code_rule_test`, `import_test`, `ports_test` and the `holler-pane` doctests
(both `compile_fail,E0046`).

**Behaviour-removal spot check, repeated on the merged tree.** The import now calls `main`'s copy of
`is_claude_code_command`, so I checked that the tests still depend on it. I changed its body to `false` and ran
`cargo test -p holler-cli --test pane_verbs -- import::harness`. Result: 3 failed, 4 passed. The 3 that failed were
`a_pane_running_claude_is_left_alone_and_the_others_import_as_opencode`,
`a_dry_run_reports_the_pane_left_alone_and_writes_nothing` and `a_pane_left_alone_blocks_nothing_even_unlabelled`.
The 4 that passed pin the refusal and the unreadable paths, which do not depend on the rule. I restored the file with
`git checkout`, and the tree is clean.

## Tier 1 results

| Check | Command | Result |
|---|---|---|
| Workspace tests | as above | exit 0, PASS |
| Clippy | `cargo clippy --workspace --all-targets -- -D warnings` | exit 0, PASS |
| Lint | `bash scripts/lint.sh` | exit 0, PASS (size warnings only) |
| Changelog | `bash scripts/changelog-check.sh` | `changelog-check: ok`, PASS |
| Docs canary | `cargo test -p holler-cli --test docs_cli_test` | 3 passed, PASS |
| Guide tests | `docs_pane_control_test`, in the workspace run | 19 passed, PASS |
| Wire canary | `cargo test -p holler-cli --test wire_selftest` | 3 passed, PASS |
| Unused deps | `cargo machete` | none found, PASS |
| Rustdoc | `RUSTDOCFLAGS="-D warnings" cargo doc --no-deps -p holler-pane` | exit 0, PASS |
| Whitespace | `git diff --check origin/main...HEAD -- . ':(exclude)docs/handoffs'` | clean, PASS |
| Merge | `git merge-tree --write-tree HEAD origin/main` | exit 0, PASS |

**Cross-check of F's results:** every command F reported gives the same result here: 2600 passed, 36 ignored in 167
groups; 19 guide tests and 3 docs tests; clippy, lint, changelog-check, machete and rustdoc clean; the PR's diff is 21 files
outside the handoffs. The built binary's `holler pane import --help` says "refused (usage) until #831's adoption step
lands", as F reports. There is no discrepancy.

## Tier 2 results

| Check | Method | Result |
|---|---|---|
| Coverage per criterion | Unchanged since cycle 2. Every backing test passes on the merged tree | PASS |
| Test quality | No test authored or changed by this story this cycle. The one test function kept in the merge (`section_3_names_every_flag_of_pane_import`) is unchanged from cycle 2 and still passes beside `main`'s new tests. No test duplicates `main`'s `claude_code_rule_test` (one file, `main`'s) | PASS |
| Behaviour not implementation | The mutation above | PASS |
| Prose against S's item 2 | `git grep "lands" ... \| grep 831`: every hit names #831's adoption (or last) step, except ADR-0021:1145, `main`'s accurate "whichever of #835 and #831 lands second". No place still says the adoption waits for the Claude Code harness | PASS |
| Type safety / API / protocol | No interface change. No golden drift | PASS |
| Security / I7 | The text still says "a name only, never a command line". There is no secret in the new text | PASS |
| File size | `lint.sh` passes. Every `.rs` file in the PR's diff is under 900 lines | PASS |

## Acceptance criteria status

| Criterion | Status | Backing test |
|---|---|---|
| A `claude` pane is left alone: one `info` `harness-claude-code`, `skipped`, no record, no harness call | PASS | `import::harness::a_pane_running_claude_is_left_alone_and_the_others_import_as_opencode` |
| `bash` panes and empty sessions import as OpenCode, unchanged | PASS | the same test, and #650's fixtures |
| A dry run shows the same finding and writes nothing | PASS | `a_dry_run_reports_the_pane_left_alone_and_writes_nothing` |
| A pane left alone blocks nothing | PASS | `a_pane_left_alone_blocks_nothing_even_unlabelled` |
| `--adopt-claude-code` is `usage` naming #831, before any port call | PASS | `adopt_claude_code_is_usage_naming_831_and_writes_nothing`, `cli_surface_test` |
| An unreadable harness is `harness-unreadable`, a problem, never a guess | PASS | `a_failed_ps_...`, `a_failed_command_name_read_...`, `a_lost_chain_...` |
| One constant, one predicate, exact `claude` | PASS | `claude_code_rule_test` (`main`'s), the mutation above |
| A required port method, with fake and adapter behaviour | PASS | the `compile_fail` doctests in `ports.rs`, host conformance case 10, `fake_host_test`, `foreground_test` |
| Every decorator forwards (`foreground_commands`, `observe_pane`, `observe_panes`) | PASS | `pane_integration::bind`, `wired` (`main`'s), `observe` |
| `pane list` over the real wiring shows an unmanaged pane's directory and change time (R3-3) | PASS | the two R3-3 pins in `pane_integration::observe` |
| ADRs and CHANGELOG record it | PASS | `the_adrs_name_the_switch_and_the_two_new_codes`, `changelog-check` |
| #652's guide is told (brief section 6) | PASS | `docs_pane_control_test::import_findings_are_named_in_section_3_or_10`, `section_3_names_every_flag_of_pane_import` |

## Blocking issues

None.

## Advisory notes

- **F's Known issue 1** (two of `main`'s sentences go stale when this PR merges: ADR-0021 section 15 item 17's "until
  this harness exists" and #831's CHANGELOG "waits for #835") is for S to rule on. No test pins either.
- **The guide is 986 lines.** It is markdown, so the 900 guard does not apply.
- Earlier advisories still stand: the `rustfmt` drift in new test files, `harness.rs:2`'s "every pane process" (S's
  advisory 2, test-only and optional), and A-dup's test-only twins.
- **Evidence:** no test in this cycle relies on new unchanged-code facts, so `evidence.md` gets no new entry.
