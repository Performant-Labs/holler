# Handoff-F: Phase 5 - #647 part 2: `unattached` and `agent-cannot-dispatch`

**Date:** 2026-10-12 (11:10 MDT)
**Branch:** `issue-647-implementation` (on top of T's RED `a3a9e2b`)
**Issue:** #647 (brief `docs/handoffs/647-part2-brief.md`; A's PASS `handoff-A.md`; T's RED `handoff-T-red.md`)

## What was done

Production files only. F wrote no test and edited none.

`holler-pane`:
- `crates/holler-pane/src/ports/agent_view.rs` (new): `AgentView` and `TurnAgent`, both `#[non_exhaustive]`. `AgentView` has an empty `Default`, `with_turns` (keeps the order given) and `with_dispatchers` (sorts and drops duplicates, the one place that rule lives). `TurnAgent` has `TurnAgent::new`.
- `crates/holler-pane/src/ports.rs`:
  - `mod agent_view` and its re-export;
  - the required `HostPort::attached_clients(&PaneName) -> Result<u32, PaneError>` and `HarnessPort::agent_view(&HarnessEndpoint, &str, usize) -> Result<AgentView, PaneError>`, each documented;
  - the module doc's sixth bullet now links both methods and `AgentView`, as the brief allowed once the methods exist.
- `crates/holler-pane/src/ports/router.rs`: `HarnessRouter::agent_view` goes through `by`. The routing table's row of endpoint-keyed methods names it.
- `crates/holler-pane/src/lib.rs`: re-exports `AgentView` and `TurnAgent`. The module doc names the amendment.
- `crates/holler-pane/src/findings.rs`:
  - `Unattached`, then `AgentCannotDispatch`, appended to the enum, `ALL` and `code()`;
  - `remedy_for(pane, fix, agent)` is the one table, and `remedy` delegates with `None`;
  - `Finding::with_agent`, applied after `with_fix` (A's W-2);
  - `relaunch_with_agent` builds `holler pane relaunch PANE --agent=KEY`;
  - the module doc's remedy rule now admits a parsed `AgentKey` in the joined form (W-2), and the `session`/`ports` field docs name the new finding.
- `crates/holler-pane/src/reconcile.rs`: module doc only. Step 1 names `host.attached_clients` and `harness.agent_view`. The single-process paragraph names `host.attached_clients` and lists `agent-cannot-dispatch` among the rules it skips.
- `crates/holler-pane/src/reconcile/observe.rs`:
  - the module doc's bound reads "at most nine port calls", in D13's order;
  - `check_tmux` asks `attached_clients` only once `ps` answered (`check_attached`): `0` is `unattached`, an error is `observe-failed` naming `host.attached_clients`;
  - `check_pane` calls `agent::check` after `check_session_of_record`, borrowing `&server`.
- `crates/holler-pane/src/reconcile/observe/agent.rs` (new, A's W-8):
  - rules 1 to 5 of D6, and `RECENT_TURNS = 3`;
  - the finding with the session of record and the pane's port, its remedy set through `with_agent` when there is exactly one dispatcher;
  - the message, with W-3's profile clause;
  - a failed view is `observe-failed` naming `harness.agent_view`.
- `crates/holler-pane/src/reconcile/observe/process.rs`: module doc. The Observe bullet names `host.attached_clients`, and says the chain never asks `agent_view`.

Test kit (`holler-pane-testkit`):
- `src/host.rs`:
  - `HostOp::AttachedClients` (`host.attached_clients`) and a per-session client count;
  - the scenario methods `attach_client` and `detach_clients`, which bypass the faults and the log and answer `pane-not-found` for a missing session;
  - the port method `attached_clients`, after the faults.
- `src/harness.rs`:
  - `HarnessOp::AgentView` (`harness.agent_view`);
  - `seed_turns(port, &str, Vec<TurnAgent>)`, keyed by the port's data directory and the session;
  - `seed_dispatchers(port, Vec<AgentKey>)`, keyed by port;
  - `agent_view` in `abort`'s check order: faults, `at.port()`, `World::reach`, `World::known`, then the newest `max_turns` seeded turns and the seeded dispatchers.
- `src/claude.rs`: `ClaudeOp::AgentView`. `agent_view` checks the faults, then `at.process()?`, then answers the empty view.
- `src/harnesses.rs`: one forward.

Adapters:
- `crates/holler-adapter-host/src/tmux.rs`: `ATTACHED_FORMAT` (`#{session_attached}`), `attached_listing` (the `list-panes -s -t =NAME:` listing `ps` uses) and the strict `parse_attached`. The `Refusal` docs name the method.
- `crates/holler-adapter-host/src/lib.rs`:
  - `TmuxHost::attached_clients`, which fails as `ps` fails;
  - `OP_ATTACHED_CLIENTS` and `BAD_ATTACHED_LIST`, which names `tmux list-panes` and quotes nothing;
  - a module-doc paragraph and an `attached_clients` column in the errors table.
- `crates/holler-adapter-opencode/src/agents.rs` (new): `agent_view`, the strict shape checks, `can_dispatch` (the permission-rule reading), and the dispatchers.
- `crates/holler-adapter-opencode/src/lib.rs`: `mod agents;`, `OP_AGENT_VIEW`, the one-line method, and a module-doc sentence.
- `crates/holler-adapter-opencode/src/prompt.rs` (not in the Blast radius; A's W-1 asked for the edit): `messages`, the crate's one reader of `GET /session/:id/message`. It quotes no body (`Call::unexpected_status`). `from_messages` now calls it.
- `crates/holler-adapter-opencode/src/server.rs`:
  - `agent_list`, the one reader of `GET /agent`;
  - `as_default`, the one statement of OpenCode's default-agent rule;
  - `usable` rebuilt on both, with its refusal messages unchanged.
- `crates/holler-adapter-claude/src/lib.rs`: `agent_view` calls `at.process()?` and answers the empty view. Nothing is read. Module doc bullet.

CLI wiring:
- `crates/holler-cli/src/pane/wiring/bind.rs`: `BindingHost::attached_clients`, `Wired<dyn HostPort>::attached_clients` and `Wired<dyn HarnessPort>::agent_view`, each a one-line forward. The module doc names the new methods and says no wrapper has a default to fall back on (W8).
- `crates/holler-cli/src/pane/wiring.rs`: `Unwired` answers `not-implemented` from both.

Docs:
- `docs/pane-control.md`:
  - section 5's two rows, whose last cells are pinned by acceptance 17;
  - "eight name none";
  - an `unattached` paragraph with its terminal limit, and an `agent-cannot-dispatch` paragraph with the profile rule and the read;
  - the Claude Code paragraph;
  - a section 12 bullet for the verb no story owns.
- `docs/adr/ADR-0021.md` section 12, three clauses: the process/port distinction (A's W-4), the exact `task`-rule wording (see Deviations), and the bound of the message read (A's W-7).
- `CHANGELOG.md`: one bullet at the top of `### Enhancements`, linking #633 and #647.

## Design decisions

1. **The default-agent rule is shared by a split, not by making `usable` `pub(crate)`** (D15; A's W-1 offered both). `usable` answers for one key found by name, with errors as its result. Used as a filter, that would mean matching `agent-unusable` against `unavailable` once per listed name. So the rule moved into `server::as_default(&Value) -> Option<Result<(), &str>>`, and `usable` and `agents.rs` both call it. `usable`'s messages ("agent KEY is a subagent", "agent KEY is hidden") are byte-identical. The opt-in `verify_serving_refuses_a_subagent_a_hidden_agent_and_an_unknown_key` passes on a real 1.19.0.
2. **Two shared readers, each returning `(Reply, Vec<Value>)`** (W-1). Returning the reply lets a step that finds an element of the wrong shape name the route and the status. The message-list reader quotes no body (`unexpected_status`), so `session_activity`'s fallback stops quoting too, as A noted. No test pinned the old quote: `grep -rn "a list of messages" crates/*/tests` finds nothing. The agent-list reader keeps `verify_serving`'s `unexpected` (60 bytes) wording.
3. **`agent_view` makes three reads in this order: `known` (`GET /session/:id`), the messages, the agents.** The first is the existence check `abort` makes, so an unknown session is `session-not-found` by the crate's one rule. The messages are read even for `max_turns == 0`, so a malformed list is still reported. Doctor always asks for 3.
4. **The reply's shape is checked strictly.** Each of these is `unavailable`, never a silently skipped rule: a message with no `info` object and string `info.role`; an agent with no string `name` or `mode` (`as_default` returns `None`). A rule list the adapter does not know (an object, a rule missing a string field, an unknown action) is only that agent's `can_dispatch: None`, as the ADR says.
5. **`can_dispatch` scans the rules from the newest.** Rules of any other permission are skipped. The first rule found whose permission is `task` or `*` with pattern `*` decides. If it denies, the agent can still dispatch when a newer `task`/`*` rule for another pattern allows or asks. With no such rule the agent can dispatch. `ask` counts as not-deny: a person can allow it, so the agent does not "deny every subagent". All 14 rows of T's table pass. See Deviations for the `*` and `ask` cases.
6. **`RECENT_TURNS` lives in `reconcile/observe/agent.rs`, beside its rule.** D6 said "one constant in `observe.rs`", written before A's W-8 moved the rule to the child module. It is still one constant, in the `observe` module tree.
7. **The fake's turns are keyed by data directory and session; its dispatchers by port.** A session's turns live in its data directory, as its id does (OpenCode's servers can share one). A server's agents come from its own project. T's tests read turns only on the port they seeded, so either keying passes. The data directory is the faithful one.
8. **`parse_attached` needs at least one line, and every line must be the same decimal count.** With `-s`, tmux prints the session's one count on each pane's line, so an empty listing or two counts that disagree are not an answer. `decimal` (unchanged) refuses empty, signed, fractional and word values.
9. **`TurnAgent` is `#[non_exhaustive]` too.** T allowed it. A later field, such as a turn's time, joins without breaking a caller.
10. **The messages.**
    - `unattached`: "no tmux client is attached to the tmux session NAME, so no Herdr pane shows it; no Holler verb attaches one". It carries D11's two substrings.
    - `agent-cannot-dispatch` names the turn count, the session of record (quoted) and the agents the turns ran as (quoted). Then, either the orchestrator (quoted), or "N agents that can (up to three quoted, "and K more"): relaunch the pane with one of them as its agent (--agent=KEY)". For a profile member it adds "...relaunch it with --profile P too, or the profile keeps the old agent and the next pass reports profile-drift" (W-3).

## Reuse / extend-vs-new

The brief's Reuse map (D4, D14, D15) is followed, as A amended it:
- **Extended:** `HostPort` and `HarnessPort` gain methods, with no new port. `HarnessRouter` keeps its single `by` rule. `FindingKind` and the one remedy table gain kinds, with `remedy` kept as a delegate. `check_tmux` and the server chain of `check_pane` gain the reads. The kit's `FakeHost`, `FakeHarness`, `FakeClaude` and `FakeHarnesses` gain the scenario methods and ops. The wrappers gain one-line forwards. The OpenCode adapter's `known`, `Call::unexpected_status` and `string_at` are reused, never copied.
- **Refactor-to-extend, specified by D4 and A's W-1:** `from_messages`'s read becomes `prompt::messages`. `verify_serving`'s `GET /agent` read becomes `server::agent_list`, and `usable`'s rule becomes `server::as_default`.
- **New objects, each placed where the brief and A said:**
  - `ports/agent_view.rs` (Scope 2, A's W-5);
  - `reconcile/observe/agent.rs` (A's W-8; the `observe/process.rs` shape);
  - `holler-adapter-opencode/src/agents.rs` (D15).

  `can_dispatch` and the `native` test are new code with nothing on `main` to share, as D15 states.

## Architecture notes for A

- **Layers.** Ports (`holler-pane`) and reconcile (`holler-pane`), the test kit, three adapters, and CLI wiring.
- **Dependencies.** No new crate or dependency, and no new `#[allow]`.
- **Contract changes.**
  - Two required trait methods: every implementation is listed in the brief's Wrapper table, and the compiler holds them.
  - Public items: `AgentView`, `TurnAgent`, `FindingKind::{Unattached, AgentCannotDispatch}`, `FindingKind::remedy_for`, and the kit's `HostOp::AttachedClients`, `HarnessOp::AgentView` and `ClaudeOp::AgentView` with their scenario methods.
  - `remedy`'s signature is unchanged (`tx_switch.rs` calls it).
  - No JSON field is added or changed: a finding's shape is the same, with two new `kind` codes.
  - No error code, protocol, hub or persisted-state change. `cli-surface.txt` is unchanged.
- **Read-only.** Both rules write nothing. `check_session_of_record` keeps returning `()`, and no existing signature changes. The new reads join the doctor allowlist T extended (`HOST_READS`, `HARNESS_READS`).
- **#839 overlap (W-2).** `findings.rs` now carries `remedy_for` and `with_agent`. Whichever of #839 and this story merges second reconciles the two remedy mechanisms and the module doc, as A's note says.

## Deviations from spec / wireframe

1. **ADR-0021 section 12's OpenCode parenthetical was sharpened.** It said "no later `task` rule allows another pattern", but the governing definition is "deny the `task` permission for every subagent". OpenCode matches a rule whose permission is `*` against `task` too, and `ask` lets a person allow a dispatch. So `[deny task *, allow * general]` and `[deny task *, ask task general]` do not deny every subagent. The adapter answers both `Some(true)`, and the parenthetical now reads "no later rule whose permission is `task` or `*` allows another pattern or asks for it". The change is conservative: it can only turn `Some(false)` into `Some(true)`, so it can only remove findings, never add one. No test pins either case.
2. **Two more ADR clauses:** a single-process harness's empty view is for a process endpoint, with `server-unsupported` for a port (A's W-4). The OpenCode read is the whole list (A's W-7, with the measured `limit` fact below).
3. **`prompt.rs` is edited, and it is not in the Blast radius.** A's W-1 required the split, and its notes ask F to record it here.
4. **`usable` stays private:** see Design decision 1. The rule is still written once.
5. **`RECENT_TURNS` lives in `observe/agent.rs`:** see Design decision 6.

## Tier 1 self-check (incl. tests now GREEN)

```
$ cargo build --workspace --tests --keep-going           -> exit 0 (no warning, no error)
$ cargo test --workspace --no-fail-fast -- --skip roster_stays_accurate_under_concurrent_body_load
    171 test binaries: 2687 passed, 0 failed, 39 ignored (the opt-in real tests); exit=0
    the new files: holler-pane agent_view_test 5/5, findings_test 7/7; testkit fake_harness_agent_test 6/6;
    adapter-host attached_test 5/5; adapter-opencode agent_view_test 13/13; holler-cli pane_verbs
    (doctor/attach.rs, doctor/agent.rs), pane_integration (forwarding.rs, bind.rs, wired.rs) and
    docs_pane_control_test all pass
$ cargo test -p holler-pane --doc                        -> 11 passed (ports.rs: the two new compile_fail,E0046,
                                                            PidsOnly, NoObservation and the two compile_fail,E0639)
$ cargo clippy --workspace --all-targets -- -D warnings  -> exit 0
$ rustfmt --edition 2021 --check <each touched crate root and file>   -> clean (formatted per file, never --all)
$ bash scripts/lint.sh                                   -> exit 0 (warns only; largest touched files:
                                                            adapter-opencode lib.rs 779, ports.rs 741, harness.rs 692)
$ bash scripts/changelog-check.sh                        -> changelog-check: ok
$ cargo doc --no-deps -p holler-pane -p holler-pane-testkit -p holler-adapter-{opencode,host,claude} -> no warning
```

After the workspace run, two edits were doc-comment only: the `agents.rs` module doc's W-7 paragraph and one reflowed line in `findings.rs`. After them, the crates involved were re-checked: OpenCode clippy is clean and `agent_view_test` passes 13/13; `holler-pane` clippy is clean and `findings_test` passes 7/7; rustfmt is clean.

Extra checks (opt-in, each on scratch instances it starts):

```
$ HOLLER_TEST_OPENCODE=1 cargo test -p holler-adapter-opencode --test real_opencode_test -- --ignored --test-threads=1
    14 passed (ac647_agent_view_reads_a_real_servers_turns_and_agents proves info.agent on 1.19.0 (D15);
    ac647_a_scratch_server_without_project_agents_has_no_dispatcher backs acceptance 18; ac12 runs harness
    conformance case 17 on a real server; verify_serving_refuses_... passes over the refactored reader)
$ cargo test -p holler-adapter-host --test real_tmux_test -- --ignored      (tmux 3.7c, private socket)
    9 passed (the_tmux_host_passes_the_host_conformance_suite includes case 11)
```

Two more probes:
- **`E0639`.** A scratch crate outside the repository, depending on `holler-pane` by path. A literal of `AgentView` (with or without `..Default::default()`) and of `TurnAgent` each fail with exactly `error[E0639]: cannot create non-exhaustive struct using struct expression`, so T's two `E0639` doc tests hold for the right reason.
- **W7 mutation check**, run by a script kept outside the repository. Each wrapper body in `bind.rs` was replaced in turn with a constant. The wrapper tests were run (`cargo test -p holler-cli --test pane_integration -- forwarding bind:: wired::`), and the file was restored byte for byte by sha256.
  - `BindingHost::attached_clients` returning `Ok(0)` fails 5 tests: `forwarding::the_binding_host_forwards_attached_clients` and `..._over_the_wired_host_...` (W1), `bind::the_binding_host_passes_the_host_conformance_suite` (W2), and the two `wired::` socket tests.
  - `Wired<dyn HostPort>::attached_clients` returning `Ok(0)` fails 6 tests: `forwarding::the_wired_host_forwards_attached_clients` and `..._over_the_wired_host_...` (W1), `bind::the_wired_host_passes_the_host_conformance_suite` (W2), `forwarding::a_misconfigured_wired_host_answers_its_error_from_attached_clients` (W3), and the two `wired::` tests.
  - `Wired<dyn HarnessPort>::agent_view` returning `Ok(AgentView::default())` fails 4 tests: `bind::the_wired_harness_forwards_every_method_the_default_armed_ones_included` (W4), `bind::a_misconfigured_wired_harness_answers_its_error_from_every_method` (W5), and the two `wired::` tests.
  - The unmutated control passes 22 of 22.

## Facts measured outside the repository (this phase)

On 2026-10-12 at 11:03 MDT I started a scratch `opencode serve --pure` (OpenCode 1.19.0) on port 48177. It was laid out as `real_opencode/rig.rs` lays one out: `HOME` and the `XDG_*` variables inside a scratch root, the rig's dead-end `CONFIG`, and the five `OPENCODE_DISABLE_*` flags. The model guard held: `GET /config/providers` listed only `deadend`.

- One session got three user messages with `noReply`, as `build`, `plan` and `build`.
- `GET /session/:id/message` answered all three.
- `?limit=1` answered `[build]`, the newest message, and `?limit=2` answered `[plan, build]`: the newest N, oldest first.
- So the route takes a `limit`, but it counts every message of either role, and a user turn can carry many assistant messages.
- The server's process group was killed and the scratch root removed afterwards.

This answers A's W-7: there is a limit, and it is not used. A limit cannot promise the last three user turns without paging, and T's hermetic stub serves the route only without a query, so adding one would mean editing T's tests, which F does not do.

## Evidence appendix

`docs/handoffs/647/evidence.md`: 15 entries, each with its `file:line` and a verbatim excerpt.

## Tests that look wrong (for T)

None.

T resolved acceptance 9's "a message without an agent is skipped" in favour of the ADR's "`None` when it recorded none": such a message is a turn `TurnAgent { agent: None, can_dispatch: None }`, and that is what was built.

## Known issues

- **The bound of the message read (W-7, hedged).** Each OpenCode pane whose record names no agent costs `GET /session/:id`, the session's whole message list and `GET /agent` on every pass, within one 10 s call. A list past 64 MiB, or past the call's bound, is `observe-failed` on every pass while it lasts, and no finding is raised. If the live fleet's sessions make this heavy, a follow-up could page the route with a growing `limit` (it needs a stub that serves a query).
- **W-3, for the operator:** whether the remedy itself should carry `--profile P` for a member. Today the message says it, and the remedy is the table's.
- **W-2, for whoever merges second of #839 and this story:** fold the two remedy mechanisms in `findings.rs` into one, and file the follow-up A named.
- **Brief Q1** (a verb that attaches a client) **and Q2** (merging before the cutover sign-off) are still the operator's. The guide's section 12 and the ADR's "Deferred" list name the verb.

## Files changed

```
CHANGELOG.md
docs/adr/ADR-0021.md
docs/pane-control.md
crates/holler-pane/src/lib.rs
crates/holler-pane/src/ports.rs
crates/holler-pane/src/ports/agent_view.rs            (new)
crates/holler-pane/src/ports/router.rs
crates/holler-pane/src/findings.rs
crates/holler-pane/src/reconcile.rs
crates/holler-pane/src/reconcile/observe.rs
crates/holler-pane/src/reconcile/observe/agent.rs     (new)
crates/holler-pane/src/reconcile/observe/process.rs
crates/holler-pane-testkit/src/host.rs
crates/holler-pane-testkit/src/harness.rs
crates/holler-pane-testkit/src/claude.rs
crates/holler-pane-testkit/src/harnesses.rs
crates/holler-adapter-host/src/lib.rs
crates/holler-adapter-host/src/tmux.rs
crates/holler-adapter-opencode/src/lib.rs
crates/holler-adapter-opencode/src/agents.rs          (new)
crates/holler-adapter-opencode/src/prompt.rs
crates/holler-adapter-opencode/src/server.rs
crates/holler-adapter-claude/src/lib.rs
crates/holler-cli/src/pane/wiring.rs
crates/holler-cli/src/pane/wiring/bind.rs
```

Records: `docs/handoffs/647/handoff-F.md`, `docs/handoffs/647/evidence.md`, and the `decisions.md` entry.
