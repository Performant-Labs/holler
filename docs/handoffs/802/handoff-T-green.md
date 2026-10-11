# Handoff-T-green: Phase 7 (Workflow script; the role doc's Phase 6) - #802 `HarnessPort::serve_command` takes a `ServeSpec`; the harness server runs in the pane's tmux session

**Date:** 2026-10-11
**Branch:** issue-802-implementation (worktree `.claude/worktrees/0802-serve-command`, head `7ede9c7`)
**Issue:** #802 (closes #695)
**Handoff-F reviewed:** `docs/handoffs/802/handoff-F.md`
**Handoff-T-red:** `docs/handoffs/802/handoff-T-red.md`

**VERDICT: GREEN, no blocking issue.** No UI surface, so U is N/A and the run is ready for S.

## GREEN confirmation

F changed no test file (`git diff --name-only 26148ba 7ede9c7 | grep /tests/` is empty).

```text
$ cargo test --workspace -- --skip roster_stays_accurate_under_concurrent_body_load
148 test binaries: passed=2070 failed=0 ignored=25, exit 0      # matches F's count exactly
```

Every target named in handoff-T-red passes on the final code, including `pane_verbs` (258),
`fake_harness_test`, `harness_conformance_test` (the new case 16 and both new mutants caught),
`fake_host_test`, `host_conformance_test`, `hermetic_test` (31), `attach_test`, `fake_tmux_test`
(29), `ports_test`, `pane_registry_test`, and the `holler-pane` doctests: 5, including the two new
`compile_fail` doctests on `ServeSpec`, at `ports.rs` lines 113 and 116.

### The tests pin behaviour: mutation spot checks

Each mutation below removed one behaviour from F's production code. I ran the targeted test
binary against it, then restored the file with `git checkout`. The tree is clean apart from the one
test edit described below.

| # | Mutation (production) | Caught by |
|---|---|---|
| M1 | `server::start` drops the act's `port-in-use` re-check | `launch::serve::the_port_is_checked_again_just_before_the_server_runs` |
| M2 | `server::start` skips `verify_serving` | `a_verify_refusal_rolls_back_and_stops_the_server`, `the_act_runs_the_planned_argv_after_the_command_then_waits_and_verifies`, `launch_records_what_the_fakes_show` |
| M3 | `wait_healthy` asks once, no retry | `launch::serve::the_health_wait_retries_until_the_server_answers` |
| M4 | `TmuxHost::run` skips `check_cwd` | `fake_tmux_test::run_refuses_a_cwd_that_is_empty_relative_or_missing` |
| M5 | `OpenCodeHarness::serve_command` stops refusing an agent | `hermetic_test::ac8_a_spec_with_an_agent_is_refused_and_one_without_verifies` |
| M6 | the plan calls `serve_command` with `ServeSpec::default()` instead of the in-flight spec | 5 tests, including `relaunch_passes_the_in_flight_agent_to_serve_command` and `relaunch_with_agent_replaces_the_stored_key` |

## Test repaired in this phase

The change is in `crates/holler-adapter-opencode/tests/hermetic_test.rs`,
`ac8_a_spec_with_an_agent_is_refused_and_one_without_verifies`. I added
`assert!(stub.lines().is_empty(), ...)` after the no-agent `verify_serving`. The test's doc says
that call "sends nothing", but before this edit nothing asserted it. F flagged the gap in the
handoff as a note, not a defect. The test passes, rustfmt is clean, and clippy on the crate is
clean. The file is now 813 lines, under the 900-line lint limit. This edit was not committed.

## Tier 1 results

