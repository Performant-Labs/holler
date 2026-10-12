# Decisions journal, #647 part 2 (earlier attempt that stopped at the plan check)

Moved from `docs/handoffs/647/decisions.md` so part 1 record (PR #701) stays byte for byte.


## A (Phase 3, plan review), part 2 run `0647-agent-cannot-dispatch` — 2026-10-11T02:29:13-06:00
- **Decided:** BLOCK, with 2 blocks and 9 warns (`handoff-A.md`, which replaces part 1's on this branch; part 1's is at
  `e612878`).
  - B-1: the brief is part 1's plan, merged as #701 at 18:56 MDT on 2026-10-09. It does not mention the issue's open,
    operator-confirmed amendments, `agent-cannot-dispatch` (17:40 MDT) and `unattached` (19:03 MDT).
  - B-2: its Test plan and Decision 10, taken literally, re-declare and stub the merged engine. Four sibling verbs and
    #645's tests import that API, and the change would rename the `--json` values `server-wedged` and `server-down`.
  - The warns are the part-2 brief's checklist:
    - no frozen port observes either finding, and I6 forbids files, so each needs a default-armed read under a recorded
      operator carve-out (the #645 precedent);
    - order the change against #642's in-flight `HarnessPort::serve` amendment;
    - the issue's `unattached` cures (`launch`, and the operator's own tmux-attach tool) do not fit the remedy table,
      and `AgentKey` allows a leading `-` in a remedy;
    - wrapper doubles rather than test-kit edits;
    - the I5 bound and the read-only allowlist change;
    - route the `SpecField` row;
    - give the run its own handoff slug;
    - re-run the brief gate.
- **Assumed:**
  - The worktree name `0647-agent-cannot-dispatch` states the run's intended scope; the run arguments did not otherwise
    say it.
  - Issue #647's live text and its GraphQL edit history are authoritative for what is open and when it landed.
- **Hedged:**
  - B-2 is a block, not a warn, because the automated T phase follows the Test plan literally, and `findings.rs`,
    `reconcile.rs` and `pane/doctor.rs` would be overwritten on a branch whose agent merges its own PR.
  - Whether `unattached` belongs in this run is the operator's decision (both amendments are operator-confirmed). A only
    requires that the brief states it.
- **Evidence:**
  - Read: the brief in full; the outside brief result r1; part 1's `handoff-S.md` and this journal.
  - Live issue data: `gh issue view 647` and its `userContentEdits` diffs; epic #633's decision 8 and hot-spot rules;
    #642's issue and branch (`ac13a0e`, a brief only); PR #701's body and merge time.
  - Code on `dcfc01e`:
    - `findings.rs`, `reconcile.rs` and `reconcile/observe.rs` (structure and the `pub` items);
    - `ports.rs:86-281`, `argv.rs` (`AgentKey`), `tx_launch.rs:47-49, 600-661` and `profile_diff.rs:1-120`;
    - ADR-0021 lines 48, 100-140, 154, 208, 401-412, 430 and 730-745;
    - #645's brief and its PR #723 file list.
  - Greps: the outside users of `reconcile::`/`findings::`; `impl HarnessPort/HostPort for`; the operator tool's name (none); `transcript`,
    `agent` and `attached` in the test kit and ports (none); the brief's mentions of the open scope (0).
