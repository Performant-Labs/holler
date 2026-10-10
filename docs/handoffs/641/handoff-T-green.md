# Handoff-T-green: Phase 7 - #641 host adapter (tmux sessions, process control, the launcher primitive)

**Date:** 2026-10-09
**Branch:** issue-641-implementation (worktree `.claude/worktrees/0641-host-adapter`, on top of db56a87)
**Issue:** #641
**Handoff-F reviewed:** `docs/handoffs/641/handoff-F.md`
**Handoff-T-red:** `docs/handoffs/641/handoff-T-red.md`

## Test change in this phase (T's own file)

F reported no wrong test and flagged one coverage gap. `a_window_gone_before_its_tag_is_ok_and_signals_nothing`
pinned only the brief's `can't find window: @7`. tmux 3.7c actually prints `no such window: @7` for the tag of a
closed window (F's probe, Design decision 1). Before this phase, only the opt-in real-tmux AC 11 and AC 13 reached that
text, so CI never did. The test now loops over both stderrs and asserts `Ok` and an empty kill record for each.
`fake_tmux_test.rs` grows from 745 to 753 lines, still under 900. No production code was changed.

## GREEN confirmation

All runs were from the worktree on 2026-10-09, between 19:00 and 19:06 MDT, with tmux 3.7c.

```
$ cargo test -p holler-adapter-host
fake_tmux_test:  test result: ok. 28 passed; 0 failed; 0 ignored; ... finished in 0.81s
real_tmux_test:  test result: ok. 0 passed; 0 failed; 9 ignored        (not opted in: "needs tmux; run with --ignored")

$ cargo test -p holler-adapter-host --test real_tmux_test -- --ignored      (run twice, before and after the test edit)
test result: ok. 9 passed; 0 failed; 0 ignored; ... finished in 1.04s-1.06s
afterwards: no /tmp/hlr-tmux-* directory, and no demo-* session on the default socket

flake check (fake_tmux_test binary, after the edit): 24 runs as 3 x 8 parallel copies, 0 failures;
10 sequential runs, 28/28 every time
```

**Mutation spot-check (do the tests pin behaviour?).** I removed one behaviour at a time from F's committed source,
ran `fake_tmux_test`, and restored the file with `git checkout`. `git status` shows `src/` unchanged afterwards. Every
mutant was killed:

| Mutant (production source) | Killed by |
|---|---|
| `classify` drops `no such window` (tmux.rs) | `a_window_gone_before_its_tag_is_ok_and_signals_nothing` (the new case) |
| `new_window` drops the `env --` prefix (tmux.rs) | `run_prefixes_a_one_element_argv_with_env` |
| `escape_cwd` stops doubling `#` (tmux.rs) | `ensure_session_escapes_the_cwd_for_tmux` |
| `kill_group` drops `LC_ALL=C` (exec.rs) | `stop_owned_terms_only_owned_live_groups_then_kills_a_live_survivor` |
| `parse_listing` owns every live pane, whatever its tag (tmux.rs) | `stop_owned_terms_only_owned_live_groups_then_kills_a_live_survivor` |
| `parse_listing` counts a dead pane (`pane_dead` 1) as live (tmux.rs) | `a_tagged_pane_listed_dead_counts_as_gone`, `stop_owned_terms_only_...` |
| `settle` never sends the `kill -s 0` group probe (W-14, lib.rs) | `a_group_that_outlives_its_pane_is_killed_after_the_grace`, `a_session_that_ends_on_term_still_has_its_group_checked`, `a_kill_failure_other_than_no_such_process_is_unavailable` |

## Tier 1 results

