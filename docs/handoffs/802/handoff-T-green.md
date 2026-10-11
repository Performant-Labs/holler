# Handoff-T-green: Phase 7 (Workflow script; the role doc's Phase 6), re-run after the brief's round-1 amendment - #802 `HarnessPort::serve_command` takes a `ServeSpec`; the harness server runs in the pane's tmux session

**Date:** 2026-10-11
**Branch:** issue-802-implementation (worktree `.claude/worktrees/0802-serve-command`, head `0dc4e8b`)
**Issue:** #802 (closes #695)
**Handoff-F reviewed:** `docs/handoffs/802/handoff-F.md`
**Handoff-T-red:** `docs/handoffs/802/handoff-T-red.md`
**Brief:** `docs/handoffs/802-brief.md` as amended in `0dc4e8b` (13 findings applied)

**VERDICT: BLOCK.** F must change one function. The production code is unchanged since the first
T-green (`7ede9c7`, PASS). The brief was amended afterwards, and its new Scope 2 text rules out
what `tx_launch::server::wait_healthy` does. I wrote a test that pins the amended rule. It fails on
F's code for the right reason, and it passes with a three-line fix that I tried and then reverted.

## What the amended brief adds, and how the code measures up

| Amended text | Check | Result |
|---|---|---|
| Scope 2: "each `health` call is a live step with the budget checked before it ... and the ADR's last sentence must keep holding for the wait too" | `server.rs` read; new test | **FAIL** (below) |
| AC 1: two `compile_fail,E0639` doctests, one per form | `ports.rs` ~113-118: two doctests; `cargo test -p holler-pane --doc` | PASS |
| AC 2: complete call logs via `assert_eq!`, `acted == false` | `an_agent_refusal_in_the_plan_is_exit_3_and_writes_nothing` asserts `calls.harness == [ServeCommand]` and empty `herdr`, `host` and `probes` lists, exit 3, no step phrase. The step phrase is printed only when `acted` is true, so its absence covers `acted == false`. | PASS |
| AC 4: escape `p#S`->`p##S`, `q;`->`q\;`; the real-tmux test uses another directory | `fake_tmux_test::run_escapes_the_cwd_for_tmux` (both cases plus `#(...)`); `real_tmux_test::run_works_in_the_given_cwd_not_the_sessions` (opt-in, 9/9 on F's run) | PASS |
| AC 6: the run's budget, a short `TxOptions.budget`, a frozen fake, a bounded run time | `launch.rs::assert_waits_out_the_budget` (700 ms budget, `took < 3 s`) used by both 4a and 4b | PASS |
| AC 8: "loudly" = exit 1, `unavailable`, "still answers", generation unchanged | `relaunch_fails_when_the_old_server_survives` asserts all four | PASS |
| AC 12: CHANGELOG names the behaviour changes | names `timeout` not `unavailable`, and the port re-check | **Partial** (advisory below) |

## Blocking issue (F must address)

**B-1. The health wait asks `health` without the run's budget being checked first.**
`crates/holler-pane/src/tx_launch/server.rs`, `start` (lines ~93-97) and `wait_healthy` (~37-53):

- `start` checks the budget, calls `host.run`, then enters `wait_healthy`. The wait's first act is
  `harness.health(port)`, with no budget check after `run`. A `run` that returns after the budget
  has run out is followed by a `health` call anyway. The run then takes the budget plus `run`'s
  bound plus `health`'s bound (OpenCode's 2 s). That breaks ADR-0021 section 12's sentence, "A run
  takes at most the budget plus one port call's own bound plus its rollback calls", which the
  amended Scope 2 says "must keep holding for the wait too". Before #802 the engine had a
  `budget.check()` between `serve` and `health` (`92fbc6e` `tx_launch.rs` ~628-630), so this
  removed a check.
- `wait_healthy` also asks "once more when the deadline comes during a sleep". That is a `health`
  call started once the budget has run out. Scope 2 rules it out: "each `health` call is a live step
  with the budget checked before it".

**Fix that turns the new test GREEN** (tried, then reverted with a file copy; F owns the final form):
check the deadline before each ask in `wait_healthy`:

```rust
loop {
    if Instant::now() >= deadline {
        return Ok(false);
    }
    if harness.health(port)? {
```

With that change applied: `pane_verbs` 259/259 passed (the new test included), and every
`holler-pane-testkit` target passed (conformance 15/15, `fake_harness_test` 23/23). F should
also update `wait_healthy`'s doc ("It asks at least once, and once more when the deadline
comes..."), `Budget::deadline`'s doc ("ends the wait after its first poll"; with the check, an
unrepresentable deadline asks nothing), and the ADR's health-wait paragraph if its wording
changes.

## Test added in this phase

`crates/holler-cli/tests/pane_verbs/launch/serve.rs`,
`no_health_poll_starts_once_the_budget_has_run_out` (integration, the engine through the rig's
fakes, the same tier as `the_budget_bounds_a_slow_launch`).

