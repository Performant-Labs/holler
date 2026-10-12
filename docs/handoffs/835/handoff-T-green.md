# Handoff-T-green: #835 - `holler pane import` leaves Claude Code panes alone, with `--adopt-claude-code`

**Date:** 2026-10-11 (18:56 MDT)
**Branch:** `issue-835-implementation` at `9bd101b` (base `2d217dd`; `origin/main` is `bda1069`)
**Issue:** #835
**Handoff-F reviewed:** `docs/handoffs/835/handoff-F.md`
**Handoff-T-red:** `docs/handoffs/835/handoff-T-red.md`

**Verdict: BLOCK.** The branch is GREEN on its own. On a merge with current `origin/main`, one test from #652 fails,
and a PR's CI runs on that merge. Main's operator guide does not name the two new import codes. The fix is in the docs
and the merge, which belong to F. No test needs to change.

## Test repaired in this phase (F's "Tests that look wrong" 1)

- `crates/holler-cli/tests/pane_verbs/import/harness.rs`, `spec_names()`: the code read `spec["name"]`, but a profile
  spec's pane is under `pane` (`ProfileSpec.pane`, `crates/holler-pane/src/profile.rs:181`, no serde rename). It now
  reads `spec["pane"]`. This was my bug from Phase 4, and F was right. The two tests it broke now pass.
  `import::harness` passes 7 of 7, and `pane_verbs import::` passes 46 of 46.
- F's note 2 (`rustfmt --check` drift in the new test files) is cosmetic. CI runs no `rustfmt`, so I left it.

## GREEN confirmation

```
$ cargo test -p holler-cli --test pane_verbs import::
test result: ok. 46 passed; 0 failed; 0 ignored; 0 measured; 290 filtered out

$ cargo test --workspace --no-fail-fast -- --skip roster_stays_accurate_under_concurrent_body_load   (CI's step; pipeline.config.json test.unit)
exit 0. 153 result groups: 2377 passed, 0 failed, 32 ignored
```

That is F's count of 2375 plus the two tests repaired above. Every Phase 4 test passes:

- `import_test`, `claude_code_rule_test`, `ports_test` and the two `compile_fail` doctests in `ports.rs`
- `fake_host_test` and `host_conformance_test`, with both new mutant breaks now failing case 10 as they should
- `fake_tmux_test` and `foreground_test`
- `cli_surface_test`
- `pane_integration`: `bind`, `wired`, and `observe`'s two R3-3 pins
- `pane_verbs import::`

**Behaviour-removal spot checks.** Each mutation was a temporary edit to production code. Each file was restored and
`sha256sum -c` came back OK.

| Mutation | Result |
|---|---|
| `is_claude_code_command` always returns `false` | 3 of the left-alone `import::harness` tests FAIL (`a_pane_running_claude_...`, `a_dry_run_reports_...`, `a_pane_left_alone_blocks_nothing_...`), and so does `the_rule_is_the_exact_command_name_claude`. The other 4 harness tests do not depend on the rule and still pass, as they should. |
| `parse_foreground` keeps dead panes (`"0" \| "1" => push`) | `foreground_commands_names_every_live_pane_in_tmux_order` FAILS. |

The tests pin behaviour, not wiring.

## Tier 1 results

| Check | Command | Result |
|---|---|---|
| Lint | `bash scripts/lint.sh` | exit 0, PASS |
| Changelog | `bash scripts/changelog-check.sh` | `changelog-check: ok`, PASS |
| Clippy | `cargo clippy --workspace --all-targets -- -D warnings` | exit 0, PASS |
| Workspace tests | as above | exit 0, PASS |
| Docs canary | `cargo test -p holler-cli --test docs_cli_test` | 3 passed, PASS |
| Wire canary | `cargo test -p holler-cli --test wire_selftest` | 3 passed, PASS |
| Unused deps | `cargo machete` | none found, PASS |
| Whitespace | `git diff --check` | clean, PASS |
| Real tmux (ignored suite) | `cargo test -p holler-adapter-host --test real_tmux_test -- --ignored` on tmux 3.7c | 9 passed, including `the_tmux_host_passes_the_host_conformance_suite` (case 10). PASS |
| **Merge with `origin/main`** | `git merge-tree` (clean, no conflict), then the workspace suite on a scratch merge commit in a throwaway worktree with its own target dir, since removed | **1 FAILED**: `docs_pane_control_test::import_findings_are_named_in_section_3_or_10`. 154 groups: 2396 passed, 1 failed. **FAIL** |

The failing output on the merge:

```
---- import_findings_are_named_in_section_3_or_10 stdout ----
panicked at crates/holler-cli/tests/docs_pane_control_test.rs:103:5:
sections 3 and 10 do not name these import codes:
  `harness-claude-code`
  `harness-unreadable`
```

F's cross-check: my results match F's handoff on every command F ran. The one difference is the repaired test, now
GREEN.

## Tier 2 results