| Check | Expected | Actual | Result |
|---|---|---|---|
| `bash scripts/lint.sh` | exit 0 | exit 0 (warns: `fake_tmux_test.rs` 753 lines, plus two files in other crates) | PASS |
| `bash scripts/changelog-check.sh` | ok | `changelog-check: ok` | PASS |
| `cargo clippy --workspace --all-targets -- -D warnings` | no warning | Finished, no warning | PASS |
| `rustfmt --check --edition 2021` on the 3 src files and 3 test files | clean | clean | PASS |
| `cargo machete` | no unused dep | none found | PASS |
| `cargo test -p holler-cli --test wire_selftest` (canary) | pass | 3 passed | PASS |
| `cargo test -p holler-cli --test docs_cli_test` | pass | 3 passed | PASS |
| `bash scripts/test-hooks.sh` | pass | all ok | PASS |
| `cargo test --workspace --no-fail-fast -- --skip roster_stays_accurate_under_concurrent_body_load` (CI's form) | pass | 127 suites: 1407 passed, 4 failed, 14 ignored | PASS (environmental failures, below) |
| `HOLLER_STATE_DIR=<empty scratch dir> cargo test -p holler-cli --test logging_test` | pass | 11 passed | PASS |
| AC 7 grep (`pkill\|killall\|pgrep\|pidof` in `src/`, outside comments) | nothing | nothing | PASS |
| `grep -rn unsafe crates/holler-adapter-host` | nothing | nothing | PASS |

The 4 workspace failures are `holler-cli/tests/logging_test.rs` (`debug_flag_beats_env`, `env_none_loses_to_flag_noisy`,
`banner_names_resolved_level_and_format`, `log_output_stays_off_stdout`), each `Unexpected success ... code=0` from
`holler --debug none roster`. The helper inherits the environment, and this machine runs a live hub that the default
state directory reaches. With an empty `HOLLER_STATE_DIR`, all 11 pass. No `holler-cli` file changed on this branch, and
`holler-cli` does not depend on `holler-adapter-host`. A CI runner has no hub. **Cross-check with F:** these are the same
counts and the same 4 tests F reported, so there is no discrepancy.

## Tier 2 results

| Check | Method | Result |
|---|---|---|
| Coverage of every AC | each AC mapped to a test (below) | PASS |
| Test quality and proportion | 28 default-run tests plus 9 opt-in tests, each named for one behaviour; the 7 mutants above each fail at least one test; no two tests pin the same vector. The fake tier is the cheapest that sees the real argv and environment (a unit test cannot). No test is redundant. | PASS |
| Type safety | clippy `-D warnings` is clean; no `unsafe`; no `unwrap`/`expect` in `src/` (F's note, spot-read) | PASS |
| Error paths | timeout, missing binary, missing-class and other stderr, bad argv, bad session dir, malformed `new-window`, out-of-range pid, failed tag, failed `kill` are all tested | PASS |
| Data integrity | `n/a`: no store or schema. Ownership is a tmux tag, pinned by the stop tests and the mutants | PASS |
| API contract | the conformance suite passes on real tmux (AC 1); `Timeout.op` equals `HostOp::as_str` (AC 6a) | PASS |
| Security | argv never goes through a shell (AC 6d, 11); `;` and `#` are escaped (6d, 6g, 11); exact targets (6e, 12); signals only to owned groups (6h, 7, 3); `what` and `usage` never echo the argv or cwd sentinel (6c, 6g) | PASS |
| Secrets absent from errors | `HLR_SENTINEL_641=s3cr3t` and the cwd sentinel are asserted absent from `what`/`message` | PASS |
| Concurrency and flakes | waits are invariants and polls, with no fixed sleep as an assertion; 34 suite runs with no flake | PASS |
| Protocol, goldens, `docs/protocol/v2.md` | not touched: the adapter has no wire surface | n/a |
| Migrations | none | n/a |
| Evidence appendix | `evidence.md` has F's 6 entries. The new test case relies on tmux's behaviour (a probe in `handoff-F.md`), not on unchanged source, so I added no entry | PASS |
| Playwright / browser | none in this repo | n/a |

## Acceptance criteria status

| AC | Status | Backed by |
|---|---|---|
| 1 Conformance | PASS | `the_tmux_host_passes_the_host_conformance_suite` (real tmux) |
| 2 Ensure twice | PASS | `ensure_session_twice_makes_one_session` |
| 3 Stop only owned | PASS | `stop_owned_kills_only_the_owned_process` |
| 4 Escalation, W-14 | PASS | `stop_owned_escalates_to_kill` |
| 5 Missing session typed | PASS | `missing_session_is_typed` |
| 6a-6i No tmux by default | PASS | the 28 `fake_tmux_test` tests, as mapped in `handoff-T-red.md` |
| 7 No broad kill | PASS | `no_broad_kill_in_source` and the grep |
| 8 Quality gates | PASS | Tier 1 above (the workspace failures are environmental) |
| 9 Isolation | PASS | `hosts_are_built_by_one_helper_and_real_tmux_tests_use_private_sockets`; the grep is clean on `real_tmux_test.rs`. The AC's literal grep over all of `tests/` also hits `fake_tmux_test.rs:327,330`, which is the AC 6f socket-flag case against the fake. That scoping is W-20, journalled at RED. |
| 10 CHANGELOG | PASS | one `[Unreleased]`/`Enhancements` entry links #641 and names no host; `changelog-check` ok |
| 11 Argv and cwd exact | PASS | `argv_and_cwd_pass_exactly` |
| 12 Targets exact | PASS | `targets_are_exact` |
| 13 Run in session cwd | PASS | `run_works_in_the_session_cwd` |
| 14 Refuse missing or relative dir | PASS | `run_refuses_a_missing_or_relative_directory` |

## Blocking issues

None.

## Advisory notes

- F's Decision 1 (`no such window`) was probed only on tmux 3.7c. The default-run test now pins that both texts are
  read as a closed window, whatever the version.
- AC 9's literal grep is wider than its intent (see the AC 9 row). S may want the brief's wording reconciled with W-20.
  This needs no code change.
- `logging_test` fails on any developer machine with a live hub. That is a pre-existing test-isolation gap in
  `holler-cli`, outside this story, and is worth a follow-up issue.

T-green complete, no blocking issues. No UI surface — U is N/A, ready for S.
