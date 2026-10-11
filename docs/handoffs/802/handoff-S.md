# Handoff-S: Phase 10 - #802 `HarnessPort::serve_command` takes a `ServeSpec`; the harness server runs in the pane's tmux session  (spec audit)

**Date:** 2026-10-11 (08:48 MDT)
**Branch:** `issue-802-implementation` (worktree `.claude/worktrees/0802-serve-command`, head `a33a01f`, base `92fbc6e`).
`origin/main` is now at `263a2d7` (06:42 MDT), three commits ahead: #816 (#808, the Herdr pane label), #815 and #814.
A trial merge (`git merge-tree`) conflicts in three files. See advisory 1.
**Issue:** #802 (epic #633; closes #695). I read it live with `gh issue view 802`:
- Its last edit (05:03 MDT) predates `c03544a` (05:08 MDT), the commit that populated the brief. It has no comments.
- Decisions 1-7 and "Out of scope" are identical in the issue and the brief.
- The brief adds the round-1 clarifications (`0dc4e8b`). Their own preamble says they "clarify and add" and change no
  decision. AC 5, the one with a reading question, has the same text in both.

**Brief:** `docs/handoffs/802-brief.md` as amended in `0dc4e8b`.
**Handoffs reviewed (all under `docs/handoffs/802/`):**
- `handoff-A.md` (round 2; round 1 at `0421ec1`) and `handoff-A-dup.md`;
- `handoff-T-red.md` (round 2; round 1 at `26148ba`);
- `handoff-F.md` (round 3; rounds 1 and 2 at `7ede9c7` and `f27d5d0`);
- `handoff-T-green.md` (round 3; round 1 with M1-M6 at `3fe111d`, the BLOCK at `dab3cd3`, round 2 at `2a32fc8`);
- `decisions.md` and `evidence.md`.

**Outside gates.** Every round ran on `deepseek-v4-pro`, per its `.usage.json`.
- Brief round 1: BLOCK, applied in `0dc4e8b`.
- Brief round 2: BLOCK, not applied. The operator relaunched with `designApproved`. I checked its four points against the
  code (see Spec compliance).
- Diff round 1: BLOCK on B-1, answered by F's round 3 in the contract text.
- Diff round 2: PASS.

**Verdict:** PASS

## A precondition

Met.
- `handoff-A.md` (round 2, on the amended brief): PASS, 0 blocks. W-1, W-2 and W-4 are binding, and the code meets all
  three.
- `handoff-A-dup.md` (the diff `92fbc6e..9861930`): PASS, 0 blocks. Its four warns concern test code, file size and a
  reading note for #642 part 3.

## T precondition

Met. `handoff-T-green.md` (round 3) reports "Blocking issues: None".

- **RED.**
  - Round 1 (`26148ba`): compile-level RED at the new API, which the brief's RED note allows for a signature amendment.
    A naive engine on the new signatures then failed 21 `pane_verbs` cases on their assertions.
  - Round 2 (`563288a`): three assertion failures, an extra `health` call each, after the brief's I5 amendment.
- **GREEN.**
  - The verify command (`cargo test --workspace -- --skip roster_stays_accurate_under_concurrent_body_load`) gives 148
    binaries: 2073 passed, 0 failed, 25 ignored. T then added T3, so `pane_verbs` is 262/262.
  - Nine production mutants were each caught by the targeted tests: M1-M6 in round 1, M7-M9 in round 3.
  - Tier 1 is clean: clippy `-D warnings`, `lint.sh`, `changelog-check.sh`, `cargo machete`, rustdoc `-D warnings`,
    rustfmt on the touched files, `docs_cli_test`, `wire_selftest` and the hook tests.
