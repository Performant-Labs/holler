# Handoff-A-dup: #831 A Claude Code harness for panes  (anti-duplication gate)

Two passes, newest first: the gate after F's rework (S's REWORK), then the first pass.

## Rework 1: the gate after S's REWORK (2026-10-11, 22:19 MDT)

**Date:** 2026-10-11
**Branch:** `issue-831-implementation` (worktree `.claude/worktrees/0831-claude-harness`)
**Diff base:** `adad6a2` (the first pass's gate commit)   **Diff head:** `7bd92cb`
**Reuse map:** `docs/handoffs/831/handoff-A.md`, "Reuse map (for the Phase 7 gate)"
**Verdict:** PASS

### Summary

PASS: 0 `block`, no new `warn`, and 6 `warn`s carried unchanged from the first pass. This cycle builds no new object and
no parallel path, and it removes one near-copy that the first pass found (W7).

The cycle has three commits:

- `ccec57c`: S's handoff only.
- `7260b5e`: F's rework, made inside the merge of `origin/main` at `037405f` (#855 and #856).
- `7bd92cb`: T-green's two test edits.

**What F changed beyond main's files.** I diffed F's merge result against a trial merge of its two parents
(`git merge-tree --write-tree ccec57c 037405f`, tree `cc5793b`). F changed only the files its handoff names:

- in code: `wiring/env.rs`, where one literal is now the existing constant, and three doc comments (`wiring/bind.rs`,
  `harnesses.rs` and `vocab.rs`);
- in the docs: `CHANGELOG.md`, ADR-0003, ADR-0021 and `docs/pane-control.md`.

**The first pass's warns, closed or narrowed:**

- **W7 is fixed.** `claude_bin` passes `holler_pane::pane::CLAUDE_CODE_COMMAND`.
  - In production code, `"claude"` is now written only in `HarnessKind::as_str` (the harness id), in the constant, and
    in `HARNESS_IDS`. The other copies are a hub unit test and `holler-body`.
  - `refuse_in_use` (`tx_launch/process.rs:65`) compares with the constant through `is_claude_code_command`, as the new
    doc in `env.rs` says.
- **W8 is fixed in the record.** ADR-0021 section 15 item 2 now says what holds, and I checked it again:
  - #835's branch is still at `f1d1bd0`, and its worktree (`0835-import-leave-alone`) has no uncommitted change.
  - These are identical on both branches: the constant's value, the rule, the signature of `foreground_commands`,
    `holler-adapter-host/src`, `FakeHost`, the host suite and `fake_tmux_test.rs`.
  - Only doc comments differ, among them two lines of `claude_code_rule_test.rs`'s module doc.
  - The branch now holds `origin/main`, which is still `037405f`, and no conflict marker is left.
- **W6's doc half is fixed; its `Resolving` half is closed.**
  - `harnesses.rs` and section 15 item 4 now say what the rigs do. The one `FakeClaude` rig (`pane_claude/rig.rs`)
    links through `FakeHarnesses::start` and `stop`. The three older rigs keep their own `LinkedHost`, as the new doc
    names them (`pane_verbs/launch`, `profile_verbs/apply` and `pane_integration/bind`).
  - `Resolving` is a test double over an OpenCode `FakeHarness`, not a `wiring.rs` decorator. The process methods it
    leaves out fall back to the same `process-unsupported` default that its inner type gives.

**The merge adds no parallel path.** F changed no code to fit #855 or #856 in.

- #856's roster builds its pane cells from `pane list`'s `PaneRow`. `PaneRow::from` reads no harness port. The roster's
  own `harness` column (`roster_cmd.rs:449`) is the hub's body field, unchanged since before #648.
- Two files auto-merged, `pane_integration/scenario.rs` and `wired.rs`. They carry only #831's endpoint migration and
  the usage check of #835's `foreground_commands`. Neither copies a helper that #855 moved into `scratch/`.
- The three conflicts were resolved by keeping both sides' intent:
  - `CHANGELOG.md`: both entries are kept.
  - ADR-0003: this branch's SPEC FLAGS line, and main's `say` line without "Until #648 ...".
  - ADR-0021 "Deferred": main's removal of the #648 bullet, and this branch's #839 and #850 bullets.

**T-green's two edits copy nothing.**

- `hermetic_test.rs`: `a_dialog_and_an_unknown_status_are_never_an_activity` reuses the file's `Rig`, `entry` and
  `endpoint`. It is the only test of `harness-dialog-open` and of the `unavailable` for an unknown status.
- `lifecycle.rs`: one assertion added to an existing test.

### Findings

All six are carried from the first pass. This cycle did not touch their files (`git diff adad6a2 7bd92cb` is empty for
each), so the line numbers still hold.

| # | Severity | File:line | Finding | Suggested fix |
|---|---|---|---|---|
| 1 | warn (W1, carried) | `crates/holler-cli/src/pane/args.rs:171`, `crates/holler-pane/src/import/files.rs:115`, `crates/holler-pane/src/pane.rs:188` | **A harness name is parsed twice, and `as_str` is a hand-written list.**<ul><li>`parse_harness` and `harness_kind` are two serde parses of the same name.</li><li>`rework_test.rs:174` pins every serde name to `HARNESS_IDS`, and REWORK 1's doc now cites that test. No test pins `as_str` to the serde names.</li><li>F deferred the shared parse because #835 rewrites `import/files.rs`. That follows the coexistence rule.</li></ul> | <ul><li>Pin `as_str(kind)` to the serde name in `rework_test.rs`'s `every_kind` loop. It is test-only and does not overlap #835.</li><li>Fold the two parses into one `HarnessKind` parser at item 17, which edits the import once #835 lands.</li></ul> |
| 2 | warn (W2, carried) | `crates/holler-pane/src/reconcile/observe/process.rs:144-208` | The single-process chain repeats the server chain's mismatch and `--fix` skeleton, the orchestrator skip predicate included. | Follow-up: one `--fix` gate helper in `observe.rs` that both `repair`s call. |
| 3 | warn (W3, carried) | `crates/holler-cli/src/pane/doctor.rs:81-87`, `crates/holler-pane/src/tx_switch.rs:258-278`, `crates/holler-cli/src/interrupt_cmd.rs:81-84` | `relaunch_recorded`'s `Launched.pane` is unwrapped three times. | Follow-up: have `relaunch_recorded` return the stored `Pane`. |
| 4 | warn (W4, carried) | `crates/holler-cli/src/interrupt_cmd.rs:99-117` | `route` copies the `--pane` arm of `say_cmd::route_target_by`. | Follow-up: make `route_target_by` generic over the resolver's answer. |
| 5 | warn (W5, carried) | `crates/holler-adapter-claude/src/exec.rs:1-143` | A third private copy of the bounded runner, by the documented pattern. #696 is OPEN, and none of its comments names this file yet (checked with `gh issue view 696 --comments`). | At the PR step, comment on #696 to add this file to its scope. |
| 6 | warn (W9, carried) | `crates/holler-cli/tests/pane_verbs/switch.rs` (893 lines), `crates/holler-cli/tests/pane_verbs/launch/rig.rs` (852) | Near `lint.sh`'s 900-line limit. Neither this cycle nor the merge changed them. | Name both in the PR body. The next story that touches either splits it first. |

**Reviewed and accepted, no finding:**

- **The `Bindings` doc (`wiring/bind.rs`).** It records that `pane doctor --fix` can relaunch several Claude Code panes at
  once, and that nothing reads the pairs that may cross. The code matches: the resolver is an OpenCode adapter type
  (`holler_adapter_opencode::Resolver`), handed only to the OpenCode harness (`wiring.rs:204`), and the Claude adapter
  takes none. The concurrency itself predates this cycle (S Advisory 8).
- **The new `vocab.rs` doc.** It names `holler_pane::pane::HarnessKind` from `holler-proto`, which is a lower crate.
  These are code spans, not intra-doc links, so `holler-proto` gains no dependency. Section 15 item 1 asks for this
  sentence.
- **Where `env.rs` records the coupling.** It documents that the PATH name and tmux's command name are the same name,
  next to the code that reads it. It does not add this to the constant's doc, which is the text #835 shares, so the
  known doc-comment conflict with #835 does not grow.

### Notes for F

Not required: the verdict is PASS. For the PR step:

- post W5's comment on #696;
- list W2 to W4 and W9 in the PR body as follow-ups, or file them.

W1's pin is a test-only edit that T can make in any later loop. Its shared parse belongs to item 17.

### Patterns referenced

1. The trial merge `git merge-tree --write-tree ccec57c 037405f` (tree `cc5793b`), diffed against `7260b5e`.
2. `crates/holler-pane/src/pane.rs` (the constant, the rule and `as_str`), `tx_launch/process.rs`, and
   `crates/holler-cli/src/pane/wiring.rs` with `wiring/{env,bind}.rs`.
3. `crates/holler-cli/src/roster_cmd.rs` and `pane/list.rs` (`PaneRow`), and `pane_integration/{scenario,wired,bind}.rs`.
4. `issue-835-implementation` at `f1d1bd0` (worktree `0835-import-leave-alone`, clean).
5. `crates/holler-pane/tests/rework_test.rs` (the serde-name pin) and `crates/holler-adapter-claude/tests/hermetic_test.rs`.

---

# First pass: Phase 7 - #831 A Claude Code harness for panes  (anti-duplication gate)

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
