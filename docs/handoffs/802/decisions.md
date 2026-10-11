# Decisions — #802 `HarnessPort::serve_command` takes a `ServeSpec`; the harness server runs in the pane's tmux session

## A (Phase 3, up-front plan review) — 2026-10-11T05:29:31-06:00
- **Decided:** PASS on `docs/handoffs/802-brief.md` at `c03544a` (sha256 `e895a4d9f9e72424...`), with no blocks and thirteen warns. The full detail is in `handoff-A.md`.
  - **The objects and the layers are right.**
    - Both ports are amended in place: `HarnessPort::serve` becomes the pure `serve_command`, a required `verify_serving` is added, and `HostPort::run` gains `cwd`.
    - `ServeSpec` sits beside `HerdrSpec` in `ports.rs`, and `AGENT_UNSUPPORTED` beside `PROMPT_UNSUPPORTED`.
    - `HostPort` becomes the one place that starts and stops a pane's processes. The adapter's own spawn, poll, reaper and group kill are deleted, not kept alongside.
    - The engine composes the two ports, and every refusal stays in the plan.
    - The test kit extends its existing objects, and the two fakes still share no state.
  - **W-1 (binding).** `run` refuses a `cwd` that is not an absolute path to an existing directory, as `usage`. This keeps the host adapter's narrowing 5: no program starts silently in `$HOME`. It extends the brief's rule for an empty or relative directory.
  - **W-2 (binding reading of AC 5).** No allowlist filter. The argv holds the adapter's own literals and the configured `Isolated` pairs, and never anything from the adapter's own process environment. T proves it with an exact pin, plus either a re-executed child test or a source scan. The `ProcessEnv::Isolated` doc now names the server's command line too.
  - **W-3.** OpenCode's `verify_serving` answers `agent-unsupported` for a spec with an agent, failing closed. The trait doc says what `Ok` and a refusal mean. The fake answers from the agent its own server was started with, so it can model #642 part 3's check.
  - **W-4.** The engine, the conformance suite and the real rig's launcher use one `pub` health-wait function.
  - **W-5.**
    - Build one `ServeSpec` per run, in `Plan::new`, as an exhaustive literal; both `serve_command` and `verify_serving` get it.
    - The argv goes in `Seen`.
    - The `port-in-use` re-check sits right before the server's `run`.
  - **W-6.** The whole server step goes into the child module of `tx_launch.rs`, which is 787 lines.
  - **W-7.** AC 4's relaunch case keeps the record's and the spec's directory equal (E0 refuses a change); the session was made for another directory.
  - **W-8.** The test kit's Phase 7 candidates:
    - `runs()` gains the `cwd`;
    - the scenario method never makes `run` fail;
    - cross-port order is proven with the existing hooks;
    - the docs of both fakes are updated.
  - **W-9.** The host adapter's crate docs, and its now-unused read of the session's directory.
  - **W-10.** The ADR edits beyond Scope 7:
    - the `HostPort` amendment;
    - "Decisions taken" items from 10;
    - the #695 Deferred bullet;
    - a rollback kept apart from a crash;
    - why `agent-unsupported` is a refusal;
    - the limits that move with the server.
  - **W-11 (follow-ups).**
    - No test starts the server under a tmux pty.
    - Fact 1's precondition, that the TUI is the session's current window, now matters for every pane.
  - **W-12 (low).** `with_agent` as a builder on `self`; the env-prefix helper beside `ProcessEnv`.
  - **W-13 (low).** The merge order with #664's `profile_verbs/rig.rs`.
- **Assumed:**
  - tmux starts a program in `$HOME` when the directory `-c` names is missing. The host crate records this for `#{session_path}` (`tmux.rs:267-271`, PR1 and PR5). I did not probe it again for a literal `-c`.
  - `opencode serve` behaves under a tmux pty as it does with null stdio. Nothing in this story verifies it (W-11).
  - Nothing wires the adapters in production yet: `Wiring::connect` hands out `Unwired` until #649. So the credential path and the interim agent refusal reach no live pane before #649 and #642 part 3.
  - The brief's citations hold at `c03544a`. Of the items its outside review marked "needs verification", I checked these against the code: NV-3 to NV-12.
  - Two of those items I did not check:
    - **NV-1.** The `compile_fail` precedents exist (`error.rs:324,481,490`). That E0639 refuses a struct expression of a `#[non_exhaustive]` type, with or without `..Default::default()`, is rustc's rule; I did not compile it here, and T's RED shows it.
    - **NV-2.** tmux's limit on command size is #642 part 3's concern.
- **Hedged:**
  - W-2 is a warn with a binding reading, not a block. That reading keeps an existing contract and contradicts nothing else in the brief. If S reads AC 5's parenthetical as an allowlist, the question goes to the operator.
  - W-1 is a warn: F can meet it within the brief.
  - W-3 rests on #642 part 3's decision 3, as `gh issue view 642` shows it: an unusable agent is a refusal with a code the adapter owns. I did not prescribe the fake's code for a mismatch.
  - The brief cites the review handoff of the first re-scoped run. It is not on disk. The stopped worktree `0802-serve-spec` holds only the review of the brief before the re-scope. Three of its warns still apply and are carried here:
    - the exhaustive literal (now W-5);
    - the `with_agent` builder (W-12);
    - the ADR placement (W-10).
  - No `survey.md` exists for this automated run, so the brief's Scope served as the reuse map.
