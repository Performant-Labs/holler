# Handoff-T-green: #835 - `holler pane import` leaves Claude Code panes alone, with `--adopt-claude-code` (cycle 2)

**Date:** 2026-10-11 (19:40 MDT)
**Branch:** `issue-835-implementation` at `4604693` (F cycle 2 over T-red's two-parent merge `526bbd3` of `origin/main` `bda1069`)
**Issue:** #835
**Handoff-F reviewed:** `docs/handoffs/835/handoff-F.md` (cycle 2)
**Handoff-T-red:** `docs/handoffs/835/handoff-T-red.md` (cycle 2)

**Verdict: PASS.** This handoff replaces cycle 1's BLOCK (at `25ff495`). Both of its blocking issues are resolved:
the branch now contains `origin/main`, and the guide and #840's records are corrected. `git fetch` at 19:40 MDT shows
`origin/main` still at `bda1069`, the merge base, so the branch is the PR's merge result: CI tests the same tree.

## Tests repaired in this phase

None. F's "Tests that look wrong (for T)" is empty, and no test needed a change.

## GREEN confirmation

```
$ cargo test -p holler-cli --test docs_pane_control_test
test result: ok. 17 passed; 0 failed     (import_findings_are_named_in_section_3_or_10 ok;
                                          section_3_names_every_flag_of_pane_import ok)

$ cargo test --workspace --no-fail-fast -- --skip roster_stays_accurate_under_concurrent_body_load   (CI's step)
exit 0. 154 result groups: 2398 passed, 0 failed, 32 ignored
```

That is T-red cycle 2's run on the same merged tree (2396 passed, 2 failed) with its two RED tests now passing, and it
equals F's count. Cycle 1's suite (import, rule, ports, host fake, conformance, adapter, CLI surface, pane_integration
`bind`/`wired`/`observe`, `pane_verbs import::`) is inside that run and still passes.

**Behaviour-removal spot check.** A temporary edit of `docs/pane-control.md` renamed `harness-unreadable` and
`--adopt-claude-code`. Both tests failed, each naming exactly the removed text:

```
test import_findings_are_named_in_section_3_or_10 ... FAILED     `harness-unreadable`
test section_3_names_every_flag_of_pane_import ... FAILED        --adopt-claude-code
test result: FAILED. 15 passed; 2 failed
```

The file was restored from a copy, and `sha256sum -c` came back OK. `git status` was clean afterwards. Cycle 1's
production-code mutations (the rule, the `parse_foreground` filter) cover code F did not change in this cycle.

## Tier 1 results

| Check | Command | Result |
|---|---|---|
| Workspace tests | as above | exit 0, PASS |
| Clippy | `cargo clippy --workspace --all-targets -- -D warnings` | clean, PASS |
| Lint | `bash scripts/lint.sh` | exit 0, PASS |
| Changelog | `bash scripts/changelog-check.sh` | `changelog-check: ok`, PASS |
| Docs canary | `cargo test -p holler-cli --test docs_cli_test` | 3 passed, PASS (the new synopsis parses) |
| Wire canary | `cargo test -p holler-cli --test wire_selftest` | 3 passed, PASS |
| Unused deps | `cargo machete` | none found, PASS |
| Rustdoc | `RUSTDOCFLAGS="-D warnings" cargo doc --no-deps -p holler-pane` | clean, PASS (the new intra-doc link in `unmanaged.rs` resolves) |
| Whitespace | `git diff --check origin/main...HEAD -- crates docs/pane-control.md docs/adr CHANGELOG.md` | clean, PASS |

**Cross-check of F's results:** every command F reported gives the same result here: 17 and 3 passed, 2398 passed in
the workspace, clippy, lint and changelog clean. No discrepancy.

## Tier 2 results

| Check | Method | Result |
|---|---|---|
| Coverage per criterion | each criterion maps to a test (below), including A's W4 pin | PASS |
| Test quality | No test changed in this cycle. The new `section_3_names_every_flag_of_pane_import` reads the flags from the clap tree, so it is not vacuous (it found the verb's five other flags), and it does not duplicate #652's heading test, which checks only required flags. The spot check shows both doc tests fail for the right reason. | PASS |
| Docs accuracy against the code | I read each new claim of the guide against the source. `observe-failed` comes only from `Server::finding` for the two harness calls (`import/observe.rs:177-187`; `grep ObserveFailed` finds no other producer). The four `usage` causes are `Input::read`'s checks, in that order, before `plan` and before any pane file is read (`pane/import.rs:145-146`, `:174-197`, and `orchestrators`). The ADR anchor `#13-migration-from-the-launchers-files` is the one the guide already uses at `:767`. | PASS |
| Stale records (cycle 1's blocking issue 2) | `grep`: no "#831 and #832" or "#831 and #835" remains in `crates` or ADR-0021; `unmanaged.rs:66` and `:86` and ADR-0021 section 14 now say the import does not call the registry rule. | PASS |
| Type safety / API / protocol | No code change in this cycle: doc comments only. No golden drift. | PASS |
| Security / I7 | The guide says the read is "a name only, never a command line", which matches the adapter (cycle 1's `foreground_test` pin). No secret in any new text. | PASS |
| File size | `docs_pane_control_test.rs` is 549 lines. `docs/pane-control.md` is 817 lines; `lint.sh` checks only `crates/**/*.rs`. | PASS |

## Acceptance criteria status

| Criterion | Status | Backing test |
|---|---|---|
| A `claude` pane is left alone: one `info` `harness-claude-code`, `skipped`, no record, no harness call | PASS | `import::harness::a_pane_running_claude_is_left_alone_and_the_others_import_as_opencode` |
| `bash` panes and empty sessions import as OpenCode, unchanged | PASS | same test, and #650's fixtures |
| A dry run shows the same finding and writes nothing | PASS | `a_dry_run_reports_the_pane_left_alone_and_writes_nothing` |
| A pane left alone blocks nothing | PASS | `a_pane_left_alone_blocks_nothing_even_unlabelled` |
| `--adopt-claude-code` is `usage` naming #831, before any port call | PASS | `adopt_claude_code_is_usage_naming_831_and_writes_nothing`, `cli_surface_test` |
| An unreadable harness is `harness-unreadable`, a problem, never a guess | PASS | `a_failed_ps_...`, `a_failed_command_name_read_...`, `a_lost_chain_...` |
| A required port method, with fake and adapter behaviour | PASS | `ports.rs` `compile_fail` doctests, host conformance case 10, `fake_host_test`, `foreground_test`, `fake_tmux_test` |
| Every decorator forwards (`foreground_commands`, `observe_pane`, `observe_panes`) | PASS | `pane_integration::bind`, `wired`, `observe` |
| `pane list` over the real wiring shows an unmanaged pane's directory and change time (R3-3) | PASS | `pane_integration::observe`'s two R3-3 pins |
| ADRs and CHANGELOG record it | PASS | `the_adrs_name_the_switch_and_the_two_new_codes`, `changelog-check` |
| #652's guide is told (brief section 6) | PASS | `docs_pane_control_test::import_findings_are_named_in_section_3_or_10`, `section_3_names_every_flag_of_pane_import` |

## Blocking issues

None.

## Advisory notes

- **The guide is 817 lines** (F's Known issue 1), past #652's soft "about 800". No check enforces it. S's call.
- **"before any file is read"** in the exit-code paragraph: `Input::read` lists the state root's directory before the
  `--orchestrator` check, but reads no pane file and calls no port first. The sentence is accurate for files; no change
  needed.
- **Section 10, step 1's extra sentence** (the real run also runs while the Claude Code sessions run) is F's design
  decision 3, beyond W4's list. It is correct: the real run observes the panes again (`plan` runs in both modes).
- Cycle 1's advisories still stand: `rustfmt --check` drift in new test files (CI has no fmt step), and F's design
  decision 1 (a pane whose tmux read failed still gets its server asked), which is harmless since the pane is blocked.
- **Evidence:** this cycle's tests rely on `ImportFinding::ALL` (in the diff) and the clap tree (in the diff), so no
  `evidence.md` entry is added. F's three cycle-2 facts cover the guide's claims about unchanged code.
