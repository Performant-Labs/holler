# Handoff-T-green: Phase 7 - #645a `pane switch` and `pane reset` (GREEN + Tier 2)

**Date:** 2026-10-09
**Branch:** issue-645-implementation (worktree `.claude/worktrees/0645-switch-reset`, head `fa59fe7`)
**Issue:** #645 (part 1 of 2, 645a)
**Handoff-F reviewed:** `docs/handoffs/645/handoff-F.md`
**Handoff-T-red:** `docs/handoffs/645/handoff-T-red.md`

Every build ran with `CARGO_BUILD_JOBS=4`. The load average was about 16 on 24 cores.

## GREEN confirmation

F changed no test file: `git diff a912c4b fa59fe7 -- crates/holler-cli/tests` is empty. F listed no test as looking
wrong, and T repaired none.

```
$ cargo test -p holler-cli --test pane_verbs
test result: ok. 113 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out
```

RED was 94 passed and 19 failed. All 19 pass now, and so do the 94 that passed before, doctor's tests included.

**Spot-check: the tests fail when the behaviour is removed.** Each mutation below was applied to
`crates/holler-pane/src/tx_switch.rs`, `pane_verbs` was run, and the file was restored with `git checkout`. The tree is
clean afterwards.

| Mutation | Tests that fail |
|---|---|
| P5 skipped (`let _ = check_unclaimed(..)`) | `switch_to_another_panes_session_is_refused` |
| P4 skipped (`let _ = check_listed(..)`) | `switch_to_a_deleted_session_changes_nothing` |
| O1's mismatch ignored (`if true \|\| !shown_differs(..)`) | `switch_mismatch_after_select_records_nothing`, `reset_mismatch_records_nothing` |
| `last_observed.at` not set | 5: ACs 1, 6, 10, 13, 14 |
| `harness.health` not set to `Healthy` | the same 5 |
| no reconcile step in `message` | 5: ACs 7, 8, 9, 18, 19 |
| no "created and is not recorded" note | `reset_failure_after_create_names_the_unrecorded_session` |

All 8 mutations are caught. The first try at the P5 mutation deleted the call outright. That left `check_unclaimed`
unused, so the build failed under `dead_code`, which says nothing about the tests. The retry above keeps the helper
in use.

## Tier 1 results

