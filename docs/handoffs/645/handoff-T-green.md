# Handoff-T-green: Phase 7 - #645a `pane switch` and `pane reset` (GREEN + Tier 2, round 2)

**Date:** 2026-10-09
**Branch:** issue-645-implementation (worktree `.claude/worktrees/0645-switch-reset`, head `37f2101`. That is F's
round-2 commit, the merge of `origin/main` `d9eabbb` into `da53aba`.)
**Issue:** #645 (part 1 of 2, 645a)
**Handoff-F reviewed:** `docs/handoffs/645/handoff-F.md` (round 2, the A-dup BLOCK rework)
**Handoff-T-red:** `docs/handoffs/645/handoff-T-red.md`
**Previous GREEN:** round 1 is in git at `583e9d4`. Its mutation table and AC table still hold, and are summarised below.

Every build ran with `CARGO_BUILD_JOBS=4`. `origin/main` is still `d9eabbb` (fetched at 23:36 MDT) and is an ancestor of
HEAD.

## What changed since round 1

- **F (round 2):** merged `origin/main`, added four sentences to ADR-0021's "Switch and reset as built (#645)"
  paragraph, and corrected two doc comments in `tx_switch.rs`. No code changed. `git diff 583e9d4 HEAD` on the story's
  three production files and two test files shows only the `tx_switch.rs` comments.
- **T (this round): a test repair, test files only.** The new ADR sentence says that a failure before `select_session` is
  called has no reconcile step. The doc comment says the same, "a reset's `create_session` included". No test pinned this.
  Mutating `SwitchFailure::message` to always append the step (`tx_switch.rs:143`, `if self.acted` changed to `if true`)
  left **all 20** switch and reset tests green. The repair:
  - `crates/holler-cli/tests/pane_verbs/switch.rs`: new helper `failed_before_the_act`, which is `failed` plus an assertion
    that no message contains `to reconcile`. It is used at all 9 refusal and pre-act failure sites: P1-P5, the health
    `timeout`, the orchestrator refusal, `--profile` scope and usage.
  - `crates/holler-cli/tests/pane_verbs/reset.rs`: AC 17's refusals use the helper. There is a new test,
    `reset_create_failure_changes_nothing`, for the brief's A1 row (`acted: false`, `created: None`): a failed
    `create_session` gives exit 1 `unavailable` with no step and no "was created", calls no `select_session`, and leaves
    the record and the TUI unchanged.
  - The evidence entry for the fake's `create_session` fault is in `evidence.md`, under "Added by T (Phase 7, GREEN,
    round 2)".

## GREEN confirmation

```
$ cargo test -p holler-cli --test pane_verbs
test result: ok. 154 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out
```

154 is the 153 after the merge plus the one new reset test.

**Spot-checks: the tests fail when the behaviour is removed** (`tx_switch.rs:143`, restored with `git checkout` each
time, and the tree is clean afterwards):

| Mutation | Before the repair | After the repair |
|---|---|---|
| no step ever (`if false`) | 5 fail (ACs 7, 8, 9, 18, 19) | 5 fail (the same) |
| a step always (`if true`) | **0 fail** | 8 fail: `reset_create_failure_changes_nothing`, `reset_refusals_create_nothing`, `switch_in_a_profile`, `switch_refuses_an_unhealthy_server`, `switch_refuses_the_orchestrators_pane_unless_as_operator`, `switch_to_a_deleted_session_changes_nothing`, `switch_to_another_panes_session_is_refused`, `switch_unknown_pane` |

Round 1's other mutations (P4 and P5 skipped, O1 ignored, `at` and `health` not set, no "created" note) target code that
round 2 did not change, and their tests are unchanged. `switch_usage` is not reached by the `if true` mutation, because
usage errors are reported before the engine runs. Its assertion holds and is harmless.

## Tier 1 results

