# Handoff-A: Phase 3 - #802 `HarnessPort::serve_command` and `ServeSpec`; the server runs in the pane's tmux session  (up-front plan review)

**Date:** 2026-10-11
**Branch:** issue-802-implementation (worktree `.claude/worktrees/0802-serve-command`, head `c03544a`, on `92fbc6e`)
**Brief reviewed:** `docs/handoffs/802-brief.md` (sha256 `e895a4d9f9e72424...`)   **Reuse map:** no separate file. This automated run has no `survey.md`, so the brief's Scope names the objects to extend, and the findings below complete the map.   **Wireframe:** N/A (no UI surface)
**Verdict:** PASS

## Summary

The verdict is PASS, with no blocks and thirteen warns. The plan extends existing objects throughout and builds no parallel path.

- **The ports.** Both ports are amended in place, under the amend-first rule:
  - `HarnessPort`: `serve` becomes the pure `serve_command`, and a required `verify_serving` is added.
  - `HostPort`: `run` gains `cwd`.

  `ServeSpec` sits beside `HerdrSpec` in `ports.rs`, and `AGENT_UNSUPPORTED` beside `PROMPT_UNSUPPORTED`.
- **Who starts and stops processes.** `HostPort` (`run` and `stop_owned`) becomes the one place that starts and stops a pane's processes. The adapter's own spawn, boot poll, reaper and group kill are deleted, not kept alongside. So `harness.pid` stops being a second source of truth, and #695 closes through the existing TERM, grace, KILL path.
- **The engine.** It composes the two ports, as ADR-0021 section 5 asks of a pure engine, and every refusal stays in the plan.
- **The test kit.** The plan extends `FakeHarness`, `LinkedHost`, `HookedHarness`, the one conformance table and runner, and `Mutant`. The two fakes still share no state.

Two warns are binding readings that T and F must apply. In both places the brief leaves a gap that would otherwise produce drift:

- **W-1.** `run`'s new `cwd` must keep the host adapter's guarantee that no program starts silently in `$HOME`. A missing directory is refused, not only an empty or relative one.
- **W-2.** AC 5's credential test means that nothing from the adapter's own environment reaches an argv. It is not an allowlist filter, which would break `ProcessEnv::Isolated`'s contract.

After those, two warns matter most:

- **W-3.** OpenCode's `verify_serving` fails closed for a spec with an agent.
- **W-4.** The engine, the conformance suite and the real rig share one health-wait function.

## Findings

