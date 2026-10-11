# Handoff-A: Phase 3 - #802 `HarnessPort::serve_command` and `ServeSpec`; the server runs in the pane's tmux session  (up-front plan review, round 2)

**Date:** 2026-10-11
**Branch:** issue-802-implementation (worktree `.claude/worktrees/0802-serve-command`, head `dab3cd3`)
**Brief reviewed:** `docs/handoffs/802-brief.md` as amended in `0dc4e8b` (sha256 `16db698d23e2428c...`)   **Reuse map:** no separate file. This automated run has no `survey.md`, so the brief's Scope and round 1's findings serve as the map.   **Wireframe:** N/A (no UI surface)
**Verdict:** PASS

Round 1 of this review (PASS on the unamended brief, sha256 `e895a4d9...`) is at `git show 0421ec1:docs/handoffs/802/handoff-A.md`. This file replaces it and carries its findings forward with their status.

## Summary

**Why this round.** The run resumed at T-green with `resumeArchChanged`. T-green blocked (its B-1) on a rule the amended brief adds to Scope 2, so the driver came back to A before T and F run again.

**Verdict: PASS, with no blocks.** The amendment clarifies and adds. It changes no object, layer or contract that round 1 approved. Its new I5 rule for the health wait matches the pattern the engine's act already follows: a `budget.check()` before every live step.

**What changes for T and F.** One finding of round 1 now conflicts with the amended text:

- Round 1's W-4 described the one shared wait as answering "`Ok(false)` at the deadline". F built it in `probe.rs`'s `wait_until` shape: ask first, then compare.
- **W-4 is restated here, and it is binding.** The one wait checks the run's deadline before every `health` call, including the first call after `run`.
- This is the change T-green's B-1 asks of F. It keeps one wait function shared by the engine, the conformance suite and the real rig.

**New warns.**

- W-14 (low): `Budget::check` and `Budget::deadline` must agree on when the budget ends.
- W-15 (follow-up): the plan's reads have no budget checks. The gap predates #802, and #802 is not held to it.
- W-16 (low): the real rig keeps its own copy of `BOOT_POLL`.

Round 1's other warns stand, and F has implemented them. W-1 and W-2 stay binding. W-7 is absorbed by the amended AC 4.

## Findings