- **Phase boundaries, which I checked with `git diff --stat`.**
  - The T phases touched no `crates/*/src/**`.
  - The F phases touched no `crates/*/tests/**`, with one exception: round 2's one-line `BOOT_POLL` import in
    `real_opencode/rig.rs` (A's W-16). T-red handed it to F, and T-green round 2 accepted it.
  - A-dup touched only `docs/handoffs/802/`.

## Acceptance criteria

Test paths are under `crates/`. "Mutant" means a production mutation that T applied and that the test caught.

| # | Criterion | Proving test or evidence | Status |
|---|---|---|---|
| 1 | `ServeSpec::default()` has no agent; `with_agent(key)` holds the key; two `compile_fail,E0639` doctests, one per form | `holler-pane/tests/ports_test.rs:571` `serve_spec_default_has_no_agent_and_with_agent_holds_the_key`. The doctests are at `holler-pane/src/ports.rs:113` (`ServeSpec { agent: None }`) and `:116` (`..Default::default()`); `cargo test -p holler-pane --doc` gives 5 passed (F round 3, T-green round 1) | Met |
| 2 | Plan order: `serve_command` in the plan before any live call; `host.run` gets exactly that argv after `ensure_session`; `agent-unsupported` is exit 3, with nothing written, no `ensure_pane`, no rollback and no reconcile step, asserted on the exact call logs | `holler-cli/tests/pane_verbs/launch/serve.rs:59` `an_agent_refusal_in_the_plan_is_exit_3_and_writes_nothing` (JSON and text). It asserts `calls.harness == [ServeCommand]` and that `calls.herdr`, `calls.host` and `calls.probes` are empty, with `assert_eq!`. It also asserts no `CasPut`, P still at generation 1 and no reconcile step in the output. `:94` covers a relaunch: nothing is stopped, and the server and record are unchanged. `:146` `serve_command_runs_in_the_plan_before_any_other_call` uses a hook snapshot at `ServeCommand`. The exact argv after `ensure_session` is `:170` (host log `[EnsureSession, Run, Run]`, `runs()` equal to the planned argv). Mutant M6 | Met |
| 3 | The in-flight agent reaches `serve_command` on launch and relaunch, including a relaunch whose `--agent` differs from the recorded key; the act order is `ensure_session`, the command, the server, the wait, `verify_serving`, the session of record, `attach_tui` | `serve.rs:170` `the_act_runs_the_planned_argv_after_the_command_then_waits_and_verifies`, with snapshots at the server's start and at `VerifyServing` and the full step list. `serve.rs:246` `relaunch_passes_the_in_flight_agent_to_serve_command` (`orchestrator` recorded, `feature-implementor` asked), plus the no-flag case. `relaunch.rs:576` `relaunch_with_agent_replaces_the_stored_key`. Mutants M2 and M6 | Met |
| 4 | Directory: `run` gets `plan.spec.host.cwd` for the command and the server; a relaunch into a session made for another directory passes the spec's; tmux `-c <cwd>`; empty or relative is `usage`; `;` and `#` escaped | Engine: `serve.rs:170` (both runs in `/srv/demo`) and `serve.rs:271` `a_relaunch_runs_the_server_in_the_spec_directory_not_the_sessions` (session made in `/srv/elsewhere`, which it keeps). Fake tmux: `fake_tmux_test.rs:183` (`-c` is the given `cwd`), `:486` `run_refuses_a_cwd_that_is_empty_relative_or_missing` (`""`, two relative forms and a missing dir are `usage`, no `new-window`, never echoed), `:511` `run_escapes_the_cwd_for_tmux` (`p##S`, `q\;`, `##(touch x)`), `:546` `run_checks_the_session_then_the_argv_then_the_cwd`. Real tmux (opt-in): `real_tmux_test.rs:421` `run_works_in_the_given_cwd_not_the_sessions` (the program's own `pwd -P` is B; the session's path stays A). Mutants M4 and M7 | Met |
| 5 | OpenCode `serve_command`: an agent is refused `agent-unsupported`; without one, the argv is `start()`'s, pinned; starts nothing; no credential in any argv | `holler-adapter-opencode/tests/hermetic_test.rs:566` pins the literal argv under `Inherit` and `Isolated`: the `env` prefix, `serve --port P --hostname 127.0.0.1`, then `--pure`. `:610`: the same prefix as `tui_argv`. `:624`: no process starts and nothing is sent. `:654`: an agent is refused by `serve_command` and by `verify_serving`, and with none, `Ok(())` with nothing sent. `:701` with `:675`: a re-executed child holds a secret-looking variable in its own environment, and no argv carries its name or value, under either `ProcessEnv` (A's W-2 reading; see Spec compliance). Mutant M5 | Met |
| 6 | Health wait: a server that never answers ends the run as `timeout` (`pane.launch` / `pane.relaunch`) within a generous bound and rolls back; `boot_poll` defaults to 150 ms | `launch.rs:272` `launch_times_out_when_the_server_never_gets_healthy`: through `rig.engine` with a 700 ms budget, `Err(TxFailure { Timeout("pane.launch"), acted: true })` in under 3 s, `StopOwned` called, the server `Killed`, the Herdr pane closed, nothing recorded. `launch.rs:255` covers a frozen server holding the port. `serve.rs:299` checks `boot_poll` is 150 ms. `serve.rs:306` shows the wait retries (M3). `serve.rs:337`, `:381` and `:400` pin the I5 rule, a deadline check before every ask, in the engine and in the shared `wait_healthy`. The relaunch op has no test of its own; see advisory 2 | Met (advisory 2) |
| 7 | A `verify_serving` refusal after a healthy start rolls back, and the server is stopped; a launch with no agent calls `verify_serving`, and it answers `Ok(())` | `serve.rs:419` `a_verify_refusal_rolls_back_and_stops_the_server`: exit 3, `StopOwned` called, no `CreateSession`, the Herdr pane closed, the server `Killed`, nothing recorded. `launch.rs:67` `launch_records_what_the_fakes_show`: a no-agent launch whose steps include `VerifyServing` and which succeeds. `hermetic_test.rs:654` covers OpenCode's `Ok(())`. `serve.rs:462` covers a server gone after the verify (T3, mutant M9). Mutant M2 | Met |
| 8 | Relaunch: `stop_owned` stops the running server, a new one comes up on the same port, and the old one is gone; a survivor fails loudly (`unavailable`, exit 1, "still answers", generation unchanged) | `serve.rs:552` `relaunch_stops_the_old_server_and_starts_one_on_the_same_port`: host `[StopOwned, EnsureSession, Run]`, a new pid on `PORT`. `relaunch.rs:496` `relaunch_fails_when_the_old_server_survives`: exit 1, `unavailable`, "still answers", no `#695`, no server argv run, generation unchanged | Met |
| 9 | A `panes.json` written before still reads; a `launch` record has no `harness.pid` key | `holler-hub/tests/pane_registry_test.rs:229`: the stored fixture holds `pid: 4242` and loads unchanged; written back without a pid it has no key and reloads as `None`. `serve.rs:573` `a_launched_record_has_no_pid_key` checks the reply and the stored record. The brief review's round-2 B-3 asked for exactly this split | Met |
| 10 | The conformance suite passes for `FakeHarness` and `OpenCodeHarness` (opt-in, through its launcher); the new agent case holds; the new mutant is caught | `holler-pane-testkit/tests/harness_conformance_test.rs:54`: the fake passes 16 cases. `:302`: `DropsAgent` fails `serve-command-carries-the-agent`. `:309` adds an extra mutant, `ServeCommandStarts`, which fails `launch-then-healthy`. Real OpenCode: `real_opencode_test.rs:56` (opt-in, `Recording::launch`) passed 9/9 against opencode 1.19.0 in F round 2. Round 3 changed doc comments only (`git diff 2a32fc8 7ee7d8e -- crates/`). I took this run as reported | Met |
| 11 | `AGENT_UNSUPPORTED` is `agent-unsupported`, exit 3, listed wherever `PROMPT_UNSUPPORTED` is and in the ADR rows of Scope 7 | `ports_test.rs:588` `agent_unsupported_is_a_refusal_with_exit_3`. The constant is beside `PROMPT_UNSUPPORTED` (`ports.rs:315-319`). Every listing of `prompt-unsupported` (`ports.rs`, `ports_test.rs`, ADR-0021) now also has `agent-unsupported`. ADR-0021 has it in the launch/relaunch row (`:606`), the `profile apply` row (`:617`) and the open-codes list (`:668`), and not in the switch/reset row | Met |
| 12 | ADR-0021 and the code docs as in Scope 7; a `CHANGELOG.md` `[Unreleased]` entry linking #802 and #695 that names the behaviour changes; the Scope 3 deletions are gone | ADR: every place Scope 7 names is edited (see Spec compliance). `CHANGELOG.md:390-408` is under `## [Unreleased]` / Enhancements. It links #633, #695, #649 and #802, says it replaces the part 1 entry's statement, and names the `timeout` change and the loss of the adapter's own-process check. Deletions: a grep finds no `boot_try`, `OP_SERVE`, `BOOT_INTERVAL`, `KILL_BOUND`, `reap_when_it_exits`, `wait_until_up`, `refuse_a_held_port`, `workdir` or adapter `kill_group` (the remaining `kill_group`s are the prober's and the host adapter's, both older). `dead_code = "deny"` (`Cargo.toml:33`) with clean clippy proves nothing is left unused | Met |

