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

## F (Workflow Phase 6, implement against RED) — 2026-10-11T06:26:17-06:00
- **Decided:**
  - Implemented the brief and A's warns against T's suite with no test edited; every target that
    was RED compiles and passes (the full verify command: 2070 passed, 0 failed).
  - The API is T's contract exactly: `ServeSpec` (`with_agent` a builder on `self`, W-12a) with
    the two `compile_fail,E0639` doctests on its doc; `serve_command` / `verify_serving`;
    `HostPort::run(name, cwd, argv)`; `AGENT_UNSUPPORTED`; `TxOptions.boot_poll` (`BOOT_POLL`,
    150 ms); `tx_launch::wait_healthy`; `HarnessOp::{ServeCommand, VerifyServing}`;
    `FakeHarness::{start_server, serve_specs}`; `ServerView.agent`; `FakeHost::runs()` with the
    `cwd`; `run_harness_conformance(fresh, launch)` with cases `launch-then-healthy` and
    `serve-command-carries-the-agent`.
  - The engine's server step is the child module `tx_launch/server.rs` (W-6): the `port-in-use`
    re-check, `host.run` of exactly the planned argv in the spec's directory, the health wait to
    the budget's end (`timeout`, the verb's op), `verify_serving`. `stop_old` moved there too, which
    keeps `tx_launch.rs` at 807 lines.
  - W-1: the tmux adapter's `run` refuses an empty, relative or missing `cwd` as `usage`, after the
    session and argv checks and before any window, without echoing it. W-9: the existence read
    reuses `tmux::panes`; `session_dir`, `SESSION_PATH`, `is_session_dir` and `BAD_SESSION_DIR` are
    deleted.
  - W-2: `ProcessEnv::prefix` (beside `ProcessEnv`) is the one env prefix of the server's and the
    TUI's argv; it holds only the adapter's literals and the configured `Isolated` pairs.
  - W-3: OpenCode's `verify_serving` refuses `agent-unsupported` for an agent and answers `Ok(())`,
    sending nothing, without one. The fake answers from its running server's agent, refusing a
    mismatch with its own open code `agent-mismatch`.
  - Scope 3's deletions are complete (`dead_code` is denied and clippy is clean).
  - ADR-0021 (Scope 7, W-10 a-f, W-11b) and `CHANGELOG.md` updated; decisions 1-4 recorded as the
    operator's, 5-7 as review round 1's, and "Decisions taken" items 10-13 added.
- **Assumed:**
  - tmux starts a `new-window -c <dir>` program in `$HOME` when the directory is missing (A's
    reading of `tmux.rs:267-271` for `#{session_path}`). Not probed for a literal `-c`; the refusal
    makes it moot, and the real-tmux test `run_refuses_a_missing_or_relative_directory` passes.
  - The `opencode serve` argv behaves under a tmux pty as it does spawned directly (W-11a, open).
- **Hedged:**
  - `stop_old`'s move into the child module is a structural choice the brief did not ask for; it is
    the relaunch's server step, and it buys headroom for #644 part 2 and #664 in `tx_launch.rs`.
  - The fake's `agent-mismatch` code and its `start_server` timeout op (`harness.serve_command`)
    are the fake's choices; T's tests assert only `Refused` and `Timeout`.
- **Evidence:**
  - `cargo build --workspace --tests`; `cargo clippy --workspace --all-targets -- -D warnings`;
    `scripts/lint.sh` (exit 0); `scripts/changelog-check.sh`; `cargo machete`; rustfmt check on the
    changed crates; `RUSTDOCFLAGS=-D warnings cargo doc` on the four changed crates.
  - `cargo test --workspace -- --skip roster_stays_accurate_under_concurrent_body_load`: 2070
    passed, 0 failed, 25 ignored.
  - Opt-in real runs on private servers: `real_tmux_test` 9/9 (`--ignored`) and
    `real_opencode_test` 9/9 (`HOLLER_TEST_OPENCODE=1`), the conformance suite through the new
    launcher against a real `opencode serve`.
  - The two struct literals compiled outside `holler-pane` fail with `error[E0639]`.

