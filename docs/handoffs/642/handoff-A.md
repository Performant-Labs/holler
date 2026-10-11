# Handoff-A: #642 part 3 - The OpenCode adapter applies and verifies the pane's agent, and sends prompts  (up-front plan review)

**Date:** 2026-10-11
**Branch:** `issue-642-implementation` (base `origin/main` at `263a2d7`; the branch's two commits touch only `docs/handoffs/`)
**Brief reviewed:** `docs/handoffs/642-brief.md` (as amended in round 1, `b3d29c3`; outside brief gate round 2: PASS with NV-1 on this same precondition)   **Reuse map:** the brief's own decisions D1 to D11 and evidence E1 to E32 (this run has no separate `survey.md`)   **Wireframe:** N/A (no UI surface)
**Verdict:** BLOCK

## Summary

BLOCK: 2 `block` findings and 9 `warn` findings.

1. **The plan cannot be built on this run's base.** Every object it extends exists only in #802's unmerged tree (`3fe111d`). In this worktree, `HarnessPort` still has `serve(name, port) -> u32`, and no file under `crates/` names `ServeSpec`, `serve_command` or `verify_serving`. #802 is open and still running in its own worktree. Without it, T's tests do not compile, and F could reach GREEN only by rebuilding #802 outside this story's blast radius: a second path to a change another run owns.
2. **D4(b)'s `mode == "primary"` is stricter than OpenCode's own rule.** OpenCode refuses a default agent only when it is missing, has `mode == "subagent"`, or is hidden. Its third mode, `"all"`, is the default for any agent defined in config without a `mode`. The plan would refuse such an agent `agent-unusable` with a false message ("is a subagent (mode all)"), and T would pin the wrong predicate in tests.

Measured against `3fe111d`, the tree its evidence describes, the rest of the plan extends the right objects:

- the one `env` prefix builder (`prefix_with`);
- `server.rs` for the agent;
- `Call`, `known`, `json_of` and `poll` for the new requests;
- a new `prompt.rs` that keeps `lib.rs` under the lint limit.

It adds no dependency, and it keeps the adapter on `holler-pane` alone (ADR-0021 section 5). D2's precedence claim also holds: `env` applies assignments in order, so a later pair replaces both an earlier `Isolated` pair and an inherited value (checked live).

The warns name gaps to close in the same amendment:

- a rig wrapper that would silently answer the trait's defaults;
- two readers of `/session/status` that disagree on what "running" means;
- a scope rule for held questions that differs from `holler-body`'s;
- `op` strings and constants with no stated home;
- a path for prompt text into an error message;
- stale "one prefix" docs;
- where the new tests' fixtures live;
- the prompt carve-out that this story makes executable.

## Findings

| # | Severity | Plan element | Drift dimension | Finding | Suggested fix |
|---|---|---|---|---|---|
| 1 | block | "Depends on: #802 merged"; D2, D4, D5, D8; E1-E10, E25-E31; the RED note | dependency direction / coexistence | Every extension point (`ServeSpec`, `serve_command`, `verify_serving`, `refuse_an_agent`, #802's `ProcessEnv`) is absent from this worktree. #802 is OPEN and in flight elsewhere. T's tests would not compile. F could reach GREEN only by rebuilding #802 (40 files under `crates/`, +2,604/-1,029) outside the blast radius. | Do not build on this base. After #802 merges, start a fresh run from a worktree cut from that `origin/main`, with E1-E10, E25-E31 and D8's line counts re-cut against the merged code. |
| 2 | block | D4(b) "`mode == "primary"`"; acceptance 4 and 5 | contract shape (consistency with the adapted API) | OpenCode 1.19.0 has three modes. A config agent defaults to `"all"`. The server's own `default_agent` rule refuses only `mode === "subagent"` and `hidden === true`. The plan would refuse a valid agent with a false "is a subagent (mode all)". | Amend D4(b) to OpenCode's rule: refuse only `mode == "subagent"`. Make an element without a string `mode` `unavailable`. Add acceptance cases (`"all"` is `Ok`) and an E-cut from the binary, as E32 does. |
| 3 | warn | Acceptance 11's third test; the rig's launcher | pattern consistency (port wrappers) | The rig's `Recording` forwards every required `HarnessPort` method but not the default-armed `send_prompt` and `session_activity`. Through `rig.harness`, `session_activity` is always `Idle`, so the check passes without testing anything. | `Recording` forwards both methods. `Rig::serve(i)` gains a spec through one extension (`serve_with(i, &ServeSpec)`, which `serve` calls), not a copy. |
| 4 | warn | D7(b) vs `settled` | pattern consistency / anti-duplication | Two readers of `GET /session/status` in one crate: `settled` counts only `type == "busy"` as running, while D7(b) counts any `type != "idle"` (`retry` and a missing `type` too). | Use one private predicate and one "200 whose JSON is an object" read beside `settled`. The brief either moves `settled` onto them or says in writing why abort keeps `== busy`. |
| 5 | warn | D7(a) "an item whose `sessionID` is `session`" | pattern consistency (with `holler-body`) | `holler-body` counts a question held by the session or any of its child sessions (`session_in_scope`). D7(a) counts an exact match only, so a subagent's question reads `Busy`, and `pane reset` tells the operator to wait for a turn that cannot end. | Decide it in the brief: either count children (`parentID`, already parsed by `top_level_ids`), or keep the exact match and record the divergence in ADR-0021 section 8 and the crate doc. |
| 6 | warn | D4 `op`; D6 and D7 (no `op` named); D8 "lib.rs only wires" | cross-cutting (error ops) / file structure | The crate's rule is that a timeout's `op` is `harness.<method>`, matched to the test kit's `HarnessOp`. `HarnessOp` has `VerifyServing` but no `SendPrompt` or `SessionActivity`, and the test kit is outside the blast radius. | Name `OP_VERIFY_SERVING`, `OP_SEND_PROMPT` and `OP_SESSION_ACTIVITY` in lib.rs's `OP_*` table. Tests pin the two new ones as literals until #684. State where the new `Route`s go. |
| 7 | warn | D5 "a constant in ... `server.rs`" | naming / public surface | `server` is a private module, so a `pub const` there has no path outside the crate. The Herdr precedent it cites is `pub const` in a public module, and its tests compare against the constant. | Add `pub use server::AGENT_UNUSABLE;` at the crate root (as `pub use tui::{...}` does), and have tests compare `AGENT_UNUSABLE.as_str()`. |
| 8 | warn | D6 "no message ever contains the prompt text"; acceptance 7 | cross-cutting (message hygiene) | D6 routes a non-204 answer through `Call::unexpected`, which quotes up to 60 bytes of the reply body. A server that echoes the request in an error body would put the prompt's first bytes into the message. The stub in acceptance 7 never echoes, so the test cannot fail. | Use a body-free form for the `prompt_async` reply: a `Call` method naming the route, port and status only. Alternatively, the stub echoes the posted body in its 500 and HTML replies. |
| 9 | warn | Acceptance 12's doc list | one source of truth (docs) | Five doc sites say the server and the TUI share one `env` prefix. For a spec with an agent, the server's prefix is the TUI's plus one pair. ADR-0021's launch paragraph still describes the agent refusal as `agent-unsupported` with nothing written. | List these sites in acceptance 12. `prefix_with`'s doc carries `Isolated`'s "never a secret" rule. Pin "the TUI's prefix plus exactly one pair" in a test. |
| 10 | warn | New `tests/agent_test.rs` and `tests/prompt_test.rs` using "the suite's" helpers | anti-duplication (test fixtures) | `pane_name()`, `serving()`, `config()`, `quick()`, `BIN`, `SLACK` and the message assertions are private to `hermetic_test.rs`, and another test crate cannot reach them. Copied into both new files, that is three copies of about 100 lines. | Name the home now: one `tests/support/` module included by `#[path]`, as `stub.rs` is, which also shrinks `hermetic_test.rs`. |
| 11 | warn | D6 `send_prompt`, against the choke-point rule | layering (prompt choke point) | `pane reset --first` prompts through `HarnessPort::send_prompt` with no park check, outside #646's gate. ADR-0021 section 8 records this as the operator's carve-out, so it is not drift. After this story, only #649's wiring separates that path from a parked pane's server. | No code change. The section 8 edit says the adapter now sends for real and the gate still does not see this path. The adapter's `send_prompt` stays pure transport. |

### Details

**1. The base does not contain #802 (block).**

I checked on 2026-10-11, in this review, after a `git fetch` (handoff written at 07:23 MDT):

- **`origin/main` is `263a2d7`**, and this branch's merge-base is the same commit. `gh issue view 802` says OPEN, and no PR is merged.
- **This worktree has the pre-#802 port.** `crates/holler-pane/src/ports.rs:186` declares `fn serve(&self, name: &PaneName, port: u16) -> Result<u32, PaneError>`. A grep finds no `ServeSpec`, `serve_command` or `verify_serving` in any `.rs` file under `crates/`. The adapter's `server.rs` is 189 lines and spawns a `Child`. `lib.rs` is 555 lines and `hermetic_test.rs` is 798.
- **#802 is in flight now.** It runs on branch `issue-802-implementation` in worktree `.claude/worktrees/0802-serve-command`, tip `0dc4e8b`. Its last commit (07:00 MDT) amends #802's brief after its own T-green (`3fe111d`), so the code this plan quotes may still change before #802 merges.
- **What would happen.** T's acceptance 1-6 and 9 call `serve_command`, `verify_serving` and `ServeSpec::with_agent`. In this tree that is a compile error, not the assertion failure the brief's RED note requires.
  - F could go GREEN only by rebuilding #802: the `holler-pane` port and the launch engine (`tx_launch/server.rs`), `HostPort::run`'s `cwd`, the host adapter, the test kit's fake and conformance suite, and the CLI rigs. That is `git diff --shortstat 263a2d7...3fe111d -- crates`: 40 files, +2,604/-1,029 (49 files with #802's docs and handoffs).
  - The brief allows comment-only edits in `holler-pane` and "no other crate". Rebuilding #802 here would be an unplanned edit of shared code another run owns, which `parallel-agent-coexistence.md` (merge hygiene, last bullet) calls a drive-by: "stop and ask".
- **The brief already knows.** Its Dependency note says "the precondition ... is not met by this worktree", and the round-2 outside gate raised the same point as NV-1. A PASS at the brief gate does not make the plan buildable here.

**2. D4(b)'s mode rule (block).**

Each fact below comes from the installed `opencode-ai` 1.19.0 binary, the version E21 probed and E32 cut, using E32's method. The cuts are verbatim.

- **Three modes.** The `/agent` element has `mode:o.Literals(["subagent","primary","all"])`.
- **A config agent defaults to `"all"`.** OpenCode builds it as `t=r[n]={name:n,mode:"all",permission:...,options:{},native:!1}`, then applies the config's own value with `t.mode=s.mode??t.mode`. So a config or Markdown agent without `mode` is `"all"`.
- **The server's own `default_agent` rule.** OpenCode refuses only these cases:
  - `if(!t)throw Error(`default agent "${n.default_agent}" not found`)`
  - `if(t.mode==="subagent")throw Error(`default agent "${n.default_agent}" is a subagent`)`
  - `if(t.hidden===!0)throw Error(`default agent "${n.default_agent}" is hidden`)`

  Its fallback takes the first agent with `t.mode!=="subagent"&&t.hidden!==!0`.

D4's refusal messages are these three, transcribed. The one difference is the mode test: `mode == "primary"` instead of `mode != "subagent"`.

- **The consequence.** A project agent defined without `mode`, which is the ordinary way to define one, is refused `agent-unusable` with "agent KEY is a subagent (mode all)". OpenCode would run that same agent as the default.
- **Why the evidence did not show it.** E21 lists only built-in agents, which are never `"all"`.
- **This repository is unaffected.** Its own `opencode.json` declares `orchestrator` with `"mode": "primary"`, so the operator's case passes. A project that omits `mode` does not.

**The fix:**

- Amend D4(b) to "an element with `name == key` whose `mode` is not `"subagent"` and whose `hidden` is not `true`".
- Use "agent KEY is a subagent" only when `mode == "subagent"`. The message then carries no text from the reply.
- An element without a string `mode` is not the JSON the step needs: `unavailable`.
- Add the cut above as an E-section. Acceptance 4 gains `"mode":"all"` → `Ok`, and acceptance 5 keeps `"subagent"` → refused.

If the operator does want "primary only", the brief must say so in writing, give the reason, and change the message for `"all"` ("is not a primary agent"). That is the decision to make before T writes tests.

**3. The rig's `Recording` drops the two default-armed methods.**

- `tests/real_opencode/rig.rs:264-297` (at `3fe111d`) implements `HarnessPort for Recording` by forwarding nine methods to `inner`. It does not forward `send_prompt` or `session_activity`, which have trait defaults (E4).
- `Rig.harness` is a `Recording` (`:326-327`). So `rig.harness.session_activity(..)` answers `Ok(Idle)` without a request, and acceptance 11's "`session_activity` reads `Idle` once the turn has ended" cannot fail.
- `rig.harness.send_prompt(..)` answers `prompt-unsupported`, which F would notice. A fix that forwards only `send_prompt` would leave `session_activity` untested.
- **The fix:** `Recording` forwards both methods, and the brief says so.
- **`Rig::serve(i)` (`:412-430`) hard-codes `ServeSpec::default()`.** Acceptance 11 needs one server with `plan`. Add `serve_with(i, &ServeSpec)` and make `serve(i)` call it, so the port-retry loop is not copied.
- `raw` (`:320-323`) and `Recording::launch_on` (`:210-245`) already do what acceptance 11 needs.

**4. One reading of `/session/status`.**

- **Two predicates.**
  - `settled` (`src/lib.rs:491-499`, abort's confirmation) reads the route as `json_of(&reply).filter(Value::is_object)`, and treats a session as running only when `type == "busy"`.
  - D7(b) counts a session as running when its entry's `type` is anything but `"idle"`, so `retry` and a missing `type` count too (acceptance 8, backed by E32).
- After this story, the crate would answer "is this session running?" two ways on the same route. An abort whose session sits in `retry` reads "settled", while `session_activity` reads `Busy`.
- **The fix:** put one private predicate beside `settled`: the entry exists and its `type` is not `idle`. Read "a 200 whose JSON is an object" through one helper, which both use (`settled` raises `unexpected` on `None`; `session_activity` falls back).
- **The decision for O:** either `settled` uses the predicate too (an abort is "seen" only once the entry is gone or `idle`), which needs a hermetic case and a sentence in the brief; or the brief records why abort keeps `== busy`. Either is fine; leaving it unsaid is how Phase 7 ends up arguing about a near-duplicate.

**5. Whose held question counts.**

- **`holler-body`'s rule.** A question or permission request held by the attached session or any of its child sessions is in scope (`session_in_scope`, `crates/holler-body/src/http_attach_driver/connection.rs:375-377`, used at `:394` and `:432`, #382).
- **D7(a)'s rule.** The plan's reader of the same two global routes matches `sessionID` exactly.
- **Safety holds either way.** A subagent's question leaves the parent's status entry busy, so `session_activity` answers `Busy` and `pane reset` refuses all the same.
- **The message is wrong for the operator.** `check_idle` (`crates/holler-pane/src/tx_switch.rs:313-329`) words `Busy` as "is running a turn; reset it once it ends". A turn blocked on a subagent's question never ends without an answer.
- **It is the common case.** An orchestrator pane, the agent this epic exists for, dispatches subagents. It is also the open question in the issue thread's last comment.
- **The decision for the brief:**
  - (a) Count child sessions. The adapter already reads `parentID` in `top_level_ids` (`src/lib.rs:527-536`); one `GET /session` gives `session`'s children.
  - (b) Keep the exact match, and record in ADR-0021 section 8 and the crate doc that a child's question reads `Busy`.

  Option (a) matches the existing pattern.

**6. `op` strings and constants: where they go.**

- **The rule.** The crate doc (`src/lib.rs:21-24`) says every `timeout`'s `op` is `"harness.<method>"`, "the test kit's `HarnessOp` strings". The `OP_*` table (`:118-125`) says the tests "pin equal to these".
- **The gap.** `HarnessOp` (`crates/holler-pane-testkit/src/harness.rs:47-57` at `3fe111d`) has `VerifyServing` but no `SendPrompt` or `SessionActivity`. The test kit is outside the blast radius, and #684 owns the fake's recording of these methods. The brief names an `op` only for `verify_serving`.
- **The fix:**
  - The brief names `OP_VERIFY_SERVING`, `OP_SEND_PROMPT` and `OP_SESSION_ACTIVITY` (`harness.verify_serving`, `harness.send_prompt`, `harness.session_activity`) in lib.rs's `OP_*` table.
  - The tests pin `HarnessOp::VerifyServing.as_str()`, and the two new strings as literals until #684 adds variants.
  - The comment at `:118-119` says which two have no `HarnessOp` counterpart yet.
- **D8's "lib.rs only wires" should allow these constants.** It should also say where the six new `Route`s go:
  - either lib.rs's route table (`:338-376`), where every `Route` lives today and from which `attach.rs` takes `GET_SESSION` and `SELECT`;
  - or beside their only callers in `server.rs` and `prompt.rs`, as `server.rs` keeps `HEALTH`.

  lib.rs has room (599 lines, plus about 60).

**7. `AGENT_UNUSABLE` needs a public path.**

- **The Herdr precedent.** `GRID_UNREACHABLE` is a `pub const` in the public module `plan` (`crates/holler-adapter-herdr/src/plan.rs:39`, `lib.rs:39 pub mod plan;`), and its tests import it (`tests/plan_splits_test.rs:10`, `:100`). The test kit's rule, at `herdr.rs:50`, is to compare against the constant, never the literal.
- **The gap.** The OpenCode adapter's `server` is private (`mod server;`, `src/lib.rs:113`).
- **The fix:** re-export at the crate root, as `pub use tui::{TmuxConfig, TmuxSocket};` (`:116`) does.
- **Context, no change needed.** The test kit's `FakeHarness::verify_serving` refuses the same mismatch with its own open code, `agent-mismatch` (`harness.rs:40-42`, `:332-344`). The port lets each implementation own its code (E3), so this is not drift. The ADR's tables should name `agent-unusable` as the OpenCode adapter's code, not as the only possible answer.

**8. Prompt text in an error message.**

- **The path.** `Call::unexpected` (`src/lib.rs:430-443`) quotes up to 60 bytes of the reply body (`excerpt`, `:569-582`). Transport errors (`HttpError::Garbled`) carry no body bytes, so the excerpt is the one way the prompt could reach a message.
- **Why the test cannot catch it.** If the server echoes the request in an error reply (a 400 validation error quoting its input, say), the prompt's first bytes reach the `unavailable` message. Acceptance 7's search for a distinctive text passes only because the stub never echoes.
- **The fix:**
  - **Structural (preferred):** add a `Call` method that names the route, the port and the status only, used for the `prompt_async` reply. That extends `Call`; it is not a second error builder.
  - **Otherwise:** make the stub's 500 and HTML replies echo the posted body, so the test can fail.

**9. The docs that would turn false.**

These doc sites say the server and the TUI share one prefix, and each becomes false for a spec with an agent:

- `ProcessEnv`'s doc (`src/lib.rs:151-153`): "both argvs begin with the one `env` prefix";
- `prefix`'s doc (`:172-175`): "every OpenCode argv ..., the server's and the TUI's";
- `server::serve_command`'s doc (`src/server.rs:21-26`);
- `tui_argv`'s doc (`src/tui.rs:165`): "The prefix is the one the server's argv starts with too";
- the crate doc's fact-2 bullet (`src/lib.rs:48-51`).

`ac8_the_server_and_the_tui_share_one_env_prefix` (`tests/hermetic_test.rs:610`) keeps passing because it uses `ServeSpec::default()`, so nothing would flag the stale docs.

**The fix:**

- Add these sites to acceptance 12.
- Pin the new form in `agent_test.rs`: with an agent, the server's argv is the TUI's prefix plus exactly the one pair, up to the binary.
- `prefix_with`'s doc repeats `ProcessEnv::Isolated`'s "never put a secret here: every pair goes on an argv (`ps`, `#{pane_start_command}`)". It is now a second way to put a pair on an argv, and #649 will be looking for a way to pass a provider credential.
- ADR-0021's "Launch and relaunch as built (#644)" (`3fe111d` line 489) says the plan's `agent-unsupported` is "exit 3 with nothing written". After this part, the real adapter refuses an agent from `verify_serving` in the act, and the rollback stops the server.
  - That is consistent with existing practice: the act's second `port-in-use` check is a refusal after acting.
  - The paragraph should say it alongside the code-table rows (`:599`, `:661`) that acceptance 12 already names.

**10. Where the new hermetic tests' fixtures live.**

- **What the plan uses.** Acceptance 1 and 6 use "the suite's `pane_name()`", `serving`, `BIN` and `SLACK`. Those are private items of `tests/hermetic_test.rs` (`:30-160`, `:531-560` at `3fe111d`), and `agent_test.rs` and `prompt_test.rs` are separate test crates that cannot reach them.
- **No single pattern exists.**
  - Machinery is shared by `#[path = "support/..."]`: `stub.rs`, and `fake_tmux.rs`.
  - Small fixtures are copied with different values: `attach_test.rs` has its own `quick()`, `SLACK` (500 ms) and `BIN` (`:37-54`).

  So this is a `warn`, not a `block`.
- **The risk.** Copying `config()`, `serving()`, `isolated()`, `quick()`, `pane_name()` and the message assertions (`unavailable`, `timeout_op`, `assert_one_line_naming`, `assert_within`) into both new files makes three copies of about 100 lines.
- **The fix:** name one `tests/support/` module for the hermetic server fixtures, included by `#[path]` with per-item `#[allow(dead_code)]` as `stub.rs:138` does. It also takes lines out of `hermetic_test.rs` (813), which helps acceptance 9.
- **At Phase 7,** I will treat a verbatim copy of `config()` or `serving()` in two new files as a parallel path.

**11. The prompt choke point.**

- **The path.** `pane reset --first` sends through `HarnessPort::send_prompt` (`crates/holler-pane/src/tx_switch.rs:215-219`). `tx_switch.rs` has no park check, and #646's gate (`pane-parked` and the others) is in `crates/holler-cli/src/say_cmd.rs:215-223`, on the routed verbs.
- **It is decided, not drift.** ADR-0021 section 8 records this as the operator's carve-out (`3fe111d` lines 470-482): "the prompt gate of #646, park state included, does not see it".
- **This story makes it executable.** Today the path is inert, because the adapter's `send_prompt` is the refusing default. Afterwards, only #649's wiring separates `pane reset --first` from a parked pane's server.
- **No code change.** The section 8 edit should say that the adapter now sends for real, that the gate still does not see this path, and that #646 and #649 decide whether it moves. The adapter's `send_prompt` stays pure transport, with no hold, limit or park logic: control-plane behaviour belongs at the choke point, not in an adapter.

### Checked and sound (against `3fe111d`)

- **D2, one prefix builder.** `prefix_with` extends `ProcessEnv::prefix` (`src/lib.rs:171-187`); `prefix()` delegates to it, and `tui_argv` is unchanged.
  - Ordering holds: `env -i A=1 OPENCODE_CONFIG_CONTENT=x OPENCODE_CONFIG_CONTENT='{"default_agent":"plan"}' env` prints the later value.
  - Under `Inherit`, `env -u OPENCODE_DISABLE_TERMINAL_TITLE OPENCODE_CONFIG_CONTENT=...` both removes the title switch and replaces an inherited value.
  - Checked on this machine (uutils `env` 0.8.0); POSIX `env` applies its operands in order.
  - `AgentKey`'s charset (E5) keeps the JSON free of `=`, whitespace and quotes.
- **D4 and the engine.** `verify_serving` runs after the health wait, and a refusal is the run's failure with the server stopped by the rollback (`crates/holler-pane/src/tx_launch/server.rs:19-21`, `:100` at `3fe111d`). That matches D4.
  - Conformance case 16 now takes its "argv carries the key" branch: the pair `OPENCODE_CONFIG_CONTENT={"default_agent":"conformance-agent"}` contains the key (`conformance/harness.rs:459-494`).
  - No conformance case calls `verify_serving` with an agent, so the stub run needs no `/config` or `/agent`.
- **D6 reuses `known` as `abort` does** (`src/lib.rs:314-323`, `:470-487`). Submodules reach the crate root's private `Call`, `known`, `json_of`, `poll` and `string_at` as `server.rs` already does (`use crate::{json_of, OpenCodeConfig}`). That is not a new seam.
- **D7 reads question and permission items as plain JSON** (`sessionID` through `string_at`), not through `holler-body`'s `OcQuestionRequest`. Importing those types would break ADR-0021 section 5's rule (the adapter depends on `holler-pane` alone).
- **D8's sizes leave room under 800 lines.** At `3fe111d`: `lib.rs` 599 (about +60), `server.rs` 76 (about +120), new `prompt.rs` (about 200), `hermetic_test.rs` 813 (changed in place; smaller if finding 10 moves fixtures out), `real_opencode_test.rs` 373 (about +150), `rig.rs` 530 (about +30).
- **The stub already does what the new tests need:** replies per method and path, a frozen mode, HTML pages, and each request's raw line and body (`tests/support/stub.rs:1-14`, `:99-167`).
- **D5's exit class has precedent.** A refusal raised in the act phase after acting, then rolled back, matches the second `port-in-use` check (`tx_launch/server.rs:11-13`).

## Notes for O

There is no O in this automated path. The script stops with `needs-amendment`, and the notes below are for whoever amends the brief.

1. **Do not resume this run** (`resumeFromRunId` replays this BLOCK). Wait for #802 to merge.
   - The branch name is fixed (`issue-642-implementation`), so remove this worktree and branch first.
   - Then cut a fresh worktree from the `origin/main` that contains #802 and start a fresh run.
2. **Re-cut the evidence against the merged #802 code:** E1-E10, E25-E31 and D8's line counts. Delete the "Checked at the brief gate ... not met" paragraph once it no longer applies.
3. **Amend D4(b) per finding 2:** `mode` is not `"subagent"`, using OpenCode's own rule, with an E-cut from the 1.19.0 binary. Add the `"all"` acceptance case. If the operator wants "primary only" instead, state it and the reason in D4, and change the message for `"all"`.
4. **Fold the warns that change the plan into the brief, so T and F see them as requirements:**
   - 3: `Recording` forwards both methods; add `serve_with`.
   - 4: one status predicate; decide `settled`.
   - 5: the child-session decision.
   - 6: `OP_*` names; where the `Route`s go.
   - 7: re-export `AGENT_UNUSABLE`.
   - 8: a body-free failure for `prompt_async`.
   - 9: the doc sites, and the ADR launch paragraph.
   - 10: the fixture home.

   Finding 11 needs only the ADR section 8 sentence.

## Patterns referenced

1. `crates/holler-adapter-opencode/src/lib.rs` at `3fe111d` (whole file): `ProcessEnv::prefix`, the `OP_*` and `Route` tables, `Call::send_by` and `unexpected`, `known` and `session_reply`, `settled` and `poll`, `top_level_ids`, `json_of` and `excerpt`.
2. `crates/holler-adapter-opencode/tests/real_opencode/rig.rs:196-323` and `tests/support/stub.rs` at `3fe111d`: the delegating `HarnessPort` wrapper and the shared test machinery.
3. `crates/holler-pane-testkit/src/harness.rs:40-73` and `:320-345`, and `conformance/harness.rs:23-56` and `:459-494` at `3fe111d`: `HarnessOp`, the fake's own `agent-mismatch`, and case 16.
4. `crates/holler-body/src/http_attach_driver/connection.rs:375-432`; `crates/holler-adapter-herdr/src/plan.rs:39` with its `lib.rs:39`; `crates/holler-pane/src/tx_switch.rs:204-222` and `:310-329`: the child-session scope rule, an adapter-owned open code, and the consumer of `send_prompt` and `session_activity`.
5. `docs/adr/ADR-0021.md` at `3fe111d` (sections 2 and 8, lines 484-492, 599, 661, 808-828), with `.aftersight/releases/claude/workflow/parallel-agent-coexistence.md` ("Merge / coordination hygiene").
