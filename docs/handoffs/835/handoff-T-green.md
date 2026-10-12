# Handoff-T-green: #835 - `holler pane import` leaves Claude Code panes alone, with `--adopt-claude-code` (cycle 5: after F cycle 6, the `harness-claude-code` message)

**Date:** 2026-10-12 (05:58 MDT)
**Branch:** `issue-835-implementation` at `5dd002e` (F cycle 6's commit). `git fetch` at about 05:52 MDT: `origin/main`
is still `b76f4d2`, an ancestor of `HEAD`, and `git merge-tree --write-tree HEAD origin/main` exits 0.
**Issue:** #835
**Handoff-F reviewed:** `docs/handoffs/835/handoff-F.md` (cycle 6)
**Handoff-T-red:** `docs/handoffs/835/handoff-T-red.md` (cycle 2)

**Verdict: PASS.** This handoff replaces cycle 4's PASS (at `6787d99`). F cycle 6 changed one production string: the
message of `Outcome::claude_code` (`crates/holler-pane/src/import/assess.rs:104-106`), per S's cycle-4 REWORK item 1.

## Tests repaired in this phase

F's "Tests that look wrong (for T)" says none, and no test is wrong. I applied S's Advisory 3 (test-only, optional),
which changes only the module doc of `crates/holler-cli/tests/pane_verbs/import/harness.rs`; no assertion changed:

- line 2: "the command name of every pane process in the pane's tmux session" is now "the command name of the
  foreground process of each tmux pane in the pane's session", which is what `HostPort::foreground_commands` reads
  (`ports.rs:387-392`, `evidence.md` section "T-green, cycle 5");
- line 9: "naming #831 until that story lands" is now "naming #831, / until #831's adoption step lands", with the
  phrase on one source line, so `git grep -n "until #831's adoption step lands"` now finds the test's doc beside the
  help, the message and the documents (S's Advisory 2 list).

No test pins the message's wording, and none should: the message is a constant whose only contract is its finding
code and level, which `harness.rs:163` (the text prefix `  info harness-claude-code: `) and the JSON assertions pin.
Pinning the prose would make #831's adoption step edit a test for a wording change.

## GREEN confirmation

```
$ cargo test --workspace --no-fail-fast -- --skip roster_stays_accurate_under_concurrent_body_load   (CI's step)
exit 0. 167 result groups: 2600 passed, 0 failed, 36 ignored     (on 5dd002e, before the doc edit)

$ cargo test -p holler-cli --test pane_verbs                       -> 347 passed, 0 failed  (after the doc edit)
$ cargo test -p holler-cli --test pane_verbs -- import::harness    -> 7 passed
$ cargo test -p holler-cli --test docs_cli_test                    -> 3 passed
$ cargo test -p holler-cli --test wire_selftest                    -> 3 passed
```

**Behaviour-removal spot check, repeated on `5dd002e`.** I changed `is_claude_code_command`'s body
(`crates/holler-pane/src/pane.rs:244-246`) to `false` and ran `import::harness`: 3 failed
(`a_pane_running_claude_is_left_alone_and_the_others_import_as_opencode`,
`a_dry_run_reports_the_pane_left_alone_and_writes_nothing`, `a_pane_left_alone_blocks_nothing_even_unlabelled`),
4 passed (the refusal and the three unreadable paths, which do not depend on the rule). I restored the file with
`git checkout -- crates/holler-pane/src/pane.rs`, then re-ran the whole `pane_verbs` target: 347 passed.

## Tier 1 results

| Check | Command | Result |
|---|---|---|
| Workspace tests | as above | exit 0, PASS |
| Clippy | `cargo clippy --workspace --all-targets -- -D warnings` (after the doc edit) | exit 0, PASS |
| Lint | `bash scripts/lint.sh` (after the doc edit) | exit 0, PASS (size warnings only; `harness.rs` is 341 lines) |
| Changelog | `bash scripts/changelog-check.sh` | `changelog-check: ok`, PASS |
| Docs canary | `cargo test -p holler-cli --test docs_cli_test` | 3 passed, PASS |
| Wire canary | `cargo test -p holler-cli --test wire_selftest` | 3 passed, PASS |
| Unused deps | `cargo machete` | none found, PASS |
| Rustdoc | `RUSTDOCFLAGS="-D warnings" cargo doc --no-deps -p holler-pane` | exit 0, PASS |
| Whitespace | `git diff --check origin/main -- . ':!docs/handoffs'`; `git diff --check` | clean, clean, PASS |
| Stale claim | `git grep -n "cannot drive" -- crates/holler-pane/src/import` | prints nothing (exit 1), PASS |
| Merge | `git merge-tree --write-tree HEAD origin/main` | exit 0, PASS |

**Cross-check of F's results:** every command F reported gives the same result here: 2600 passed, 0 failed,
36 ignored in 167 groups; `import::harness` 7 passed; clippy, lint, changelog-check, machete and rustdoc clean; the
"cannot drive" grep is empty; `origin/main` unchanged. There is no discrepancy.

## Tier 2 results

| Check | Method | Result |
|---|---|---|
| S's four conditions on the message | Read `assess.rs:104-106`. (1) No claim that Holler cannot drive Claude Code. (2) It says the import does not adopt such a pane until #831's adoption step lands, naming `--adopt-claude-code`, true on the merged tree (`CHANGELOG.md:11-15`, ADR-0021 section 15 item 17). (3) "until #831's adoption step lands" is on one source line (`:105`). (4) A `&str` literal with no observed text | PASS |
| Coverage per criterion | Unchanged since cycle 2; every backing test passes | PASS |
| Test quality | The only test change is a doc comment. No test added, none redundant | PASS |
| Behaviour not implementation | The mutation above | PASS |
| Type safety / API / protocol | No interface change; the finding code and level are unchanged; no golden drift | PASS |
| Security / I7 | The message is a constant: no observed text, no command line, no secret | PASS |
| File size | Every `.rs` in the PR's diff is under 900 lines | PASS |

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

- S's Advisory 3 is done (above). S's Advisory 6 (the `rustfmt` drift in new test code, the test-only Herdr wrappers,
  A-dup's W2 twins) is carried, unchanged and non-blocking.
- The doc edit is uncommitted, for the phase commit, with this handoff and the `evidence.md` and `decisions.md`
  appends.
- `evidence.md`: one new entry (section "T-green, cycle 5") for the port doc the test header now paraphrases.