## T (Phase 7, verify GREEN + Tier 2) — 2026-10-11T06:36:24-06:00
- **Decided:** PASS. The suite is GREEN on `7ede9c7`: 2070 passed, 0 failed, 25 ignored, the same as F. Tier 1 is clean (lint, changelog check, clippy `-D warnings`, `docs_cli_test`, `wire_selftest`, machete, hook tests, doctests). Tier 2 finds no blocking issue. U is N/A (no UI), and the run is ready for S.
- **Decided:** Repaired one test. `hermetic_test::ac8_a_spec_with_an_agent_is_refused_and_one_without_verifies` now asserts that the stub received no request for the no-agent `verify_serving`, which its doc already claimed (F's note). No production code was touched.
- **Assumed:** F's opt-in real runs (`real_tmux_test`, `real_opencode_test`, 9/9 each) stand as F reported them. They need private tmux and opencode servers and are not CI steps, so I did not re-run them.
- **Hedged:** W-11(a) stays open: no test runs the server argv under a tmux pty. This is deferred to #649, as F proposes.
- **Evidence:**
  - Six production mutations were each caught by the targeted tests, then reverted with `git checkout`. M1, the act's port re-check; M2, `verify_serving` skipped; M3, the health wait with no retry; M4, the host's `check_cwd`; M5, OpenCode's agent refusal; M6, the plan passing a default `ServeSpec`.
  - Details are in `handoff-T-green.md`.

## T (Phase 7, verify GREEN + Tier 2, re-run after the brief's round-1 amendment) — 2026-10-11T07:28:09-06:00
- **Decided:** BLOCK, for F. The amended Scope 2 says each `health` call of the wait is a live step with the run's budget checked before it, and that ADR-0021 section 12's bound (the budget plus one port call's own bound) holds for the wait too. `server::start` calls `wait_healthy` straight after `host.run` with no budget check, and `wait_healthy` asks once more at the deadline. Before #802 the engine checked the budget between `serve` and `health`.
- **Decided:** Added `launch::serve::no_health_poll_starts_once_the_budget_has_run_out` (engine tier, `set_delay`, no sleep). It is RED on F's code: harness calls are `[ServeCommand, Health, Health, Health]` where the expected list is `[ServeCommand, Health, Health]`. It is GREEN with a deadline check before each ask in `wait_healthy`, which I tried and reverted.
- **Assumed:** The amended brief's sentence is binding even though some pre-existing plan and act steps (the plan's reads, `list_sessions` then `create_session`) also run two port calls without a check between them. Those are outside #802's diff. The wait is #802's own code and the brief names it.
- **Hedged:** The CHANGELOG clause about the adapter's own-process check is advisory (AC 12 / Scope 7), so it is not part of the BLOCK.
- **Evidence:** `handoff-T-green.md` (RED output, the trial fix's results, and the Tier 1 run on this tree).

## A (Phase 3, up-front plan review, round 2: the amended brief, after T-green's BLOCK with `resumeArchChanged`) — 2026-10-11T07:39:47-06:00
- **Decided:** PASS on `docs/handoffs/802-brief.md` as amended in `0dc4e8b` (sha256 `16db698d23e2428c...`), with no blocks. `handoff-A.md` now holds round 2; round 1 is at `0421ec1`.
  - **The amendment clarifies and adds.** It moves no object, layer or contract that round 1 approved. Its I5 rule for the health wait is the act's existing pattern: a budget check before every live step.
  - **W-4 restated, binding.** The one shared `wait_healthy` checks the deadline before every `health` call, including the first after `run`. Once the deadline has passed it answers `Ok(false)` without asking.
    - The engine passes the run budget's end.
    - No second loop and no extra `budget.check()` in `start`: the rule lives in the one function.
    - Four docs get updated: `wait_healthy`, `Budget::deadline`, step 3 of the module doc, and the ADR's health-wait paragraph.
    - This is T-green's B-1. Round 1's wording ("`Ok(false)` at the deadline") let F build it in `probe.rs`'s ask-then-compare shape.
  - **New warns.**
    - W-14 (low): `Budget::check` and `Budget::deadline` must agree for a budget the clock cannot hold.
    - W-15 (follow-up for O; a reading for S): the plan's four reads, and `session_of_record`'s two calls, run with no budget check between them. This predates #802, and #802 is not held to it.
    - W-16 (low; Phase 8): the real rig copies `BOOT_POLL` instead of importing it.
  - **Round 1's warns.** All stand and are implemented. W-1 and W-2 stay binding. The amended AC 4 absorbs W-7.
- **Assumed:**
  - T-green's report that the loop-top check turns its new test GREEN, with `pane_verbs`, the conformance suite and `fake_harness_test` all passing. I did not re-run it, and no code was changed in this phase.
  - The CLI is the only production caller of the engine, and it passes `TxOptions::default()`. So W-14's overflow case cannot be reached today. I checked this by grep.
- **Hedged:**
  - W-15 is a follow-up, not a block. It predates #802, and the amended sentence sits in the health-wait bullet.
  - The brief review's round 2 (`802-brief-result-r2.md`, BLOCK, unapplied) raised four points. All four are about test design or wording (AC 2, 4, 5 and 9). None changes an object, a layer or a contract. The operator relaunched past that gate.
- **Evidence:**
  - Diffs and records:
    - `git diff c03544a HEAD -- docs/handoffs/802-brief.md`, which is the whole amendment;
    - `802-brief-result-r1.md` and `802-brief-result-r2.md`;
    - `802-diff-result-r1.md`: the diff gate never ran, because the prompt ceiling refused it;
    - `handoff-T-green.md`, `handoff-F.md` and `decisions.md`.
  - Read in full:
    - `tx_launch.rs` (807 lines) and `tx_launch/server.rs` (101);
    - `92fbc6e`'s `bring_up` and `Plan::observe`.
  - Read in part:
    - `probe.rs:92-106` and `:297-308`;
    - `conformance/harness.rs:1-60` and `:500-545`;
    - `real_opencode/rig.rs:1-60` and `:180-270`;
    - `launch/rig.rs:642-688`, `launch/serve.rs:320-400`, `testkit/src/host.rs:1-60`;
    - `ports.rs:1-40` and `:200-262`;
    - `holler-adapter-host/src/lib.rs:466-492`;
    - ADR-0021 lines 100-200 and 280-300, plus section 12 (740-760).
  - Greps:
    - the callers of `wait_healthy`, `BOOT_POLL` and `DEFAULT_BUDGET`;
    - `TxOptions` construction in the CLI;
    - the production callers of `serve_command`, `verify_serving` and `.pid`;
    - absolute-path checks in `holler-pane` and the CLI (there are none).
