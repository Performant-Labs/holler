# Handoff-T-green: Phase 7 - #645a `pane switch` and `pane reset` (GREEN + Tier 2, round 3)

**Date:** 2026-10-09
**Branch:** issue-645-implementation (worktree `.claude/worktrees/0645-switch-reset`, head `62aae72`, F's round-3 commit)
**Issue:** #645 (part 1 of 2, 645a)
**Handoff-F reviewed:** `docs/handoffs/645/handoff-F.md` (round 3, the outside diff gate's r2 BLOCK, B-1)
**Handoff-T-red:** `docs/handoffs/645/handoff-T-red.md`
**Previous GREEN:** round 1 at `583e9d4` (mutation table, AC table), round 2 at `4443dd4` (the no-step-before-the-act
repair). Both still hold and are summarised below.

Every build ran with `CARGO_BUILD_JOBS=4`. `origin/main` is still `d9eabbb` (fetched 23:51 MDT) and is an ancestor of HEAD.

## What changed since round 2

- **F (round 3):** `check_health`'s remedy fallback in `crates/holler-pane/src/tx_switch.rs` is now an explicit `match`
  instead of `Option::unwrap_or_else` (B-1). The behaviour is the same. Two doc comments changed (NIT-1 and the
  `check_health` doc), and evidence entries were added for NV-1. `git diff 4443dd4 62aae72 -- crates/` is that one file.
- **T (this round): NIT-2, a doc comment in a test file only.** The module doc of
  `crates/holler-cli/tests/pane_verbs/reset.rs:1-3` said "both formats, one envelope", which reads as if one run covered
  both formats. It now says "a text run and a JSON run on separate rigs, the same exit code, a valid envelope", which is
  what `both_with` (`switch.rs:119-141`) does. No test logic changed.

## GREEN confirmation

```
$ cargo test -p holler-cli --test pane_verbs
test result: ok. 154 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out
$ cargo test -p holler-cli --test pane_verbs -- switch:: reset::
test result: ok. 21 passed; 0 failed; 0 ignored; 0 measured; 133 filtered out
```

**Spot-check: the test fails when B-1's `Some` arm is removed.** I changed `Some(remedy) => remedy` to
`Some(_remedy) => doctor_command(pane, false)`, so the table's remedy is dropped:

```
test switch::switch_refuses_an_unhealthy_server ... FAILED
test result: FAILED. 20 passed; 1 failed; 0 ignored; 0 measured; 133 filtered out
```

It was restored with `git checkout`, and the tree held only T's doc edit afterwards. The `None` arm cannot be reached for a
named pane (`findings.rs:127-131`, already in `evidence.md`), so no test can pin it, and it does not need one. Round 2's
`acted` mutations (`if true` and `if false`) and round 1's mutations target code that round 3 did not change. Their tests
did not change either.

## Tier 1 results

| Check | Command | Expected | Actual | Result |
|---|---|---|---|---|
| Workspace tests | `cargo test --workspace --no-fail-fast -- --skip roster_stays_accurate_under_concurrent_body_load` (CI's form) | exit 0 | exit 0. 1642 passed, 0 failed, 14 ignored, 135 result lines. Includes `pane_verbs` 154, `pane_cli_process`, `cli_surface_test`, `docs_cli_test` and `wire_selftest`. | PASS |
| Clippy | `cargo clippy --workspace --all-targets -- -D warnings` | clean | clean | PASS |
| Lint | `bash scripts/lint.sh` | exit 0 | exit 0 | PASS |
| CHANGELOG | `bash scripts/changelog-check.sh` | ok | `changelog-check: ok` | PASS |
| rustfmt (AC 25) | `rustfmt --check --edition 2021` on the five story files | exit 0 | exit 0 | PASS |
| Unused deps | `cargo machete` | none | none | PASS |
| No new dependency | `git diff origin/main -- '*Cargo.toml' \| wc -l` | 0 | 0 | PASS |
| Server start / API smoke | n/a | n/a | in-process verbs over ports; the binary answers `not-implemented` until #649 | N/A |

**Cross-check against F.** F's counts reproduce: workspace 1642 / 0 / 14 over 135 lines, `pane_verbs` 154, the
`switch::`/`reset::` filter 21, clippy, lint, changelog-check, rustfmt and the Cargo.toml diff. There is no discrepancy.

## Tier 2 results

| Check | What was verified | Method | Result |
|---|---|---|---|
| B-1 fix | The `match` keeps the behaviour: `switch_refuses_an_unhealthy_server` still sees `run holler pane relaunch demo-c1r1`, and fails when the `Some` arm is dropped. No `unwrap`/`expect` remains on the line. | Test, mutation, read | PASS |
| NV-1 evidence | F's round-3 excerpt of `StoreScope::resolve` matches `crates/holler-cli/src/pane/profile_scope.rs:159-173` verbatim | Read source against `evidence.md` | PASS |
| Coverage | ACs 1-26 are backed as in rounds 1 and 2. Round 3 adds no behaviour. | Read | PASS |
| Test quality | NIT-2 settled. No test added or removed. Sizes: `switch.rs` 613 and `reset.rs` 288 lines, both under 900. | Read, `wc -l` | PASS |
| Invariants, not durations | No sleeps added | Read | PASS |
| Security, protocol, frozen files | Unchanged: no goldens, no wire change, no frozen file, no `#[allow]` | diff stat | PASS |
| Evidence appendix | Nothing new from T. The tests rely on no new fact outside the diff. | Read | PASS |

## Acceptance criteria status

ACs 1-26: **PASS**, backed by the same tests as round 1 (table at `583e9d4`), with round 2's additions (ACs 3-6, 10-12 and
17 also assert no reconcile step before the act). For this round:

- **AC 9 / the health refusal (I6):** `switch_refuses_an_unhealthy_server`, re-pinned against the `match` by the mutation
  above.
- **AC 25:** clippy, rustfmt, lint and the workspace tests all pass.
- **AC 26:** `changelog-check: ok`.

## Blocking issues

None.

## Advisory notes

- A-dup's warns 2-4 and the brief's follow-ups F-1 and F-3 still need issues filed by O. Once the `screen_text` fold is
  filed, the comment in `tx_switch.rs` that says "a follow-up of #645" should name that issue.
- F's observation stands: a `timeout` from reset's `create_session` may leave a session the message cannot name. That is
  a 645b question.
- `cargo fmt --all -- --check` still reports diffs in untouched files that predate this branch. This is housekeeping
  outside the story.

T-green complete, no blocking issues. No UI surface, so U is N/A. Ready for S.