## Spec compliance

**The operator's decisions (1-4) and review round 1's (5-7), as implemented:**

1. **The agent is applied at server start.** This PR carries the agent to the port, and #642 part 3 applies it. The
   OpenCode adapter refuses an agent until then, failing closed.
2. **The port itself changed.** `serve_command` and `verify_serving` are required trait methods, with no default
   (`ports.rs:238-264`), and every implementation and double was updated.
3. **`ServeSpec`** is `#[non_exhaustive]`, derives `Debug, Clone, Default, PartialEq, Eq`, and has
   `pub agent: Option<AgentKey>`. It sits beside `HerdrSpec` and is re-exported from the crate root. The engine builds it
   with every field named (`tx_launch.rs:322-324`).
4. **The server runs in the pane's tmux session.**
   - `serve_command` is pure: it builds the argv and starts nothing (`holler-adapter-opencode/src/server.rs:27-44`).
   - The adapter's spawn, boot poll, reaper and group kill are deleted.
   - `HostPort::run` and `stop_owned` start and stop the server.
   - Launch writes `pid: None` (`tx_launch.rs:387`).
5. **The agent check after start.** `verify_serving` runs after the health wait and before the session of record
   (`tx_launch/server.rs:106-107`), and a refusal goes through the existing rollback, which now stops the server. OpenCode
   answers `Ok(())` with nothing sent for a spec without an agent. That is decision 5's literal text, which F kept against
   the diff gate's B-1 remediation; the trait doc and ADR decision 5 now say the same.
