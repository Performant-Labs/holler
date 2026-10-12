# Decision journal: #647 part 2 (`unattached` and `agent-cannot-dispatch`)

Run record for the part-2 build run. The brief is `docs/handoffs/647-part2-brief.md`. Part 1's record is in `docs/handoffs/647-part1-record/` until the run's last step moves it back.

## A (Phase 3, up-front plan review) — 2026-10-12T09:47-06:00
- **Decided:**
  - PASS, with 0 blocks and 8 warns (`handoff-A.md`).
  - Kept every preparer decision marked "to be confirmed at the plan check": D1, D2, D4-D11, D13-D15, D17 and D18.
  - Two of them are kept with conditions:
    - D2/D15's empty view from Claude Code is answered only for a `Process` endpoint. Both implementations call `at.process()?` first, and `FakeClaude` gets `ClaudeOp::AgentView` (W-4).
    - D8's `remedy_for` is kept, provided `with_agent` comes after `with_fix`, `Finding` gets no new field, and the module doc's remedy rule is amended (W-2). For a pane in a profile, the message names `--profile` (W-3).
  - Corrected the Reuse map: `from_messages` is not a shared reader today. Sharing it, as D4 specifies, needs a split in `prompt.rs` that quotes no body (W-1).
  - Resolved where the new types go: `src/ports/agent_view.rs` (W-5).
  - Named the second read-only assertion: `pane_verbs/doctor.rs:338-342` (W-6).
  - Recommended `reconcile/observe/agent.rs` for the rule (W-8).
  - Asked for a stated bound on the message read (W-7).
- **Assumed:**
  - #839 (F done at `d480424`, not merged) lands its remedy builders as they are on its branch.
  - The OpenCode 1.19.0 facts in the brief hold. The opt-in real test proves them.
  - The doctor rig stays the world of the doctor tests, built from `sample_pane`, which has no agent.
- **Hedged:**
  - W-3: whether the remedy itself should carry `--profile` is a change to part 1's rule, so it is the operator's call.
  - W-7: whether 1.19.0's message route takes a limit was not measured.
  - W-8: the size estimates for `ports.rs` and `observe.rs` are mine, from counting the planned additions, not from built code.
- **Evidence:**
  - Read in full: `ports.rs`, `ports/router.rs`, `ports/process.rs`, `reconcile.rs`, `reconcile/observe.rs`, `reconcile/observe/process.rs`, `findings.rs`, the testkit's `host.rs`, `harness.rs` and `harnesses.rs`, and `holler-adapter-opencode/src/{lib.rs,server.rs,prompt.rs}`.
  - Read in part: `claude.rs`, `holler-adapter-claude/src/lib.rs`, `holler-adapter-host/src/{lib.rs,tmux.rs}`, `holler-cli/src/pane/{relaunch.rs,wiring.rs,wiring/bind.rs}`, `doctor/{rig.rs,read_only.rs,surface.rs}`, `pane_verbs/doctor.rs`, `pane_integration/bind.rs`, and ADR-0021 sections 2, 3, 9, 12 and 15, plus "Deferred" and "Decisions taken".
  - Commands: `grep -rn 'impl[^{]*HostPort for\|impl[^{]*HarnessPort for'` (every implementation); `git diff 6eff9cc issue-839-implementation -- crates/holler-pane/src/{findings.rs,reconcile/observe.rs,ports.rs,lib.rs}` (#839's overlap); `wc -l` on every file in the Blast radius.