| Check | Command | Expected | Actual | Result |
|---|---|---|---|---|
| Lint | `bash scripts/lint.sh` | exit 0 | exit 0; warnings are size notices only (`fake_tmux_test.rs` 803 among them), none fail | PASS |
| Changelog | `bash scripts/changelog-check.sh` | ok | `changelog-check: ok` | PASS |
| Clippy | `cargo clippy --workspace --all-targets -- -D warnings` | exit 0 | exit 0 | PASS |
| Workspace tests | `cargo test --workspace -- --skip roster_stays_accurate_under_concurrent_body_load` | 0 failed | 2070 / 0 / 25 ignored | PASS |
| Docs CLI | `cargo test -p holler-cli --test docs_cli_test` | pass | 3 passed | PASS |
| Wire canary | `cargo test -p holler-cli --test wire_selftest` | pass | 3 passed | PASS |
| Unused deps | `cargo machete` | none | none found | PASS |
| Hook tests | `bash scripts/test-hooks.sh` | exit 0 | exit 0 | PASS |
| Doctests | `cargo test -p holler-pane --doc` | pass | 5 passed (2 new `compile_fail`) | PASS |

There is no server to start and no HTTP API to smoke-test: this story is a library and port
change, and #649 wires it into the CLI.

**Cross-check against F's report.** My results match F's: the workspace count, clippy, lint,
changelog, machete and docs_cli. I did not re-run F's opt-in real runs (`real_tmux_test --ignored`
9/9, and `HOLLER_TEST_OPENCODE=1 real_opencode_test` 9/9). They need private tmux and opencode
servers and are not CI steps. F's results for them stand as reported.

## Tier 2 results

| Check | Method | Result |
|---|---|---|
| Coverage per AC | AC 1 to 12 mapped below, each to a passing test | PASS |
| Test quality | Each test names a behaviour and fails alone for the right reason (shown in RED and by M1 to M6). Each sits at the cheapest sufficient tier: engine order runs through the rig's fakes, and the argv pin and the tmux `-c` checks are unit tests. Nothing is duplicated: the TERM/grace/KILL proof is not copied, per Scope 6, and the adapter's deleted spawn tests were removed with the behaviour. | PASS |
| Type safety | Clippy with `-D warnings`. `ServeSpec` is `#[non_exhaustive]`, and the engine builds it as an exhaustive literal (W-5). | PASS |
| Error paths | `agent-unsupported` in the plan (exit 3, nothing written), `port-in-use` in the plan and in the act, the health-wait `timeout` with the verb's op, the `verify_serving` refusal with rollback, and `usage` for a bad `cwd` are each tested. | PASS |
| Data integrity | A stored record with `pid: 4242` still loads, and a launched record has no `pid` key (`pane_registry_test`, `a_launched_record_has_no_pid_key`). The record is still written by one compare-and-swap. | PASS |
| API contract | The port signatures match brief Scope 1 and the pinned T-red contract. `AGENT_UNSUPPORTED` is `agent-unsupported`, class Refusal, exit 3. | PASS |
| Security | No secret from the adapter's environment reaches an argv (the re-executed child test, W-2). The `cwd` refusal does not echo the path. `#` and `;` in `cwd` are escaped for tmux. | PASS |
| Timing | The health wait polls until a deadline; there is no fixed sleep in the engine. The tests assert order and outcome, not durations. | PASS |
| Protocol and goldens | No protocol, hub or `holler-body` change, so no golden or `v2.md` drift. | N/A |
| Migration | No schema change: `HarnessInfo.pid` keeps its serde. | PASS |
| Scope 3 deletions | The removed items no longer appear in `holler-adapter-opencode`. `kill_group` in `holler-pane/src/probe.rs` is the prober's own, pre-existing function. | PASS |
| Playwright / visual | No such surface in this repo. | N/A |
| Evidence appendix | F's `evidence.md` covers the unchanged-code facts the tests rely on: rollback `stop_owned`, `acted` and the reconcile step, open-code class, `pid` serde, the host's TERM/grace/KILL, the `argv[0]` rule, and fault logging. I added nothing. | PASS |

## Acceptance criteria status