6. **The directory comes from the spec.**
   - The signature is `run(name, cwd, argv)`.
   - The tmux adapter passes `-c` through the existing `escape_cwd` (`tmux.rs:71`), and `TmuxHost::run` refuses a bad
     directory as `usage` (`holler-adapter-host/src/lib.rs:467-487`).
   - The engine passes the spec's `cwd` for both runs.
   - `OpenCodeConfig.workdir` is deleted, and fact 6 is rewritten.
7. **Every refusal stays in the plan.** `serve_command` is the first call in `Plan::observe` (`tx_launch.rs:341-343`).
   The argv is kept in `Seen.server` and passed unchanged to `host.run`. A plan error converts to `acted: false`.

**Scope items:**

- **Scope 1.**
  - There is one env-prefix helper, `ProcessEnv::prefix` (`holler-adapter-opencode/src/lib.rs:171-187`), used by
    `tui_argv` and `serve_command`.
  - The credential path is stated in the crate docs and the ADR: through the tmux session, never on an argv.
  - The tmux size limit is recorded, and the `{"default_agent": KEY}` rule binds #642 part 3.
- **Scope 2.**
  - The engine's new code is in the child module `tx_launch/server.rs`.
  - The port is checked again just before the server's `run`.
  - `TxOptions.boot_poll` is `BOOT_POLL`, 150 ms.
  - `wait_healthy` has no budget of its own: it takes the run budget's end as its deadline and checks it before every
    ask.
  - `stop_old`'s "until #695" comment and message are gone.
  - The ADR records what the adapter's wait gave up (`boot_try`, the fast exit, the own-process check) and why.
- **Scope 3.** The argv, the refusal and the deletions are as listed. The crate docs and the `server.rs` header are
  rewritten.
- **Scope 4.**
  - The fakes still share no state. `LinkedHost` links them: `run` calls `FakeHost::run`, then `start_server`, then fires
    the hooks.
  - `FakeHarness::serve_command` goes through the fault switch as `HarnessOp::ServeCommand`, records each spec, and
    returns a self-describing argv.
  - `start_server` is outside the fault switch.
  - Setup code calls the scenario method, and the hooks were re-keyed to `after_server_start`.