| Check | Method | Result |
|---|---|---|
| Coverage per criterion | each criterion maps to a test (below) | PASS |
| Test quality | I re-read the suite. Each test names one behaviour, and the mutations above show they fail for the right reason. Nothing is redundant: #650's `a_failed_ps_is_observe_failed_...` was removed and replaced, not duplicated. | PASS |
| Type safety | clippy `-D warnings` is clean. No new `#[allow]` in the diff. | PASS |
| Error paths | a failed `ps`, a failed name read (`unavailable` and `pane-not-found`), a lost chain, a malformed listing, a missing session, a missing tmux binary, a timeout | PASS |
| Data integrity | A Claude Code pane gets no record and is not in `fleet`. A dry run writes nothing. A failed or lost read is a `problem`, and a real run is refused. | PASS |
| API contract | `HostPort::foreground_commands` and `HerdrPort::observe_pane` are required (the `compile_fail,E0046` doctests). The `HostOp` name equals the adapter's `OP_*`. No protocol change and no golden drift. | PASS |
| Security / I7 | The adapter reads `#{pane_current_command}` only (a name), never a start command or argv. `foreground_test` pins that there is no `start_command` in the format. | PASS |
| File size | The largest changed test file is `fake_tmux_test.rs` at 820 lines (limit 900), and `lint.sh` passes. | PASS |
| Docs consistency after the merge | `docs_pane_control_test` (#652, on main) requires `docs/pane-control.md` sections 3 or 10 to name every `ImportFinding`. | **FAIL**, see Blocking issues |

## Acceptance criteria status

| Criterion | Status | Backing test |
|---|---|---|
| A `claude` pane is left alone: one `info` `harness-claude-code`, `skipped`, no record, no harness call | PASS | `a_pane_running_claude_is_left_alone_and_the_others_import_as_opencode` |
| `bash` panes and empty sessions import as OpenCode, unchanged | PASS | same test, A and C records |
| A dry run shows the same finding and writes nothing | PASS | `a_dry_run_reports_the_pane_left_alone_and_writes_nothing` |
| A pane left alone blocks nothing | PASS | `a_pane_left_alone_blocks_nothing_even_unlabelled` |
| `--adopt-claude-code` is `usage` naming #831, before any port call | PASS | `adopt_claude_code_is_usage_naming_831_and_writes_nothing`, `cli_surface_test` |
| An unreadable harness is `harness-unreadable`, a problem, never a guess | PASS | `a_failed_ps_...`, `a_failed_command_name_read_...`, `a_lost_chain_...` |
| A required port method, with fake and adapter behaviour | PASS | `ports.rs` doctests, conformance case 10, `fake_host_test`, `foreground_test`, `fake_tmux_test` |
| Every decorator forwards | PASS | `bind::the_binding_host_...`, `bind::the_wired_host_...`, `wired.rs` |
| ADRs and CHANGELOG record it | PASS on the branch | `the_adrs_name_the_switch_and_the_two_new_codes`, `changelog-check` |
| #652's guide is told (brief section 6) | **FAIL after merge** | `docs_pane_control_test::import_findings_are_named_in_section_3_or_10` |

## Blocking issues

1. **F must merge `origin/main` into the branch and extend `docs/pane-control.md`.**
   - **What fails:** after the merge, `import_findings_are_named_in_section_3_or_10` fails because the guide names
     neither `harness-claude-code` nor `harness-unreadable`. The test is from #652. It iterates `ImportFinding::ALL`, so
     it picks up the two codes this story adds. CI on the PR's merge ref will fail on it.
   - **Section 3 or 10** must name both codes in backticks: what each means and its level.
   - **The guide's `pane import` text** should also state:
     - the new default (a pane running `claude` is left alone and later shows in `pane doctor` as
       `unregistered-herdr-pane`), with the rule's limits;
     - that a failed tmux read is now `harness-unreadable` (a problem that blocks the pane) rather than an
       `observe-failed` warning;
     - that `--adopt-claude-code` is refused until #831.
   - Brief section 6 asked for this. F's Known issue 1 flagged it, but it was not yet known to fail a test.
2. **In the same merge, correct #840's two claims about #835** (F's Known issue 1 and A's W3). On main they say #835
   calls `unmanaged::is_unmanaged` / `unregistered`, but #835 calls `holler_pane::pane::is_claude_code_command` instead:
   - `crates/holler-pane/src/unmanaged.rs:66` and `:84`
   - `docs/adr/ADR-0021.md:972` (section 14)

   No test catches this, but after the merge the records would contradict ADR-0021 section 13's "Not the registry's
   rule" bullet.

After the merge, re-run the workspace suite, `docs_pane_control_test` and `docs_cli_test`. The `--adopt-claude-code`
help text and any guide command line must still parse.

## Advisory notes

- **`docs_pane_control_test::every_verb_is_named_...`** builds each verb's heading form from clap's *required*
  arguments. `--adopt-claude-code` is optional, so that test does not change. It passed on the merge.
- **F's design decision 1** keeps asking the server of a pane whose tmux read failed. That is harmless, the pane is
  blocked anyway, and #650's `tmux-session-missing` report is unchanged. S may weigh it.
- **`rustfmt --check` drift in new test files** is cosmetic. CI has no fmt step.
- **Evidence:** I appended one entry to `evidence.md` for the `ProfileSpec.pane` key that `spec_names()` relies on.
