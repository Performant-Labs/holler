# Handoff-F: Phase 6 (Workflow script; the role doc's Phase 5), round 2 - #802 the health wait checks the run's deadline before every ask

**Date:** 2026-10-11
**Branch:** issue-802-implementation (worktree `.claude/worktrees/0802-serve-command`, on `563288a`)
**Issue:** #802 (closes #695)

Round 1 of this handoff (the whole #802 implementation, GREEN at `7ede9c7`) is at
`git show 7ede9c7:docs/handoffs/802/handoff-F.md`. It still describes every other file of the PR.
This round changes only what T-green's B-1 and A's round 2 asked for. No test was edited, and
nothing is committed, because the Workflow script commits this phase.

| Field | Value |
|---|---|
| GitHub issue | #802 |
| Working branch | `issue-802-implementation` |
| Build plan phase | Workflow Phase 6 (F), the round after T-green BLOCK -> A PASS (round 2) -> T-red PASS (round 2) |
| Input documents read | `802-brief.md` (as amended in `0dc4e8b`), `handoff-T-green.md` (B-1), `handoff-A.md` (round 2: W-4 restated and binding, W-14 to W-16), `handoff-T-red.md` (round 2), `decisions.md`, `evidence.md`, round 1's `handoff-F.md`, ADR-0021 section 2 and section 12 |
| Acceptance criteria count | 12. This round touches AC 6 (amended Scope 2) and AC 12 (the CHANGELOG clause) |
| Handoff document path | `docs/handoffs/802/handoff-F.md` |

## What was done

- `crates/holler-pane/src/tx_launch/server.rs`
  - **W-4 / B-1.** `wait_healthy` checks the deadline at the top of its loop, before every
    `health` call, the first one included. Once the deadline has passed it answers `Ok(false)`
    without asking. After a `false`, it sleeps for the smaller of `poll` and the time left,
    measured after the ask.
  - **No second check.** `start` has no `budget.check()` between `run` and the wait. A comment
    says why: the rule lives in the one wait.
  - **Docs.** Updated the docs of `wait_healthy` and step 3 of the module doc.
- `crates/holler-pane/src/tx_launch.rs`
  - **W-14.** `Budget` now holds its end, computed once in `Budget::start`. `check` compares the
    monotonic clock against that end, and `deadline()` returns it. So `check` and the wait agree
    by construction: an ask happens exactly when `check` would pass.
  - **A budget the clock cannot hold.** A limit too large to add to an `Instant` (hundreds of
    billions of years) is halved until it fits. It still never runs out in practice, for `check`
    and the wait alike. The fields `started` and `limit` are gone.
  - **Docs.** Updated the docs of `Budget`, `start`, `check` and `deadline`, and the module doc's
    "Bound (I5)" bullet.
- `crates/holler-adapter-opencode/tests/real_opencode/rig.rs` (**W-16**, a test-support file;
  see Design decisions)
  - It imports `holler_pane::tx_launch::BOOT_POLL` and drops its own copy of 150 ms. The rig's
    own `BOOT` (30 s) stays, as A asked.
- `docs/adr/ADR-0021.md`, section 2, the paragraph "The health wait moved into the engine"
  - The wait has no deadline, budget or poll count of its own.
  - It checks the budget's end before every ask, the first one after the run included, so section
    12's bound holds for the wait too.
  - A try that hangs costs at most `Timeouts.health`. That cost is the "one port call's own bound"
    of section 12, not something inside the budget, so the old wording "inside the budget" was
    corrected.
- `CHANGELOG.md` (T-green's AC 12 advisory, which A passed to F)
  - The #802 entry now says that never adopting a running server rests on the plan's
    `port-in-use` check and the re-check before the server starts.
  - It also says the adapter's own check, that a healthy answer came from the server it started,
    is gone.

## Design decisions

- **The deadline check sits at the top of the loop, as A's W-4 asks.** I considered a check
  between `run` and the wait in `server::start`. I rejected it because A's W-4 and Phase 8 rule
  out a second `budget.check()`, and because the wait's two other callers (the conformance suite
  and the real rig) would not get the rule. T's two direct tests pin the rule in the shared
  function.
- **The sleep is measured after the ask:** `min(poll, deadline - now)`.
  - The time left is read again after `health` returns. Reusing the value read before the ask
    would oversleep the deadline by up to one `health` call's duration.
  - When `health` itself ran past the deadline, the sleep is zero and the next top check ends the
    wait.
  - So a call ends at most one `health` bound past the deadline, which is the bound the doc
    states.
- **The sleep test is deterministic.** It relies on `std::thread::sleep` never sleeping less than
  it is asked to (std docs: "It will never sleep less"). The wait's sleep is capped at the time
  left, so when it wakes the deadline has passed and the top check ends the wait without another
  ask. That is how T's 60 s poll / 200 ms window test is exact.
- **W-14: compute the end once, A's first option.** A's second option was to have both sides use
  the time remaining. I rejected it because `wait_healthy` takes a deadline `Instant`, a
  signature T's tests pin, so the engine would still have to turn the time remaining into an
  `Instant`, with the same overflow question.
  - With one `end`, `check` (`now < end`) and the wait (asks while `now < deadline`) are the same
    predicate.
  - For a limit the clock cannot hold, halving keeps `check`'s meaning before this change ("never
    runs out" in practice). The alternative, falling back to `started`, would turn such a budget
    into an immediate `timeout`. Halving ends: a zero limit always fits. It runs only for an
    absurd limit; for any real budget the first `checked_add` succeeds.
- **W-16 is the one edit in a test path, and it is deliberate.**
  - The role doc says F writes and edits no tests. This edit changes no test, no assertion and no
    value: the constant was a copy of `tx_launch::BOOT_POLL` (150 ms), and the rig now imports it.
  - A assigned it to F ("W-14 and W-16 are small, and F can do them in the same pass"). T-red, who
    owns the test files, also handed it to F ("W-14, W-16 and the CHANGELOG clause go in the same
    pass, per A").
  - A lists it among Phase 8's first checks, so leaving it would only send the run back here.
  - T may revert it or redo it at Phase 7 if T wants the edit under its own name.

## Reuse / extend-vs-new

Extended in place; no new object.
- The one shared `wait_healthy` gained the check (W-4). There is no second loop and no extra
  `budget.check()`.
- `Budget` keeps its role and API (`start`, `check`, `expired`, `deadline`). Only its inside
  changed (W-14).
- The rig now imports the engine's `BOOT_POLL` instead of copying it (W-16).

## Architecture notes for A

None that are structural.
- No module boundary, public signature or dependency changed.
- `wait_healthy`'s documented contract is narrower: it no longer "asks at least once"; a deadline
  already past asks nothing. Its three callers are the engine, the conformance suite and the real
  rig. All of them pass a deadline in the future at the start, so only the edge cases change.
- `Budget` is private to `tx_launch`.
- `tx_launch.rs` is 820 lines (it was 807; the lint fails at 900, and the guideline says to
  decompose before about 850). `server.rs` is 106 lines.

## Deviations from spec / wireframe

None. Amended Scope 2 ("each `health` call is a live step with the budget checked before it";
"the ADR's last sentence must keep holding for the wait too") now holds. A's four W-4 docs are
updated, plus the module doc's I5 bullet in `tx_launch.rs`.

## Tier 1 self-check (incl. tests now GREEN)

```text
# RED before the change (T-red round 2's three tests):
$ cargo test -p holler-cli --test pane_verbs -- launch::serve
test result: FAILED. 12 passed; 3 failed      # the three assertions handoff-T-red.md shows

# After the change:
$ cargo test -p holler-cli --test pane_verbs -- launch::serve
test result: ok. 15 passed; 0 failed
# the three new tests, 5 more runs:  ok. 3 passed; 0 failed  (x5, 1.20 s each)

$ cargo build --workspace --tests                          -> Finished, exit 0
$ cargo clippy --workspace --all-targets -- -D warnings    -> exit 0
  (and, after touching the crate roots so they are re-checked:
   cargo clippy -p holler-pane -p holler-adapter-opencode --all-targets -- -D warnings -> exit 0)
$ bash scripts/lint.sh            -> exit 0 (size warnings only; tx_launch.rs 820)
$ bash scripts/changelog-check.sh -> changelog-check: ok   (also after the last wording edit)
$ cargo machete                   -> no unused dependencies
$ rustfmt --edition 2021 --check crates/holler-pane/src/lib.rs crates/holler-pane/src/tx_launch.rs \
    crates/holler-adapter-opencode/tests/real_opencode_test.rs   -> clean
$ RUSTDOCFLAGS="-D warnings" cargo doc --no-deps -p holler-pane -p holler-pane-testkit \
    -p holler-adapter-opencode    -> clean
$ cargo test -p holler-cli --test docs_cli_test   -> 3 passed (after the last ADR/CHANGELOG edit)

# The pipeline's verify command (whole workspace):
$ cargo test --workspace --no-fail-fast -- --skip roster_stays_accurate_under_concurrent_body_load
run 1: 148 binaries, passed=2072 failed=1 ignored=25
       the one failure: remote_admin_test::remote_admin_traffic_never_moves_the_bodys_last_seen
       (hub/body traffic; #802 touches nothing on that path). Re-run alone 3 times: 21/21 each.
run 2: 148 binaries, passed=2073 failed=0 ignored=25, exit 0     # 2070 + T-green's 1 + T-red's 2

# Opt-in, against a real binary (a local opencode 1.19.0; private scratch dir, tmux server and ports):
$ HOLLER_TEST_OPENCODE=1 cargo test -p holler-adapter-opencode --test real_opencode_test \
    -- --ignored --test-threads=1
test result: ok. 9 passed   (ac12 runs the conformance suite through the rig's launcher, which
                             waits with the changed wait_healthy and the imported BOOT_POLL)
# Afterwards no `opencode serve --port 481xx` process was left.
```

## Evidence appendix

`docs/handoffs/802/evidence.md`.
- Round 2 moved the `tx_launch.rs` line numbers of two entries (`roll_back` is now 721-727, and
  the `acted` mapping is 573-576).
- Round 2 added two entries that T's round-2 tests rely on:
  - `FakeHarness::health` logs every call and answers `true` only for a running server
    (`harness.rs:346-349`);
  - `FaultSwitch::set_delay` (`fault.rs:79-83`).
- Every excerpt was re-checked against its cited lines.

## Tests that look wrong (for T)

None. One note: W-16 edited `real_opencode/rig.rs`, T's file (see Design decisions). It is an
import in place of a copied constant, and no test logic changed.

## Known issues

- **A flake outside #802.** `remote_admin_test::remote_admin_traffic_never_moves_the_bodys_last_seen`
  failed once in the parallel workspace run: `last_seen` moved by one second. It passed 3/3 alone
  and in the next full run. It is hub and body code that #802 does not touch.
- **A fact outside the repo.** The sleep test's determinism rests on `std::thread::sleep` never
  sleeping less than asked. That guarantee is in the std docs, not in this repository, and
  `rust-src` is not installed here. So it is listed here, not in `evidence.md`.
- **Carried from round 1, unchanged.**
  - W-11(a): no test starts the server under a tmux pty. It goes with #649.
  - W-13: the merge order with #664.
  - W-15 (A, for O): the plan's reads and `session_of_record`'s two calls have no budget check
    between them. This predates #802, and #802 is not held to it.

## Files changed

This round:
- `crates/holler-pane/src/tx_launch/server.rs` (production)
- `crates/holler-pane/src/tx_launch.rs` (production)
- `crates/holler-adapter-opencode/tests/real_opencode/rig.rs` (test support: W-16, the import only)
- `docs/adr/ADR-0021.md`
- `CHANGELOG.md`

The PR's production files, both rounds (round 1's list, unchanged):
- `crates/holler-pane/src/{ports.rs,lib.rs,tx_launch.rs,tx_launch/server.rs}`
- `crates/holler-pane-testkit/src/{harness.rs,host.rs,conformance/harness.rs,conformance/host.rs}`
- `crates/holler-adapter-opencode/src/{lib.rs,server.rs,exec.rs,tui.rs,attach.rs}`
- `crates/holler-adapter-host/src/{lib.rs,tmux.rs}`
- `crates/holler-cli/src/pane/wiring.rs`
- `docs/adr/ADR-0021.md`
- `CHANGELOG.md`

Handoff: `docs/handoffs/802/handoff-F.md` (this file), `docs/handoffs/802/evidence.md` (line
numbers and two entries), `docs/handoffs/802/decisions.md` (appended).