- **Scope 5.**
  - The launcher is generic over the subject: `Fn(&S, &PaneName, &Argv) -> Result<(), PaneError>`.
  - The suite waits with the engine's `wait_healthy`, every `BOOT_POLL`, within `DEFAULT_BUDGET`.
  - Case 2 was renamed and lost its pid check, and case 16 was added. `DOCUMENTED_CASES` was updated.
  - `PingSession` seeds its session when the server is launched.
  - The real rig runs the argv in its own process group, keeps the pid for its `Guard`, and runs the model guard after
    the wait.
- **Scope 6.** The TERM/grace/KILL tests were not copied. The argv the plan built is the one `run` started:
  `launch.rs:67` checks that `start_server` of the logged argv answers the running server's pid. `stop_owned` stops
  that server (`serve.rs:419`, `:552`). The real-tmux run test takes the `cwd`.
- **Scope 7.** The ADR edits are made in every place the brief names:
  - the port-table rows for `HostPort` and `HarnessPort`;
  - section 2's new "The harness server as amended (#802)", with decisions 1-7 and the health-wait paragraph;
  - facts 1, 2, 6 and 7;
  - section 5's side effect;
  - section 8's "first amendment" phrase;
  - "Launch and relaunch as built", "Close as built" and section 11;
  - the code-table rows and the open-codes list;
  - "Deferred to named stories" and "Decisions taken" 10-13.

  The code docs Scope 7 names are updated (`ports.rs`, `holler-pane/src/lib.rs`, the OpenCode crate docs, the test kit's
  `harness.rs` and `conformance/harness.rs`). README and `docs/` hold no statement of the old placement:
  `docs/research/opencode-pane-spike.md` is the spike's historical record.

**Deviations. Each is documented, none is silent, and none needs rework:**

1. **`with_agent` is a builder on `self`**, called as `ServeSpec::default().with_agent(key)`, where Scope 1 wrote
   `ServeSpec::with_agent(AgentKey)`. A's round-1 W-12(a) set this: the workspace's `with_*` convention, and the only way
   to keep one style once `ServeSpec` gains fields. AC 1 is met.
2. **`run` also refuses a missing directory**, beyond the brief's empty or relative one. This is A's binding W-1: it
   keeps the host adapter's narrowing 5, so no program starts silently in `$HOME`. It extends the brief and contradicts
   nothing.
3. **AC 5's parenthetical is read as A's binding W-2.** The parenthetical says "an adapter configured with a
   secret-looking `Isolated` variable that is not on the adapter's own list". The test reads it as "nothing from the
   adapter's own environment reaches an argv". It does not read it as an allowlist filter over the configured `Isolated`
   pairs.

   I agree with A, and I am not holding #802 on it. An allowlist contradicts the brief's own Scope 1 ("one helper ...
   for both the TUI argv and the server argv", where the TUI has always put every `Isolated` pair on its command line). It
   also contradicts the `ProcessEnv::Isolated` contract ("Each gets exactly these variables"), and the crate has no
   "adapter's own list".

   The security intent ("No credential goes into an argv") holds for production: under `Inherit`, the server's argv
   carries only `env -u OPENCODE_DISABLE_TERMINAL_TITLE`, the binary and its flags. `Isolated` is test-only and documented
   "Never put a secret here". See advisory 7.
4. **`switch.rs` grew from 827 to 833 lines.** Scope 4 asked for a line-neutral edit there, or for the two doubles to
   move. Each double needed the required `verify_serving` delegate. The file is 67 lines under the 900-line gate, and
   A-dup recorded it (W-3).
5. **`stop_old` moved into `tx_launch/server.rs`**, and **`OpenCodeConfig::bin()` was extracted** from `attach.rs`.
   These are structural, recorded by F, and approved by A (W-6) and A-dup. They keep `tx_launch.rs` under the size gate,
   and the server argv and the TUI argv share one UTF-8 check.

**The brief gate's round-2 BLOCK.** It was not applied before the relaunch. Each of its points is met by what was built:

- **B-1.** "No reconcile step" is observed in the CLI output (`occurrences(STEP_PHRASE) == 0`), and `acted` is what
  drives it (`evidence.md`, `holler-cli/src/pane/launch.rs:435-442`).
