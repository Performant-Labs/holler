# Handoff-T-green: Phase 7 - #645a `pane switch` and `pane reset` (GREEN + Tier 2, round 5)

**Date:** 2026-10-10 (00:58 MDT)
**Branch:** issue-645-implementation (worktree `.claude/worktrees/0645-switch-reset`, head `a1f01aa`, F's round-5 commit)
**Issue:** #645 (part 1 of 2, 645a)
**Handoff-F reviewed:** `docs/handoffs/645/handoff-F.md` (round 5, the outside diff gate's r4 BLOCK, B-1)
**Handoff-T-red:** `docs/handoffs/645/handoff-T-red.md`
**Previous GREEN:** round 1 at `583e9d4` (mutation table, AC table), round 2 at `4443dd4`, round 3 at `940e338`, round 4 at
`48f2395`. All still hold for the branch's own content.

Every build ran with `CARGO_BUILD_JOBS=4`. `origin/main` was fetched at 00:52 MDT and is now **`bd5e825`** (#642 part 2,
merged 00:50 MDT), which is **not** an ancestor of HEAD.

## What changed since round 4

- **F (round 5):** `tx_switch.rs` calls `select_session` directly in `switch` and maps its error with `acted`; the
  observation moved to a private `observe`, also mapped with `acted`. Doc comments on `acted`, `read` and `reset::run`.
  Behaviour, call order and messages are unchanged.
- **T (this round): one test added**, answering F's coverage note and the r4 gate's NV-2.
  `pane_verbs/switch.rs` `switch_observation_failure_names_the_reconcile_step` (`:433-467`, unit tier over the rig's
  fakes, as its neighbours). It injects `fail_next(HarnessOp::ShownSession, Timeout)` after a good select and asserts:
  exit 1 `timeout` in both formats; each message ends with the reconcile step; the record equals its pre-run value and
  no `CasPut` was made; the harness calls are exactly `[Health, ListSessions, SelectSession, ShownSession]`; and the TUI
  shows S2 (the act moved it, which is why the step is printed). Before this, only the mismatch path (an `Ok` that
  differs) exercised `observe`'s `acted` mapping; this pins the `Err` path. It does not duplicate AC 7 (mismatch,
  `unavailable`) or AC 8 (the select's own failure). A reset twin is not added: `created` comes from the same closure, and
  AC 19 already pins it on the observation's path. `switch.rs` is now 649 lines.

## GREEN confirmation

```
$ cargo test -p holler-cli --test pane_verbs --test pane_cli_process --test cli_surface_test --test docs_cli_test --test wire_selftest
cli_surface_test  3 passed; docs_cli_test 3 passed; pane_cli_process 35 passed; pane_verbs 162 passed; wire_selftest 3 passed
$ cargo test -p holler-cli --test pane_verbs -- switch:: reset::
test result: ok. 22 passed; 0 failed; 0 ignored; 0 measured; 140 filtered out
```

**Mutations on F's round-5 code** (each restored from a scratch copy; `cmp` clean, `git status` shows only the new test):

| Mutation in `tx_switch.rs` | Tests that fail |
|---|---|
| `observe(..).map_err(acted)?` becomes `observe(..)?` (`acted: false`) | `switch_mismatch_after_select_records_nothing` (AC 7), `reset_mismatch_records_nothing` (AC 19), and the new `switch_observation_failure_names_the_reconcile_step`. 19 passed, 3 failed |
| `select_session(..).map_err(acted)?` becomes `select_session(..)?` | `switch_select_failure_names_the_reconcile_step` (AC 8), `reset_failure_after_create_names_the_unrecorded_session` (AC 18). 20 passed, 2 failed |

F's two self-check mutations reproduce exactly, and the `acted` boundary is pinned on both sides of the call.

## Tier 1 results

| Check | Command | Expected | Actual | Result |
|---|---|---|---|---|
| Workspace tests | `cargo test --workspace --no-fail-fast -- --skip roster_stays_accurate_under_concurrent_body_load` (CI's form), 00:53-00:57 MDT | exit 0 | exit 0. 137 result lines: 1664 passed, 0 failed, 16 ignored (round 4's 1663 + the new test) | PASS |
| Clippy | `cargo clippy --workspace --all-targets -- -D warnings` | exit 0 | exit 0 | PASS |
| Lint | `bash scripts/lint.sh` | exit 0 | exit 0 (size warnings only) | PASS |
| CHANGELOG | `bash scripts/changelog-check.sh` | ok | `changelog-check: ok` | PASS |
| rustfmt | `rustfmt --check --edition 2021` on the five story files | exit 0 | exit 0 | PASS |
| Unused deps | `cargo machete` | none | none | PASS |
| Canary | `cargo test -p holler-cli --test wire_selftest` | ok | 3 passed | PASS |
| Merges into `origin/main` | `git merge-tree --write-tree HEAD origin/main` | exit 0 | **exit 1: `CONFLICT (content)` in `docs/adr/ADR-0021.md`** | **FAIL** |
| AC 24 / AC 25 against `origin/main` | `git diff origin/main -- docs/adr/ADR-0021.md \| grep -c '^@@'`; `git diff origin/main -- '*Cargo.toml' Cargo.lock \| wc -l` | 4; 0 | **5; 26** (#642 part 2's changes, shown reversed) | **FAIL** |
| Server start / API smoke | n/a | n/a | in-process verbs over ports; the binary answers `not-implemented` until #649 | N/A |

**Cross-check against F.** Every F count reproduces on F's tree, plus one for the new test: `pane_verbs` 161 + 1,
`switch::`/`reset::` 21 + 1, workspace 1663 + 1, `pane_cli_process` 35, `cli_surface_test` 3, `docs_cli_test` 3. F's
ADR-0021 "4 hunks" and empty Cargo diff were true against `cec1f82`; `origin/main` moved to `bd5e825` at 00:50 MDT, about
when F finished, so the discrepancy is the base, not F's measurement.

## Tier 2 results

| Check | What was verified | Method | Result |
|---|---|---|---|
| Coverage | ACs 1-26 backed as in rounds 1-4; the `acted` mapping of the observation's `Err` path now has a test | Read, mutation | PASS |
| Test quality | The new test names one behaviour, fails in isolation under the `observe` mutation for the right reason (message lacks the step), sits at the rig tier like AC 7/8, and asserts outcomes (exit, code, message, record, TUI), not internals beyond the call log the suite already asserts | Read, mutation | PASS |
| Invariants, not durations | No sleep added | Read | PASS |
| Security, protocol, frozen files | No golden or wire change, no `#[allow]`, no frozen file; F's diff is `tx_switch.rs` and a doc line in `reset.rs` | `git diff 48f2395 a1f01aa --stat` | PASS |
| `origin/main`'s new code vs this story | #642 part 2 changes `holler-pane` doc comments only (`lib.rs:32-33`, `ports.rs:13-15, 174-175`): `HarnessPort` is now final, signatures unchanged. Its real `select_session` "sends nothing when there is none [no TUI]" and fails; this story's `acted: true` on that path still errs on the safe side (a harmless reconcile step) | `git diff HEAD...origin/main -- crates/holler-pane` | PASS (no code impact) |
| Mergeability | ADR-0021 conflicts in "Deferred to named stories" | `git merge-tree` | **FAIL** (blocking, below) |

## Acceptance criteria status

ACs 1-23 and 26: **PASS**, backed by the same tests as rounds 1-4, plus the new observation-failure test under AC 7/8.
ACs 24 and 25: **PASS on the branch's own base, FAIL on the tree that will land**, exactly the case S's round-1 REWORK
named. The Workflow script opens the PR without merging `main`, so this must be fixed before S.

## Blocking issues

1. **F must merge `origin/main` (`bd5e825`, #642 part 2) and resolve the `docs/adr/ADR-0021.md` conflict by keeping both
   sides.** It is the only conflict; `CHANGELOG.md` auto-merges. The conflict is in "Deferred to named stories"
   (around `:628-644` of the merged file):
   - Keep this branch's mismatch-code bullet ("decided, `unavailable` (exit 1), section 8: #645 for switch and reset; #644
     to follow for launch and relaunch") in place of `origin/main`'s still-open form ("... : #644 and #645"), and keep the
     **PROPOSED (#645, pending the operator)** bullet.
   - Take `origin/main`'s `HerdrPort` bullet, which drops "`HarnessPort` in its final form: #635, then #642" because #642
     has now made it final, in place of this branch's combined bullet.
   - Keep `origin/main`'s two new bullets: applying `Pane.opencode_agent` (#642's last part, after #700) and stopping the
     harness server (#695).

   After the merge: `git diff origin/main -- docs/adr/ADR-0021.md` must again show exactly this story's four hunks, the
   `*Cargo.toml`/`Cargo.lock` diff must be empty, and `evidence.md` citations into `holler-pane/src/ports.rs` and
   `lib.rs` must be re-checked for line shifts (#642 part 2 changed `ports.rs:13-15` and `:174-175`; F's `ports.rs:198-199`
   entry may have moved). No production code or test change is expected. T then re-runs GREEN on the merged tree.

## Advisory notes

- F's "Known issues" stand for O: ADR-0021 `:465` "#645's and #646's, planned" goes stale once 645a lands; #644 may
  still conflict with ADR-0021 (R-1).
- Carried over: A-dup's warns, follow-ups F-1 to F-4, and the timed-out `create_session` question for 645b.

T-green found blocking issues. F must address [merge `origin/main` `bd5e825` and resolve the ADR-0021 "Deferred to named
stories" conflict keeping both sides]. Re-run A (if architecture changed) then T before proceeding to U/S.