| # | Severity | Plan element | Drift dimension | Finding | Suggested fix |
|---|---|---|---|---|---|
| W-4 | warn (restated; binding on T and F) | Scope 2 as amended: "each `health` call is a live step with the budget checked before it"; "the wait has no deadline, budget or poll count of its own"; ADR-0021 section 12's last sentence "must keep holding for the wait too". AC 6. | cross-cutting (I5: one rule, in one place); duplication | **The shape.** `wait_healthy` (`tx_launch/server.rs:37-53`) asks first and compares after. It follows `probe.rs`'s `wait_until` (`probe.rs:297-308`). That shape suits `Child::try_wait`, a non-blocking call with no bound of its own. It does not suit `health`, a live port call whose own bound is OpenCode's `timeouts.health` (2 s). **First ask.** `start` calls `run` and then the wait, with no check between them (`server.rs:93-96`). Before #802, `bring_up` checked the budget between `serve` and `health` (`92fbc6e` `tx_launch.rs:627-630`). A `run` that returns past the budget is still followed by a `health` call. The run then takes the budget plus two port calls' bounds, which breaks section 12's bound. **Last ask.** The wait also asks once more when the deadline arrives during a sleep. | **The function.** Keep one `pub fn wait_healthy` for the engine, the conformance suite and the real rig. At the top of its loop, before every `health` call including the first, it checks `Instant::now() >= deadline`. Once the deadline has passed, it answers `Ok(false)` without asking. **The engine.** It passes the run budget's end (`budget.deadline()`) and maps `false` to `budget.expired()`, so the wait has no deadline of its own. `start` then needs no separate `budget.check()` between `run` and the wait: the rule lives in one place. **The docs.** Update `wait_healthy`'s doc (it now says "asks at least once, and once more ..."), the doc of `Budget::deadline`, step 3 of the module doc, and ADR-0021's paragraph "The health wait moved into the engine" (section 2; each ask happens only while the budget has not run out). **Phase 8.** A second loop with its own `budget.check()` in `server::start`, or anywhere else, would be a near-copy of the shared wait, and Phase 8 rejects it. |
| W-14 | warn (low) | `Budget::deadline` (`tx_launch.rs:255-259`), which W-4 now relies on | one source of truth (I5) | **Two rules.** The end of the budget is stated twice. `Budget::check` (`:242-247`) tests `started.elapsed() < limit`. The wait uses `started.checked_add(limit)`, which falls back to `started` on overflow. **Where they differ.** They agree for every budget the clock can hold. For one it cannot hold, `check` never expires, but the wait answers at once; with W-4's fix it never asks. The run then answers `timeout` with budget left. **Reach.** No caller passes such a budget: the CLI passes `TxOptions::default()` (`holler-cli/src/pane/launch.rs:78`, `relaunch.rs:55`). | Make the two agree. F chooses how. Two ways: compute the end once, in `Budget::start`, and have `check` compare against it; or have `check` and the wait both use the time remaining (`limit.saturating_sub(started.elapsed())`). At the least, the docs say what a budget the clock cannot hold does. |
| W-15 | warn (follow-up for O; also a reading for S) | Scope 2 as amended. The words "checked before every live step" in ADR-0021 section 12 and in `tx_launch.rs`'s module doc (`:30-34`). | I5 (predates #802; outside its diff) | **The plan's reads.** `Plan::observe` runs four live calls with no budget check between them: the probe (`PROBE_TIMEOUT`, 5 s), `herdr.version`, `herdr.snapshot` and the `port-in-use` `health` (`tx_launch.rs:327-346`; the same at `92fbc6e:289-309`). **`session_of_record`.** It runs `list_sessions` and then `create_session` with no check between them (`:663-671`). **#802.** It adds no live step there: `serve_command` is pure. T-green made the same observation (`decisions.md`, its second entry). | **For S.** The amended sentence binds the health wait's calls, and the act's re-check, which already has its check. It does not bind the plan's reads, whose gap predates #802, so S should not hold #802 to it. **For O.** File a follow-up. Either bring the plan's reads and `session_of_record`'s second call under the budget, or narrow the ADR's wording to "every live step of the act". |
| W-16 | warn (low; a Phase 8 candidate) | Scope 5: the real rig's launcher waits "the same way the engine waits" | duplication | `real_opencode/rig.rs:195` declares its own `BOOT_POLL` (150 ms), commented "the engine's default `boot_poll`". It copies `tx_launch::BOOT_POLL` (`tx_launch.rs:70`). The same file already imports `holler_pane::tx_launch::wait_healthy` (`:39`), and the conformance suite imports the engine's constant (`conformance/harness.rs:40`). | Import `holler_pane::tx_launch::BOOT_POLL`. The rig's own `BOOT` (30 s, for a cold real server) is a documented difference and stays. |
| W-1 | warn (binding on F and T; stands) | Decision 6; Scope 1 (`HostPort::run` gains `cwd`); AC 4 | cross-cutting (the host adapter's narrowing 5) | **The brief's layer.** The amended Scope 1 now names the layer itself: `TmuxHost::run` raises the `usage` for an empty or relative `cwd`. **W-1's addition.** W-1 adds a missing directory, refused in that same layer, so no program starts silently in `$HOME`. **Status.** Implemented: `check_cwd` and `is_usable_dir` (`holler-adapter-host/src/lib.rs:466-486`), shared with `ensure_session`. | Keep it through any rework. |
| W-2 | warn (binding reading of AC 5; stands) | AC 5's last sentence; Scope 1, "No credential goes into an argv" | pattern consistency (the `ProcessEnv` contract) | AC 5 is unchanged by the amendment. **What the argv holds.** The adapter's own literals and the configured `Isolated` pairs. Nothing from the adapter's own process environment, and no allowlist filter. **Status.** Implemented with `ProcessEnv::prefix`. | Keep it. If S reads AC 5 as an allowlist, the question goes to the operator, not to F. |
| W-3 | warn (stands) | Decision 5; Scope 3 | cross-cutting (fail closed) | **OpenCode.** Its `verify_serving` refuses `agent-unsupported` for a spec with an agent. **The fake.** It answers from the agent its own server was started with. **The trait doc.** It says what `Ok` and a refusal mean (`ports.rs:245-256`). **Status.** Implemented. | Keep it. |
| W-5 | warn (stands) | Scope 2; decision 7 | pattern consistency | **One spec.** Each run builds one `ServeSpec`, in `Plan::new`, as an exhaustive literal (`tx_launch.rs:307-311`). **The argv.** It is kept in `Seen`. **The re-check.** The `port-in-use` re-check sits right before the server's `run`. **Status.** Implemented. | Keep it. |
| W-6 | warn (stands) | Scope 2's sibling module | size and structure | **Status.** The server step lives in the child module `tx_launch/server.rs`, after the `reconcile/observe.rs` pattern. `tx_launch.rs` is 807 lines; the lint fails at 900. | W-4's fix stays inside `server.rs`. |
| W-7 | absorbed | AC 4 | consistency with E0 | The amended AC 4 now states round 1's reading. E0 refuses a change of directory. "A session made for another directory" is a tmux session left behind by an earlier pane of the same name. | None. |
| W-8 | warn (stands; Phase 8 candidates) | Scope 4 | duplication; fidelity of the fakes | **What is in place.** `runs()` carries the `cwd`. `LinkedHost::run` returns `FakeHost::run`'s answer and never fails because of the scenario method (`launch/rig.rs:653-666`). Order across ports is proven with hooks, not with a merged log. Both fakes' docs are reworded. | Keep them. |
| W-9 | warn (stands) | Decision 6; the host adapter's docs | docs as contract | **Status.** Implemented. The existence read reuses `tmux::panes`, and `session_dir`, `SESSION_PATH` and `is_session_dir` are deleted. | None. |
| W-10 | warn (stands) | Scope 7 (ADR-0021) | ADRs | **Status.** Implemented: (a) to (f) of round 1. | W-4 adds one wording change to the health-wait paragraph. |
| W-11 | warn (follow-up for O; stands) | Decision 4; Scope 5 | verifying a claim about another component | **(a)** No test starts the server under a tmux pty. It belongs with #649. **(b)** The TUI-window precondition is documented in fact 1. | O files (a) with #649. |
| W-12 | warn (low; stands) | Scope 1 | naming; placement | **Status.** Implemented: `with_agent` is a builder on `self`, and the env prefix sits beside `ProcessEnv`. | None. |
| W-13 | warn (low; stands) | Blast radius | parallel-agent coexistence | #664 has not merged. Whichever of #664 and #802 merges second adapts. | Record it in handoff-F if it happens. |

## Notes for O

PASS: no amendment is needed.

- **For T (Phase 5) and F (Phase 6).**
  - W-4 is the one production change this round asks for.
  - T-green's test `launch::serve::no_health_poll_starts_once_the_budget_has_run_out` pins it, and is RED on the current code for the right reason.
  - F makes the fix inside the shared `wait_healthy`, not in a second loop, and updates the four docs W-4 names.
  - W-14 and W-16 are small, and F can do them in the same pass.
  - T-green's CHANGELOG advisory (AC 12) also goes to F in this pass: one clause saying that never-adopt no longer has the adapter's own-process check.
- **Phase 8 checks first, in addition to round 1's list.**
  - There is exactly one health-wait loop, `wait_healthy`, and it checks before it asks.
  - No `budget.check()` duplicates the wait's own check.
  - The rig imports `BOOT_POLL` rather than copying it.
- **Round 1's list still applies.** A second argv log or recording wrapper, a second conformance runner or table, a new escape for `-c`, or a second env-prefix builder is still a rejection.
- **To file:**
  - W-15: the plan's reads, and `session_of_record`'s two calls, against I5's "checked before every live step";
  - W-11(a): an end-to-end check of the server under tmux, with #649;
  - W-13: the merge order with #664.

## Patterns referenced

- `crates/holler-pane/src/tx_launch.rs` (`Budget`, `Plan::observe`, `bring_up`, `session_of_record`, the I5 module doc) and `tx_launch/server.rs` (`wait_healthy`, `start`, `stop_old`); `92fbc6e`'s `bring_up` for the earlier check between `serve` and `health`.
- `crates/holler-pane/src/probe.rs:297-308` (`wait_until`, the shape `wait_healthy` copied, and why it does not carry over to a bounded port call).
- `crates/holler-pane-testkit/src/conformance/harness.rs` and `crates/holler-adapter-opencode/tests/real_opencode/rig.rs`: the wait's two other callers.
- `docs/adr/ADR-0021.md`: section 2 ("The harness server as amended (#802)" and the health-wait paragraph), section 5 (the engine's side effects), section 12 (I5's bound).
- `docs/handoffs/802/handoff-T-green.md` (B-1) and `decisions.md`.