- **B-2.** The real-tmux test asserts the program's own `pwd -P`, not the session's path.
- **B-3.** AC 9 is split into the stored fixture and a launched record.
- **B-4.** AC 5 is pinned on the literal argv, not on the deleted `start()`.
- **W-4.** This is AC 5's reading, deviation 3 above.

## Quality audit

**Correctness and failure handling.**
- Every new act step is inside `bring_up`, whose errors all go through `roll_back`:
  - the port re-check;
  - the server's `run`;
  - the wait's `timeout`;
  - `verify_serving`;
  - a server gone before the session of record.
- `made.session` is set right after `ensure_session`, so a failure in any of those steps stops the session's processes,
  the server among them.
- Plan refusals do no act: `agent-unsupported` and `port-in-use` are `acted: false`.
- Every refusal fails closed:
  - never-adopt is the plan's check plus the act's check again;
  - OpenCode refuses an agent in both methods;
  - `run` refuses a bad directory before any window and never echoes it.
- The record is still written by one compare-and-swap, so no write is lost.
- `Budget::start` halves a limit the clock cannot hold until it fits, so it cannot overflow. `check()` and the wait's
  deadline are one predicate (W-14).
- A crash still runs no rollback, so its server stays in the session. The ADR states this, and
  `a_crash_mid_launch_leaves_no_record` (`launch.rs:350`) pins it.

**Build guards.**
- No `unwrap`, `expect`, `panic!` or `#[allow]` is added under any `src/` (grep of the added lines).
- The largest touched files are `switch.rs` 833, `tx_launch.rs` 820, `fake_tmux_test.rs` 818 and `hermetic_test.rs`
  813, all under 900.
- `dead_code`, `unwrap_used`, `expect_used` and `panic` are denied workspace-wide, and clippy is clean (T-green, A-dup).

**Protocol.** No protocol, hub production or `holler-body` change. No golden file changed, and `docs/protocol/v2.md` is
untouched, as the brief's Out of scope requires. `agent-unsupported` is an open code: `class_of` classes it as a refusal
with no table change (evidence entry 4).

**Tests.**
- **Cross-process behaviour.**
  - Opt-in real tests cover it: `real_tmux_test` and `real_opencode_test` both passed 9/9 on F's runs.
  - CI covers the rest with the fakes.
  - The one gap is the server argv under a tmux pty end to end (W-11(a)), deferred to #649, which wires the adapters.
- **Fixed sleeps.** No test uses a fixed sleep to wait for an event:
  - the wait tests use hooks, `set_delay` or a deadline fixed by ordering;
  - two negative checks sleep before they assert that nothing spawned (`hermetic_test.rs:633`, and opt-in
    `real_opencode_test.rs:327`). The code has no spawn path left, so these are belt and braces (NIT).
- **RED-first evidence** is in both T-red rounds.

**Documentation.**
- The `CHANGELOG.md` `[Unreleased]` entry is complete (row 12).
- The ADR and the code docs agree with the code. I read decision 5, the health-wait paragraph, fact 2 and the launch
  paragraph against `tx_launch/server.rs`.
- The PR adds no new CLI surface, log event or protocol field.
- One ADR line (`ADR-0021.md:195`) runs past the paragraph's wrap width. This is a NIT.

**Public-repository privacy.**
- A grep of every added line of the diff, handoffs included, finds none of the following: personal hostnames, tailnet or
  Tailscale names, private IP ranges, `/home/` or `/Users/` paths, the operator's name or email, private domains, or
  key-shaped strings.
- The only IP is `127.0.0.1`.
- The test secret `s3cr3t-802-do-not-leak` is a sentinel that the test asserts never reaches an argv.

**Commit and PR hygiene.**
- Every branch commit has a Conventional Commit subject (`chore(#802): ...`) and a `Co-Authored-By: Claude` trailer.
  None carries a session link, which matches the merged history on `main` (`263a2d7`, `64519b5`).
- No PR exists yet. The AI disclosure goes into the PR body after the script opens it (advisory 3).

## Scope check

**Delivered: the brief's scope.**
- The port amendment, the engine, the OpenCode adapter, the host adapter, the test kit and doubles, the conformance
  suite, the ADR and docs, and `CHANGELOG.md`.
- No hub production code, protocol or `holler-body` change.
- One test file outside the blast radius list: `holler-hub/tests/pane_registry_test.rs`, test-only. AC 9 and Scope 2 name
  that crate's fixture, so it is in scope.

