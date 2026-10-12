# Handoff-A-dup: Phase 7 - #831 A Claude Code harness for panes  (anti-duplication gate)

**Date:** 2026-10-11
**Branch:** `issue-831-implementation` (worktree `.claude/worktrees/0831-claude-harness`)
**Diff base:** `d6ae539` (the `origin/main` that F's phase commit merged)   **Diff head:** `d137a1d`
**Reuse map:** `docs/handoffs/831/handoff-A.md`, "Reuse map (for the Phase 7 gate)"
**Verdict:** PASS

## Summary

PASS, with 0 `block` and 9 `warn` findings. F extended every object the Reuse map named, and none of the ten
parallel paths that map listed exists:

- **One router.** `holler_pane::HarnessRouter` (`ports/router.rs`) is the only routing table. The wiring builds
  `HarnessRouter<WiredHarness, WiredHarness>`, and the test kit's `FakeHarnesses` wraps
  `HarnessRouter<FakeHarness, FakeClaude>`.
- **One relaunch sequence.** `tx_launch::relaunch_on` and `relaunch_recorded` are the one stop, run, wait, locate
  and record sequence. `tx_switch`, `pane doctor` (through `reconcile_with`'s caller-supplied `Relaunch`) and
  `interrupt_pane` all call it. Reconcile does not import `tx_launch`.
- **One wait loop.** `tx_launch::wait_until` replaced `wait_healthy`'s loop. `wait_healthy`, `wait_located`, the
  act's idle wait and the Claude conformance suite all use it.
- **One rule each** for:
  - the port policy (`port_for`, which both the CLI's flag check and the plan call);
  - the harness check (`check_harness_unchanged`, shared by E0 and `profile apply`);
  - the login variables (`HarnessKind::login_variables`, read by the plan's refusal and the adapter's `env -u`);
  - the command name (#835's `CLAUDE_CODE_COMMAND` and `is_claude_code_command`, which no other code compares with
    `"claude"`).
- **The test kit's patterns, extended.** The forwarding pin extends the existing `Recording` and
  `call_every_method`. The Claude suite runs through the shared `run_cases`. The only rig with `FakeClaude` links
  through `FakeHarnesses::start` and `stop`.
- **The record** goes through `HarnessInfo::endpoint` and `try_from = "RawHarnessInfo"`.
- **#835's code is copied.** Its host-side copies are byte-identical to `issue-835-implementation`.

The warns are small near-copies and doc overstatements. The one I would fix first is W1: two serde parses of a
harness name, plus a hand-written name list that no test pins.

## Findings

| # | Severity | File:line | Finding | Suggested fix |
|---|---|---|---|---|
| 1 | warn | `crates/holler-cli/src/pane/args.rs:171` | **A harness name is parsed twice and named once by hand.** `parse_harness` repeats `holler-pane/src/import/files.rs:115` `harness_kind`: both turn a name into a `HarnessKind` with `serde_json::from_value(Value::String(..))`. Separately, `HarnessKind::as_str` (`pane.rs:188`) is a hand-written third list of the names, and no test pins it to the serde names. | Add one `HarnessKind::from_name(&str) -> Option<HarnessKind>` (or `FromStr`) in `pane.rs` beside `as_str`, and call it from the import and from `--harness`. Pin `as_str(kind)` to the serde name of every kind, for example in `rework_test.rs`'s `HARNESS_IDS` loop. |
| 2 | warn | `crates/holler-pane/src/reconcile/observe/process.rs:144-208` | **The single-process chain copies the server chain's mismatch skeleton.** `check_held` and `repair` repeat the shape of `observe.rs:266-369`, `check_screen` and `repair`: the `shown_differs` check, the tense switch, `ShownDrivenMismatch` with `with_fix`, and the orchestrator skip predicate (`pane.role == PaneRole::Orchestrator && request.pane != Some(&pane.name)`, at `observe.rs:359` and `process.rs:195`). Also, `port_text` and `own_ports` (`observe.rs:325-334`) work out the port again from the record, although the chain already holds `at`. They give "no port" on a path a single process never reaches. This is a sibling chain that the plan anticipated (W3), and it reuses the parent's helpers, so it is not a parallel path. | Move the `--fix` gate into one helper in `observe.rs` that both `repair`s call: not fixable, fixable without `--fix`, skipped for the orchestrator unless named. Each shape keeps only its own precondition (server lists it, or process idle) and its own act. Pass `at` to `check_session_of_record` and `foreign_session` and use its `Display`. |
| 3 | warn | `crates/holler-cli/src/pane/doctor.rs:81-87`, `crates/holler-pane/src/tx_switch.rs:258-278` | **`relaunch_recorded`'s answer is unwrapped three times.** Its `Launched.pane` is unwrapped twice with the same message ("the relaunch of {} stored no record"), and a third time in `interrupt_cmd.rs:81-84`. `relaunch_recorded` never runs `spec_only`, so its `pane` is always `Some` on success. | Have `relaunch_recorded` return the stored `Pane` (with `TxFailure` on error), or add one `Launched` accessor, and remove the three unwraps. |
| 4 | warn | `crates/holler-cli/src/interrupt_cmd.rs:99-117` | **`route` copies `say_cmd::route_target_by`'s `--pane` arm** (`say_cmd.rs:276`: `Wiring::connect().and_then(..).map_err(routing_stop)`), because `route_target_by`'s resolver must return a `String` session. Routing has two sites for the three prompt verbs. The pane gates themselves are shared (`resolve_pane_record`). | Make `route_target_by` generic over the resolver's answer (`fn(Ports, &str, Option<&str>) -> Result<T, PaneError>`), so `say`, `answer` and `interrupt` route through one function. |
| 5 | warn | `crates/holler-adapter-claude/src/exec.rs:1-143` | **A third private copy of the bounded capture runner.** The host and OpenCode adapters have the other two, and `holler-pane/src/probe.rs::run_bounded` is a fourth. This follows the documented pattern: an adapter may not depend on another, and #696 is the story that exposes one runner from `holler-pane`. F declared it in its handoff. | Add the Claude adapter's `exec.rs` to #696's scope with a comment on #696. No change in this PR. |
| 6 | warn | `crates/holler-pane-testkit/src/harnesses.rs:9`, `crates/holler-cli/tests/pane_verbs/launch/rig.rs:688`, `crates/holler-cli/tests/profile_verbs/apply/rig.rs:639`, `crates/holler-cli/tests/pane_integration/bind.rs:519` | **The one-linker rule is only half met.** The map asked the three existing `LinkedHost`s to call `FakeHarnesses::start` and `stop`. They keep their own `start_server` and per-port `kill` loops, which is consistent with the brief's rule that no assertion is edited and no rig changes shape. `FakeHarness::kill_servers_of` (`harness.rs:261`) is a fourth form of that loop. So the docs of `FakeHarnesses` and the `pane_claude` rig overstate ("No rig writes a linker of its own"). No per-rig `FakeClaude` linker exists.<br>A smaller point: the `Resolving` test double (`pane_integration/bind.rs:559`) forwards #645's two default-armed methods but not #831's four. That is harmless over `FakeHarness`, but it is not the forward-every-method rule. | Reword the two docs to "every rig that links `FakeClaude` goes through `FakeHarnesses`", or fold the three OpenCode rigs into `FakeHarnesses` in a follow-up. Forward the four process methods in `Resolving`, or say why it does not. |
| 7 | warn | `crates/holler-cli/src/pane/wiring/env.rs:107` | **`"claude"` is written a second time.** `program_bin(env, CLAUDE_BIN, "claude")` spells the default program name, but ADR-0021 section 15 item 2 says `CLAUDE_CODE_COMMAND` is "the one place the name is written". It is not a comparison, so it is not on the plan's block list. But the PATH program and tmux's foreground command name are the same fact: a `HOLLER_CLAUDE_BIN` with another basename defeats `process-in-use` and #835's import rule. | Use `holler_pane::pane::CLAUDE_CODE_COMMAND` here. Or say in the constant's doc that the PATH name is a separate fact, and name the `HOLLER_CLAUDE_BIN` limit in `docs/pane-control.md`. |
| 8 | warn | `crates/holler-pane/src/pane.rs:232`, `crates/holler-pane/src/ports.rs:339` | **"Identical hunks" with #835 is not exact.** These are identical to #835's branch: the host adapter, `FakeHost`, the host suite and the decorators' forwards. These are not:<ul><li>the doc comments of `CLAUDE_CODE_COMMAND` and `HostPort::foreground_commands` each differ by one line;</li><li>the constant sits after `impl HarnessKind` here, but right after `enum HarnessKind` on #835's branch.</li></ul>So whichever lands second meets a textual conflict in `pane.rs` and `ports.rs`. A doubled `const` would not compile, so it cannot merge silently. #835 is at S `REWORK`. Also, `origin/main` moved to `0f5835e` (#855): a trial merge conflicts only in `CHANGELOG.md`. | Make the two doc lines match #835's, or amend section 15 item 2's "text for text" sentence. Merge `origin/main` again before the PR. |
| 9 | warn | `crates/holler-cli/tests/pane_verbs/switch.rs` (893 lines), `crates/holler-cli/tests/pane_verbs/launch/rig.rs` (852) | **Two files are near the size limit.** The endpoint migration grew both, and `scripts/lint.sh` fails at 900. `holler-adapter-host/tests/fake_tmux_test.rs` (820) is #835's copy. | The next story that touches `switch.rs` or `launch/rig.rs` splits it first. Record this in the PR body. |

**Reviewed and accepted, no finding:**

- `ClaudeOp` repeats `HarnessOp`'s eleven variants: the map asked for "`HarnessOp`-style ops", and T's RED fixed the
  name.
- `ProcessSession` (the engine's intent) beside `SessionChoice` (the port's resolved id): the ADR records both "as
  built".
- Reconcile's own idle comparison: it cannot use `tx_switch::check_idle`, which is a refusal and depends on reconcile.
- The adapter's in-call held-window poll and socket linger loop: adapters keep their own bounded polls, and none uses
  the engine.
- The port-contract codes declared in `holler_pane::ports` (`abort-unsupported`, `session-mismatch`, `inbound-held`,
  `server-unsupported`): both the adapter and `FakeClaude` raise them, and the conformance suite checks them.
- The test oracles: `pane_claude/rig.rs`'s `LOGIN_VARIABLES`, and the per-file `has_pair` helpers.

## Notes for F

Not required, since the verdict is PASS. If F is re-entered for another reason, W1, W3 and W8's doc lines are each a
few minutes of work, and every other warn is a follow-up or a line in the PR body.

## Patterns referenced

1. `crates/holler-pane/src/ports/router.rs`, `tx_launch.rs` with `tx_launch/{process,server,rules,undo}.rs`,
   `tx_switch.rs`, `reconcile.rs` with `reconcile/observe.rs` and `reconcile/observe/process.rs`.
2. `crates/holler-pane-testkit/src/{harnesses,claude,harness}.rs` and `conformance/{claude,harness}.rs`.
3. `crates/holler-cli/src/pane/wiring.rs` and `wiring/{bind,env}.rs`; `say_cmd.rs`, `interrupt_cmd.rs`,
   `answer_cmd.rs` and `pane/{args,doctor,launch}.rs`.
4. `crates/holler-adapter-{host,opencode,claude}/src/exec.rs`, `holler-pane/src/probe.rs` and issue #696.
5. `issue-835-implementation` at `d71b091`; `origin/main` at `0f5835e` (#855).