| # | Severity | Plan element | Drift dimension | Finding | Suggested fix |
|---|---|---|---|---|---|
| W-1 | warn (binding on F and T) | Decision 6; Scope 1 (`HostPort::run` gains `cwd`); AC 4 | cross-cutting (a documented guard of the host adapter) | The brief refuses only an empty or relative `cwd`. Today `run` also refuses a missing directory (narrowing 5, host `lib.rs:76-77`, pinned by `fake_tmux_test.rs:466` and `real_tmux_test.rs:429`). It does so because tmux starts the program in `$HOME`, with exit 0, when its start directory is gone (`tmux.rs:267-271`). `ensure_session` of an existing session checks no directory (`lib.rs:314-333`). So after this change, a launch or relaunch into a reused session whose project directory was removed would silently start the spec's command and the server in `$HOME`. | `run` refuses a `cwd` that is not an absolute path to an existing directory, before `new-window`, as `usage`. That is what `ensure_session` answers for the same input (`lib.rs:316-330`), and the message does not echo the path. Carry the two tests' three cases (relative, missing, empty) over to the `cwd` argument. Escape with the existing `tmux::escape_cwd` (`tmux.rs:167-172`), not a new escape. This extends the brief's rule and contradicts none of it. |
| W-2 | warn (binding reading for T, F and S) | AC 5, last sentence; Scope 1, "No credential goes into an argv"; Scope 3's one env-prefix helper | pattern consistency (the `ProcessEnv` contract) | Read as an allowlist ("not on the adapter's own list"), the test needs a filter the brief never defines, and no such list exists in the crate. A filter would break `ProcessEnv::Isolated`'s contract, "Each gets exactly these variables" (`holler-adapter-opencode/src/lib.rs:148-155`). Through the one shared helper it would also change the TUI's argv (`tui.rs:161-194`). | The argv holds the adapter's own literals and the configured `Isolated` pairs, and nothing from the adapter's own process environment. F adds no filter. An allowlist would be a brief amendment, not F's call. Detail below. |
| W-3 | warn | Decision 5; Scope 3 ("`verify_serving` returns `Ok(())` for a spec without an agent") | cross-cutting (fail closed) | The brief does not say what OpenCode's `verify_serving` answers for `Some(agent)`. "A spec with an agent never reaches it" holds only for the engine: the method is public, and #642 part 3 and later doctor work may call it. Answering `Ok(())` there would claim that a spec is honoured when this adapter cannot check it. The fake has the same gap. Unless its answer comes from what its server was started with, its `verify_serving` can only answer `Ok` or an injected fault, so it cannot model the check that #642 part 3 adds. | **OpenCode:** refuse with `agent-unsupported` for `Some(_)`, the same refusal as `serve_command`, and pin that in the adapter's test. #642 part 3 replaces it. **Trait doc:** `Ok` only when the running server honours every field of the spec. Otherwise the answer is a refusal with an open code the implementation owns (#642 part 3, decision 3), which the engine's rollback undoes. **`FakeHarness`:** keep the agent that the scenario method read from the argv on `ServerView` as `agent: Option<AgentKey>` (`harness.rs:84-91`), and answer `verify_serving` from it. |
| W-4 | warn (Phase 7 candidate) | Scope 2 (the wait moves into the engine); Scope 5 ("a bounded `health` poll the same way the engine waits", and the real rig's model guard "after the poll") | duplication | Three places need the same wait: the engine; the conformance suite, after its launcher; and the real rig's launcher, before its model guard (today the guard runs inside `Recording::serve`, `real_opencode/rig.rs:191-200`). Three loops would be near-copies. | Write one `pub fn` over `&dyn HarnessPort`. Given a port, a poll interval and a deadline, it answers `Ok(true)`, `Ok(false)` at the deadline, or the port's own error. Put it in the new child module and re-export it from `tx_launch`. The engine maps `false` to its budget's `timeout` (`pane.launch` / `pane.relaunch`). The suite and the real launcher pass their own deadline. A sleep-poll has precedent in this crate (`probe.rs:297-306`). ADR section 5 names the probe runner as the engines' one direct side effect (`:221-224`), so the ADR edit says the launch engine now also sleeps between health polls. |
| W-5 | warn | Scope 2 (the plan); decision 7; AC 3 | pattern consistency (the plan derives each port's spec once) | `serve_command` (in the plan) and `verify_serving` (in the act) must see the same `ServeSpec`. `#[non_exhaustive]` has no effect inside `holler-pane`, so how the engine builds the spec decides whether a field added later (decision 3) fails to compile at the one production site or silently takes its default. Separately, the brief says only "just before `host.run`" for the `port-in-use` re-check, and the act calls `run` twice. | Build one `ServeSpec` in `Plan::new`, beside `target` (`tx_launch.rs:262-284`), with an exhaustive literal: `ServeSpec { agent: spec.opencode_agent.clone() }`, with no `..Default::default()` and no `with_agent`. Pass `&plan.<field>` to both calls. Keep the argv in `Seen`, built by `Plan::observe` before the probe (`:289-309`), and update `Seen`'s doc. Put the re-check right before the server's `run`, after the spec's command, so only one call separates the check from the start. The re-check is a refusal inside the act (rollback, `acted: true`, the reconcile step). Say so in the module doc, and add it to AC 3's order. |
| W-6 | warn | Scope 2 ("new code goes in a sibling module"); blast radius | size and structure (the lint fails at 900) | `tx_launch.rs` is 787 lines. The plan grows `bring_up` (the re-check, the server's run, the wait, the verify), `TxOptions`, `Plan` and the module doc, yet says only that "the loop" moves out. | Move the whole server step into one function in the child module: the re-check, `host.run` of the argv, the wait and `verify_serving`. The module is `tx_launch/<name>.rs`, declared `mod <name>;` in `tx_launch.rs` as `reconcile.rs:45` declares `observe`, and it can use the private `Budget`. `bring_up` then gets shorter. In handoff-F, report the final line counts of `tx_launch.rs`, `switch.rs`, `fake_tmux_test.rs` (753), `hermetic_test.rs` (798) and `launch/rig.rs` (718). `switch.rs` is 827 lines, and both of its doubles gain `verify_serving`, so a line-neutral edit is impossible; the brief's move to a sibling file is the way. |
| W-7 | warn | AC 4 ("a relaunch after a `cwd` change (a session made for another directory)") | consistency with E0 | E0 refuses a relaunch whose spec changes the directory (`tx_launch.rs:377-385`, `usage`), and the brief keeps that rule. | In this case the record and the spec agree on directory B, and the tmux session was made for directory A. This happens because `close` leaves the session and the next launch of that name reuses it (host `lib.rs:83-87`). On the fakes: call `ensure_session(name, A)` in setup, put the record and the spec at B, then assert that both `run` calls got B. A test that changes the spec's directory gets E0's `usage`, which is not this AC. |
| W-8 | warn (Phase 7 candidates) | Scope 4 (`LinkedHost`, the scenario method, the re-keyed hooks); AC 2-4, 7 and 8 | duplication; how faithful the fakes are | **(a)** AC 4 needs `run`'s `cwd`, and the fake keeps one argv log (`FakeHost::runs()`, `host.rs:124-127`). **(b)** Suppose the scenario method's refusals (another pane's server on the port, a frozen server; `World::serve`, `harness.rs:374-396`) come back as `run`'s error. Then the fake fails where the real host succeeds: tmux starts the process anyway, and only the health wait sees a server that never answers. **(c)** AC 3's order spans two ports, but `Calls` keeps one log per port (`launch/rig.rs:181-190`). **(d)** The fakes' module docs (`host.rs:11-14`, `harness.rs:12-15`) say a harness pid is never in `FakeHost::ps` and that the wrapper only kills. Neither stays true. | **(a)** Extend `runs()` with the `cwd`. Add no second log and no recording wrapper. **(b)** `LinkedHost::run` returns `FakeHost::run`'s answer; the scenario method only starts the server or does nothing. **(c)** Prove cross-port order with the rig's existing hooks, reading the fakes' state at the hooked call. For example, at the first `Health` after the run, `runs()` already holds the server argv. Do not build a merged log. **(d)** Reword both docs: a wrapper's `run` starts the fake server through the scenario method, and its `stop_owned` kills it. |
| W-9 | warn | Decision 6; Scope 7's list of code docs | docs as contract (host adapter) | Scope 7 does not list the host adapter's crate docs, and this change makes them false: "What `run` starts" (`lib.rs:20-29`, which reads the session's directory and passes `-c '#{session_path}'`), the errors-table row (`:56`), narrowing 5 (`:76-77`), "A session keeps the directory it was made in ... `run` works there" (`:88-90`), and `tmux.rs`'s `SESSION_PATH` and `new_window` docs (`:29-31`, `:75-78`). The read of the session's directory (`session_dir`, `is_session_dir`, `BAD_SESSION_DIR`) loses its purpose. | Rewrite them in the ADR's words. Then either keep one existence read before `new-window`, so that "`run` checks the session before the argv" (`lib.rs:60`) stays true, or drop it, let `new-window`'s `can't find session` answer `pane-not-found`, and reword `:60`. Delete whatever goes unused: clippy denies dead code. |
| W-10 | warn | Scope 7 (ADR-0021) | ADRs | Six edits are needed beyond Scope 7's list. | See the detail below. |
| W-11 | warn (follow-up for O) | Scope 5 (the real launcher); decision 4 | verification of a claim about another component; one precondition | **(a)** Nothing in this story starts the server the way production will: under a tmux pty, through `new-window ... -- env -- <argv>`. The real launcher spawns the argv directly, as `start()` did, and the spike did the same with `</dev/null` (`scripts/spikes/opencode-lib.sh:132-145`). **(b)** Fact 1's targeting (`=<name>:` is the session's current window) now matters for every pane, because each session holds the server's window. `run` creates that window detached. But a person may switch the client to it, for example to read the server's output, which now lands there. Then `attach_tui`'s `respawn-pane -k` lands on the server. | **(a)** Within the brief, the real rig's launcher can run the argv in its private tmux server and keep the pid for the Guard. That is a raw `new-window -d -P -F '#{pane_pid}' ... -- env -- <argv>`, and the rig already drives that server. Otherwise O files an opt-in end-to-end check in `holler-cli`, the only crate that depends on both adapters, with #649. **(b)** State the precondition in fact 1 and in the adapter docs. A later change can address the TUI by its window. |
| W-12 | warn (low) | Scope 1 (`ServeSpec::with_agent(AgentKey)`, and where the env-prefix helper lives) | naming; placement | **(a)** Across the workspace, `with_*` is a builder on `self` (host `lib.rs:188-217`, hub `live.rs:457,470`, proto `error.rs:264`, herdr `adapter.rs:89`). The only associated ones take a clock (`lockout.rs:351`, `roster.rs:306`). Outside its crate, a `#[non_exhaustive]` struct can be built only from `Default` plus builders. So an associated `with_agent(key)` forces a second style once decision 3's next field arrives. **(b)** The helper renders `ProcessEnv`, not tmux, and `tui.rs` limits itself to tmux calls (`tui.rs:1-8`). | **(a)** `pub fn with_agent(mut self, agent: AgentKey) -> Self`, called as `ServeSpec::default().with_agent(key)`, which meets AC 1. This is F's call; record it in handoff-F. **(b)** Put the helper beside `ProcessEnv` in `lib.rs`, private to the crate, called by `tui::tui_argv` and by the server argv builder. |
| W-13 | warn (low; coexistence) | Blast radius | parallel-agent coexistence | #664 seeds servers through a new `crates/holler-cli/tests/profile_verbs/rig.rs`, written against today's `serve` (the #664 brief, O3). If #664 merges first, this change must edit that file to compile, and the file is outside this brief's blast radius. | That edit is a forced consequence of the spec (the rename to the scenario method), not a drive-by. Record it in handoff-F if it happens. Both stories also edit ADR-0021, in different sections. |

### W-2. AC 5's credential test: the binding reading

- **What the argv may hold.**
  - The adapter's own literals: `env`, the `ProcessEnv` flags, the binary, `serve --port P --hostname 127.0.0.1` and `serve_args`.
  - Under `Isolated`, exactly the configured pairs. These are already documented as never a secret (`lib.rs:152-155`). That doc must now also say they go on the server's command line. Today `start()` sets them with `envs()`, which `ps` does not show (`server.rs:107-109`).
- **What the argv must never hold:** anything from the adapter's own process environment. That is the leak AC 5 guards against. For example, an implementation might rebuild the old "the server inherits the adapter's environment" by copying `std::env::vars()` into `K=V` pairs.
- **T's tests:**
  - Pin the exact argv for both `ProcessEnv` variants (the first half of AC 5).
  - Set a secret-looking variable only in the adapter process's environment, absent from the configured `Isolated` list, and show that its value appears in no argv element. There are two ways to do this:
    - Set it in a re-executed child, as `fake_tmux_test.rs:357-389` does. Do not use `std::env::set_var`, which is unsafe and races other tests.
    - Or scan the source in the style of `no_broad_kill_in_source` (`fake_tmux_test.rs:694`), checking that the adapter's `src/` never reads the process environment.
- **If S reads the parenthetical as an allowlist,** the question goes to the operator, not to F or S. An allowlist changes `ProcessEnv`'s contract and the TUI's argv.

### W-10. ADR-0021 edits beyond Scope 7's list

- **(a) The `HostPort` amendment.** `HostPort` is amended too: `run` gains `cwd`. Record that in section 2 beside the `HarnessPort` amendment (`:111-115`), and show the new methods in both rows of the port table (`:107-108`).
- **(b) The operator's decisions.** Record decisions 1 to 4 the way commit `6d19f91` recorded items 8 and 9:
  - mark them "Decided (operator, 2026-10-11)" in the section;
  - add numbered items under "Decisions taken" (`:746-771`), starting at item 10.

  Mark decisions 5 to 7 as the brief does: decided in review round 1, under the operator's rules.
- **(c) The #695 Deferred bullet.** "Stopping the harness server that `serve` started: #695" (`:742`) is settled by this change. Move it to the decided form, as `:733-738` do.
- **(d) A rollback is not a crash.** In "Launch and relaunch as built" (`:440-441`), keep the two apart.
  - A rollback now stops the server.
  - A crash runs no rollback. The server stays, tagged, in the pane's tmux session, and the next launch of that name is refused with `port-in-use`. Because there is no record, neither `close` nor `relaunch` can stop it, and no doctor pass sees it.

  The brief's "a crashed launch's server keeping its port, which a rollback now stops" merges the two cases.
- **(e) Why `agent-unsupported` is a refusal.** It needs a one-line reason, as `prompt-unsupported` has (`:405-407`). Add that until #642 part 3 the OpenCode adapter answers it for every agent.
- **(f) The limits that move with the server.**
  - `stop_owned` stops the server's process group. It does not stop the shell commands that OpenCode starts in sessions of their own (`opencode-lib.sh:260-261`; host `lib.rs:91-93`). The deleted `kill_group` did not stop them either.
  - How a provider credential reaches the server is #649's question (Scope 1). `HostPort` has no environment parameter, so #649 may need a `HostPort` amendment. Say so, rather than implying that no port change follows.

## Notes for O

No amendment is needed: the verdict is PASS.

- **Before they start:** T and F read W-1 to W-8. W-1 and W-2 are binding.
- **Phase 7 checks these first** for a near-copy or a parallel path:
  - the health wait (W-4);
  - `FakeHost::runs()` and `LinkedHost` / `HookedHarness` (W-8);
  - `FakeHarness`'s scenario method against `World::serve`;
  - the one conformance table, its runner and `Mutant`;
  - `tmux::escape_cwd`;
  - the one env-prefix helper.

  Any of the following is a rejection: a second wait loop, a second argv log or a recording wrapper, a second engine rig, a second conformance runner or table, a new escape for `-c`, or a second env-prefix builder.
- **Phase 7 also re-checks:**
  - that the missing-directory refusal exists (W-1);
  - that OpenCode's `verify_serving` answers `agent-unsupported` for an agent (W-3).
- **For O to decide or file:**
  - W-11: an end-to-end check of the server under tmux, and the TUI targeting precondition;
  - W-10 (f): the path by which #649 delivers credentials;
  - W-13: the merge order with #664.

## Patterns referenced

- `crates/holler-adapter-host/src/{lib.rs,tmux.rs}`, and `tests/{fake_tmux_test.rs,real_tmux_test.rs}`: `run`, the narrowings, `escape_cwd`, and the precedents for an environment test in a child process and for a source scan.
- `crates/holler-pane/src/tx_launch.rs` (`Plan`, `Seen`, `bring_up`, E0, `Budget`), and `reconcile.rs` with `reconcile/observe.rs` (the child-module pattern).
- `crates/holler-adapter-opencode/src/{lib.rs,server.rs,tui.rs}`: `ProcessEnv`, `start()` and `tui_argv`.
- `crates/holler-pane-testkit/src/{harness.rs,host.rs,conformance/harness.rs}` and `crates/holler-cli/tests/pane_verbs/launch/rig.rs`: the fakes, the one table, `LinkedHost` and the hooks.
- `docs/adr/ADR-0021.md`: sections 2, 5, 8, 9 and 12, "Deferred to named stories" and "Decisions taken"; commit `6d19f91`.