**Over-delivery: small and justified.**
- The extra mutant `ServeCommandStarts` and the fake's open code `agent-mismatch`, both test kit.
- The `OpenCodeConfig::bin()` extraction.
- The `Budget` end computed once (A's W-14).
- The deletion of the host adapter's session-directory read, now unused (W-9).

**Under-delivery: none against an acceptance criterion.** The relaunch half of AC 6's op is proven by code, not by a test
(advisory 2).

## Verdict

**PASS.** All twelve acceptance criteria are met, and each has a named test that asserts the behaviour. Every decision of
the issue and the brief is implemented as stated. The five deviations are documented and none is silent. Quality is
acceptable for a contract amendment of this size. Before merge the branch needs to merge `origin/main` (advisory 1).
That is not a defect of this diff.

## Advisory notes

1. **Before merge (O, or the run's agent).** The branch is 3 commits behind `origin/main`, and
   `git merge-tree --write-tree HEAD origin/main` conflicts in three files:
   - **`crates/holler-cli/tests/pane_verbs/launch.rs`.** Keep both `mod serve;` and #808's `mod record_bytes;`.
   - **`crates/holler-cli/tests/pane_verbs/launch/rig.rs`.** Keep `hooks: Hooks` and #808's
     `displaced: Arc<Armed>`, then make any #808 code that names the old hooks type compile.
   - **`docs/adr/ADR-0021.md`.**
     - "Decisions taken": #808 took item 10, so #802's items become 11-14, and section 2's cross-reference stays
       valid.
     - "Deferred": #808's `HerdrPort` bullet and #802's two rewritten bullets are all kept.

   `tx_launch.rs` auto-merges at 831 lines. That is under 900, but near the ~850 guideline. After the merge, re-run the
   verify command, clippy and `lint.sh`. GitHub runs no `pull_request` CI on a conflicting PR.
2. **AC 6, the `pane.relaunch` op.** No test runs a relaunch whose server never answers. The op comes from
   `Budget::start(options.budget, OP_RELAUNCH)` (`tx_launch.rs:217`), which predates #802 and was never tested for
   relaunch, through the shared `budget.expired()`. Follow-up: a test-only relaunch twin of `launch.rs:272`, asserting
   `Timeout("pane.relaunch")` and the rollback.
3. **PR body.** Add the AI disclosure per `CONTRIBUTING.md` with `gh pr edit` once the script opens the PR (`CLAUDE.md`:
   the script's body does not carry it).
4. **Journal.** `decisions.md` has no entry for the brief gate's round-2 BLOCK being passed by the operator's relaunch
   (`designApproved`). O's Chain Summary should record it, in the `Rigor/brief gate: BLOCKED, overridden by ...` form of
   pipeline-conventions section 1.
5. **Follow-ups carried from A, F and T.**
   - W-11(a): the server argv under a tmux pty, with #649.
   - W-13: the merge order with #664, now also with #816 (advisory 1).
   - W-15: the plan's reads and `session_of_record`'s two calls have no budget check between them. This predates #802,
     and I do not hold #802 to it.
   - A-dup's W-4 reading note for #642 part 3: an agent key is always checked on the server.
6. **Test-code cleanup (optional).**
   - A-dup W-1: one `FakeHarness` setup method in place of the `serve_command` + `start_server` pair at five sites.
   - A-dup W-2: `Seen` and `snapshot_into`, and the second `STEP_PHRASE`.
   - The stale module doc at `holler-cli/tests/pane_verbs/doctor/rig.rs:5`, which still says each server starts with
     `serve`.
   - `switch.rs` (833): move `WriterInSelect` and `SeamHarness` to a sibling file before it grows again.
7. **For the operator (no action needed for #802).** AC 5's wording ("a secret-looking `Isolated` variable that is not
   on the adapter's own list") reads more naturally as an allowlist than as what was built.
   - If an allowlist was meant, it would change `ProcessEnv::Isolated`'s contract and the TUI's argv, so it belongs in a
     story of its own, not in a rework of #802.
   - #649's credential path should rely on the tmux session's environment, as the ADR now says, and never on `Isolated`.
     Under `Isolated` the server's pairs are now on its command line, as the TUI's always were.