| Check | Command | Expected | Actual | Result |
|---|---|---|---|---|
| Lint | `bash scripts/lint.sh` | exit 0 | exit 0 (only the 600+ line warnings, none for a file this story touches) | PASS |
| CHANGELOG | `bash scripts/changelog-check.sh` | ok | `changelog-check: ok` | PASS |
| Clippy | `cargo clippy --workspace --all-targets -- -D warnings` | clean | clean | PASS |
| rustfmt (AC 25) | `rustfmt --check --edition 2021` on `tx_switch.rs`, `pane/switch.rs`, `pane/reset.rs`, `tests/pane_verbs/switch.rs`, `reset.rs` | exit 0 | exit 0 | PASS |
| Workspace tests | `cargo test --workspace -- --skip roster_stays_accurate_under_concurrent_body_load` (CI's form) | exit 0 | exit 0. 1523 passed, 0 failed, across 132 result lines. It includes `pane_verbs` 113, `pane_cli_process`, `cli_surface_test`, `docs_cli_test` and `wire_selftest`. | PASS |
| Docs CLI | `docs_cli_test` (in the workspace run) | pass | pass | PASS |
| Canary | `wire_selftest` (in the workspace run) | pass | pass | PASS |
| Unused deps | `cargo machete` | none | "didn't find any unused dependencies" | PASS |
| No new dependency | `git diff dc300ab HEAD -- '*Cargo.toml' \| wc -l` | 0 | 0 | PASS |
| Server start / API smoke | n/a | n/a | The verbs are in-process over ports. The real binary answers `not-implemented` until #649, as F showed. | N/A |

`cargo fmt --all -- --check` does report diffs, but only in files this story does not touch: `holler-body/src/acp_driver/answerable.rs`
and `holler-cli/tests/support/*`. They come from before this change, and neither CI nor `lint.sh` runs it.

**Cross-check against F's handoff.** F's `pane_verbs -- switch:: reset::` result (20 passed) agrees with T's run of the
whole target. F's "Tier 1 self-check" still has its `cargo test --workspace` line as the literal placeholder
`WORKSPACE_RESULT`, so F's handoff records no workspace result. T's run above fills that gap: exit 0, and the
`join_held_test` flake F reported did not recur. The `changelog-check` and ADR grep results match F's.

## Tier 2 results

| Check | What was verified | Method | Result |
|---|---|---|---|
| Coverage | ACs 1-19 and 23 each have a test. ACs 20-22 are covered by the existing surface suites, and ACs 24-26 by the checks below. | Mapped the RED table to the brief's ACs | PASS |
| Test quality | Each test names one AC behaviour, and the mutations show each fails in isolation for the right reason. They sit at the cheapest sufficient tier, in-process over the doctor rig's fakes; a unit test of the engine would duplicate them and miss the CLI path. Every assertion is on behaviour: exit code, envelope, text line, record, TUI, calls. The suite (599 + 267 lines) is in proportion to 26 ACs. Nothing redundant needs deleting. | Read both files; mutations | PASS |
| Invariants, not durations | No sleeps or timings. The clock is asserted as a window (`start..=end`). The concurrency case (AC 9) is deterministic: the other writer is injected inside `select_session`, so it does not depend on a race. | Read | PASS |
| Type safety / panics | No `unwrap`/`expect`/indexing in production code. P1 uses `find`, and the remedy falls back through `unwrap_or_else`. No new `#[allow]`. | Read, grep | PASS |
| Error handling | Every refusal, failure and fault path has a test: usage, P1-P5, A1 (through AC 17's no-create), A2 fault, O1 mismatch, CAS conflict. | Tests + mutations | PASS |
| Data integrity | A refusal leaves the record, its generation and the TUI unchanged (`assert_unchanged` on every refusal). A success sets exactly four fields plus generation +1. A conflict writes nothing more. | Tests | PASS |
| API contract | JSON `data` = `{verb, pane, previous}`; `data.pane` equals the stored record; text lines match the brief exactly; exit code parity text/JSON | Tests | PASS |
| Security | SESSION is typed to `[A-Za-z0-9_-]{1,64}` before any port call, and the escape case shows no raw ESC in the message. Ids go through `quoted`. Nothing is typed into a TUI: no Herdr or host call in any run (I4). No secret is involved. | Tests, read | PASS |
| Protocol / goldens | No protocol change, and no golden file touched | diff stat | N/A |
| File size / `#[allow]` | All files are under 900 lines (tx_switch 319, switch.rs 599, reset.rs 267). There is no `#[allow]` in the changed files. | `wc -l`, grep | PASS |
| Frozen files | The only `doctor.rs` change is the one-line `pub(crate) mod rig;`. No `lib.rs`, `ports.rs`, `mod.rs`, `args.rs`, `output.rs` or `Cargo.toml` changed. | diff stat | PASS |
| Evidence appendix | T added 2 entries for test-relied facts in unchanged test-kit code: the shared data directory (AC 4) and `concurrent_put`'s generation +1 (AC 9). | `evidence.md`, "Added by T" | PASS |

**Merge readiness (advisory, not a blocker).** `origin/main` has moved past `e327569` to `d9eabbb` (#643, #662 part 2,
#646 park and unpark, #663). `git merge-tree --write-tree HEAD origin/main` merges cleanly. The only shared files are
ADR-0021 and the CHANGELOG. In the merged ADR-0021, `### 8.` is at line 285, the #645 paragraph at 345, and `### 9.` at
366, so the paragraph is still inside section 8. #646's "Park and unpark as built" paragraph sets no rule for switch or
reset, which agrees with F's sentence "Neither verb checks the pane's park state". #663's `--profile` helper is used by
no verb yet ("#649 wires them in"), so `execute` calling `ports.scope.resolve` directly does not duplicate it. #644's
paragraph is still not on main, so F's form of the "Deferred" bullet stays correct.

## Acceptance criteria status

| AC | Status | Backed by |
|---|---|---|
| 1 | PASS | `switch_moves_the_tui_and_the_record_together` |
| 2 | PASS | the same test and `reset_creates_a_fresh_session_and_switches_to_it` (exact call lists); the `both` I4 check on every run |
| 3 | PASS | `switch_to_a_deleted_session_changes_nothing` |
| 4 | PASS | `switch_to_another_panes_session_is_refused` |
| 5 | PASS | `switch_refuses_an_unhealthy_server` |
| 6 | PASS | `switch_refuses_the_orchestrators_pane_unless_as_operator` |
| 7 | PASS | `switch_mismatch_after_select_records_nothing` |
| 8 | PASS | `switch_select_failure_names_the_reconcile_step` |
| 9 | PASS | `switch_record_conflict_after_the_act` |
| 10 | PASS | `switch_in_a_profile` |
| 11 | PASS | `switch_unknown_pane` |
| 12 | PASS | `switch_usage` |
| 13 | PASS | `switch_to_the_current_session_is_idempotent` |
| 14 | PASS | `reset_creates_a_fresh_session_and_switches_to_it` |
| 15 | PASS | `reset_is_doctors_remedy_for_no_session_of_record`, `reset_is_doctors_remedy_for_a_deleted_session_of_record` |
| 16 | PASS | `reset_leaves_the_old_session_as_a_stray` |
| 17 | PASS | `reset_refusals_create_nothing` |
| 18 | PASS | `reset_failure_after_create_names_the_unrecorded_session` |
| 19 | PASS | `reset_mismatch_records_nothing` |
| 20 | PASS | `process/stub.rs` has no 645 entry and keeps `// #645` (line 25); `pane_cli_process` passes |
| 21 | PASS | the `# #645` block in `cli-surface.txt` is exactly the brief's six lines; `cli_surface_test` and `docs_cli_test` pass |
| 22 | PASS | `ADR-0003.md:51-52` read as the brief gives them, with `#645` in the shared column |
| 23 | PASS | `help_names_the_arguments` |
| 24 | PASS | ADR-0021 hunks touch only section 8's end, row 340 (now 361), section 11's end and "Deferred". `Switch and reset as built (#645)` appears once, at 308, between `### 8.` (265) and `### 9.` (329). The `#645` count is 9 against main's 6. `docs_cli_test` passes. |
| 25 | PASS | clippy, rustfmt on the five files, lint.sh, workspace tests and Cargo.toml diff are all clean (Tier 1) |
| 26 | PASS | one `### Enhancements` entry under `[Unreleased]`, ending "Part of #645"; `changelog-check: ok` |

## Blocking issues

None.

## Advisory notes

- F's handoff left the `WORKSPACE_RESULT` placeholder unfilled ("Tier 1 self-check"). T's workspace run above is the
  record of that check.
- F already noted one coverage hole: no test pins the order of the usage errors when an argv has two bad arguments.
  The order follows the brief by construction (`request` types `PANE`, then `--profile`, then the target). A test is
  not needed for 645a.
- The branch needs an ordinary merge of `origin/main` (`d9eabbb`) before the PR. It merges cleanly (see "Merge
  readiness").
- `cargo fmt --all -- --check` fails on untouched files that predate this branch. That is worth a separate
  housekeeping issue, outside this story.
