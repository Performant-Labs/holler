# Handoff-T-green: #835 - `holler pane import` leaves Claude Code panes alone, with `--adopt-claude-code` (cycle 3)

**Date:** 2026-10-11 (20:42 MDT)
**Branch:** `issue-835-implementation` at `e6d5fa0`. That is F cycle 3's two-parent commit: `ccc53a0` (S's REWORK) and
`origin/main` `d6ae539` (#851). `git fetch` at 20:36 MDT shows `origin/main` still at `d6ae539`, and it is an ancestor
of the branch, so the branch is the PR's merge result.
**Issue:** #835
**Handoff-F reviewed:** `docs/handoffs/835/handoff-F.md` (cycle 3)
**Handoff-T-red:** `docs/handoffs/835/handoff-T-red.md` (cycle 2)

**Verdict: PASS.** This handoff replaces cycle 2's PASS (at `b488a33`). This cycle is S's REWORK 1, a text-only change,
and the merge with `main`.

## Tests repaired in this phase

None. F's "Tests that look wrong (for T)" is empty. S asked for no test, and no test pins this prose.

## What this cycle changed (checked)

`git show --remerge-diff e6d5fa0` (F's edits beyond the automatic merge) touches 5 files, all prose or doc comments:
`CHANGELOG.md`, `crates/holler-cli/src/pane/import.rs` (the `PaneImport` doc), `crates/holler-pane/src/import.rs`
(the module doc), `crates/holler-pane/src/import/observe.rs` (`read_tmux`'s doc) and `docs/pane-control.md` (sections 1
and 3, and section 10's fact). There is no code, test, protocol or golden change.

- Every place S named now says "foreground process", and every limits sentence names a `claude` that is not in the
  foreground (a wrapper script, a suspended job). That meets S's rule.
- The wording matches the adapter: `FOREGROUND_FORMAT` is `#{pane_dead} #{pane_current_command}`
  (`crates/holler-adapter-host/src/tmux.rs:35`), read once per pane of the session.
- `grep` for "each process", "a process named", "process in its tmux" and "names of its processes" in `crates`, the guide,
  ADR-0021 and the CHANGELOG finds only the test kit's fake, which describes the fake itself. ADR-0021:952 says "the
  command names of the pane's tmux session", and its next sentence defines them as the foreground process of each pane,
  so it is accurate.
- The built binary's `holler pane import --help` shows the new paragraph.
- The guide has 821 lines: 817 from this branch (unchanged by the rework), plus #851's 4.

## GREEN confirmation

```
$ cargo test --workspace --no-fail-fast -- --skip roster_stays_accurate_under_concurrent_body_load   (CI's step)
exit 0. 155 result groups: 2406 passed, 0 failed, 32 ignored

$ cargo test -p holler-cli --test docs_pane_control_test --test docs_cli_test --test wire_selftest
docs_cli_test 3 passed; docs_pane_control_test 17 passed; wire_selftest 3 passed
```

Every test from T-red cycles 1 and 2 is inside the workspace run and passes. The load average was about 6, and nothing
failed. F saw the `interrupt_test` races only before the merge, at a load average of 23. They did not recur here.

**Behaviour-removal spot check.** Not repeated in this cycle: no production code changed, so the cycle-1 and cycle-2
mutation checks still cover the same code (cycle 2's T-green, at `b488a33`).

## Tier 1 results

| Check | Command | Result |
|---|---|---|
| Workspace tests | as above | exit 0, PASS |
| Clippy | `cargo clippy --workspace --all-targets -- -D warnings` | exit 0, PASS |
| Lint | `bash scripts/lint.sh` | exit 0, PASS |
| Changelog | `bash scripts/changelog-check.sh` | `changelog-check: ok`, PASS |
| Docs canary | `cargo test -p holler-cli --test docs_cli_test` | 3 passed, PASS |
| Guide tests | `cargo test -p holler-cli --test docs_pane_control_test` | 17 passed, PASS |
| Wire canary | `cargo test -p holler-cli --test wire_selftest` | 3 passed, PASS |
| Unused deps | `cargo machete` | none found, PASS |
| Rustdoc | `RUSTDOCFLAGS="-D warnings" cargo doc --no-deps -p holler-pane` | exit 0, PASS (the module doc's intra-doc link resolves) |
| Whitespace | `git diff --check origin/main...HEAD -- . ':(exclude)docs/handoffs'` | clean, PASS |

**Cross-check of F's results:** every command F reported gives the same result here: 2406 passed in 155 groups, 17 and 3
passed, and clippy, lint, changelog-check and rustdoc are clean. There is no discrepancy.

## Tier 2 results

| Check | Method | Result |
|---|---|---|
| Coverage per criterion | Unchanged since cycle 2, and every backing test still passes (below) | PASS |
| Test quality | No test changed. S's REWORK says not to pin this prose with a test, and I agree: the guide tests pin the codes and flags, not sentences | PASS |
| Docs accuracy against the code | The new "foreground process of each tmux pane" read checked against `FOREGROUND_FORMAT` and `parse_foreground` in the adapter | PASS |
| Scope of the cycle | The remerge diff is limited to the 5 files above. #851's files come only from `main` and drop out of `git diff origin/main...HEAD` | PASS |
| Type safety / API / protocol | No code change. No golden drift | PASS |
| Security / I7 | The text still says "a name only, never a command line". There is no secret in the new text | PASS |
| File size | `lint.sh` passes, and no `.rs` file grew past its limit (doc comments only) | PASS |

## Acceptance criteria status

| Criterion | Status | Backing test |
|---|---|---|
| A `claude` pane is left alone: one `info` `harness-claude-code`, `skipped`, no record, no harness call | PASS | `import::harness::a_pane_running_claude_is_left_alone_and_the_others_import_as_opencode` |
| `bash` panes and empty sessions import as OpenCode, unchanged | PASS | the same test, and #650's fixtures |
| A dry run shows the same finding and writes nothing | PASS | `a_dry_run_reports_the_pane_left_alone_and_writes_nothing` |
| A pane left alone blocks nothing | PASS | `a_pane_left_alone_blocks_nothing_even_unlabelled` |
| `--adopt-claude-code` is `usage` naming #831, before any port call | PASS | `adopt_claude_code_is_usage_naming_831_and_writes_nothing`, `cli_surface_test` |
| An unreadable harness is `harness-unreadable`, a problem, never a guess | PASS | `a_failed_ps_...`, `a_failed_command_name_read_...`, `a_lost_chain_...` |
| A required port method, with fake and adapter behaviour | PASS | the `compile_fail` doctests in `ports.rs`, host conformance case 10, `fake_host_test`, `foreground_test`, `fake_tmux_test` |
| Every decorator forwards (`foreground_commands`, `observe_pane`, `observe_panes`) | PASS | `pane_integration::bind`, `wired`, `observe` |
| `pane list` over the real wiring shows an unmanaged pane's directory and change time (R3-3) | PASS | the two R3-3 pins in `pane_integration::observe` |
| ADRs and CHANGELOG record it | PASS | `the_adrs_name_the_switch_and_the_two_new_codes`, `changelog-check` |
| #652's guide is told (brief section 6), now with the foreground wording (S's REWORK 1) | PASS | `docs_pane_control_test::import_findings_are_named_in_section_3_or_10`, `section_3_names_every_flag_of_pane_import`, and a manual read |

## Blocking issues

None.

## Advisory notes

- **The guide is 821 lines,** which is S's advisory 1, plus #851's 4 lines. No check enforces the length.
- **F's design decision 5** (the `holler-pane` import module doc, beyond S's table) is consistent with the other places
  and correct.
- **`holler-cli` rustdoc with `-D warnings` fails on `main`** in files outside this diff (F's Known issue 2). CI has no doc
  step.
- Cycle 2's advisories still stand. They are the `rustfmt` drift in new test files and A-dup's warn 1 (the rig
  duplication, test-only).
- **Evidence:** no test in this cycle relies on unchanged code, so `evidence.md` gets no new entry.