| Check | Command | Expected | Actual | Result |
|---|---|---|---|---|
| Workspace tests | `cargo test --workspace --no-fail-fast -- --skip roster_stays_accurate_under_concurrent_body_load` (CI's form, merged tree before T's repair) | exit 0 | exit 0. 1641 passed, 0 failed, 14 ignored, 135 result lines. This equals F's counts and includes `pane_verbs` 153, `pane_cli_process`, `cli_surface_test`, `docs_cli_test` and `wire_selftest`. | PASS |
| After T's repair | `cargo test -p holler-cli --test pane_verbs`, then `--test docs_cli_test --test cli_surface_test --test pane_cli_process --test wire_selftest` | pass | 154; 3, 3, 34, 3 passed | PASS |
| Clippy | `cargo clippy --workspace --all-targets -- -D warnings` (before and after the repair) | clean | clean | PASS |
| Lint | `bash scripts/lint.sh` | exit 0 | exit 0. Only the existing size warnings, none for a story file. | PASS |
| CHANGELOG | `bash scripts/changelog-check.sh` | ok | `changelog-check: ok` | PASS |
| rustfmt (AC 25) | `rustfmt --check --edition 2021` on the five story files | exit 0 | exit 0 | PASS |
| Unused deps | `cargo machete` | none | none | PASS |
| No new dependency | `git diff origin/main -- '*Cargo.toml' \| wc -l` | 0 | 0 | PASS |
| Server start / API smoke | n/a | n/a | in-process verbs over ports; the binary answers `not-implemented` until #649 | N/A |

**Cross-check against F.** F's workspace counts (1641, 0 failed, 14 ignored, 135 lines), `pane_verbs` 153,
`pane_cli_process` 34, `cli_surface_test` 3, `docs_cli_test` 3, clippy, lint, changelog-check and rustfmt all reproduce.
The AC 24 checks reproduce too: the paragraph is at line 345, between `### 8.` (285) and `### 9.` (372); `#645` occurs 9
times against 6 on `origin/main`; and the diff against `origin/main` has four hunks (section 8's end, the section 9 row,
section 11's end, "Deferred"). There is no discrepancy.

## Tier 2 results

| Check | What was verified | Method | Result |
|---|---|---|---|
| ADR against code | The new sentences match `switch()`. `From<PaneError>` gives `acted: false` for the plan and for `create_session`. The `acted` closure wraps only `select_and_observe` and `cas_put`. The step names the pane without `--profile`. | Read `tx_switch.rs:150-176` | PASS |
| ADR against #663's rule | The exception's reasons hold. A doctor run named by pane without `--profile` refuses only a pane with no record (`reconcile.rs:226-236`), and `doctor <pane> --fix` is the remedy for `ShownDrivenMismatch` (`findings.rs:135-141`). F's evidence quotes match the source. | Read source against `evidence.md` | PASS |
| Coverage | Round 1's ACs 1-26 are unchanged and backed as before. The stated boundary ("no step before `select_session`") is now pinned at every pre-act exit, A1 included. | Mutation, read | PASS (after T's repair) |
| Test quality | The helper adds one behaviour, asserted where it holds, with no new runner. The new test is one A1 case at the in-process tier, which is the cheapest tier that reaches `create_session` through the CLI, and it duplicates no other test. Size: `switch.rs` 613 and `reset.rs` 288 lines, both under 900. | Read, `wc -l` | PASS |
| Invariants, not durations | No sleeps. The new test uses the deterministic `fail_next` fault. | Read | PASS |
| Data integrity | `assert_unchanged` covers the new case: record, generation and TUI. | Test | PASS |
| Security, protocol, frozen files | Unchanged from round 1: no goldens, no wire change, no frozen file, no `#[allow]` | diff stat, grep | PASS |
| Evidence appendix | One T entry for the fake's `create_session` fault (`harness.rs:292-297`, `fault.rs:72-77`) | `evidence.md` | PASS |

## Acceptance criteria status

ACs 1-26: **PASS**, backed by the same tests as round 1 (the table at `583e9d4`). Notes for this round:

- **ACs 3-6, 10-12, 17:** these now also assert that the refusal carries no reconcile step.
- **AC 24:** re-verified on the merged tree (see the cross-check above).
- **AC 25:** clippy, rustfmt, lint and the workspace tests all pass.
- **AC 26:** `changelog-check: ok`.

## Blocking issues

None. The gap found this round was in the tests, and T fixed it. The production code already behaves as the ADR states.

## Advisory notes

- A-dup's warns 2-4, and the brief's follow-ups F-1 and F-3, still need issues filed by O. `docs/handoffs/` is deleted
  before the push.
- F's observation stands: a `timeout` from reset's `create_session` may leave a session the message cannot name. That is
  a 645b question. The new test uses `unavailable`, which leaves nothing behind.
- `cargo fmt --all -- --check` still reports diffs in untouched files that predate this branch. This is housekeeping
  outside the story.
