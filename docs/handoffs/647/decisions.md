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

## T (Phase 4, author / RED) — 2026-10-12T10:25-06:00
- **Decided:**
  - RED is valid; verdict PASS (`handoff-T-red.md`).
  - T made the contract commit `45bff85` itself (ADR-0021 plus the `ports.rs` doc bullet, documentation only, verified to build and pass) before any test. The driver commits a phase with one `git add -A`, which would fold the contract into the RED commit and break acceptance 20.
  - T also made the pure move `3a09f7d` (the two harness doubles of `switch.rs` into `switch/doubles.rs`) as its own commit. The brief gave this move to F, but it moves test files, which are T's (F writes no test), and it had to precede the forwards T adds.
  - The API names the tests use where the brief left them open:
    - `AgentView::with_turns` and `with_dispatchers`, the latter sorting and dropping duplicates so the rule lives in one place;
    - `TurnAgent::new`;
    - `FakeHost::attach_client` and `detach_clients` returning `Result`, with `pane-not-found` for a missing session;
    - `FakeHarness::seed_turns(u16, &str, Vec<TurnAgent>)` and `seed_dispatchers(u16, Vec<AgentKey>)`;
    - `ClaudeOp::AgentView`.
  - Acceptance 9's "a message without an agent is skipped" conflicts with ADR-0021 section 12 ("`None` when it recorded none"). T pinned the ADR: such a message is a turn with `agent: None` and `can_dispatch: None`. It stays one of "the last `max_turns` user turns", and it stops the rule, which is the conservative reading.
  - The `compile_fail,E0639` doc tests go on `HarnessPort`'s doc in `ports.rs`, because `agent_view.rs` is F's to create.
  - Acceptance 13's negatives are each their own test, sharing one helper per group.
  - The two host-read assertions share `rig::HOST_READS` and `rig::HARNESS_READS` (A's W-6).
- **Assumed:**
  - The fixtures cut from a scratch OpenCode 1.19.0 at 10:00 MDT are what the fleet's 1.19.0 serves. The opt-in `ac647_*` real test re-proves this on a live server.
  - A profile member's message names `--profile` and the profile (A's W-3). The test asserts both appear, not the exact wording.
  - `unattached`'s message holds D11's two substrings verbatim.
- **Hedged:**
  - The two `E0639` doc tests are not RED: stable rustdoc does not check the code, and the type is missing on `main`. They are guards for `#[non_exhaustive]` after F.
  - The 12 negatives of `doctor/agent.rs`, and the similar no-finding tests, cannot fail on an assertion before the rule exists. They are RED by compile only. They guard against a rule that fires too eagerly.
  - The diff gate's evidence appendix is Phase 6's work.
- **Evidence:**
  - `cargo build --workspace --tests --keep-going` on the real tree: exit 101, only missing-API errors (`E0599`, `E0432`, `E0407`).
  - `cargo test -p holler-pane --doc`: the two new `E0046` doc tests FAILED with "Test compiled successfully, but it's marked `compile_fail`".
  - Against a throwaway API shim, restored byte for byte afterwards (`grep -rn SHIM crates/` is empty):
    - `cargo clippy --workspace --all-targets -- -D warnings` was clean;
    - every new test failed on its assertion, as listed in `handoff-T-red.md`;
    - `pane_claude` (43) and `profile_verbs` (118) passed.
  - `bash scripts/lint.sh`: exit 0.
  - The privacy grep of the added lines: clean.