- **Evidence:**
  - Read in full:
    - the brief, and its outside review `802-brief-result-r1.md` (PASS with NV-1 to NV-12);
    - the pre-re-scope `handoff-A.md` in `.claude/worktrees/0802-serve-spec`;
    - `holler-pane/src/{ports,lib,argv}.rs` and `tx_launch.rs` (all 787 lines);
    - `holler-adapter-host/src/{lib,tmux,exec}.rs`;
    - `holler-adapter-opencode/src/{lib,server,tui,exec}.rs`, and `attach.rs:1-120`;
    - `holler-pane-testkit/src/{harness,host,fault}.rs` and `src/conformance/{harness,mod}.rs`, and `conformance/host.rs:1-140`;
    - `holler-cli/tests/pane_verbs/launch/rig.rs`;
    - ADR-0021: lines 24-238 and 300-773.
  - Read in part:
    - `holler-cli/tests/pane_verbs/launch.rs` (`40-110`, `240-300`, `505-607`);
    - `holler-adapter-host/tests/fake_tmux_test.rs` (`172-235`, `352-392`, `460-506`, `690-720`) and `real_tmux_test.rs` (`270-305`, `405-452`);
    - `real_opencode/rig.rs` (`1-60`, `150-260`, `360-448`);
    - `scripts/spikes/opencode-lib.sh` (`125-175`, `250-296`);
    - `reconcile.rs:1-60` and `reconcile/observe.rs:1-40`, `125-160`;
    - `profile_snapshot.rs:25-50`.
  - Greps:
    - every `impl HostPort for` and `impl HarnessPort for` site;
    - readers of `workdir`, `boot_try`, `OP_SERVE`, `kill_group` and `.pid`;
    - the `with_*` constructors;
    - `TxOptions` literals;
    - `docs/` and `README.md` for statements of the old `serve` placement.
  - Other evidence:
    - `gh issue view 642`: part 3, decision 3;
    - #664's brief, §10 and O3;
    - the in-flight worktrees: `git diff --name-only origin/main...HEAD` showed code changes in none of them.

## T (Phase 4, author / RED) — 2026-10-11T05:58:00-06:00
- **Decided:**
  - RED is valid, and it is compile-level, as the brief's RED note allows. The 11 test targets that
    call the new API fail to build, and every error names a missing or changed API or a deleted
    field. All libraries build. The full list is in `handoff-T-red.md`.
  - The API names the tests pin, for F:
    - `ServeSpec::default().with_agent(k)`, a builder on `self`, per W-12.
    - `holler_pane::ports::AGENT_UNSUPPORTED`.
    - `tx_launch::wait_healthy(&dyn HarnessPort, port, poll, deadline) -> Result<bool, PaneError>`, per W-4.
    - `TxOptions.boot_poll`.
    - `HarnessOp::ServeCommand` and `HarnessOp::VerifyServing`.
    - `FakeHarness::start_server(&Argv) -> Option<Result<u32, PaneError>>`.
    - `FakeHarness::serve_specs()`.
    - `ServerView.agent`.
    - `FakeHost::runs() -> Vec<(PaneName, String, Argv)>`.
    - `run_harness_conformance(fresh, launch)`, with case 2 renamed `launch-then-healthy` and case 16
      `serve-command-carries-the-agent`.
  - W-9 is option (a): the host adapter's `run` keeps one existence read before the argv check, so
    the three-call shape and `run_checks_the_session_before_the_argv` stand.
  - The server's env prefix is the TUI's own, `env -u OPENCODE_DISABLE_TERMINAL_TITLE` under
    `Inherit`. This follows Scope 3's "one helper ... for both".
  - OpenCode's `verify_serving` refuses `agent-unsupported` for an agent (W-3, fail closed).
  - AC 4a and AC 4b both run through `rig.engine` with a 700 ms budget and expect `timeout`.
  - A crash leaves the started server running (W-10(d)).
  - The relaunch-survivor message drops `#695`.
  - The cross-port order is read from the fakes' logs at hooked steps (W-8c). The rig keeps no
    merged log.
- **Assumed:**
  - The plan calls `serve_command` before every other live read, the probe included. The brief
    says "in `Plan::observe`, before the probe".
  - The budget is checked before `verify_serving`, because I5 requires a check before every live
    step.
  - The `port-in-use` re-check runs on relaunch too, after `stop_old`.
  - The fake's refusal code for an agent mismatch in `verify_serving` is F's choice. The tests
    assert only `PaneError::Refused`.
- **Hedged:**
  - AC 9's "a `panes.json` written before still reads" half is a characterization pin. The test is
    `holler-hub` `pane_registry_test`, and it is green now by design.
  - The opt-in real tests (`real_tmux_test` and `real_opencode_test`) were compiled against a shim
    but not run here: there is no OpenCode, and they are `#[ignore]`.
- **Evidence:**
  - The scratch shim worktree held the new signatures, a minimal fake and adapter, and an engine
    that follows the brief. Against it:
    - every target compiled;
    - clippy `-D warnings` was clean on the tests;
    - `lint.sh` passed on the test files;
    - every authored test passed: `pane_verbs` 258/258, `fake_harness_test` 23, `fake_host_test`
      12, `harness_conformance_test` 15, `host_conformance_test` 12, `hermetic_test` 31,
      `attach_test` 23, `fake_tmux_test` 29, `pane_registry_test` 15 and `ports_test` 8.
  - A naive engine on the new signatures fails 21 `pane_verbs` cases on assertions. The messages
    are in `handoff-T-red.md`.
  - The shim was deleted afterwards.
