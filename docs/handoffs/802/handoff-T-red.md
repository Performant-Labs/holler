# Handoff-T-red: Phase 4 - #802 `HarnessPort::serve_command` takes a `ServeSpec`; the harness server runs in the pane's tmux session

**Date:** 2026-10-11
**Branch:** issue-802-implementation (worktree `.claude/worktrees/0802-serve-command`, head `0421ec1`)
**Brief / wireframe reviewed:** `docs/handoffs/802-brief.md`, `docs/handoffs/802/handoff-A.md` (W-1 to W-13), no wireframe (no UI)

**VERDICT: RED is valid.** It is compile-level for the signature amendment, as the brief's RED
note says, and it is also proven assertion-level (see "RED confirmation", part 2).

## A precondition

Confirmed: A returned PASS on the plan (Phase 3), with thirteen warns. The suite applies the
binding readings W-1 (a missing directory is refused too) and W-2 (no allowlist: nothing from the
adapter's own environment reaches an argv), and W-3, W-5, W-7 and W-8 as the tests' design.

## Files touched (test paths only, `crates/*/tests/**`)

New: `crates/holler-cli/tests/pane_verbs/launch/serve.rs` (declared `mod serve;` in `launch.rs`).
Changed: the 23 files listed under "Staged" below. No `src/`, no `Cargo.toml`, no doc, no ADR.
Every touched file stays under 900 lines (largest: `switch.rs` 833, `hermetic_test.rs` 812,
`fake_tmux_test.rs` 803, `launch/rig.rs` 793).

## The contract the tests pin (F implements exactly these names and shapes)

**`holler-pane`**
- `ServeSpec` in `ports.rs`, re-exported at the root (`holler_pane::ServeSpec`): `#[non_exhaustive]`,
  `Debug + Clone + Default + PartialEq + Eq`, `pub agent: Option<AgentKey>`, and the builder
  `pub fn with_agent(mut self, agent: AgentKey) -> Self` (A's W-12; the tests call
  `ServeSpec::default().with_agent(k)`).
- `holler_pane::ports::AGENT_UNSUPPORTED` (`agent-unsupported`, class `Refusal`, exit 3).
- `HarnessPort::serve_command(&self, &PaneName, u16, &ServeSpec) -> Result<Argv, PaneError>` and
  the required `HarnessPort::verify_serving(&self, u16, &ServeSpec) -> Result<(), PaneError>`;
  `serve` is gone.
- `HostPort::run(&self, &PaneName, cwd: &str, &Argv)`.
- `TxOptions.boot_poll: Duration`, default 150 ms.
- `pub fn holler_pane::tx_launch::wait_healthy(harness: &dyn HarnessPort, port: u16, poll: Duration, deadline: Instant) -> Result<bool, PaneError>`
  (A's W-4 one wait; the real OpenCode rig's launcher calls it). `Ok(true)`, `Ok(false)` at the
  deadline, or the port's own error.
- The `compile_fail,E0639` doctests (AC 1) live in source, so they are F's to place. The pinned text,
  on `ServeSpec`'s doc:

  ````text
  /// ```compile_fail,E0639
  /// let spec = holler_pane::ServeSpec { agent: None };
  /// ```
  /// ```compile_fail,E0639
  /// let spec = holler_pane::ServeSpec { agent: None, ..Default::default() };
  /// ```
  ````

**`holler-pane-testkit`** (`src/`, F's)
- `HarnessOp::ServeCommand` (`"harness.serve_command"`) replaces `Serve`; new `HarnessOp::VerifyServing`
  (`"harness.verify_serving"`). Both are port methods: through the fault switch, logged.
- `FakeHarness::serve_command`: pure. It records the spec (`FakeHarness::serve_specs() -> Vec<ServeSpec>`,
  call order; a failed call records nothing). It returns an argv that names the pane, the port and,
  when set, the agent (some element contains the key). The tests do not pin the format.
- The scenario method `FakeHarness::start_server(&self, &Argv) -> Option<Result<u32, PaneError>>`:
  `None` for an argv the fake's `serve_command` did not build. Otherwise today's `World::serve`
  rules: idempotent for the same pane; `unavailable` "port N is in use by the server of P" for
  another pane's running server; a `timeout` on a frozen one. It is not logged.
- `ServerView.agent: Option<AgentKey>`: the agent the argv named.
- `FakeHarness::verify_serving`: it reaches the port (killed or unserved is `unavailable`, frozen is
  `timeout`). `Ok` when `spec.agent` equals the running server's agent; otherwise a
  `PaneError::Refused` (the code is the fake's choice).
- `FakeHost::runs() -> Vec<(PaneName, String, Argv)>`: the `cwd` that `run` was given is the
  middle element. `run` never changes the session's own `cwd`.
- `run_harness_conformance(fresh, launch)`, where `launch: Fn(&S, &PaneName, &Argv) -> Result<(), PaneError>`.
  The suite polls `health` after it (bounded, the shared wait). Case ids are in
  `harness_conformance_test.rs`'s `DOCUMENTED_CASES`:
  - case 2 is renamed `launch-then-healthy`. It now also checks that the port is **not** healthy
    after `serve_command` and before the launcher (that is purity);
  - case 16 is new, `serve-command-carries-the-agent`. With `ServeSpec::default().with_agent(k)`,
    the answer is either `agent-unsupported` or an argv in which some element contains `k`;
  - case 2's pid check is gone.

**`holler-adapter-opencode`**
- `OpenCodeHarness::serve_command` with no agent returns `<env prefix> <opencode_bin> serve --port P --hostname 127.0.0.1 <serve_args>`.
  The env prefix is the **same one** `tui::tui_argv` starts with:
  - under `Inherit`, `env -u OPENCODE_DISABLE_TERMINAL_TITLE`;
  - under `Isolated`, `env -i K=V ...`.

  It starts nothing and sends nothing.
- With an agent, `serve_command` **and** `verify_serving` answer `agent-unsupported` (W-3, fail
  closed). `verify_serving` of a spec with no agent is `Ok(())` and sends nothing.
- `OpenCodeConfig.workdir` and `Timeouts.boot_try` are gone. The tests build both structs without
  them.

**`holler-adapter-host`**
- `run(name, cwd, argv)` refuses a `cwd` that is empty, relative or missing as `usage`, before any
  `new-window`, and without echoing it (W-1). It passes `-c <escape_cwd(cwd)>`, so `#` is doubled
  and a trailing `;` is escaped.
- **W-9 decided: option (a).** `run` keeps one existence read of the session before the argv
  check. So a successful run is still three tmux calls (the read, `new-window`, the tag), and
  `run_checks_the_session_before_the_argv` stands. The tests do not pin the read's format string.

**Engine** (pinned through the rig, `launch/serve.rs` and `launch.rs`)
- `serve_command` is the plan's **first** call. It comes before the probe, Herdr's
  version/snapshot and the port check, with `ServeSpec { agent: spec.opencode_agent }`, on both
  launch and relaunch.
- **In the act:**
  1. `ensure_session`.
  2. `run(cwd = spec.host.cwd)` for the spec's command.
  3. The `port-in-use` re-check.
  4. `run(cwd = spec.host.cwd)` for exactly the planned argv.
  5. The health wait (at least one poll).
  6. `verify_serving`.
  7. `create_session` / the session of record, then `attach_tui`.
- The budget is checked before `verify_serving` too (I5: before every live step).
- A record written by launch has `harness.pid == None`, with no `pid` key.
- The relaunch-survivor message no longer says `#695`.

## Tests authored

| Test (file) | AC / warn | Behaviour pinned | Tier |
|---|---|---|---|
| `serve_spec_default_has_no_agent_and_with_agent_holds_the_key` (holler-pane `ports_test.rs`) | AC 1 | default `agent == None`; `with_agent` holds/replaces the key; root re-export | unit (type) |
| `agent_unsupported_is_a_refusal_with_exit_3` (`ports_test.rs`) | AC 11 | code string, class `Refusal`, exit 3 | unit |
| `ports_compile_in_test_impl` (`ports_test.rs`, extended) | AC 1 | the amended trait signatures compile in a test double | unit (compile) |
| `serve_command_starts_nothing_and_records_its_spec` (testkit `fake_harness_test.rs`) | AC 2, 10 | the fake's `serve_command` is pure, records specs, fails through the switch | unit (fake) |
| `the_argv_carries_the_agent_and_the_server_keeps_it` (same) | AC 3, 10 | argv names the key; the started server keeps it | unit (fake) |
| `start_server_ignores_an_argv_it_did_not_build` (same) | Scope 4 | the scenario method answers `None` for any other argv, unlogged | unit (fake) |
| `verify_serving_answers_from_the_running_servers_agent` (same) | AC 7, W-3 | `Ok` on match, refusal otherwise, `unavailable` with no server | unit (fake) |
| `launch-then-healthy`, `serve-command-carries-the-agent` + mutants `DropsAgent`, `ServeCommandStarts`; `PingSession` now seeds at launch (`harness_conformance_test.rs`) | AC 10 | the suite runs through a launcher; the new case holds and both new mutants are caught | conformance |
| `run_records_each_argv_verbatim` (`fake_host_test.rs`, extended) | AC 4 | `runs()` keeps the `cwd` given; the session keeps its own | unit (fake) |
| `ac8_serve_command_is_the_server_argv_behind_the_env_prefix` (`hermetic_test.rs`) | AC 5 | exact argv under `Inherit` and `Isolated` | unit (adapter) |
| `ac8_the_server_and_the_tui_share_one_env_prefix` (same) | AC 5, Scope 3 | one helper: the server's prefix equals `tui_argv`'s | unit |
| `ac8_serve_command_starts_nothing` (same) | AC 5 | no process (stand-in binary's marker), no request to the port | unit |
| `ac8_a_spec_with_an_agent_is_refused_and_one_without_verifies` (same) | AC 5, W-3 | `agent-unsupported` from both methods; no-agent verify `Ok` | unit |
| `ac8_no_credential_of_the_adapters_environment_reaches_an_argv` + `credential_probe_child` (same) | AC 5, W-2 | a secret set only in a re-executed child's own env appears in no argv element under either `ProcessEnv` | process (re-exec) |
| `run_passes_the_argv_exactly_in_three_tmux_calls` (`fake_tmux_test.rs`, rewritten) | AC 4 | `new-window -c <cwd>`, three calls | unit (fake tmux) |
| `run_refuses_a_cwd_that_is_empty_relative_or_missing` (same) | AC 4, W-1 | `usage`, no `new-window`, no echo | unit |
| `run_escapes_the_cwd_for_tmux` (same) | AC 4 | `#` doubled, a trailing `;` escaped, `#(...)` inert | unit |
| `run_works_in_the_given_cwd_not_the_sessions`, `run_refuses_a_missing_or_relative_directory` (`real_tmux_test.rs`, opt-in) | AC 4, W-1 | real tmux: `pwd` is the given dir; missing/relative/empty is `usage` and no window | real (opt-in) |
| `an_agent_refusal_in_the_plan_is_exit_3_and_writes_nothing` (`pane_verbs/launch/serve.rs`) | AC 2 | exit 3 both formats, no reconcile step, harness calls `[ServeCommand]` only, no Herdr/host/probe/write, P generation 1 | integration (rig) |
| `an_agent_refusal_in_the_plan_leaves_a_relaunched_pane_running` (same) | AC 2 | relaunch refusal stops nothing (no host call), server and record unchanged | integration |
| `serve_command_runs_in_the_plan_before_any_other_call` (same) | AC 2 | at the `ServeCommand` hook: no Herdr `version`, no host call, no probe | integration |
| `the_act_runs_the_planned_argv_after_the_command_then_waits_and_verifies` (same) | AC 3, AC 4 | cross-port order through the hooks (W-8c); `runs()` = [command, planned argv], both in the spec's cwd; the agent reaches the spec and the server | integration |
| `relaunch_passes_the_in_flight_agent_to_serve_command` (same) | AC 3 | a differing `--agent` reaches `serve_command`; without the flag the stored key does | integration |
| `a_relaunch_runs_the_server_in_the_spec_directory_not_the_sessions` (same) | AC 4, W-7 | session remade for `/srv/elsewhere`; the server runs in the record's cwd; the session keeps its own | integration |
| `boot_poll_defaults_to_150_ms`, `the_health_wait_retries_until_the_server_answers` (same) | AC 6 | default; a server frozen at the first poll and thawed for the next is launched | unit / integration |
| `a_verify_refusal_rolls_back_and_stops_the_server` (same) | AC 7 | rollback: `stop_owned`, server killed, pane closed, no session, no record, the step once | integration |
| `the_port_is_checked_again_just_before_the_server_runs` (same) | W-5 | a server that appears after the plan is refused `port-in-use` from the act, after the command and before the server's run; never adopted, never stopped | integration |
| `relaunch_stops_the_old_server_and_starts_one_on_the_same_port` (same) | AC 8 | host calls `[StopOwned, EnsureSession, Run]`; a new pid on the same port | integration |
| `a_launched_record_has_no_pid_key` (same) | AC 9 | no `harness.pid` on the wire or in the record | integration |
| `launch_onto_a_frozen_server_times_out_before_any_session`, `launch_times_out_when_the_server_never_gets_healthy` (`launch.rs`, rewritten AC 4a/4b) | AC 6 | through `rig.engine` with a 700 ms budget: `timeout` `pane.launch`, rollback stops the server, several polls | integration (engine) |
| `a_crash_mid_launch_leaves_no_record` (`launch.rs`, extended) | W-10(d) | a crash runs no rollback: the started server stays running | integration |
| `relaunch_fails_when_the_old_server_survives` (`relaunch.rs`, reworded) | AC 8 | still `unavailable` "still answers"; no new server argv run; no `#695` in the message | integration |
| `a_record_with_a_harness_pid_still_loads_and_one_without_stores_no_key` (holler-hub `pane_registry_test.rs`) | AC 9 | the stored fixture with `pid: 4242` loads; a record written without a pid has no key | integration (store) |
| `unwired_ports_answer_not_implemented` (`pane_verbs/main.rs`, extended) | Scope 4 | `Unwired`'s amended methods answer `not-implemented` | unit |

Rig changes (`launch/rig.rs`, A's W-8):
- `LinkedHost::run` calls `FakeHost::run`, then `start_server`, and returns `FakeHost::run`'s
  answer. A refused start is not `run`'s error.
- `Rig::after_server_start` re-keys the hooks that were armed on `Serve`.
- `HookedHarness` hooks `ServeCommand` and `VerifyServing`.
- `seed_live` runs the server argv through the host and records no pid.
- `assert_matches` checks that `pid == None`, and that the server's agent equals the record's
  `opencode_agent`.

Existing assertions updated for the new plan:
- `guards.rs`: grid-occupied, port-in-use and probe-failed now see `[ServeCommand]` or
  `[ServeCommand, Health]`.
- `relaunch.rs`: pane-in-other-profile.
- AC 1's op list, through `steps()`, which folds a run of health polls into one.
- AC 11b's `runs()`.
- AC 24's budget: 1.3 s, with 500 ms per harness call.

Mechanical migration (the signature change forces it; F does not edit tests):
- `run(..)` gains the cwd argument in every test double and call site: the testkit tests,
  `fake_tmux`/`real_tmux`, `close/rig.rs`, `ports_test.rs`.
- `serve_command` and `verify_serving` are added to `WriterInSelect`, `SeamHarness`, `HealthGate`,
  `TestHarness` and `Recording`.
- `doctor/rig.rs` starts its servers through a local `start_server` helper.
- `doctor.rs` now names `ServeCommand` among the harness calls that doctor must never make.
- `attach_test.rs` loses `workdir` and `boot_try`.

The adapter's spawn tests are deleted. They pinned behaviour this story deletes: AC 8's
`serve_refuses_a_port...`, the missing binary, the frozen port, the program that exits, and
`serve_kills_its_process_group...`. `ac19_serve_refuses_a_port_that_already_serves` (real) became
`ac19_serve_command_starts_nothing_and_refuses_an_agent`.

**The real OpenCode rig** (`real_opencode/rig.rs`) now has a launcher:
- `Recording::launch_on` spawns the argv in its own process group, in the scratch project dir.
- It records the pid for the Guard.
- It waits with `wait_healthy`, then runs the model guard.
- `Recording::launch` is the conformance suite's launcher.

## RED confirmation

**1. Compile-level, the brief's valid RED for a signature amendment.**

```
cargo build --workspace --tests --keep-going
```

Every library builds. Exactly the 11 test targets that call the new API fail. Every error
names a missing or changed API, or a removed field:

```text
error: could not compile `holler-pane` (test "ports_test")
error: could not compile `holler-pane-testkit` (test "fake_harness_test" | "fake_host_test" | "harness_conformance_test" | "host_conformance_test")
error: could not compile `holler-cli` (test "pane_verbs")
error: could not compile `holler-adapter-opencode` (test "hermetic_test" | "attach_test" | "real_opencode_test")
error: could not compile `holler-adapter-host` (test "fake_tmux_test" | "real_tmux_test")
 42 E0061 this method takes 2 arguments but 3 arguments were supplied        (HostPort::run's cwd)
 24 E0599 no variant `ServeCommand` for enum `HarnessOp`;  11 … `VerifyServing`
 11 E0432 unresolved import `holler_pane::ServeSpec`;  2 … `holler_pane::ports::AGENT_UNSUPPORTED`
 11+3+3 E0599 no method `verify_serving` (FakeHarness / OpenCodeHarness / &FakeHarness)
 10+5 E0599 no method `start_server`;  6 no method `serve_specs`
 7+7+6+3 E0599 no method `serve_command` (FakeHarness / OpenCodeHarness / &FakeHarness / Recording)
 7+7 E0407 `serve_command` / `verify_serving` is not a member of trait `HarnessPort`
 7 E0046 not all trait items implemented, missing: `serve`
 7 E0061 this function takes 1 argument but 2 arguments were supplied   (run_harness_conformance's launcher)
 4+2 E0609/E0560 no field `agent` on `ServerView`
 3+2+2 E0277/E0609 FakeHost::runs() is still `(PaneName, Argv)`
 3+3 E0063 missing field `workdir` / `boot_try`                          (the deleted fields)
 1 E0609 no field `boot_poll` on `TxOptions`;  1 E0432 unresolved import `tx_launch::wait_healthy`
 3 E0050 method `run` has 4 parameters but the trait's has 3
```

The hub target compiles and its new test passes now. That test is a characterization pin for AC 9's
"still reads" half, which has to hold both before and after the change. It is not offered as RED.

**2. Assertion-level: the behaviour tests fail for the right reason once the API compiles.**

**The shim.** In a scratch worktree, outside this branch and deleted afterwards, T wrote a
throwaway shim:
- the new signatures;
- a minimal fake and adapter;
- an engine that follows the brief.

The results against that shim:
- Every test target compiles.
- `cargo clippy --workspace --all-targets -D warnings` is clean on the test code.
- `lint.sh` passes on the test files.
- Every authored test passes: `pane_verbs` 258/258, `fake_harness_test` 23, `fake_host_test` 12,
  `harness_conformance_test` 15, `host_conformance_test` 12, `hermetic_test` 31, `attach_test` 23,
  `fake_tmux_test` 29, `pane_registry_test` 15, and `ports_test` 8.

So the suite is satisfiable, and the API above is complete.

**The naive engine.** The shim's engine was then swapped for a naive port of today's engine onto
the new signatures:
- `serve_command` called in the act;
- no re-check;
- one health check, answering `unavailable`;
- no `verify_serving`;
- the session's directory.

Against it, 21 `pane_verbs` cases fail on assertions, for example:

```text
launch_records_what_the_fakes_show: left [Health, ServeCommand, Health, CreateSession, AttachTui, ShownSession]
                                    right [ServeCommand, Health, VerifyServing, CreateSession, AttachTui, ShownSession]
launch_times_out_when_the_server_never_gets_healthy: left Err(Unavailable "not healthy on 48100") right Err(Timeout "pane.launch")
an_agent_refusal_in_the_plan_is_exit_3_and_writes_nothing: no step: "...; to reconcile, run holler pane doctor --profile 'demo' ..." left 1 right 0
a_relaunch_runs_the_server_in_the_spec_directory_not_the_sessions: left "/srv/session-dir" right "/srv/demo"
relaunch_passes_the_in_flight_agent_to_serve_command: left Some(ServeSpec { agent: None }) right Some(ServeSpec { agent: Some("feature-implementor") })
the_port_is_checked_again_just_before_the_server_runs: the launch succeeded (code 0), expected port-in-use (3)
the_health_wait_retries_until_the_server_answers: code 1 "unavailable: not healthy on 48100", expected success
a_verify_refusal_rolls_back_and_stops_the_server: code 0 (verify never called), expected 3
launch_with_agent_stores_the_key_in_the_record (assert_matches): server agent None, record Some("feature-implementor")
```

The shim also failed two tests before it was fixed, which shows both are live rather than vacuous:
- `the_budget_bounds_a_slow_launch` failed while the shim's engine skipped the budget check before
  `verify_serving`.
- `serve_command_runs_in_the_plan_before_any_other_call` caught a shim that called `serve_command`
  after the probe.

## Ready for F

RED is valid. F may implement against these tests. Notes for F:
- **F does not edit tests (`crates/*/tests/**`).** If a name or shape above is wrong for a
  reason the brief supports, say so in handoff-F, and T repairs the test.
- **Line budget.** `tx_launch.rs` is 787 lines. Put the server step and `wait_healthy` in the child
  module (W-6).
- **Dead code is denied.** The deleted items of Scope 3 must go entirely: `server::serve`, `start`,
  `wait_until_up`, `OP_SERVE`, `BOOT_INTERVAL`, `KILL_BOUND`, `reap_when_it_exits`,
  `exec::kill_group` and its runner. In the host adapter, `BAD_SESSION_DIR` and `is_session_dir`
  are probably unused after the change.
- **Environment hazard.** Only this worktree's own `target/` was used, and it is a real directory,
  not a shared symlink. The shim built in a separate scratch target.

## Staged

The test files, staged by explicit path. Nothing is committed, and O commits.

- `crates/holler-adapter-host/tests/{fake_tmux_test,real_tmux_test}.rs`
- `crates/holler-adapter-opencode/tests/{attach_test,hermetic_test,real_opencode_test}.rs`
- `crates/holler-adapter-opencode/tests/real_opencode/rig.rs`
- `crates/holler-cli/tests/pane_verbs/{main,launch,relaunch,switch,doctor}.rs`
- `crates/holler-cli/tests/pane_verbs/launch/{rig,serve,guards,profiles}.rs`
- `crates/holler-cli/tests/pane_verbs/{close/rig,doctor/rig,doctor/surface}.rs`
- `crates/holler-hub/tests/pane_registry_test.rs`
- `crates/holler-pane-testkit/tests/{fake_harness_test,fake_host_test,harness_conformance_test,host_conformance_test}.rs`
- `crates/holler-pane/tests/ports_test.rs`
- `docs/handoffs/802/handoff-T-red.md`
- `docs/handoffs/802/decisions.md`