| AC | Status | Backed by |
|---|---|---|
| 1 `ServeSpec` default, `with_agent`, E0639 | PASS | `serve_spec_default_has_no_agent_and_with_agent_holds_the_key`, the 2 `compile_fail` doctests |
| 2 Plan order; `agent-unsupported` writes nothing | PASS | `an_agent_refusal_in_the_plan_is_exit_3_and_writes_nothing`, `..._leaves_a_relaunched_pane_running`, `serve_command_runs_in_the_plan_before_any_other_call` |
| 3 Agent reaches `serve_command`; act order | PASS | `the_act_runs_the_planned_argv_after_the_command_then_waits_and_verifies`, `relaunch_passes_the_in_flight_agent_to_serve_command` |
| 4 Directory: `run` gets the spec's `cwd`; tmux `-c`, refusal, escaping | PASS | `a_relaunch_runs_the_server_in_the_spec_directory_not_the_sessions`, `run_passes_the_argv_exactly_in_three_tmux_calls`, `run_refuses_a_cwd_that_is_empty_relative_or_missing`, `run_escapes_the_cwd_for_tmux` |
| 5 OpenCode argv pin, starts nothing, agent refused, no credential | PASS | `ac8_serve_command_is_the_server_argv_behind_the_env_prefix`, `ac8_the_server_and_the_tui_share_one_env_prefix`, `ac8_serve_command_starts_nothing`, `ac8_a_spec_with_an_agent_...`, `ac8_no_credential_of_the_adapters_environment_reaches_an_argv` |
| 6 Health wait: `timeout`, rollback; `boot_poll` 150 ms | PASS | `launch_times_out_when_the_server_never_gets_healthy`, `launch_onto_a_frozen_server_times_out_before_any_session`, `boot_poll_defaults_to_150_ms`, `the_health_wait_retries_until_the_server_answers` |
| 7 Verify refusal rolls back and stops the server; no-agent verify `Ok` | PASS | `a_verify_refusal_rolls_back_and_stops_the_server`, the conformance cases, `ac8_a_spec_with_an_agent_...` |
| 8 Relaunch stops the old server; a survivor fails loudly | PASS | `relaunch_stops_the_old_server_and_starts_one_on_the_same_port`, `relaunch_fails_when_the_old_server_survives` |
| 9 Old `panes.json` reads; no `pid` key | PASS | `a_record_with_a_harness_pid_still_loads_and_one_without_stores_no_key`, `a_launched_record_has_no_pid_key` |
| 10 Conformance passes; new case; new mutant caught | PASS | `harness_conformance_test` (the fake). The real OpenCode run is opt-in and passed 9/9 on F's run. |
| 11 `AGENT_UNSUPPORTED` code, exit 3, listed | PASS | `agent_unsupported_is_a_refusal_with_exit_3`; ADR rows are S's to audit against Scope 7 |
| 12 ADR/doc edits, CHANGELOG entry, deletions | PASS (mechanical part) | Changelog check ok; the `[Unreleased]` entry links #802 and #695; clippy confirms the deletions are complete. The ADR wording is S's Tier 3. |

## Blocking issues

None.

## Advisory notes

- **W-11(a), carried from F, for O and S.** No test starts the server under a tmux pty through
  `new-window ... -- env -- <argv>`. That end-to-end check belongs with #649.
- Under `ProcessEnv::Isolated`, the configured pairs now appear on the server's command line,
  visible in `ps`. Before this change they were set through `envs()`. This is by design (A's
  W-2), and the crate doc states it. The operator should keep credentials out of `Isolated`, which
  #649's credential path must respect.
- `fake_tmux_test.rs` (803), `hermetic_test.rs` (813), `switch.rs` (833) and `launch/rig.rs` (793)
  are close to the 900-line limit. The next story to touch them should split them first.
- The mutation runs left mutant builds in this worktree's `target/`. The next `cargo` invocation
  rebuilds from the restored sources. No source file other than the test edit above differs from
  `7ede9c7`.