- **Setup:** the budget is 600 ms. A hook on the plan's port check sets every host call 400 ms
  slow, so `ensure_session` ends inside the budget and `run` ends past it. There is no sleep in test
  code; the slowdown is the fault switch's `set_delay`.
- **Assertions:** `Err(TxFailure { Timeout("pane.launch"), acted: true })`. Host calls are
  `[EnsureSession, Run, StopOwned]`. Harness calls are exactly `[ServeCommand, Health, Health]`
  (the plan's port check and the act's re-check). No record is written.
- **Margins:** about 200 ms on each side of the budget, and the outcome is an ordering, not a
  duration.
- **RED on F's code** (the assertion about the missing behaviour, not a setup error):

```text
---- launch::serve::no_health_poll_starts_once_the_budget_has_run_out stdout ----
assertion `left == right` failed: no health call after the run: the budget had run out
  left: [ServeCommand, Health, Health, Health]
 right: [ServeCommand, Health, Health]
test result: FAILED. 12 passed; 1 failed (launch::serve)
```

The prior phase's test repair (`hermetic_test`, `stub.lines().is_empty()`) is committed in `3fe111d`.

## GREEN confirmation (everything except the new test)

```text
$ cargo test --workspace --no-fail-fast -- --skip roster_stays_accurate_under_concurrent_body_load
148 binaries: passed=2070 failed=1 ignored=25
  the one failure: launch::serve::no_health_poll_starts_once_the_budget_has_run_out (B-1)
  2070 is F's count, so every pre-existing and Phase-5 test is still GREEN
```

| Check | Command | Result |
|---|---|---|
| Clippy | `cargo clippy --workspace --all-targets -- -D warnings` | exit 0, PASS |
| Lint | `bash scripts/lint.sh` | exit 0 (size notices only), PASS |
| Changelog | `bash scripts/changelog-check.sh` | `ok`, PASS |
| Docs CLI | `cargo test -p holler-cli --test docs_cli_test` | 3 passed, PASS |
| Wire canary | `cargo test -p holler-cli --test wire_selftest` | 3 passed, PASS |
| Unused deps | `cargo machete` | none, PASS |
| Doctests | `cargo test -p holler-pane --doc` | 5 passed (the 2 `compile_fail,E0639`), PASS |
| Hook tests | `bash scripts/test-hooks.sh` | exit 0, PASS |
| rustfmt | `rustfmt --edition 2021 --check` on the new test file | clean (after formatting), PASS |
| Workspace tests | as above | **1 FAIL (B-1, the new test)** |

F's opt-in real runs (`real_tmux_test`, `real_opencode_test`, 9/9 each) were not re-run. They
need private servers and are not CI steps. F's results stand as reported.

## Tier 2 results

| Check | Method | Result |
|---|---|---|
| Coverage per AC | AC 1-12, each mapped to a test (first T-green table, still valid), plus the new I5 test for amended Scope 2 | PASS once B-1 is fixed |
| Test quality | The new test names one behaviour, fails alone for the right reason, sits at the engine tier and duplicates neither `the_budget_bounds_a_slow_launch` (slow harness, stops before the server step) nor 4a/4b (frozen server, polls until the budget ends) | PASS |
| Timing | The new test asserts the call order and the outcome, not a duration. It uses `set_delay`, with no `thread::sleep` | PASS |
| I5 budget | `server::start` / `wait_healthy` as above | **FAIL (B-1)** |
| Type safety, error paths, data integrity, API contract, security, migration | Unchanged since the first T-green, whose mutation checks M1 to M6 still apply (the production code is the same) | PASS |
| Protocol, goldens, Playwright | No such change or surface | N/A |
| Evidence appendix | The new test relies on `roll_back` calling only `stop_owned` and `herdr.close` (already in `evidence.md`), on the fault switch logging every call (already there) and on `set_delay` (`holler-pane-testkit/src/fault.rs` ~79-83, a test-kit fact shown in the diff's context). Nothing added. | PASS |

## Acceptance criteria status

AC 1-5 and 7-11: PASS (as in the first T-green; the amended wording is checked in the table at the
top). AC 6: **FAIL** until B-1 is fixed. The wait's `timeout` and rollback pass, but the amended
Scope 2 rule about the budget, which AC 6 now cites, does not hold. AC 12: PASS for the mechanical
part, with one advisory below.

## Advisory notes

- **AC 12 / Scope 7 (for F with B-1, or for S).** The CHANGELOG entry says the engine "checks the
  port again just before the server starts". It does not say what the amended Scope 7 asks it to
  name: that never-adopt now rests on those two checks *without the adapter's check that a healthy
  answer came from the server it started*. The ADR states this (ADR-0021 ~163-165); the CHANGELOG
  does not. One clause would fix it.
- W-11(a), carried over: no test starts the server under a tmux pty. That check belongs with #649.
- `Isolated` pairs now appear on the server's command line (by design, A's W-2). Credentials must
  stay out of `Isolated`.
- `serve.rs` is about 500 lines after the new test. `fake_tmux_test.rs` (803), `hermetic_test.rs`
  (813), `switch.rs` (833) and `launch/rig.rs` (793) are close to the 900-line limit.
