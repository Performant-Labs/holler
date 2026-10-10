# Handoff-T-green: Phase 7 - #645a `pane switch` and `pane reset` (GREEN + Tier 2, round 4)

**Date:** 2026-10-10 (00:34 MDT)
**Branch:** issue-645-implementation (worktree `.claude/worktrees/0645-switch-reset`, head `29fa0ff`, F's round-4 commit)
**Issue:** #645 (part 1 of 2, 645a)
**Handoff-F reviewed:** `docs/handoffs/645/handoff-F.md` (round 4, S's REWORK item 1: merge `origin/main`)
**Handoff-T-red:** `docs/handoffs/645/handoff-T-red.md`
**Previous GREEN:** round 1 at `583e9d4` (mutation table, AC table), round 2 at `4443dd4`, round 3 at `940e338`. All still
hold; round 4 changes no code or test of this story.

Every build ran with `CARGO_BUILD_JOBS=4`. `origin/main` was fetched at 00:26 MDT and is `abdcbb6`, an ancestor of HEAD.

## What changed since round 3

- **F (round 4):** `29fa0ff` is a merge commit with parents `ddb6fc3` (S's REWORK) and `abdcbb6` (`origin/main`). It brings
  in #713 (#660, the output envelope conformance suite) and #714 (#640 part 3, the Herdr adapter). The ADR-0021 conflict
  is resolved by keeping both sides, and five `evidence.md` citations or excerpts are corrected.
- `git diff ddb6fc3 HEAD -- crates/holler-pane crates/holler-cli/src crates/holler-cli/tests/pane_verbs/switch.rs
  crates/holler-cli/tests/pane_verbs/reset.rs` touches only `holler-pane`'s `error.rs`, `pane.rs`, `ports.rs` and
  `reconcile.rs`, which are #714's doc comments. None of this story's files changed.
- **T (this round): nothing.** F flags no wrong test, and none needed repair.

## GREEN confirmation

```
$ cargo test -p holler-cli --test pane_verbs --test pane_cli_process --test cli_surface_test --test docs_cli_test --test wire_selftest
cli_surface_test  3 passed; 0 failed
docs_cli_test     3 passed; 0 failed
pane_cli_process 35 passed; 0 failed   (34 + #660's new stub test)
pane_verbs      161 passed; 0 failed   (154 + #660's 7)
wire_selftest     3 passed; 0 failed
$ cargo test -p holler-cli --test pane_verbs -- switch:: reset::
test result: ok. 21 passed; 0 failed; 0 ignored; 0 measured; 140 filtered out
$ cargo test -p holler-cli --test pane_cli_process -- a_forced_diagnostic_in_json_mode_leaves_stdout_one_envelope
test stub::a_forced_diagnostic_in_json_mode_leaves_stdout_one_envelope ... ok
```

**Spot-check on the merged tree: the tests still bite.** I made P4 (`check_listed`, `tx_switch.rs:256`) always pass
(`if true || ...`):

```
test switch::switch_to_a_deleted_session_changes_nothing ... FAILED
test result: FAILED. 20 passed; 1 failed; 0 ignored; 0 measured; 140 filtered out
```

I restored it with `git checkout`, re-ran the 21 (`ok. 21 passed`), and `git status --short` was empty. #660's stub test
now picks `pane launch` (#644), since switch and reset have left `STUBS`. The `// #645` comment is kept with no entries.

## Tier 1 results

| Check | Command | Expected | Actual | Result |
|---|---|---|---|---|
| Workspace tests | `cargo test --workspace --no-fail-fast -- --skip roster_stays_accurate_under_concurrent_body_load` (CI's form, `ci.yml:128`), 00:26-00:30 MDT | exit 0 | exit 0. 137 result lines: 1663 passed, 0 failed, 16 ignored | PASS |
| Clippy | `cargo clippy --workspace --all-targets -- -D warnings` | exit 0 | exit 0 | PASS |
| Lint | `bash scripts/lint.sh` | exit 0 | exit 0 | PASS |
| CHANGELOG | `bash scripts/changelog-check.sh` | ok | `changelog-check: ok` | PASS |
| rustfmt (AC 25) | `rustfmt --check --edition 2021` on the five story files | exit 0 | exit 0 | PASS |
| Unused deps | `cargo machete` | none | none | PASS |
| Canary | `cargo test -p holler-cli --test wire_selftest` | ok | 3 passed | PASS |
| No new dependency (AC 25) | `git diff origin/main -- '*Cargo.toml' Cargo.lock \| wc -l` | 0 | 0 | PASS |
| Server start / API smoke | n/a | n/a | in-process verbs over ports; the binary answers `not-implemented` until #649 | N/A |

**Cross-check against F.** Every F count reproduces: workspace 1663 / 0 / 16 over 137 lines, `pane_verbs` 161, the
`switch::`/`reset::` filter 21, `pane_cli_process` 35, `cli_surface_test` 3 and `docs_cli_test` 3. Clippy, lint,
changelog-check, rustfmt and the Cargo diff also match. There is no discrepancy.

## Tier 2 results

| Check | What was verified | Method | Result |
|---|---|---|---|
| AC 24 on the merged tree | `git diff origin/main -- docs/adr/ADR-0021.md` has 4 hunks: the #645 paragraph at `:345`, between `### 8.` (`:285`) and `### 9.` (`:372`); the section 9 row; the section 11 sentence; the "Deferred" bullets. `#645` occurs 9 times (6 on `origin/main`). #714's `pane launch` row (ending `grid-unreachable`) and its `HarnessPort`/`HerdrPort` bullet are kept, not reverted | Read the diff, `grep` | PASS |
| Conflict residue | No `<<<<<<<`, `=======` or `>>>>>>>` line in `ADR-0021.md`, `CHANGELOG.md` or `process/stub.rs` | `grep -c` | PASS |
| CHANGELOG merge | The #645 entry sits under `[Unreleased]` beside #647's and #714's entries, and its "Part of #645" is kept | Read the diff | PASS |
| Evidence appendix | Checked the moved citations myself: `error.rs:691-698` (`excerpt`), `:654-656` and `:676-682` (`Display`) match their excerpts verbatim on the merged tree | Read source against `evidence.md` | PASS |
| Coverage | ACs 1-26 are backed as in rounds 1-3. The merge adds no behaviour to this story | Read | PASS |
| Test quality | No test added or removed by this story this round. Sizes: `switch.rs` 613 and `reset.rs` 288, both under 900 | `wc -l` | PASS |
| Invariants, not durations | No sleep added | Read | PASS |
| Security, protocol, frozen files | No golden or wire change, no `#[allow]`, no frozen file | diff stat | PASS |

## Acceptance criteria status

ACs 1-26: **PASS**. They are backed by the same tests as rounds 1-3 (table at `583e9d4`, round 2's additions at
`4443dd4`). S's two NOT-MET rows are now met on the tree that will land:

- **AC 24:** exactly the four hunks against `origin/main` (`abdcbb6`), and the branch contains `origin/main`.
- **AC 25:** clippy, rustfmt, lint and the workspace tests in CI's form pass on the merged tree, #713's and #714's tests
  included. The Cargo diff is empty.

## Blocking issues

None.

## Advisory notes

- F's "Known issues" stand for O. ADR-0021 `:465` says "#645's and #646's, planned", which goes stale once 645a lands. #644
  and #642 part b may conflict with ADR-0021 again (R-1).
- Carried over: A-dup's warns, follow-ups F-1 to F-4, and the timed-out `create_session` question for 645b.
- `cargo fmt --all -- --check` still reports diffs in untouched files that predate this branch. They are outside this story.

T-green complete, no blocking issues. No UI surface, so U is N/A. Ready for S.
