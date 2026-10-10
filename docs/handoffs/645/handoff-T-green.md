# Handoff-T-green: Phase 7 - #645a `pane switch` and `pane reset` (GREEN + Tier 2, round 6)

**Date:** 2026-10-10 (01:20 MDT)
**Branch:** issue-645-implementation (worktree `.claude/worktrees/0645-switch-reset`, head `6dd44f3`, F's round-6 commit,
a merge with parents `7435610` and `bd5e825`)
**Issue:** #645 (part 1 of 2, 645a)
**Handoff-F reviewed:** `docs/handoffs/645/handoff-F.md` (round 6, T-green round 5's BLOCK: merge `origin/main`)
**Handoff-T-red:** `docs/handoffs/645/handoff-T-red.md`
**Previous GREEN:** round 1 at `583e9d4` (mutation table, AC table), round 2 at `4443dd4`, round 3 at `940e338`, round 4 at
`48f2395`, round 5 at `7435610` (the observation-failure test and its mutation table). All still hold for the branch's own
content.

Every build ran with `CARGO_BUILD_JOBS=4`. `origin/main` was fetched at 01:09 MDT and is still **`bd5e825`**, which is now
an ancestor of HEAD.

## What changed since round 5

- **F (round 6):** merged `origin/main` (`bd5e825`: #715 and #642 part 2). The one conflict, ADR-0021's "Deferred to named
  stories", is resolved by keeping both sides, as round 5's blocking issue asked. `evidence.md` citations moved by the
  merge are updated. No production code and no test changed: `git diff 7435610 6dd44f3 -- tx_switch.rs pane/switch.rs
  pane/reset.rs tests/pane_verbs/switch.rs tests/pane_verbs/reset.rs` is empty.
- **T (this round):** no test changed. F lists no test that looks wrong.

## GREEN confirmation

```
$ cargo test -p holler-cli --test pane_verbs --test pane_cli_process --test cli_surface_test --test docs_cli_test --test wire_selftest
cli_surface_test 3 passed; docs_cli_test 3 passed; pane_cli_process 35 passed; pane_verbs 162 passed; wire_selftest 3 passed
$ cargo test -p holler-cli --test pane_verbs -- switch:: reset::
test result: ok. 22 passed; 0 failed; 0 ignored; 0 measured; 140 filtered out
$ cargo test -p holler-pane
98 passed, 0 failed
```

**Spot-check mutation on the merged tree** (restored from a scratch copy; `cmp` clean, `git status` empty afterwards):

| Mutation in `tx_switch.rs` | Tests that fail |
|---|---|
| `observe(ports, &record, &target).map_err(acted)?` (`:177`) becomes `observe(..)?` | `switch_mismatch_after_select_records_nothing` (AC 7), `reset_mismatch_records_nothing` (AC 19), `switch_observation_failure_names_the_reconcile_step`. 19 passed, 3 failed |

Identical to round 5's result, so the merge did not weaken what the suite pins.

## Tier 1 results

| Check | Command | Expected | Actual | Result |
|---|---|---|---|---|
| Workspace tests | `cargo test --workspace --no-fail-fast -- --skip roster_stays_accurate_under_concurrent_body_load` (CI's form), 01:09-01:13 MDT | exit 0 | exit 101: 140 result lines, 1699 passed, **1 failed** (`body_run_test::fresh_hello_and_presence_on_every_reconnect`), 25 ignored | FLAKE (see below) |
| The failed target, repeated | `cargo test -p holler-cli --test body_run_test`, 5 runs | ok | 10 passed, 0 failed, all 5 runs | PASS |
| Clippy | `cargo clippy --workspace --all-targets -- -D warnings` | exit 0 | exit 0 | PASS |
| Lint | `bash scripts/lint.sh` | exit 0 | exit 0 | PASS |
| CHANGELOG | `bash scripts/changelog-check.sh` | ok | `changelog-check: ok` | PASS |
| rustfmt | `rustfmt --check --edition 2021` on the five story files | exit 0 | exit 0 | PASS |
| Unused deps | `cargo machete` | none | none | PASS |
| Canary | `cargo test -p holler-cli --test wire_selftest` | ok | 3 passed | PASS |
| Merges into `origin/main` | `git merge-tree --write-tree HEAD origin/main` | exit 0 | exit 0 | PASS |
| AC 24 / AC 25 against `origin/main` | `git diff origin/main -- docs/adr/ADR-0021.md \| grep -c '^@@'`; `git diff origin/main -- '*Cargo.toml' Cargo.lock \| wc -l` | 4; 0 | 4; 0 | PASS |
| Conflict markers | `git grep -nE '^(<<<<<<<\|>>>>>>>\|=======$)'` | none | none | PASS |
| Server start / API smoke | n/a | n/a | in-process verbs over ports; the binary answers `not-implemented` until #649 | N/A |

**The one workspace failure is not this story's.** `fresh_hello_and_presence_on_every_reconnect` panicked at
`body_run_test.rs:499` with "hub did not report listening within 10s: Disconnected" after 0.68s: the restarted hub exited
before it logged `listening`, which is what a hub that cannot bind its fixed restart port does while other worktrees'
builds and tests run on this machine. This story changes nothing in `holler-hub`, `holler-body`,
`holler-cli/tests/support` or `body_run_test.rs` (`git diff origin/main --stat` on them is empty), and the target passed in
5 of 5 re-runs. F's run of the same command at 01:02-01:06 MDT was exit 0 with 1700 passed. That is 1699 + this flake.

**Cross-check against F.** Every F count reproduces: `pane_verbs` 162, `switch::`/`reset::` 22, `pane_cli_process` 35,
`cli_surface_test` 3, `docs_cli_test` 3, `wire_selftest` 3, `holler-pane` 98, workspace 1700 (1699 + the flake), 25 ignored,
140 result lines, ADR-0021 4 hunks, Cargo diff empty.

## Tier 2 results

| Check | What was verified | Method | Result |
|---|---|---|---|
| Coverage | ACs 1-26 backed as in rounds 1-5; no test or production change this round | `git diff 7435610 6dd44f3` on the story files | PASS |
| Test quality | Unchanged since round 5; the mutation above still fails the same three tests | Mutation | PASS |
| Merge resolution | "Deferred to named stories" now holds this branch's mismatch-code bullet (decided, `unavailable`) and PROPOSED bullet, then `origin/main`'s `HerdrPort` bullet without "`HarnessPort` in its final form", its `opencode_agent` bullet and its #695 bullet. Nothing of either side is lost | `git diff origin/main -- docs/adr/ADR-0021.md` | PASS |
| Evidence appendix | The moved citations match source verbatim: `ports.rs:199-200` (`select_session`), `attach.rs:7-8`, `:74-95`, `:202-204`, `hermetic_test.rs:26-27`, ADR-0021 `:314-319` and `:351-357` | Read each cited range against its excerpt | PASS |
| Invariants, not durations | No sleep added | Read | PASS |
| Security, protocol, frozen files | No golden or wire change, no `#[allow]`, no secret; the merge's own files are `origin/main`'s | `git diff origin/main --stat` | PASS |

## Acceptance criteria status

ACs 1-26: **PASS**, backed by the same tests as rounds 1-5. ACs 24 and 25 now hold against `origin/main` as well as on the
branch's base, which was round 5's one failure.

## Blocking issues

None.

## Advisory notes

- `body_run_test::fresh_hello_and_presence_on_every_reconnect` flaked once under machine load (hub restart on the same
  port exits before `listening`). It is outside this story; worth an issue if it recurs in CI.
- F's "Known issues" stand for O: ADR-0021 `:485` "#645's and #646's, planned" goes stale once 645a lands; the "home
  screen" wording for a `None` that can mean "cannot tell" belongs to the F-4 fold; #644 or #642's last part may still
  conflict with ADR-0021 (R-1).
- Carried over: A-dup's warns, follow-ups F-1 to F-4, and the timed-out `create_session` question for 645b.

T-green complete, no blocking issues. No UI surface — U is N/A, ready for S.
