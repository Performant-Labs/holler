# [feat(pane): doctor finds a tmux session no Herdr pane shows (`unattached`) and a pane that runs as an agent that cannot dispatch (`agent-cannot-dispatch`): part 2 of #647](https://github.com/Performant-Labs/holler/issues/647)

Part of epic #633. This is the complete statement of **part 2** of story #647, written as one text against `main` at `bda1069` (#840 merged, 2026-10-11) and **refreshed on 2026-10-12 against `main` at `6eff9cc`** (#835, #831, #665, #667, #648 and #853 have merged since); every excerpt (E1 and following) was re-cut from that tree, except E42 and E43, which are cut from a named commit of #839's brief, a story that has not merged. Part 1 (the reconcile engine and `holler pane doctor`, 16 finding kinds on `main` now) merged as PR #701 (`e612878`, 2026-10-09 18:56 MDT) and is the record in `docs/handoffs/647/`, which this story does not touch. Part 2 is the two amendments the issue parked: `agent-cannot-dispatch` (operator-confirmed 2026-10-09 17:40 MDT, epic decision 8, from pfleet#372) and `unattached` (2026-10-09 19:03 MDT, from the c4r1 incident).

It starts from the operator's decision of 2026-10-11, the last comment on #647: the contract change is **allowed**; `agent-cannot-dispatch` and `unattached` may add read-only port methods, "default-armed or required as the contract amendment decides"; #835 and #832 set the pattern; and #649's wrappers must forward every new method. It also settles every finding of the earlier plan check (`docs/handoffs/647-part2/earlier-attempt/handoff-A.md`, verdict BLOCK, B-1, B-2, W-1 to W-9); the table in "Where this brief is the decision" says where each is settled.

**Marks.** A decision marked *(operator)* was taken by the operator and is quoted from the issue or the ADR. A decision marked *(decided by the preparer, to be confirmed at the plan check)* is the preparer's judgment: the plan check (A) re-reads it against the code and either keeps it or sends it back, and the operator-only questions are at the end of the section.

## Where this brief is the decision

**D1. The scope is both amendments, in one PR that closes #647.** *(decided by the preparer, to be confirmed at the plan check)* Both findings need the same kind of change (a read-only port method, the ADR amended first, a wrapper test, a finding row in the guide), and they edit the same files. The pipeline script opens the PR with `Closes #647.` (E50), which is right only if both land. Rejected: split `unattached` into its own issue (the #665 precedent): it would amend the same ADR section and the same traits twice, in two PRs that clash, for no saving. If the plan check or the build run must drop one, the PR body is edited (`gh pr edit`) so it does not close #647.

**D2. The story starts after #835 and #831, which have merged, and is built on `main` at `6eff9cc`.** *(decided by the preparer, to be confirmed at the plan check)* #835 (PR #860, `4e691d9`) made the rule this story follows, required port methods (E5, E6, E33), and edited the same `ports.rs`, the same wrappers (E22, E24), the same test doubles and the same ADR lines. #831 (PR #857, `b76f4d2`) changed **every `HarnessPort` method keyed by `port: u16` to be keyed by `&HarnessEndpoint`** (E41), added the second harness (`holler-adapter-claude`, the kit's `FakeClaude`) and the one router `HarnessRouter` of the two shapes, and split the reconcile chain by the harness's shape (E10). So the method is `agent_view(at: &HarnessEndpoint, session, max_turns)`; `HarnessRouter` routes it by endpoint like every endpoint-keyed method (its module doc has the table); a single-process (Claude Code) endpoint answers an empty view; and no doctor chain calls it for a Claude Code pane (D6, rule 2). Also merged since the first text: #665 (PR #858: `profile-drift`, the last finding kind), #667 (`0f5835e`: the apply scenario and its shared end-state helper, E44), #648 (PR #856) and #853 (PR #861). The alternatives of the first text (stacking on either branch; building on #835 alone with `port: u16`) are moot and dropped.

**D3. Both new port methods are required, with no default.** *(operator, by the rule of 2026-10-11, ADR decisions 12 and 23, applied by the preparer)* E51 says the contract changes by changing the port, so an omission is a compile error; E33 says why a default is wrong here: E2 was default-armed, and the review of #849-era found it answering an empty observation, silently, over the real wiring; #835 made it required. E4 is the shape this story does not repeat. Cost, stated: every implementation of `HostPort` and `HarnessPort` in the workspace gets a method (the list is in "Blast radius"); the compiler finds them. Rejected: default-armed methods "that answer cannot-tell" (the operator's decision allows them; they are the exact failure the wrapper tests below exist to catch, and `session_activity` shows the cost of a default that is a plausible answer, `Idle`).

**D4. The two methods, and what already answers.** *(decided by the preparer, to be confirmed at the plan check)*
- `HostPort::attached_clients(&self, name: &PaneName) -> Result<u32, PaneError>`: the number of tmux clients attached to the session, as tmux counts them (`#{session_attached}`). A fresh session has none; a missing session is `pane-not-found`, as `ps` answers. Modelled on #835's `foreground_commands` (E6).
- `HarnessPort::agent_view(&self, at: &HarnessEndpoint, session: &str, max_turns: usize) -> Result<AgentView, PaneError>` (D5).
- **Reuse, nothing copied.** `foreground_commands` (#835) reads process names, not clients: it does not answer `unattached`, though the tmux adapter can take both values from the same `list-panes` listing `ps` already makes. `observe_pane` (#832) is Herdr's working directory and screen time: Herdr reports no tmux session. `session_activity` and `verify_serving` do not give turns or permissions, but `verify_serving` already calls `GET /agent` and owns the "agent usable as a default" rule (E18): `agent_view` calls that one function, it does not restate the rule. `from_messages` already reads `GET /session/:id/message` (E19): the reader is shared. Both routes exist (E20). Rejected: two harness methods (one for turns, one for agents): two wrapper arms, two fakes, two calls per pane, and the "can dispatch" of a turn needs the agent list anyway; raw JSON in the port (adapter text in `holler-pane`, I6, I7); a method that returns the finished `cannot-dispatch` boolean (the opt-out and the "ships an orchestrator" judgments belong to reconcile, so a second harness reports observations and never findings).

**D5. `AgentView`.** *(decided by the preparer, to be confirmed at the plan check)* A `#[non_exhaustive]` struct with builders and a `Default` that is empty, as `PaneObservation` is in `ports.rs` (E3 shows the host trait; E4 the defaults this story does not repeat): `turns: Vec<TurnAgent>` (the last `max_turns` user turns of the session, oldest first) and `dispatchers: Vec<AgentKey>` (sorted, no duplicates). `TurnAgent { agent: Option<AgentKey>, can_dispatch: Option<bool> }`. The exact meaning of each field is in the ADR text below. "Cannot tell" is an empty view or a `None`, never a guess; a server whose reply has the wrong shape is `unavailable` (an `observe-failed` finding), because a rule that is silently skipped on a version change is the failure this story is about. A name the server uses that is not an `AgentKey` is left out of `dispatchers` and is `agent: None` in a turn. Rejected: `turns` as raw message ids or times (not needed by any rule; the hub's feed is not this port's business).

**D6. The `agent-cannot-dispatch` rule.** *(operator for the finding's existence and its remedy text; the rule's details are decided by the preparer, to be confirmed at the plan check)* The finding is raised for a pane when all of these hold, each from an observation: (1) **the record names no agent** (`opencode_agent` is `None`). That is the only "opted out deliberately" the data has: a pane whose record names an agent chose one, `build` included, which is the pfleet#372 acceptance ("does not fire for a pane running `build` deliberately"); a pane whose record names a dispatching agent is a different problem (a server that lost its `default_agent`) and raises nothing here. No new `Pane` field. (2) The pane's harness is a server harness (OpenCode): `HarnessKind` has two variants on `main` and `check_pane` branches on `pane.harness.kind.shape()`, handing a single-process pane to the child module `process` (E10), so the agent check is added to the server-shape path only, after `check_session_of_record`, and a Claude Code pane never reaches it. (3) Its server is healthy and lists the session of record (the trust rule of E12). (4) `agent_view(&at, session_of_record, RECENT_TURNS)` answers at least one turn and **every** turn has `can_dispatch == Some(false)`, with `RECENT_TURNS = 3`, one constant in `observe.rs`. (5) `dispatchers` is not empty: the project "ships an orchestrator". "Recent hub-delivered turns" can only mean the recent turns of the session of record, the stand-in for DRIVEN that #645 chose and E14 records: OpenCode records the agent of a turn, not who sent it. Rejected: reading `.opencode/agents/` or `opencode.json` (I6; and on an Aftersight pane the orchestrator lives elsewhere, epic decision 8); `Pane.role` as the gate (the epic says the role is the fleet role, not the agent); a majority of turns instead of all (a pane that mixes agents is partly working); a stored "was reported" flag (part 1 keeps no memory between runs).

**D7. "Ships an orchestrator", and how an agent "cannot dispatch".** *(decided by the preparer, to be confirmed at the plan check)* The adapter reports what the server says; the rule is one definition. An agent **cannot dispatch** when the server's permission rules for it deny the `task` permission for every subagent: in OpenCode 1.19.0 the rules are a list of `{permission, action, pattern}`, and the last rule whose permission is `task` or `*` and whose pattern is `*` is `deny`, with no later `task` rule that allows another pattern (measured: "Facts measured outside the repository"). The project **ships an orchestrator** when `dispatchers` is not empty: an agent the server lists that is usable as a default (the `verify_serving` rule: not a subagent, not hidden), is not one of OpenCode's own (`native` is false) and can dispatch. Anything else the adapter cannot read (a rule list of another shape, an agent missing from the list) is `None`, which stops the rule. Rejected: counting every dispatching primary agent (OpenCode's built-in `plan` is one: a false positive on every pane); a list of agent names in the ADR (a name is the project's).

**D8. The remedy of `agent-cannot-dispatch`.** *(operator for "naming the cure (`relaunch --agent KEY`)"; the form is decided by the preparer, to be confirmed at the plan check)* `holler pane relaunch PANE --agent=KEY`, with KEY the one dispatcher when `dispatchers` has exactly one; with several, no remedy, and the message lists up to three (quoted) and says to pass one. The form `--agent=KEY`, not `--agent KEY`, because `AgentKey::parse` accepts a leading `-` and capitals (E17), so `-x` is a valid key and `--agent -x` would read as a flag. KEY reaches a message and a remedy only as a parsed `AgentKey`. The one table (E8) stays one table: `FindingKind::remedy_for(self, pane, fix, agent: Option<&AgentKey>)` is the table, and `remedy(self, pane, fix)` (the signature `tx_switch.rs` calls, E40) delegates with `None`; the `Finding` is built with its key through `Finding::with_agent`. `--fix` is not offered: the finding is `not-fixable`, and `--fix` never changes a pane's agent, silently or not (the issue; ADR decision 16 says a relaunch never changes the agent silently, and this one names it on the command line). Rejected: `remedy` gains a parameter (breaks an outside caller for one kind); a remedy with a `KEY` placeholder (not a command); `holler pane relaunch PANE` alone (does not change the agent: decision 16).

**D9. The `unattached` rule.** *(operator for the finding and its acceptance: "a running, registered session with no attached client raises it; the same pane attached raises nothing"; the rule's edges are decided by the preparer, to be confirmed at the plan check)* Raised when `host.ps` answered for the pane (the session exists) and `attached_clients` is `0`. It is not gated on Herdr, on the pane's hold or on its role: the operator sees nothing in every case. It is independent of `herdr-pane-missing` and `tmux-session-missing`: a missing session raises only the latter (no `attached_clients` call is made); a failed `attached_clients` is `observe-failed` naming `host.attached_clients` and no `unattached`. It is raised for a Claude Code pane too: `check_tmux` runs for every pane before `check_pane` branches on the harness's shape (E10, E11), and a Claude Code pane's tmux session needs a client in a Herdr pane as much as an OpenCode pane's does.

**D10. The `HerdrSnapshot` amendment the ADR defers: none is needed, and the deferral is already closed.** *(decided by the preparer, to be confirmed at the plan check)* The ADR's old sentence ("`HerdrSnapshot`, which waits for a contract amendment before #647 and #650 need it") was replaced when #808 added the label (the "Deferred" bullet now reads "the label is added by #808"); a search of `docs/adr/` for `HerdrSnapshot` finds nothing, and the earlier plan check's line 741 no longer exists. The only fact that would let `unattached` say "attached in the Herdr pane the record names" is what process that Herdr pane runs, and Herdr answers that with whole command lines (`pane.process_info`, argv included), which I7 and ADR section 13 refuse. So the rule is "no client at all", and its limit is written in the ADR, the guide and the message: a client attached in some other terminal hides the finding. This is the issue's own acceptance ("no attached client"). Rejected: adding `foreground_processes` to `HerdrPane` (a Herdr read of command lines, a wider `HerdrSnapshot`, a change #650's join does not need); a `HerdrPort` method that returns the pane's tty (no source says Herdr reports one; the spike lists `tty` only as an optional field).

**D11. The cure of `unattached` that is a Holler verb: there is none today, so the remedy is none, and the finding says why.** *(decided by the preparer, to be confirmed at the plan check; the operator's answer to Q1 below may add a verb)* The earlier plan check's W-3 asked for "a remedy that exists today, or `null` with the reason in the message". Evidence: the TUI is an `opencode attach` that runs inside the pane's tmux session (E34); `launch` and `relaunch` ensure the Herdr pane, the tmux session, the server and the TUI, and nothing in `tx_launch`, the Herdr adapter or the host adapter attaches a tmux client to a Herdr pane (E35 states the intent: one Herdr pane shows one tmux session, without saying what attaches it). `pane launch` refuses a registered pane (`pane-exists`), and `pane relaunch` does not attach a client, so naming either would be a false remedy. The issue's other cure is the operator's own tmux-attach tool, which is not a Holler verb and is not named in code, tests or output (public repository). So `unattached` has `remedy: null`, and its message says that no tmux client is attached to the session, so no Herdr pane shows it, and that no Holler verb attaches one. Rejected: `holler pane relaunch PANE` as the remedy (a claim the code does not back, and a relaunch stops the pane's server for nothing); a new verb `holler pane attach` now (new write methods on `HerdrPort` or `HostPort`, a keystroke into a Herdr pane against I4's spirit, an ADR 0003 surface and a flag set: its own story; the ADR "Deferred" list names it with no owner).

**D12. `SpecField` row for `opencode_agent`: already done, nothing to build.** *(operator, ADR decision 18, #644)* E30 shows `SpecField::OpencodeAgent` and `ALL: [SpecField; 16]` on `main`; E31 item 18 records it. The earlier W-7 is settled; this story edits no line of `profile_diff.rs`.

**D13. Reconcile's chain and the read-only guarantee.** *(decided by the preparer, to be confirmed at the plan check)* `check_tmux` (E11) gains the `attached_clients` call after a successful `ps`, and it runs for both shapes (E10); the agent check is a new function called after `check_session_of_record` in the server-shape path of `check_pane` (E10), so it can use the same `Server` value. The module doc's bound (E9) becomes "at most nine port calls" for a server's chain, and the list of a single process's chain (the child module `process`) gains `attached_clients`; and the read-only allowlist (E15) adds exactly `HostOp::AttachedClients` and `HarnessOp::AgentView`. Neither rule writes the record. The agent check calls `agent_view` only when rules (1) to (3) of D6 hold, so a pane with a recorded agent costs no extra call. I6 stays true: E13.

**D14. The test kit gets the support; no wrapper doubles are needed for the engine tests.** *(decided by the preparer, to be confirmed at the plan check)* The earlier W-5 wanted test-local doubles because the kit lacked support. #684 (the kit) is closed, and the support is reusable, so it goes in the kit: `FakeHost` gains `attach_client(name)` and `detach_clients(name)` and the op `HostOp::AttachedClients`, and a fresh fake session has **no** client, as a fresh real one has none (a conformance case pins it, so the fake cannot drift); `FakeHarness` gains `seed_turns(port, session, turns)`, `seed_dispatchers(port, keys)` and `HarnessOp::AgentView`. A "healthy world" of the doctor rig attaches one client per pane in `make_live` (E52), so every existing doctor test keeps its meaning. The kit files are listed in "Blast radius" (E37, E38, E53 show where).

**D15. The real implementations.** *(decided by the preparer, to be confirmed at the plan check)* `holler-adapter-host`: `attached_clients` reads `#{session_attached}` through the exact-target listing `ps` uses, with a parser as strict as `ps`'s (a value that is not an unsigned integer is `unavailable`; a missing session is `pane-not-found`). `holler-adapter-opencode`: a new module `agents.rs` (so `lib.rs`, 741 lines, grows by the method and its `mod` line only) implements `agent_view` from `GET /session/:id/message` and `GET /agent`, existing routes (E20); the "usable as a default" rule is `server::usable`'s, shared. Unknown session is `session-not-found` (the check `abort` makes). A reply that is not the JSON a step needs is `unavailable` and quotes no reply (the adapter's rule). The message field that names the agent of a user turn is **not verified on 1.19.0** (the SDK types of 1.14.39 give `UserMessage.agent`; see "Facts measured outside the repository"): the build run proves it with the opt-in real-OpenCode test and cuts the message fixture from the answer before the adapter relies on it. `Unwired` answers `not-implemented` for both. `ClaudeHarness` (`holler-adapter-claude`) and the kit's `FakeClaude` answer an empty view (D2); `HarnessRouter` and the kit's `FakeHarnesses` route it by endpoint, and `Wired<dyn HarnessPort>` forwards it (E23).

**D16. What `--fix` does: nothing new.** *(operator, the issue)* `--fix` repairs `shown-driven-mismatch` only. For both new findings `fix` is `not-fixable`. A second `doctor` run reports each standing finding again exactly once and writes nothing for them ("once per pass", the 2026-10-09 split).

**D17. Finding order and the `ALL` pin.** *(decided by the preparer, to be confirmed at the plan check)* The two kinds are appended to `FindingKind` and `ALL` in this order, `Unattached` then `AgentCannotDispatch`, **after the last kind present on `main` at build time** (E7: the 16 kinds on `main`, the last `profile-drift` (#665), after `pane-waiting` and `claude-login-missing` (#831); the build run's list has 18). `findings_test.rs` pins every code in order (E16); the build run edits that list, not a copy. Codes: `unattached`, `agent-cannot-dispatch`; neither equals a closed error code or an import finding code (`import_test.rs` checks the import side).

**D18. Review rigor: `second-opinion`.** *(decided by the preparer, to be confirmed at the plan check)* The issue's Pipeline line says `rigor: in-session`; part 1 ran `second-opinion` on the orchestrator's instruction (its brief, C-1). This story amends two traits, adds a real implementation in two adapters, and edits the ADR, so the outside model (`deepseek-v4-pro`, DeepSeek's hosted chat endpoint) reads the brief and the diff. The CLAUDE.md prerequisites hold: `.env` with the managed block and `DUAL_REVIEW=1`, `DEEPSEEK_API_KEY` from the operator's store (never printed), one real completion proven before the run is journalled above `in-session`. Rejected: `in-session` (the issue's line, written before the contract change was allowed); `panel` (no cross-vendor reason).

**D19. The handoff slug.** *(the operator's instruction of 2026-10-11)* The brief is `docs/handoffs/647-part2-brief.md`; the run's record is `docs/handoffs/647-part2/`; the earlier attempt's files are kept under `docs/handoffs/647-part2/earlier-attempt/`. **The pipeline script cannot write to that directory by an argument**: see "Pipeline", which gives the exact launch arguments and the pre- and post-run steps.

### The earlier plan check, settled

| Finding | Settled by |
|---|---|
| B-1 the brief planned work already on `main` | This brief is written from the live issue's two open amendments and `main`; nothing of part 1 is re-planned. |
| B-2 literal Decision 10 re-declares and stubs the merged engine | Extend-only: new variants, new reads in the chain, new test submodules. No existing `pub` item or JSON value changes (D8 keeps `remedy`'s signature; D17 appends). RED is a failing assertion against the working engine. |
| W-1 no port observes either input; I6 forbids files | D3, D4, D5: two required read-only methods, amended in the ADR first. |
| W-2 order with #642's `serve` change | #642 is closed and merged; its `ServeSpec` change is on `main` (E1). The order that remains is against #835 and #831 (D2). |
| W-3 `unattached` observation and remedy | D9, D10, D11. |
| W-4 `AgentKey` allows a leading `-` in a remedy | D8: `--agent=KEY`, built only in the one table, parse-tested. |
| W-5 test kit has no support; wrapper doubles | D14: kit support, not doubles. |
| W-6 pinned count, chain bound, allowlist, DRIVEN, opt-out | D13 (bound, allowlist), D17 (the pin), D6 (turns of the session of record; opt-out is the record). |
| W-7 `SpecField` row for `opencode_agent` | D12: already on `main`. |
| W-8 handoff slug | D19 and "Pipeline". |
| W-9 re-run the brief gate | The pipeline's brief gate runs on this brief; the earlier PASS is not carried forward. |

### Questions only the operator can answer

- **Q1. Should a Holler verb that attaches a pane's tmux session to its Herdr pane be built, and by which story?** Today none exists (D11), so `unattached` detects and names no command. Building it is a new write (a port method on Herdr or the host, a verb and flags, ADR 0003, I4 questions). The preparer's default: not in this story; file a follow-up if wanted.
- **Q2. Does part 2 start before the live cutover (#654) is signed off?** The issue parked it "until after the cutover, when the live fleet shows which incidents recur". The 2026-10-11 comment allows the contract change and says the brief starts from it, so the preparer treats the park as lifted for the build; merging changes what `doctor` reports during the soak (it will raise `agent-cannot-dispatch` and `unattached` on live panes), which is the operator's call.

## Scope

1. **Contract first, its own commit, before any test:** the ADR-0021 amendment in the next section, and the module doc of `crates/holler-pane/src/ports.rs` (the list that begins "Amended since:" has five bullets, E1; part 2 adds a sixth).
2. **`holler-pane`:** the two required methods and the types `AgentView` and `TurnAgent` (in a new `crates/holler-pane/src/agent_view.rs` re-exported from `lib.rs`: `ports.rs` is 621 lines on `main` and the two methods and their two doc tests add about 70, so the types would pass the 700-line mark the table below sets); two `FindingKind`s with `code()`, `remedy_for`, `Finding::with_agent`; the reads in `reconcile/observe.rs`.
3. **Test kit:** `FakeHost`, `FakeHarness`, their ops and conformance cases (D14).
4. **Adapters:** `holler-adapter-host` (`attached_clients`), `holler-adapter-opencode` (`agent_view`, new `agents.rs`) (D15).
5. **Wiring:** `BindingHost`, `Wired<dyn HostPort>`, `Wired<dyn HarnessPort>`, `Unwired` forward or refuse both (the "Wrapper forwarding" section).
6. **Docs:** `docs/pane-control.md` section 5 (two rows, the "command it names" cells, the limit of `unattached`), the CHANGELOG bullet, the module docs that name the bound or the amendment list (the `bind.rs` doc E21, `reconcile.rs` and `observe.rs` module docs).
7. **Out of scope:** a verb that attaches a client (Q1); `--fix` for either finding; a stored "already reported" state; DRIVEN (#839); the Claude Code harness's answers beyond "empty view" (#831); any `Pane` field; `profile-drift` (#665); the roster (#648).

## The ADR-0021 amendment that lands first

One commit, `docs(adr): contract amendment for the attached-clients and agent-view reads (#647)`, before any test or code. Locate each edit by its words (the line numbers of the ADR move whenever a story lands; every quoted anchor below was checked against `main` at `6eff9cc`). Text in quotation marks is verbatim.

**1. Section 2, the port table.** In the `HostPort` row, change "and the read-only `foreground_commands(name)` (#835, section 13; #831, section 15)" to "the read-only `foreground_commands(name)` (#835, section 13; #831, section 15) and the required `attached_clients(name)` (#647 part 2, section 12)". In the `HarnessPort` row, after "`locate_process` and `login_status`", add: ", and the required `agent_view(at, session, max_turns)` (#647 part 2, section 12)". Each cell's last column is unchanged.

**2. Section 2, after the sentence that ends "…where a default would answer silently in place of a port the wrapper did not forward"** (#835's), add this sentence: "#647 part 2 adds two more required reads, `HostPort::attached_clients` and `HarnessPort::agent_view` (section 12), for the same reason; #649's wrappers forward both."

**3. Section 9, the `pane doctor` row.** Replace "the reported `herdr-version-unsupported` and," with "the reported `herdr-version-unsupported`, `unattached` and `agent-cannot-dispatch` (#647 part 2, section 12), and,", keeping the rest of the sentence.

**4. Section 12**, directly after the paragraph that begins "**Decided (#647): no hub timer.**", add:

> **Decided (#647, part 2): `unattached` and `agent-cannot-dispatch`, each from one new read-only port method.** Reconcile reads the world only through the ports (I6), and no port could observe either finding. The operator allowed the contract change on 2026-10-11 (decisions taken, below). Both methods are **required**, with no default, for the reason in section 2 (#835): a default answers silently in place of a port a wrapper did not forward, and the compiler finds a wrapper that left one out. #649's wrappers forward both, and a test per wrapper pins it.
>
> - **`HostPort::attached_clients(name) -> Result<u32, PaneError>`**: the number of tmux clients attached to the session `name`, as tmux counts them (`#{session_attached}`). A fresh session made by `ensure_session` has none. A missing session is `pane-not-found`, as `ps` answers. A count only: never a client's name, tty or process (I7).
> - **`HarnessPort::agent_view(at, session, max_turns) -> Result<AgentView, PaneError>`** (`at` is the pane's `HarnessEndpoint`, #831): what the harness server says about the agents of `session`, in one call. `HarnessRouter` routes it by endpoint, and a single-process harness answers an empty view. `AgentView` is `#[non_exhaustive]`, with builders and an empty default, as `PaneObservation` is.
>   - `turns`: for each of the last `max_turns` user turns of `session`, oldest first, a `TurnAgent { agent, can_dispatch }`. `agent` is the key the server recorded for the turn, `None` when it recorded none or a name that is not an `AgentKey`. `can_dispatch` is `Some(false)` only when the server's permission rules for that agent deny the `task` permission for every subagent (OpenCode: the last rule whose permission is `task` or `*` and whose pattern is `*` is `deny`, and no later `task` rule allows another pattern), `Some(true)` when they do not, and `None` when the server does not say, the agent is not in its list, or the rules have a shape the adapter does not know.
>   - `dispatchers`: the agents the server lists that a pane could run as its default (not a subagent and not hidden: the rule `verify_serving` applies), that are not the harness's own (OpenCode: `native` is false), and whose `can_dispatch` is `Some(true)`. Sorted by key, without duplicates; a name that is not an `AgentKey` is left out.
>   - An implementation that cannot tell answers an empty view, never a guess. A session the server does not have is `session-not-found`; a reply of the wrong shape is `unavailable`. The OpenCode adapter reads `GET /session/:id/message` and `GET /agent`, and never a file.
> - **`unattached`** is raised when `host.ps` answered for the pane (the session exists) and `attached_clients` is `0`. The rule is "no tmux client at all", not "a client in the Herdr pane the record names": telling which Herdr pane displays a tmux session needs Herdr's `pane.process_info`, whose answer is whole command lines (I7; section 13 refuses it), so a client attached in some other terminal hides the finding. `HerdrSnapshot` and `HerdrPane` do not change. The finding names no command: no Holler verb attaches a tmux client to a Herdr pane (`launch` and `relaunch` attach the TUI inside the tmux session; `launch` refuses a registered pane), and its message says so.
> - **`agent-cannot-dispatch`** is raised when all of these hold: the record names no agent (`opencode_agent` is absent; a pane whose record names one chose it, `build` included); the pane's harness is OpenCode; its server is healthy and lists the session of record; `agent_view(at, session_of_record, 3)` answers at least one turn and every turn has `can_dispatch` `Some(false)`; and `dispatchers` is not empty, which is what "the project ships an orchestrator" means. "Recent hub-delivered turns" are the recent turns of the session of record, which I2 makes the session the hub drives: OpenCode records a turn's agent, not who sent it. The remedy is `holler pane relaunch PANE --agent=KEY` when `dispatchers` holds exactly one KEY; with several there is no remedy and the message lists up to three. `--fix` does nothing for either finding.
> - **Reads and writes.** A server's chain makes at most nine port calls, each bounded by I5 (`attached_clients` and `agent_view` are the two new ones), and a single process's chain gains `attached_clients`. Neither finding writes the record, and each is raised once per pass.

**5. "Deferred to named stories"**, a new bullet: "A Holler verb that attaches a pane's tmux session to its Herdr pane, the cure for `unattached`: no story owns it; until one does, the finding names no command (section 12)."

**6. "Decisions taken"**, one new item with the next free number at merge time: "**Part 2 of #647 may add read-only port methods** (operator, 2026-10-11, #647, section 12): `HostPort::attached_clients` and `HarnessPort::agent_view`, required like #835's, so a wrapper that does not forward one is a compile error." On `main` today the list ends at item 33 (#853's two items follow #835's 30 and 31), so the item is 34 unless another story lands first; renumber at merge.

## Acceptance

Each item is a test. Every test fails on `main` (for a new method, by not compiling) and passes with the change; T (author) confirms each RED before F writes code. Fixtures are neutral names (`demo-c1r1`, `orchestrator`), no real host, user or path.

**Contract and types (`holler-pane`).**
1. `HostPort` and `HarnessPort` each have one `compile_fail,E0046` doc test **per new method**, in which an implementation provides every other method and omits that one (#835's `PidsOnly` omits only one method too; a test that omits several does not pin each) (E5).
2. `AgentView` has an empty `Default`; a struct literal outside the crate does not compile (a `compile_fail,E0639` doc test, as `ServeSpec`'s); the builders build `turns` and `dispatchers`.
3. `findings_test.rs`: `CODES` lists `unattached` and `agent-cannot-dispatch` after the last kind of `main`; codes are unique, kebab-case and equal to their JSON form (E16).
4. `remedy_for`: `AgentCannotDispatch` with one key `orchestrator` gives `holler pane relaunch demo-c1r1 --agent=orchestrator`; a key `-x` gives `--agent=-x` and **parses through the real `relaunch` clap tree** (`verb_harness::parse::try_parse`) as the agent `-x`; with `None` it gives no remedy; `Unattached` gives none in every state; `remedy` (the old signature) still compiles and answers every old kind as before.

**Test kit.**
5. `FakeHost`: a fresh session has 0 clients; `attach_client` twice gives 2; `detach_clients` gives 0; a missing session is `pane-not-found`; `faults` fails the call with `HostOp::AttachedClients` and changes nothing.
6. The host conformance suite has a case "attached-clients of a fresh session is zero and of a missing one is `pane-not-found`"; the suite's `Mutant` (a host that always answers 1, one that answers 0 for a missing session) fails it; the real tmux adapter passes it in the opt-in real run.
7. `FakeHarness`: `agent_view` of a seeded session returns the newest `max_turns` seeded turns, oldest first, and the seeded dispatchers; an unseeded session on a live server returns an empty view; an unknown session is `session-not-found`; a killed or frozen server fails as the other reads do. The harness conformance suite has the matching case, and its `Mutant` fails it. `FakeClaude` and `ClaudeHarness` answer an empty view for the endpoint they are given (D2), pinned by one case of the Claude conformance suite (`conformance/claude.rs`) that `claude_conformance_test.rs`'s `Mutant` fails; `FakeHarnesses` and `HarnessRouter` route the call by endpoint, so a `Port` endpoint reaches the server fake and a `Process` endpoint the Claude fake.

**Adapters.**
8. `holler-adapter-host` over the fake tmux: `#{session_attached}` values `0`, `1`, `2` give 0, 1, 2; an empty or non-numeric value is `unavailable` naming the call and quoting no output; a missing session is `pane-not-found`; the tmux argv is built without a shell and with the exact target (`=NAME:`), as `ps`'s.
9. `holler-adapter-opencode`, hermetic stub, fixtures under `tests/fixtures/agents/` cut from OpenCode 1.19.0 (the commands are in "Facts measured outside the repository"): a `build` agent whose rules end in `task deny *` has `can_dispatch` `Some(false)`; a custom primary agent with `task allow *` and `native: false` is in `dispatchers`; `plan` (native) and a subagent and a hidden agent are not; a table of rule lists decides `Some(false)`, `Some(true)` and `None` (deny then allow of one pattern is `Some(true)`; an object where a list is expected is `None`); user turns only, newest `max_turns`, oldest first; a message without an agent is skipped; an unknown session is `session-not-found`; a body that is not a JSON array is `unavailable` naming the route and quoting nothing. An opt-in real-OpenCode test reads `agent_view` from a real server in the scratch rig and **is where the message's agent field is proved** (D15).

**Reconcile and `holler pane doctor` (`crates/holler-cli/tests/pane_verbs/doctor/`, two new submodules, so the 560-line `doctor.rs` grows by two `mod` lines).**
10. `unattached`: a registered pane whose fake session has 0 clients raises exactly one `unattached` (its pane, its position `r?c?` in rowcol, `remedy: null`, `fix: not-fixable`, a message that says no tmux client is attached and no Holler verb attaches one); after `attach_client` on the same fake, the same pass raises nothing; `--format=json` passes the envelope helper and carries the stable code; text and JSON exit codes are equal. (The issue's acceptance, with #638's fakes.)
11. `unattached` edges: a pane whose session is missing raises `tmux-session-missing` only and the fake records no `AttachedClients` call; a failed `attached_clients` raises `observe-failed` naming `host.attached_clients` and no `unattached`; a parked pane and an orchestrator pane with 0 clients raise it too.
12. `agent-cannot-dispatch`, positive: record without an agent, healthy server with the session of record, `agent_view` of 3 turns all `can_dispatch: Some(false)` as `build` and `dispatchers: [orchestrator]` raises one finding on the pane with the session of record and the port, remedy `holler pane relaunch demo-c1r1 --agent=orchestrator`; the same with `--fix`: `fix: not-fixable`, the pane's record equal to before (its agent included), no `serve_command` recorded by the fake harness, no `select_session`.
13. `agent-cannot-dispatch`, negatives (each its own test): the record names `build`, and the record names `orchestrator` (opted in): nothing; `dispatchers` empty: nothing; one of the three turns is `Some(true)` or `None`: nothing; an empty view: nothing; two dispatchers: the finding with **no remedy** and a message that lists both quoted; an unhealthy server, a server without the session of record, a record without a session: `agent_view` is never called (the fake's calls show no `AgentView`) and nothing is raised.
14. Once per pass: a second pass over the same world reports the same two findings once each, nothing new, and the stored records are equal after both passes (these rules write nothing).
15. `read_only.rs`: the allowlist is extended by exactly `HostOp::AttachedClients` and `HarnessOp::AgentView`, and the full pass with and without `--fix` makes no other call (E15).
16. The existing doctor tests all pass unchanged in meaning: both rigs that build a live world attach a client to each pane they build (`doctor/rig.rs` `make_live`, E52, and `pane_claude/rig.rs` `seed_claude` and `seed_opencode`, which also build the worlds of `pane_claude/doctor.rs` and `mixed.rs`), and a test proves that a world built without that call would raise `unattached` for every pane (so the attach is what keeps them clean, not the rule being off).

**Wrapper forwarding** is its own section below (items W1 to W6 are acceptance items too).

**Docs and gates.**
17. `docs_pane_control_test`: section 5 names both codes (E36); the table rows say "none" for `unattached`'s command and `holler pane relaunch PANE --agent=KEY` for the other; the guide states the limit of `unattached` (a client in any terminal counts).
18. The shared end-state helper `assert_end_state` (E44; #667 moved it to `pane_integration/scratch/verbs.rs`, and #649's scenario and `profile_apply_scenario/{scenario,grid,probe}.rs` all call it) asserts "no finding except `unattached` for each scratch pane", with a comment saying why (a detached scratch tmux session has no client by design). The default run still compiles and passes.
19. `cli-surface.txt`, ADR 0003 and the clap tree are unchanged (no verb or flag is added); `scripts/lint.sh` passes (no file reaches 900 lines, E39); `cargo fmt` is run per file, never workspace-wide; clippy is clean; no `unsafe`; the privacy grep of the added lines finds no personal host, user, path or tool name.
20. The ADR amendment is its own, first commit, and the code commits cite the section.

## Wrapper forwarding

#849-era review found `observe_pane` answering an empty observation over the real wiring because a wrapper had not forwarded a default-armed method; the pin that followed is E26 (the conformance suite over the wrapper, one recording check per wrapper, the failed-build case), and ADR section 14 says "A wrapper port must delegate it" (E32). #835 repeats it for the host (E27, E28). Here both new methods are **required**, so a wrapper that omits one does not compile; the tests below catch the other failure, a wrapper that compiles and forwards wrongly (answers a constant, drops an argument, calls the wrong inner method).

| New method | Implementations that must forward or answer | The test that fails if one does not forward |
|---|---|---|
| `HostPort::attached_clients` | `BindingHost` (E22); `Wired<dyn HostPort>` (E24) | **W1** `pane_integration/forwarding.rs` (new): over a recording inner host that answers `Ok(7)` for one name and `Err(Unavailable)` for another, each wrapper returns 7 and the same error, and the inner host recorded exactly one `attached_clients` call per ask, with the name given. **W2** the host conformance suite (its new case, D14) runs over `BindingHost` and over `WiredHost` (E27 is the model): the missing-session case fails for a wrapper that answers a constant. |
| `HostPort::attached_clients` | `Wired` built from a misconfiguration; `Unwired` | **W3** a misconfigured `WiredHost` answers its configuration error from `attached_clients`, and `wired.rs`'s socket list gains `host.attached_clients` (E28); `Unwired` (E29) answers `not-implemented` (the existing `Unwired` test is extended). |
| `HarnessPort::agent_view` | `Wired<dyn HarnessPort>` (E23); `HarnessRouter` (routes by endpoint) and the kit's `FakeHarnesses` (one forward) | **W4** `bind.rs`: `Recording` answers a distinctive `AgentView` (one turn as `orchestrator`, one dispatcher `reviewer`) and records its arguments; `call_every_method` gains `agent_view`; the expected-calls list gains `"agent_view"`; the test asserts the wrapper's answer equals the inner one's and the inner call saw `(48100, "ses_1", 3)` (E25). **W5** `a_misconfigured_wired_harness_answers_its_error_from_every_method` loops over `call_every_method`, so it covers the new method with the same edit; `wired.rs` gains `harness.agent_view`. |
| both | the test doubles: for the host `LinkedHost` (four copies: `pane_integration/bind.rs`, `pane_claude/rig.rs`, `profile_verbs/apply/rig.rs`, `pane_verbs/launch/rig.rs`), `LostChain` (#835), `TestHost`, the host `Mutant`; for the harness `Recording` (two: `pane_integration/bind.rs` and the real-OpenCode rig), `Resolving`, `HealthGate`, `WriterInSelect`, `SeamHarness`, `HookedHarness` (two), `ServerOnly` (`holler-pane/tests/claude_kind_test.rs`), `TestHarness`, the two `Mutant`s (OpenCode's and Claude's conformance tests) | compile-enforced. **W6** a double that wraps a fake forwards to it in one line; the doctor tests of items 10 and 16 attach their client through the fake that a wrapping double wraps, so a double that answered a constant would make them fail. |

Also required: **W7** (T verifies, F records in its handoff) a **mutation check**: replace each wrapper's body in turn with a constant (`Ok(0)`, `Ok(AgentView::default())`) and show that W1, W2, W4 fail for it; revert. **W8** the module doc of `bind.rs` (E21) is edited so that it names the new methods, and says that no method of these wrappers has a default to fall back on.

## Blast radius

Re-measured with `wc -l` on 2026-10-12: the numbers are this worktree, which is `main` at `6eff9cc` merged in (so #835 and #831 are counted); a dash is a file that does not exist yet. `scripts/lint.sh` fails a `crates/**/*.rs` file at 900 lines and warns at 600 (E39); docs are not under the gate.

| File | lines | Change | Fits |
|---|---|---|---|
| `crates/holler-pane/src/ports.rs` | 621 | the two required methods; two `compile_fail,E0046` doc tests (one per method, modelled on `PidsOnly`, E5); the module doc's sixth bullet; `mod agent_view;` | about 700: warn, under 900 |
| `crates/holler-pane/src/agent_view.rs` (new) | | `AgentView`, `TurnAgent`, builders, the `E0639` doc test, re-exported from `lib.rs` | yes |
| `crates/holler-pane/src/ports/router.rs` | 145 | the `agent_view` arm (by endpoint) and a row of the routing table in its module doc | yes |
| `crates/holler-pane/src/findings.rs` | 379 | two kinds, `remedy_for`, `Finding::with_agent` | yes |
| `crates/holler-pane/src/reconcile.rs`, `reconcile/observe.rs`, `reconcile/observe/process.rs` | 533, 449, 239 | module docs (the bound; the single process's list); the two reads and the rule; `RECENT_TURNS` | observe.rs about 540: warn |
| `crates/holler-pane/src/lib.rs` | 81 | re-exports | yes |
| `crates/holler-pane/tests/findings_test.rs`, `ports_test.rs` | 55, 613 | `CODES` (16 to 18); `TestHost`/`TestHarness` methods | `ports_test.rs` about 640 |
| `crates/holler-pane-testkit/src/host.rs` | 278 | `attach_client`, `detach_clients`, `HostOp::AttachedClients` | yes |
| `crates/holler-pane-testkit/src/harness.rs` | 631 | `seed_turns`, `seed_dispatchers`, `HarnessOp::AgentView`, the impl | about 700: warn, under 900 |
| `crates/holler-pane-testkit/src/claude.rs`, `harnesses.rs` | 623, 150 | `FakeClaude`: an empty view; `FakeHarnesses`: one forward | yes |
| `crates/holler-pane-testkit/src/conformance/host.rs`, `conformance/harness.rs`, `conformance/claude.rs` | 319, 612, 494 | one case each | harness.rs about 660: warn |
| `crates/holler-pane-testkit/tests/host_conformance_test.rs`, `harness_conformance_test.rs`, `claude_conformance_test.rs` | 254, 346, 297 | the `Mutant`s | yes |
| `crates/holler-pane-testkit/tests/fake_host_test.rs`, `fake_harness_test.rs` | 372, 783 | the fake's cases; **the harness cases go in a new `fake_harness_agent_test.rs`**, not in the 783-line file | yes |
| `crates/holler-adapter-host/src/lib.rs`, `src/tmux.rs` | 521, 336 | `attached_clients`, its listing and parser | yes |
| `crates/holler-adapter-host/tests/attached_test.rs` (new) | | the cases of item 8 (**not** in `fake_tmux_test.rs`, 820 lines; `foreground_test.rs` is the file of the same kind for `foreground_commands`) | yes |
| `crates/holler-adapter-opencode/src/lib.rs`, new `src/agents.rs` | 764 | `mod agents;` and a one-line method in `lib.rs`; the logic in `agents.rs` | lib.rs about 775: warn |
| `crates/holler-adapter-opencode/tests/agent_view_test.rs` (new), `tests/fixtures/agents/*.json` (new), `tests/real_opencode/**` | rig 580 | the hermetic cases; the opt-in real case; the rig's `Recording` gains a forward | yes |
| `crates/holler-adapter-claude/src/lib.rs` | 375 | `agent_view`: an empty view (the crate has its own `agents.rs`, which this story does not touch) | yes |
| `crates/holler-cli/src/pane/wiring.rs`, `wiring/bind.rs` | 468, 489 | `Unwired` arms; the forwarding arms (`BindingHost`, `Wired` x2); the module doc | bind.rs about 530: warn |
| `crates/holler-cli/tests/pane_integration/bind.rs`, `wired.rs`, `main.rs`, new `forwarding.rs`; `scratch/verbs.rs` | 693, 142, 126; 146 | W1 to W6; `mod forwarding;`; the scenario end state (`assert_end_state`, E44) | yes |
| `crates/holler-cli/tests/pane_verbs/doctor.rs`, `doctor/rig.rs`, `doctor/read_only.rs`, new `doctor/attach.rs` and `doctor/agent.rs` | 562, 449, 147 | two `mod` lines; the client in `make_live`; the allowlist; the two rule suites | yes |
| `crates/holler-cli/tests/pane_claude/rig.rs` | 482 | the client in `seed_claude` and `seed_opencode`; `LinkedHost`'s forward | yes |
| test doubles that gain a one-line forward | | `pane_verbs/launch/rig.rs` (852), `profile_verbs/apply/rig.rs` (776), **`pane_verbs/switch.rs` (893)**, `pane_integration/bind.rs`, `pane_integration/observe.rs`, `pane_verbs/doctor/surface.rs` (601), `pane_verbs/import/harness.rs` (341; `LostChain` is #835's), `holler-pane/tests/claude_kind_test.rs` (300) | **`switch.rs` is already at 893 with two harness doubles that each need the new method (about nine lines each after `rustfmt`), so F first moves the two doubles to a sibling file `pane_verbs/switch/doubles.rs` in a commit of their own, a pure move; `launch/rig.rs` takes about thirteen lines (865).** |
| `crates/holler-cli/tests/docs_pane_control_test.rs` | 610 | none expected (it reads the code's tables) | yes |
| `docs/adr/ADR-0021.md`, `docs/pane-control.md`, `CHANGELOG.md` | 1604, 1008, 1079 | the amendment; section 5; one bullet at the top of `### Enhancements` (E45), linking /issues/647 | docs |

Not edited: `crates/holler-pane/src/profile_diff.rs` (D12), `crates/holler-cli/src/pane/doctor.rs` (generic over kinds), `crates/holler-cli/tests/fixtures/cli-surface.txt`, the hub and proto crates, and the `Ports` struct (this story adds methods, not a port, so it adds no field to the `Ports` literals #839 changes), and `crates/holler-pane/src/ports/process.rs` (the endpoint type, unchanged).

## Depends on

Verified with `gh issue view` and `git log origin/main` on 2026-10-12: closed and on `main`: #637, #638, #642 (the OpenCode adapter and the agent), #644 (launch, relaunch, `--agent`, the `SpecField` row), #645, #649 (the wiring), #684 (the kit), #700 (`Pane.opencode_agent`), #802, #808, #832, part 1 of #647 (PR #701), **#835** (PR #860, `4e691d9`: import leaves panes alone; the required-method rule), **#665** (PR #858), **#667** (`0f5835e`), **#648** (PR #856), **#853** (PR #861, `6eff9cc`). **#831** (PR #857, `b76f4d2`: the Claude Code harness, `HarnessEndpoint`) is merged, and the issue stays open for its adoption step, `say --pane` and a real-binary test, none of which this story touches. #654 (the live cutover, operator-run) is open: see Q2. Open and not needed: #839, #803.

## Clashes with stories in flight

Read-only look on 2026-10-12. The first text of this section (2026-10-11) compared the story with #835, #831, #667, #665 and #648; all five are now merged and are no longer clashes, they are the code the story is built on (D2). What is left:

| Story | Shared files and hunks | What it means here |
|---|---|---|
| **#839** DRIVEN follows the session of record (brief at `1e1f203`, no code; its worktree has a refreshed brief, unmerged) | `ports.rs` only for a new port `HubPort` (this story adds methods to existing traits, so it adds no field to the 15 `Ports` literals #839 changes); `reconcile.rs`, `reconcile/observe.rs` (reads `hub.advertised()` once per pass, records `driven`, adds a finding); `findings.rs`; `pane_integration/bind.rs`; ADR-0021 sections 2, 5, 8, 11 and the "Deferred" DRIVEN bullet; `CHANGELOG.md`; `docs/pane-control.md` sections 3, 4, 10, 12 (E42, E43) | Mechanical. No dependency either way: D6 uses the session of record, which stays right when DRIVEN is observed (I2). The chain's call bound (D13) is edited by both: whoever merges second sums them. The `ALL` pin (D17) and the `CODES` list are appended to by both: whoever merges second appends after the other's kinds. |
| **#803** the wizard builds panes through `profile apply` (brief refreshed, unmerged) | `CHANGELOG.md` and the guide at most (the wizard and `profile apply` are not in the Blast radius) | None. |

**Merge order.** The five stories the first text ordered ahead of or behind this one have merged. Of the two that remain, nothing in the code depends on the order of #839 and this story, which is the operator's to swap; whichever lands second re-reads the other's hunks of the shared files above. Shared ADR-0021 and CHANGELOG hunks resolve mechanically: each story adds a bullet at the top of `### Enhancements`, a row in the section 2 table, a "Decisions taken" item (renumber to the next free number), and the "Deferred" list is appended to by this story and #839.

## Pipeline

`rigor: second-opinion` (D18). Phase order from CLAUDE.md: O, A (this brief), T (author, RED), F, T (verify, GREEN), A (anti-duplication), S. `uiSurface: false`. The GitHub issue is the source of truth; this brief is the plan the run is given (`briefPath`). The brief gate is run on this brief; the earlier PASS (r1 of 2026-10-11 02:22 MDT, which passed part 1's brief) is not carried forward.

**The pipeline script cannot write a different handoff directory.** The release `claude-pipeline-v0.1.7` (`.aftersight/releases/claude/workflow/coding-pipeline.workflow.mjs`) fixes the run's handoff directory as `docs/handoffs/<issueNumber>` (E46, E47); there is no `slug`, `handoffDir` or `handoffRoot` argument. The brief gate's files are named `docs/handoffs/<issueNumber>-brief-sidecar-r<N>.md` and `...-brief-result-r<N>.md`, and the diff gate's the same with `diff` (E48). Only `briefPath` is an argument. The branch is `issue-<N>-implementation` and the PR is `Implements #<N>` with body `Closes #<N>.` (E50). `issueNumber` must be an integer. So a run with `issueNumber: 647` writes into **part 1's record** (`docs/handoffs/647/`, and the loose `docs/handoffs/647-brief*` files) and its role phases read every file there as a prior phase's handoff. The earlier attempt did exactly that.

**Launch arguments** (the rest as CLAUDE.md gives them): `issueNumber: 647`, `briefPath: 'docs/handoffs/647-part2-brief.md'`, `uiSurface: false`, `rigor: 'second-opinion'`, `models: { F: 'claude-opus-5-5' }`, `removeHandoffsBeforePr` **not set** (its cleanup does `git rm -r docs/handoffs/647 docs/handoffs/647-*`, which would delete this story's record and part 1's: E49). Run from this worktree: it is a fresh worktree on `issue-647-implementation`, with `origin/main` at `6eff9cc` (#835, #831, #853 merged) merged in and the brief and the slug directory committed on it.

**Least-bad route: move part 1 aside before the run, and move the run's files into the slug after it.** Both steps are commits on the run's branch; the squash merge shows only the net change.
1. **Before the launch** (one commit, `chore(handoffs): set part 1's record aside for the part-2 run (#647)`): `mkdir docs/handoffs/647-part1-record`; `git mv docs/handoffs/647/* docs/handoffs/647-part1-record/` (the directory itself disappears); `git mv docs/handoffs/647-brief.md docs/handoffs/647-brief-result-r1.md docs/handoffs/647-brief-result-r1.md.prompt.txt docs/handoffs/647-brief-result-r1.md.usage.json docs/handoffs/647-brief-sidecar-r1.md.prompt.txt docs/handoffs/647-part1-record/`. Now `docs/handoffs/647/` is empty for the run, and no `647-brief*` file exists to be overwritten. `docs/handoffs/647-part2-brief.md` and `docs/handoffs/647-part2/` stay.
2. **The run** writes `docs/handoffs/647/*`, `docs/handoffs/647-brief-*` and `docs/handoffs/647-diff-*`.
3. **After the run, before the merge** (one commit, `chore(handoffs): file the part-2 run under 647-part2 and restore part 1's record (#647)`): `git mv docs/handoffs/647/* docs/handoffs/647-part2/` (no name collides: the earlier attempt is in `647-part2/earlier-attempt/`); `git mv docs/handoffs/647-brief-* docs/handoffs/647-diff-* docs/handoffs/647-part2/` for whichever exist; `git mv docs/handoffs/647-part1-record/* docs/handoffs/647/` and the five loose part-1 files back to `docs/handoffs/`; `git rm -r` the empty `647-part1-record`. Check: `git diff --stat origin/main -- docs/handoffs/647 docs/handoffs/647-brief.md docs/handoffs/647-brief-result-r1.md` is empty.
4. A stopped run is resumed from the worktree and its history, and a resume before step 3 finds the run's files in `docs/handoffs/647/` where the script expects them.

Rejected: a symlink `docs/handoffs/647 -> 647-part2` (git refuses a pathspec beyond a symbolic link, `git add -A` would work but any role that stages a path would fail, and a symlink must not reach `main`); a new issue number for the part (the issue is #647, `Closes #647.` is right for the whole story, and the commits would cite another number); editing the release (it is pinned and not this repository's).

After the script opens the PR, the run's own agent checks the body against `CONTRIBUTING.md` and adds the AI disclosure with `gh pr edit`, verifies CI and the gates, and merges (squash, branch deleted) as CLAUDE.md says. If the PR does not deliver both findings, the agent edits `Closes #647.` to `Part of #647.`

## Rules for this story

See the epic (#633): the contract section there is fixed except by its amend-first rule, which this story follows (D3; the ADR first). Issue first, tests first, Holler's pipeline and hooks apply (`bash scripts/setup-hooks.sh` in a fresh clone; branch `issue-647-implementation`, fresh worktree, Conventional Commits with the `Co-Authored-By` trailer, `cargo fmt` on the files touched and never `cargo fmt --all`, clippy clean, no new `unsafe`, files under 900 lines). Edit only the files in the Blast radius. No test touches a live fleet, a running pane or a real Herdr session: verification is the fakes, the hermetic stub, the fixtures and temporary directories; the opt-in real tests run only on scratch instances they start. The repository is public: no personal host name, tailnet name, address or account name, and not the name of the operator's own attach tool, in code, tests, fixtures, the ADR, the guide, handoffs or commit messages. Never print or commit a credential.

## AI assistance

Drafted by Claude (Sonnet 5.5); the maintainer is accountable, per CONTRIBUTING.md. Filed by an AI agent (Claude Code) at the operator's direction. The pull request body carries the disclosure `CONTRIBUTING.md` asks for.

## Facts measured outside the repository

Measured on 2026-10-11 with OpenCode 1.19.0 (the installed CLI), in a temporary directory with `HOME` and the `XDG_*` variables pointed at it, so no real configuration was read; no server was started. They are the fixtures' source and are not claims about `GET /agent`'s wire shape until the opt-in real test confirms them.

- `opencode agent list` prints, per agent, `name (mode)` and a JSON list of permission rules `{"permission", "action", "pattern"}`. The built-ins are `build`, `compaction`, `explore`, `general`, `plan`, `summary`, `title`.
- `opencode debug agent NAME` prints one agent's info as JSON with the keys `name`, `description`, `options`, `permission` (the rule list), `mode`, `native`, `tools`. `hidden` is absent unless set.
- With a project `opencode.json` of `{"agent":{"build":{"permission":{"task":"deny"}},"orchestrator":{"mode":"primary","description":"x","permission":{"task":"allow"}}}}`: `build` is `mode: primary`, `native: true`, its rules end with `{"permission":"task","action":"deny","pattern":"*"}` after `{"permission":"*","action":"allow","pattern":"*"}`, and `tools.task` is `false`; `orchestrator` is `mode: primary`, `native: false`, its rules end with `{"permission":"task","action":"allow","pattern":"*"}`, and `tools.task` is `true`. With no project config, `build` has no `task` rule (so it can dispatch) and `plan` has `{"permission":"task","action":"deny","pattern":"general"}` (a deny for one subagent, not for all).
- The older SDK type definitions (version 1.14.39, a different shape for `permission`) give a user message an `agent: string` field. **Not verified on 1.19.0:** that field on a message of `GET /session/:id/message`; and that `GET /agent` serves the same info as `debug agent`. The first fixtures are therefore cut from the opt-in real run (D15), and the fixtures above are the starting point for the rule tests.
- To regenerate the agent fixtures: run the two `opencode debug agent` commands above in such a directory with the config shown and save the JSON, trimming `prompt` and every `external_directory` rule that names a path.

## Evidence (verbatim, from `main` at `6eff9cc`, merged into this branch)

Each excerpt is cut by the command shown; the line numbers on the left come from `awk`, not from typing. An excerpt marked in flight (E42 and E43, #839's brief) is cut from a named commit of that story's branch (`git show <sha>:<file>`), so it is stable but is not `main`'s; every other excerpt is `main`'s at `6eff9cc`. Run the commands from the root of a worktree of this repository (the `.aftersight/releases/claude` link must exist for the pipeline excerpts) with that commit available.

**E1.** The module doc of `ports.rs` on main lists five amendments (#645, #802, #832, #835, #831); part 2 adds a sixth.

`sed -n '12,41p' crates/holler-pane/src/ports.rs | awk -v n=12 '{printf "%d| %s\n", n+NR-1, $0}'`

````text
12| //! **Frozen when #637 merges**, after which a change goes through the epic's
13| //! amend-first rule. [`HerdrPort`], and the minimal data types it takes and returns,
14| //! stay provisional until the spike #636 (Herdr) reports. [`HarnessPort`] is final:
15| //! the spike #635 (OpenCode) confirmed it, and #642 built it. Amended since:
16| //!
17| //! - #645's two default-armed seam methods, [`HarnessPort::send_prompt`] and
18| //!   [`HarnessPort::session_activity`], whose defaults keep every impl that does not
19| //!   override them as it was;
20| //! - #802's: `serve` became the pure [`HarnessPort::serve_command`], which takes a
21| //!   [`ServeSpec`] and returns the server's argv, beside the required
22| //!   [`HarnessPort::verify_serving`]; and [`HostPort::run`] takes the directory it starts
23| //!   its program in. The host starts and stops a pane's processes, its harness server
24| //!   among them;
25| //! - #832's: [`HerdrPort::observe_pane`] and its batch form [`HerdrPort::observe_panes`],
26| //!   what Herdr says about a pane that no record names;
27| //! - #835's: the required [`HostPort::foreground_commands`], the command name of the
28| //!   foreground process of each pane of a tmux session (how the import tells a pane that
29| //!   runs Claude Code), and [`HerdrPort::observe_pane`] lost its default. Both are required
30| //!   so that the compiler finds every implementation, a wrapper port included: a default
31| //!   answers silently in place of a port a wrapper forgot to forward;
32| //! - #831's, a second harness (ADR-0021 section 15): every [`HarnessPort`] method keyed by a
33| //!   server's `port: u16` is keyed by a [`HarnessEndpoint`] instead (a port, or a single
34| //!   process), the change of the port that #802 decision 2 set as the pattern, so a call
35| //!   site that forgets the endpoint does not compile; `shown_session` and `attach_tui` take
36| //!   one too. Four default-armed methods for a single-process harness, each defaulting to
37| //!   the refusal [`PROCESS_UNSUPPORTED`] ([`HarnessPort::process_command`],
38| //!   [`HarnessPort::new_session_id`], [`HarnessPort::locate_process`],
39| //!   [`HarnessPort::login_status`]); [`ServeSpec`] gains the model and the directory; and
40| //!   [`Activity::WaitingPermission`]. [`HarnessRouter`] is the one router of the two
41| //!   shapes, so [`Ports`] keeps one harness.
````

**E2.** #832's `observe_pane` was default-armed; #835 made it required, for the reason the doc gives: a wrapper that forgot it answered an empty observation, silently. The shape this story follows.

`sed -n '284,295p' crates/holler-pane/src/ports.rs | awk -v n=284 '{printf "%d| %s\n", n+NR-1, $0}'`

````text
284|     /// What Herdr says about the pane `pane` that a record does not hold: its working
285|     /// directory and when its screen last changed (#832). It is an observation (I6), read
286|     /// fresh and never stored; `pane list` and `pane get` show it for a pane no record
287|     /// names.
288|     ///
289|     /// An implementation that cannot tell says so with `None` fields (an empty
290|     /// [`PaneObservation`]) and never guesses. `pane-not-found` when Herdr has no such pane.
291|     ///
292|     /// Required since #835, with no default. The default answered an empty observation, so a
293|     /// wrapper port that did not forward it reported nothing, silently, over the real wiring;
294|     /// now the compiler finds every implementation.
295|     fn observe_pane(&self, pane: &PaneId) -> Result<PaneObservation, PaneError>;
````

**E3.** `HostPort` on main has five methods and no default; the first four are the ones that came before #835.

`sed -n '371,386p' crates/holler-pane/src/ports.rs | awk -v n=371 '{printf "%d| %s\n", n+NR-1, $0}'`

````text
371| pub trait HostPort: Send + Sync {
372|     /// Make the tmux session `name` exist, working in `cwd`.
373|     fn ensure_session(&self, name: &PaneName, cwd: &str) -> Result<(), PaneError>;
374| 
375|     /// Run `argv` (never through a shell) in the session `name`, starting it in `cwd`,
376|     /// which need not be the directory the session was made in (#802: the launch engine
377|     /// passes the pane spec's directory, for the spec's command and for the harness
378|     /// server alike).
379|     fn run(&self, name: &PaneName, cwd: &str, argv: &Argv) -> Result<(), PaneError>;
380| 
381|     /// Stop the processes the session `name` owns: what `run` started there, the pane's
382|     /// harness server among them (#802).
383|     fn stop_owned(&self, name: &PaneName) -> Result<(), PaneError>;
384| 
385|     /// The process ids running in the session `name`.
386|     fn ps(&self, name: &PaneName) -> Result<Vec<u32>, PaneError>;
````

**E4.** `HarnessPort`'s two default-armed methods (#645), the shape this story does not repeat; #831 added four more default-armed methods on top of them (lines 519-559), each defaulting to a refusal, none an answer.

`sed -n '490,517p' crates/holler-pane/src/ports.rs | awk -v n=490 '{printf "%d| %s\n", n+NR-1, $0}'`

````text
490|     /// Queue `text` as a prompt to `session` at `at`, through the harness API, never by
491|     /// typing into a TUI (I4). `pane reset --first` calls it (#645).
492|     ///
493|     /// The default refuses with [`PROMPT_UNSUPPORTED`]: an impl that does not send
494|     /// prompts says so as a stable refusal, which the real adapter replaces (#642).
495|     fn send_prompt(
496|         &self,
497|         at: &HarnessEndpoint,
498|         session: &str,
499|         text: &str,
500|     ) -> Result<(), PaneError> {
501|         let _ = (at, session, text);
502|         Err(PaneError::Refused {
503|             code: PROMPT_UNSUPPORTED,
504|             message: "this harness does not queue a prompt".to_owned(),
505|         })
506|     }
507| 
508|     /// What `session` at `at` is doing now (#645: `pane reset` refuses a conversation that
509|     /// is not [`Activity::Idle`]). A single process holds one session at a time, so its
510|     /// activity is the process's.
511|     ///
512|     /// The default answers [`Activity::Idle`], so an impl that does not report activity
513|     /// refuses nothing.
514|     fn session_activity(&self, at: &HarnessEndpoint, session: &str) -> Result<Activity, PaneError> {
515|         let _ = (at, session);
516|         Ok(Activity::Idle)
517|     }
````

**E5.** #835 made the rule: `HostPort` has no default method, and a doc test (`PidsOnly`) pins that an implementation that omits one does not compile (`compile_fail,E0046`). Part 2's per-method doc tests (acceptance 1) are modelled on it.

`sed -n '341,370p' crates/holler-pane/src/ports.rs | awk -v n=341 '{printf "%d| %s\n", n+NR-1, $0}'`

````text
341| /// The machine a pane lives on: tmux sessions and the processes in them (the adapter
342| /// is `holler-adapter-host`, #641).
343| ///
344| /// **Blocking.** Every method is synchronous. Call from `spawn_blocking` (or a
345| /// thread) in async code. Every method returns within I5's bound (default 10 s) or
346| /// with [`PaneError::Timeout`]. An implementation is `Send + Sync`.
347| ///
348| /// No method has a default (#835), so an implementation that answers only the four methods
349| /// that came before `foreground_commands`, a wrapper port included, does not compile:
350| ///
351| /// ```compile_fail,E0046
352| /// use holler_pane::{Argv, HostPort, PaneError, PaneName};
353| ///
354| /// struct PidsOnly;
355| ///
356| /// impl HostPort for PidsOnly {
357| ///     fn ensure_session(&self, _: &PaneName, _: &str) -> Result<(), PaneError> {
358| ///         Ok(())
359| ///     }
360| ///     fn run(&self, _: &PaneName, _: &str, _: &Argv) -> Result<(), PaneError> {
361| ///         Ok(())
362| ///     }
363| ///     fn stop_owned(&self, _: &PaneName) -> Result<(), PaneError> {
364| ///         Ok(())
365| ///     }
366| ///     fn ps(&self, _: &PaneName) -> Result<Vec<u32>, PaneError> {
367| ///         Ok(Vec::new())
368| ///     }
369| /// }
370| /// ```
````

**E6.** `HostPort::foreground_commands` (#835): names only, one list per session, `pane-not-found` for a missing session. The model for `attached_clients`.

`sed -n '388,397p' crates/holler-pane/src/ports.rs | awk -v n=388 '{printf "%d| %s\n", n+NR-1, $0}'`

````text
388|     /// The command name of the foreground process of every live pane in the session `name`,
389|     /// one per pane, in the order tmux lists them (#835): what tmux reports as
390|     /// `#{pane_current_command}`, the basename of that process's `argv[0]`. Names only, never
391|     /// an argv, a command line or a program's configuration (I7). A session with no pane
392|     /// process answers an empty list; a missing session is `pane-not-found`, as `ps` answers.
393|     ///
394|     /// The import reads it to leave alone a pane that runs Claude Code
395|     /// ([`crate::pane::is_claude_code_command`]), and a Claude Code launch to refuse a session
396|     /// that already runs one (`process-in-use`, #831).
397|     fn foreground_commands(&self, name: &PaneName) -> Result<Vec<String>, PaneError>;
````

**E7.** The end of `FindingKind` and `ALL` on main (16 kinds; the last is `ProfileDrift`, #665; #831 added `PaneWaiting` and `ClaudeLoginMissing` before it). Part 2 appends two after the last kind present at build time.

`sed -n '76,108p' crates/holler-pane/src/findings.rs | awk -v n=76 '{printf "%d| %s\n", n+NR-1, $0}'`

````text
76|     /// A pane's single process (Claude Code) waits for a person: a permission prompt or a
77|     /// question; the message names which (#831). No Holler verb answers one.
78|     PaneWaiting,
79|     /// The processes of the Claude Code panes do not use the subscription login (#831): once
80|     /// per pass that holds such a pane. The message names the login, never a command that
81|     /// reads one, and no Holler verb sets a login (I7).
82|     ClaudeLoginMissing,
83|     /// The pane's record differs from its profile's spec on at least one field of
84|     /// `profile_diff`'s table, the comparison `profile show` makes (#665). The message names
85|     /// the fields and no value. Which side is right is the operator's choice, so no remedy.
86|     ProfileDrift,
87| }
88| 
89| impl FindingKind {
90|     /// Every kind, in declaration order.
91|     pub const ALL: &'static [FindingKind] = &[
92|         FindingKind::HerdrVersionUnsupported,
93|         FindingKind::ObserveFailed,
94|         FindingKind::UnregisteredHerdrPane,
95|         FindingKind::HerdrPaneMissing,
96|         FindingKind::TmuxSessionMissing,
97|         FindingKind::ServerWedged,
98|         FindingKind::ServerDown,
99|         FindingKind::NoSessionOfRecord,
100|         FindingKind::SessionOfRecordMissing,
101|         FindingKind::TuiForeignSession,
102|         FindingKind::ShownDrivenMismatch,
103|         FindingKind::StraySession,
104|         FindingKind::UnmanagedStalled,
105|         FindingKind::PaneWaiting,
106|         FindingKind::ClaudeLoginMissing,
107|         FindingKind::ProfileDrift,
108|     ];
````

**E8.** The one remedy table. `remedy(self, pane, fix)` has an outside caller (`tx_switch.rs`), so part 2 adds `remedy_for` and leaves this signature.

`sed -n '133,171p' crates/holler-pane/src/findings.rs | awk -v n=133 '{printf "%d| %s\n", n+NR-1, $0}'`

````text
133|     /// The command a finding of this kind tells the operator to run, for the pane `pane`
134|     /// (`None` when it is about no pane) once `--fix` left it at `fix`; `None` when no Holler
135|     /// verb repairs it. The one table of remedies:
136|     ///
137|     /// - a read that failed: run the pass again, `holler pane doctor [<pane>]`;
138|     /// - the pane's process or its harness is gone or wrong: `holler pane relaunch <pane>`;
139|     /// - the pane has no usable session of record: `holler pane reset <pane>`, which creates
140|     ///   one (choosing an existing session is the operator's, never a guess);
141|     /// - a mismatch: `holler pane doctor <pane> --fix` while a fix can repair it, relaunch when
142|     ///   it cannot or failed, nothing once it is fixed;
143|     /// - a pane that waits for a person, and a login that is not the subscription's (#831):
144|     ///   none, since only a person answers the one and logs in for the other.
145|     pub fn remedy(self, pane: Option<&PaneName>, fix: FixState) -> Option<String> {
146|         match self {
147|             FindingKind::HerdrVersionUnsupported
148|             | FindingKind::UnregisteredHerdrPane
149|             | FindingKind::UnmanagedStalled
150|             | FindingKind::StraySession
151|             | FindingKind::PaneWaiting
152|             | FindingKind::ClaudeLoginMissing
153|             | FindingKind::ProfileDrift => None,
154|             FindingKind::ObserveFailed => Some(doctor_command(pane, false)),
155|             FindingKind::HerdrPaneMissing
156|             | FindingKind::TmuxSessionMissing
157|             | FindingKind::ServerWedged
158|             | FindingKind::ServerDown
159|             | FindingKind::TuiForeignSession => pane.map(relaunch_command),
160|             FindingKind::NoSessionOfRecord | FindingKind::SessionOfRecordMissing => {
161|                 pane.map(reset_command)
162|             }
163|             FindingKind::ShownDrivenMismatch => match fix {
164|                 FixState::Fixable | FixState::Skipped => {
165|                     pane.map(|pane| doctor_command(Some(pane), true))
166|                 }
167|                 FixState::NotFixable | FixState::Failed => pane.map(relaunch_command),
168|                 FixState::Fixed => None,
169|             },
170|         }
171|     }
````

**E9.** The chain's port-call bound is written in the module doc: seven calls for a server's chain, and a single process's chain (#831) is the child module `process` with its own list. Part 2 makes the server's chain nine and says so; `attached_clients` is called from `check_tmux`, which both chains share, so the single process's list gains it too.

`sed -n '1,10p' crates/holler-pane/src/reconcile/observe.rs | awk -v n=1 '{printf "%d| %s\n", n+NR-1, $0}'`

````text
1| //! One pane's chain in a reconcile pass (#647): observe the pane's tmux session, harness
2| //! server and TUI, compare them with its record, repair a mismatch with `--fix`, and record
3| //! what was observed. A pass runs one chain per pane, each on a thread of its own; the
4| //! parent module has the rules and what the record write means.
5| //!
6| //! A server's chain makes at most seven port calls, each bounded by I5: `ps`, `health`,
7| //! `list_sessions`, `shown_session`, and with `--fix` `select_session` and `shown_session`
8| //! again, then one `cas_put`. A single process's chain (#831) is the child module `process`:
9| //! at most `ps`, `health`, `shown_session` and `session_activity`, then with `--fix` the
10| //! caller's relaunch (a launch engine run, under its own budget) or one `cas_put`.
````

**E10.** `check_pane` is the chain: `check_tmux`, then (for a single-process harness, #831) the child module `process`, otherwise `observe_server`, `observe_screen`, `check_session_of_record`, `check_screen`, `record`. The agent check is a server-shape step, so a Claude Code pane never reaches it.

`sed -n '104,138p' crates/holler-pane/src/reconcile/observe.rs | awk -v n=104 '{printf "%d| %s\n", n+NR-1, $0}'`

````text
104| /// Run the chain of `pane`: observe, compare, repair, record. A single process's chain is
105| /// the child module `process`'s, with `relaunch` as its repair (#831).
106| pub(super) fn check_pane(
107|     ports: Ports<'_>,
108|     request: &ReconcileRequest<'_>,
109|     relaunch: Option<Relaunch<'_>>,
110|     pane: &Pane,
111| ) -> Checked {
112|     let mut findings = Vec::new();
113|     check_tmux(ports, pane, &mut findings);
114|     let at = match pane.harness.endpoint(&pane.name) {
115|         Ok(at) => at,
116|         Err(error) => {
117|             findings.push(observe_failed(pane, PANE_HARNESS, &error));
118|             return Checked {
119|                 findings,
120|                 summary: summary(pane, None, ObservedHealth::Unknown),
121|                 listed: None,
122|             };
123|         }
124|     };
125|     if pane.harness.kind.shape() == Shape::SingleProcess {
126|         return process::check(ports, request, relaunch, pane, &at, findings);
127|     }
128|     let server = observe_server(ports, pane, &at, &mut findings);
129|     let screen = observe_screen(ports, pane, &at, &mut findings);
130|     check_session_of_record(pane, &server, &mut findings);
131|     let screen = check_screen(ports, request, pane, &server, screen, &mut findings);
132|     findings.extend(record(ports, request, pane, &server, &screen));
133|     Checked {
134|         findings,
135|         summary: summary(pane, screen.shown(), server.health()),
136|         listed: server.into_sessions(),
137|     }
138| }
````

**E11.** `check_tmux` reads `host.ps` only; `unattached` extends it.

`sed -n '161,175p' crates/holler-pane/src/reconcile/observe.rs | awk -v n=161 '{printf "%d| %s\n", n+NR-1, $0}'`

````text
161| /// `tmux-session-missing` when the pane's tmux session (named as the pane) is gone.
162| fn check_tmux(ports: Ports<'_>, pane: &Pane, findings: &mut Vec<Finding>) {
163|     match ports.host.ps(&pane.name) {
164|         Ok(_) => {}
165|         Err(PaneError::PaneNotFound { .. }) => {
166|             let message = format!("the tmux session {} does not exist", pane.name);
167|             findings.push(Finding::about(
168|                 FindingKind::TmuxSessionMissing,
169|                 pane,
170|                 message,
171|             ));
172|         }
173|         Err(error) => findings.push(observe_failed(pane, HOST_PS, &error)),
174|     }
175| }
````

**E12.** `check_session_of_record` is the trust rule for the session of record: the agent rule runs only where the healthy server listed it.

`sed -n '235,260p' crates/holler-pane/src/reconcile/observe.rs | awk -v n=235 '{printf "%d| %s\n", n+NR-1, $0}'`

````text
235| /// `no-session-of-record`, or `session-of-record-missing` when the pane's healthy server
236| /// listed its sessions without it.
237| fn check_session_of_record(pane: &Pane, server: &Server, findings: &mut Vec<Finding>) {
238|     let Some(record) = pane.session_of_record.as_deref() else {
239|         let message = "the record names no session of record, and doctor never picks one";
240|         findings.push(Finding::about(
241|             FindingKind::NoSessionOfRecord,
242|             pane,
243|             message.to_owned(),
244|         ));
245|         return;
246|     };
247|     if server.sessions().is_some() && !server.has(record) {
248|         let port = port_text(pane);
249|         let message = format!(
250|             "the harness server on {port} does not have the session of record {}",
251|             quoted(record)
252|         );
253|         let finding = Finding::about(FindingKind::SessionOfRecordMissing, pane, message);
254|         findings.push(
255|             finding
256|                 .with_session(Some(record))
257|                 .with_ports(own_ports(pane)),
258|         );
259|     }
260| }
````

**E13.** Reconcile never infers state from files (I6) and reaches the world only through the ports.

`sed -n '1,12p' crates/holler-pane/src/reconcile.rs | awk -v n=1 '{printf "%d| %s\n", n+NR-1, $0}'`

````text
1| //! The reconcile engine (#647): observe Herdr, tmux and the harness, compare what it saw with
2| //! the registry, repair what the record decides, and report the rest as typed
3| //! [`Finding`]s. It never infers state from files (I6), and it reaches the world only through
4| //! the [`Ports`], so `holler pane doctor` and any later caller (the roster, a periodic loop)
5| //! call [`reconcile`] unchanged.
6| //!
7| //! One pass, over every pane in scope at once (a thread per pane, so a pass takes about as
8| //! long as its slowest pane's chain, not the pane count times it: I5):
9| //!
10| //! 1. **Observe** each pane: `host.ps`, `harness.health`, `harness.list_sessions` and
11| //!    `harness.shown_session`; and once per pass `herdr.version` and `herdr.snapshot`. A
12| //!    healthy server's list is trusted. An unhealthy server's list only tells a wedged
````

**E14.** DRIVEN is not observed on main: reconcile compares SHOWN with the session of record. #839 changes that; this story does not depend on it.

`sed -n '51,55p' crates/holler-pane/src/reconcile.rs | awk -v n=51 '{printf "%d| %s\n", n+NR-1, $0}'`

````text
51| //!   then). `None` with `at > 0` is an observed home screen, or a pane with no TUI.
52| //! - `last_observed.driven` is left as stored. No port observes DRIVEN before #649, so the
53| //!   mismatch compares SHOWN with `session_of_record` ([`shown_differs`]), which I2 makes
54| //!   the session the hub drives.
55| //! - A record is written only when one of those values changed, or on its first
````

**E15.** The read-only allowlist of the doctor pass: part 2 adds exactly `HostOp::AttachedClients` and `HarnessOp::AgentView`.

`sed -n '70,98p' crates/holler-cli/tests/pane_verbs/doctor/read_only.rs | awk -v n=70 '{printf "%d| %s\n", n+NR-1, $0}'`

````text
70|     }
71|     let reads = [
72|         HarnessOp::Health,
73|         HarnessOp::ListSessions,
74|         HarnessOp::ShownSession,
75|     ];
76|     assert!(
77|         calls.harness.iter().all(|op| reads.contains(op)),
78|         "{:?}",
79|         calls.harness
80|     );
81|     let reads = [HerdrOp::Snapshot, HerdrOp::Version];
82|     assert!(
83|         calls.herdr.iter().all(|op| reads.contains(op)),
84|         "{:?}",
85|         calls.herdr
86|     );
87|     assert!(
88|         calls.host.iter().all(|op| *op == HostOp::Ps),
89|         "{:?}",
90|         calls.host
91|     );
92|     let reads = [PaneStoreOp::List, PaneStoreOp::Get, PaneStoreOp::CasPut];
93|     assert!(
94|         calls.panes.iter().all(|op| reads.contains(op)),
95|         "{:?}",
96|         calls.panes
97|     );
98|     assert!(calls.profiles.is_empty(), "{:?}", calls.profiles);
````

**E16.** `findings_test.rs` pins every kind code in order (16 on main) and its module doc names who appended which kind.

`sed -n '1,49p' crates/holler-pane/tests/findings_test.rs | awk -v n=1 '{printf "%d| %s\n", n+NR-1, $0}'`

````text
1| #![allow(clippy::unwrap_used, clippy::expect_used, clippy::panic)] // #647
2| //! The finding vocabulary of reconcile and `holler pane doctor` (#647 AC 25): sixteen kinds
3| //! with stable kebab-case codes, each equal to the kind's JSON form. A code is stable once
4| //! merged (ADR-0021 section 9), so the list is pinned in order; #832 appended
5| //! `unmanaged-stalled`, #831 `pane-waiting` and `claude-login-missing`, and #665
6| //! `profile-drift`, a kind that is also an error code of the closed list, classed a failure.
7| 
8| use std::collections::BTreeSet;
9| 
10| use holler_pane::error::{class_of, is_valid_code, ErrorClass};
11| use holler_pane::findings::FindingKind;
12| 
13| const CODES: [&str; 16] = [
14|     "herdr-version-unsupported",
15|     "observe-failed",
16|     "unregistered-herdr-pane",
17|     "herdr-pane-missing",
18|     "tmux-session-missing",
19|     "server-wedged",
20|     "server-down",
21|     "no-session-of-record",
22|     "session-of-record-missing",
23|     "tui-foreign-session",
24|     "shown-driven-mismatch",
25|     "stray-session",
26|     "unmanaged-stalled",
27|     "pane-waiting",
28|     "claude-login-missing",
29|     "profile-drift",
30| ];
31| 
32| #[test]
33| fn finding_kind_codes_are_stable() {
34|     let codes: Vec<&str> = FindingKind::ALL.iter().map(|k| k.code()).collect();
35|     assert_eq!(codes, CODES, "the kinds, in declaration order");
36| 
37|     let unique: BTreeSet<&str> = codes.iter().copied().collect();
38|     assert_eq!(unique.len(), codes.len(), "a duplicate code: {codes:?}");
39| 
40|     for kind in FindingKind::ALL {
41|         assert!(
42|             is_valid_code(kind.code()),
43|             "{:?} is not kebab-case",
44|             kind.code()
45|         );
46|         let json = serde_json::to_value(kind).unwrap();
47|         assert_eq!(json, kind.code(), "{kind:?} serializes as its code");
48|     }
49| }
````

**E17.** `AgentKey::parse` accepts a leading `-` and capitals, so a remedy must render `--agent=KEY`.

`sed -n '138,153p' crates/holler-pane/src/argv.rs | awk -v n=138 '{printf "%d| %s\n", n+NR-1, $0}'`

````text
138| impl AgentKey {
139|     /// Check `text` and keep it as an agent key: non-empty, and only ASCII letters,
140|     /// digits, `-` and `_` (so no whitespace, newline, `=` or `/`). Anything else is
141|     /// [`AGENT_KEY_INVALID`], in a message that states the rule and not the text.
142|     pub fn parse(text: &str) -> Result<Self, PaneError> {
143|         let allowed = |c: char| c.is_ascii_alphanumeric() || c == '-' || c == '_';
144|         if text.is_empty() || !text.chars().all(allowed) {
145|             return Err(PaneError::Refused {
146|                 code: AGENT_KEY_INVALID,
147|                 message: "an OpenCode agent key must be non-empty and use only ASCII letters, \
148|                           digits, '-' and '_'"
149|                     .to_owned(),
150|             });
151|         }
152|         Ok(Self(text.to_owned()))
153|     }
````

**E18.** `verify_serving` already reads `GET /config` and `GET /agent`, and owns the rule for an agent usable as a default (`mode`, `hidden`). `agent_view` reuses that function, it does not copy it.

`sed -n '75,126p' crates/holler-adapter-opencode/src/server.rs | awk -v n=75 '{printf "%d| %s\n", n+NR-1, $0}'`

````text
75| /// `HarnessPort::verify_serving`. A spec without an agent is `Ok` with nothing sent: its argv
76| /// applies none, so the server keeps its own default. For a spec with `key`, within one
77| /// call's bound, in this order: `GET /config` must answer an object whose `default_agent` is
78| /// `key`, which proves the variable reached the server (a server it did not reach runs as
79| /// `build`, which cannot dispatch subagents); then `GET /agent` must list `key` as an agent
80| /// OpenCode takes as a default ([`usable`]). A server that does not is [`AGENT_UNUSABLE`], in
81| /// one line naming the key and the reason and quoting no reply. A reply that is not the JSON
82| /// a step needs (the web app's page answers an unserved route with a `200`) is `unavailable`.
83| pub(crate) fn verify_serving(
84|     harness: &OpenCodeHarness,
85|     port: u16,
86|     spec: &ServeSpec,
87| ) -> Result<(), PaneError> {
88|     let Some(key) = &spec.agent else {
89|         return Ok(());
90|     };
91|     let key = key.as_str();
92|     let call = harness.call(port, OP_VERIFY_SERVING);
93|     let reply = call.send(GET_CONFIG, GET_CONFIG.label, None)?;
94|     let config = json_of(&reply)
95|         .filter(Value::is_object)
96|         .ok_or_else(|| call.unexpected(GET_CONFIG, &reply, "an object"))?;
97|     if string_at(Some(&config), "default_agent") != Some(key) {
98|         return Err(unusable(format!("the server's default agent is not {key}")));
99|     }
100|     let reply = call.send(GET_AGENT, GET_AGENT.label, None)?;
101|     let agents = json_of(&reply);
102|     match agents.as_ref().and_then(Value::as_array) {
103|         Some(agents) => usable(&call, &reply, agents, key),
104|         None => Err(call.unexpected(GET_AGENT, &reply, "a list of agents")),
105|     }
106| }
107| 
108| /// Whether the `GET /agent` list `agents` holds `key` as an agent OpenCode takes as a default
109| /// agent. That is OpenCode's own rule, which refuses a missing agent, one whose `mode` is
110| /// `subagent` and a hidden one, and nothing else: `primary` and `all` (what an agent defined in
111| /// a config without a `mode` gets) both pass. `hidden` counts only when it is `true`, since
112| /// OpenCode leaves it out otherwise. An element named `key` with no string `mode` is not the
113| /// JSON the step needs: `unavailable`.
114| fn usable(call: &Call, reply: &Reply, agents: &[Value], key: &str) -> Result<(), PaneError> {
115|     let Some(agent) = agents
116|         .iter()
117|         .find(|agent| string_at(Some(agent), "name") == Some(key))
118|     else {
119|         return Err(unusable(format!("no agent named {key}")));
120|     };
121|     match string_at(Some(agent), "mode") {
122|         None => Err(call.unexpected(GET_AGENT, reply, "agents with a string mode")),
123|         Some("subagent") => Err(unusable(format!("agent {key} is a subagent"))),
124|         Some(_) if agent.get("hidden") == Some(&Value::Bool(true)) => {
125|             Err(unusable(format!("agent {key} is hidden")))
126|         }
````

**E19.** The adapter already reads `GET /session/:id/message` (`from_messages`) and parses `info`; `agent_view` shares the reader.

`sed -n '114,130p' crates/holler-adapter-opencode/src/prompt.rs | awk -v n=114 '{printf "%d| %s\n", n+NR-1, $0}'`

````text
114| /// The fallback for a server whose status route answers no JSON object: the last message of
115| /// `session` whose `info.role` is `assistant` is [`Activity::Busy`] while it has no
116| /// `info.time.completed`; a completed one, or none, is [`Activity::Idle`]. A message list
117| /// that is not a `200` JSON array either is `unavailable`.
118| fn from_messages(call: &Call, session: &str) -> Result<Activity, PaneError> {
119|     let path = format!("{}/message", session_path(session));
120|     let reply = call.send(GET_MESSAGES, &path, None)?;
121|     let list = json_of(&reply);
122|     let Some(messages) = list.as_ref().and_then(Value::as_array) else {
123|         return Err(call.unexpected(GET_MESSAGES, &reply, "a list of messages"));
124|     };
125|     let last = messages
126|         .iter()
127|         .filter_map(|message| message.get("info"))
128|         .rfind(|info| string_at(Some(info), "role") == Some("assistant"));
129|     Ok(busy_or_idle(last.is_some_and(|info| !completed(info))))
130| }
````

**E20.** The routes the adapter knows: `GET_AGENT` and `GET_MESSAGES` exist, nothing new to declare.

`sed -n '472,498p' crates/holler-adapter-opencode/src/lib.rs | awk -v n=472 '{printf "%d| %s\n", n+NR-1, $0}'`

````text
472| const GET_AGENT: Route = Route {
473|     method: "GET",
474|     label: "/agent",
475| };
476| const PROMPT_ASYNC: Route = Route {
477|     method: "POST",
478|     label: "/session/:id/prompt_async",
479| };
480| const GET_QUESTION: Route = Route {
481|     method: "GET",
482|     label: "/question",
483| };
484| const GET_PERMISSION: Route = Route {
485|     method: "GET",
486|     label: "/permission",
487| };
488| const GET_MESSAGES: Route = Route {
489|     method: "GET",
490|     label: "/session/:id/message",
491| };
492| 
493| /// One port call in flight: the server's port, the method's `op` and its deadline.
494| struct Call {
495|     port: u16,
496|     op: &'static str,
497|     /// [`Timeouts::call`] from the method's entry (I5).
498|     deadline: Instant,
````

**E21.** The wiring's rule: every wrapper forwards every method, defaults included (D31 W-4, D33). The sentence names the default-armed methods (now also #831's four process methods) and says `observe_pane` and `foreground_commands` have no default (#835); part 2 edits it.

`sed -n '15,23p' crates/holler-cli/src/pane/wiring/bind.rs | awk -v n=15 '{printf "%d| %s\n", n+NR-1, $0}'`

````text
15| //! **Every method is forwarded (D31 W-4, D33).** Each wrapper here overrides every method of its
16| //! port, the ones the port gives a default included: `HarnessPort`'s `send_prompt` and
17| //! `session_activity`, its four process methods (#831: `process_command`, `new_session_id`,
18| //! `locate_process` and `login_status`, whose defaults refuse `process-unsupported`), and
19| //! `HerdrPort`'s `observe_panes` (#832). A default never answers in place of the inner port, and
20| //! `observe_panes` goes to the inner port's own `observe_panes`, so a list of panes stays one read
21| //! of Herdr (ADR-0021 section 14), never one `observe_pane` per pane. `HerdrPort::observe_pane`
22| //! and `HostPort::foreground_commands` have no default (#835), so the compiler holds every wrapper
23| //! to them.
````

**E22.** `BindingHost` on main: five methods forwarded one by one.

`sed -n '168,193p' crates/holler-cli/src/pane/wiring/bind.rs | awk -v n=168 '{printf "%d| %s\n", n+NR-1, $0}'`

````text
168| impl HostPort for BindingHost {
169|     fn ensure_session(&self, name: &PaneName, cwd: &str) -> Result<(), PaneError> {
170|         let offered = self.bindings.claim();
171|         self.inner.ensure_session(name, cwd)?;
172|         if let Some(pane) = offered {
173|             self.bindings.bind(pane, name.clone());
174|         }
175|         Ok(())
176|     }
177| 
178|     fn run(&self, name: &PaneName, cwd: &str, argv: &Argv) -> Result<(), PaneError> {
179|         self.inner.run(name, cwd, argv)
180|     }
181| 
182|     fn stop_owned(&self, name: &PaneName) -> Result<(), PaneError> {
183|         self.inner.stop_owned(name)
184|     }
185| 
186|     fn ps(&self, name: &PaneName) -> Result<Vec<u32>, PaneError> {
187|         self.inner.ps(name)
188|     }
189| 
190|     fn foreground_commands(&self, name: &PaneName) -> Result<Vec<String>, PaneError> {
191|         self.inner.foreground_commands(name)
192|     }
193| }
````

**E23.** `Wired<dyn HarnessPort>` forwards fifteen methods explicitly (every one of `HarnessPort`, #831's four process methods included).

`sed -n '295,379p' crates/holler-cli/src/pane/wiring/bind.rs | awk -v n=295 '{printf "%d| %s\n", n+NR-1, $0}'`

````text
295| impl HarnessPort for Wired<dyn HarnessPort> {
296|     fn serve_command(
297|         &self,
298|         name: &PaneName,
299|         port: u16,
300|         spec: &ServeSpec,
301|     ) -> Result<Argv, PaneError> {
302|         self.port()?.serve_command(name, port, spec)
303|     }
304| 
305|     fn verify_serving(&self, at: &HarnessEndpoint, spec: &ServeSpec) -> Result<(), PaneError> {
306|         self.port()?.verify_serving(at, spec)
307|     }
308| 
309|     fn health(&self, at: &HarnessEndpoint) -> Result<bool, PaneError> {
310|         self.port()?.health(at)
311|     }
312| 
313|     fn create_session(&self, at: &HarnessEndpoint) -> Result<String, PaneError> {
314|         self.port()?.create_session(at)
315|     }
316| 
317|     fn list_sessions(&self, at: &HarnessEndpoint) -> Result<Vec<String>, PaneError> {
318|         self.port()?.list_sessions(at)
319|     }
320| 
321|     fn abort(&self, at: &HarnessEndpoint, session: &str) -> Result<(), PaneError> {
322|         self.port()?.abort(at, session)
323|     }
324| 
325|     fn attach_tui(
326|         &self,
327|         pane: &PaneId,
328|         at: &HarnessEndpoint,
329|         session: &str,
330|     ) -> Result<(), PaneError> {
331|         self.port()?.attach_tui(pane, at, session)
332|     }
333| 
334|     fn select_session(&self, pane: &PaneId, session: &str) -> Result<(), PaneError> {
335|         self.port()?.select_session(pane, session)
336|     }
337| 
338|     fn shown_session(
339|         &self,
340|         pane: &PaneId,
341|         at: &HarnessEndpoint,
342|     ) -> Result<Option<String>, PaneError> {
343|         self.port()?.shown_session(pane, at)
344|     }
345| 
346|     fn send_prompt(
347|         &self,
348|         at: &HarnessEndpoint,
349|         session: &str,
350|         text: &str,
351|     ) -> Result<(), PaneError> {
352|         self.port()?.send_prompt(at, session, text)
353|     }
354| 
355|     fn session_activity(&self, at: &HarnessEndpoint, session: &str) -> Result<Activity, PaneError> {
356|         self.port()?.session_activity(at, session)
357|     }
358| 
359|     fn process_command(
360|         &self,
361|         name: &PaneName,
362|         spec: &ServeSpec,
363|         session: &SessionChoice,
364|     ) -> Result<Argv, PaneError> {
365|         self.port()?.process_command(name, spec, session)
366|     }
367| 
368|     fn new_session_id(&self) -> Result<String, PaneError> {
369|         self.port()?.new_session_id()
370|     }
371| 
372|     fn locate_process(&self, name: &PaneName, session: &str) -> Result<Option<u32>, PaneError> {
373|         self.port()?.locate_process(name, session)
374|     }
375| 
376|     fn login_status(&self) -> Result<LoginStatus, PaneError> {
377|         self.port()?.login_status()
378|     }
379| }
````

**E24.** `Wired<dyn HostPort>` forwards five methods.

`sed -n '381,401p' crates/holler-cli/src/pane/wiring/bind.rs | awk -v n=381 '{printf "%d| %s\n", n+NR-1, $0}'`

````text
381| impl HostPort for Wired<dyn HostPort> {
382|     fn ensure_session(&self, name: &PaneName, cwd: &str) -> Result<(), PaneError> {
383|         self.port()?.ensure_session(name, cwd)
384|     }
385| 
386|     fn run(&self, name: &PaneName, cwd: &str, argv: &Argv) -> Result<(), PaneError> {
387|         self.port()?.run(name, cwd, argv)
388|     }
389| 
390|     fn stop_owned(&self, name: &PaneName) -> Result<(), PaneError> {
391|         self.port()?.stop_owned(name)
392|     }
393| 
394|     fn ps(&self, name: &PaneName) -> Result<Vec<u32>, PaneError> {
395|         self.port()?.ps(name)
396|     }
397| 
398|     fn foreground_commands(&self, name: &PaneName) -> Result<Vec<String>, PaneError> {
399|         self.port()?.foreground_commands(name)
400|     }
401| }
````

**E25.** The existing pin for `WiredHarness`: a recording harness (`Recording`, lines 164-247 of the file; each of its fifteen methods answers what no default would) and `call_every_method` (lines 249-317) are the inputs; this test asserts the answers by index and lists the calls. Part 2 adds `agent_view` to `Recording`, `call_every_method` (appended last, so every index above stays) and the expected-calls list.

`sed -n '319,366p' crates/holler-cli/tests/pane_integration/bind.rs | awk -v n=319 '{printf "%d| %s\n", n+NR-1, $0}'`

````text
319| #[test]
320| fn the_wired_harness_forwards_every_method_the_default_armed_ones_included() {
321|     let inner = Arc::new(Recording::default());
322|     let port: Arc<dyn HarnessPort> = inner.clone();
323|     let wired = WiredHarness::new(Ok(port));
324|     let answers = call_every_method(&wired);
325|     assert_eq!(
326|         answers[10],
327|         Ok("Busy".to_owned()),
328|         "session_activity is the inner one's, not the default Idle"
329|     );
330|     assert_eq!(
331|         answers[9],
332|         Ok("send_prompt".to_owned()),
333|         "send_prompt is the inner one's, not the default prompt-unsupported"
334|     );
335|     assert_eq!(
336|         answers[11..],
337|         [
338|             Ok("process_command".to_owned()),
339|             Ok("new_session_id".to_owned()),
340|             Ok("Some(4242)".to_owned()),
341|             Ok("Missing".to_owned()),
342|         ],
343|         "#831's four process methods are the inner one's, not the process-unsupported defaults"
344|     );
345|     assert_eq!(
346|         *inner.calls.lock().unwrap(),
347|         vec![
348|             "serve_command",
349|             "verify_serving",
350|             "health",
351|             "create_session",
352|             "list_sessions",
353|             "abort",
354|             "attach_tui",
355|             "select_session",
356|             "shown_session",
357|             "send_prompt",
358|             "session_activity",
359|             "process_command",
360|             "new_session_id",
361|             "locate_process",
362|             "login_status",
363|         ],
364|         "each method reached the inner harness once"
365|     );
366| }
````

**E26.** How #849-era review pinned `observe_pane` through `BindingHerdr` and `LazyHerdr`: the conformance suite over the wrapper, one recording check per wrapper, the failed-build case.

`sed -n '1,14p' crates/holler-cli/tests/pane_integration/observe.rs | awk -v n=1 '{printf "%d| %s\n", n+NR-1, $0}'`

````text
1| //! D33 (#832; S's defect 1, A-dup's W-5): the wiring's two Herdr wrappers, [`BindingHerdr`]
2| //! and [`LazyHerdr`], delegate `HerdrPort::observe_pane` and `observe_panes`, the two
3| //! default-armed methods #832 added, as ADR-0021 section 14 asks of a wrapper port ("A
4| //! wrapper port must delegate it") and D31 W-4 asks of every delegating impl.
5| //!
6| //! - The Herdr conformance suite runs over `LazyHerdr` as well (the run over `BindingHerdr`
7| //!   is in `bind.rs`), so #832's case 13 (`observe_pane` of an unknown or a closed pane is
8| //!   `pane-not-found`) covers both wrappers.
9| //! - One recording check per wrapper: `observe_pane` answers the inner port's observation,
10| //!   and `observe_panes` reaches the inner `observe_panes` once and the inner `observe_pane`
11| //!   not at all, so the list stays section 14's one call to Herdr.
12| //! - A `LazyHerdr` whose build failed answers one empty observation per pane from
13| //!   `observe_panes`, the trait's contract for a failed call; its `observe_pane` answers the
14| //!   stored error (pinned in `bind.rs`'s every-method test).
````

**E27.** #835 added the same pin for the host: the host conformance suite over `WiredHost`.

`sed -n '413,420p' crates/holler-cli/tests/pane_integration/bind.rs | awk -v n=413 '{printf "%d| %s\n", n+NR-1, $0}'`

````text
413| /// #835 (A's W6): the production host is `BindingHost` over `WiredHost` over the adapter, so
414| /// `WiredHost` forwards every `HostPort` method too; the suite's `foreground_commands` case
415| /// pins the method #835 adds.
416| #[test]
417| fn the_wired_host_passes_the_host_conformance_suite() {
418|     let result = run_host_conformance(|| (WiredHost::new(Ok(Arc::new(FakeHost::new()))), ()));
419|     assert_eq!(result, Ok(()));
420| }
````

**E28.** #835 added its method to the misconfigured-socket list in `wired.rs`, and #831 keyed the harness calls by endpoint; part 2 adds its two (`host.attached_clients`, `harness.agent_view`).

`sed -n '104,118p' crates/holler-cli/tests/pane_integration/wired.rs | awk -v n=104 '{printf "%d| %s\n", n+NR-1, $0}'`

````text
104|         let wiring = Wiring::connect().expect("connect needs only the hub (D2, D26)");
105|         let ports = wiring.ports();
106|         let name = PaneName::parse("hlr649-c1r1").unwrap();
107|         let pane = PaneId::new("w1:p1");
108|         [
109|             usage_naming_the_socket("host.ensure_session", ports.host.ensure_session(&name, "/tmp")),
110|             usage_naming_the_socket("host.ps", ports.host.ps(&name)),
111|             usage_naming_the_socket("host.foreground_commands", ports.host.foreground_commands(&name)),
112|             usage_naming_the_socket("harness.health", ports.harness.health(&HarnessEndpoint::Port(48199))),
113|             usage_naming_the_socket("harness.shown_session", ports.harness.shown_session(&pane, &HarnessEndpoint::Port(48199))),
114|         ]
115|         .into_iter()
116|         .flatten()
117|         .collect::<Vec<_>>()
118|     });
````

**E29.** `Unwired` is the not-implemented port set; every new required method needs an arm here (the four process methods answer `not-implemented` too, never their `process-unsupported` defaults, #831).

`sed -n '347,442p' crates/holler-cli/src/pane/wiring.rs | awk -v n=347 '{printf "%d| %s\n", n+NR-1, $0}'`

````text
347| impl HostPort for Unwired {
348|     fn ensure_session(&self, _name: &PaneName, _cwd: &str) -> Result<(), PaneError> {
349|         Err(PaneError::NotImplemented)
350|     }
351| 
352|     fn run(&self, _name: &PaneName, _cwd: &str, _argv: &Argv) -> Result<(), PaneError> {
353|         Err(PaneError::NotImplemented)
354|     }
355| 
356|     fn stop_owned(&self, _name: &PaneName) -> Result<(), PaneError> {
357|         Err(PaneError::NotImplemented)
358|     }
359| 
360|     fn ps(&self, _name: &PaneName) -> Result<Vec<u32>, PaneError> {
361|         Err(PaneError::NotImplemented)
362|     }
363| 
364|     fn foreground_commands(&self, _name: &PaneName) -> Result<Vec<String>, PaneError> {
365|         Err(PaneError::NotImplemented)
366|     }
367| }
368| 
369| impl HarnessPort for Unwired {
370|     fn serve_command(
371|         &self,
372|         _name: &PaneName,
373|         _port: u16,
374|         _spec: &ServeSpec,
375|     ) -> Result<Argv, PaneError> {
376|         Err(PaneError::NotImplemented)
377|     }
378| 
379|     fn verify_serving(&self, _at: &HarnessEndpoint, _spec: &ServeSpec) -> Result<(), PaneError> {
380|         Err(PaneError::NotImplemented)
381|     }
382| 
383|     fn health(&self, _at: &HarnessEndpoint) -> Result<bool, PaneError> {
384|         Err(PaneError::NotImplemented)
385|     }
386| 
387|     fn create_session(&self, _at: &HarnessEndpoint) -> Result<String, PaneError> {
388|         Err(PaneError::NotImplemented)
389|     }
390| 
391|     fn list_sessions(&self, _at: &HarnessEndpoint) -> Result<Vec<String>, PaneError> {
392|         Err(PaneError::NotImplemented)
393|     }
394| 
395|     fn abort(&self, _at: &HarnessEndpoint, _session: &str) -> Result<(), PaneError> {
396|         Err(PaneError::NotImplemented)
397|     }
398| 
399|     fn attach_tui(
400|         &self,
401|         _pane: &PaneId,
402|         _at: &HarnessEndpoint,
403|         _session: &str,
404|     ) -> Result<(), PaneError> {
405|         Err(PaneError::NotImplemented)
406|     }
407| 
408|     fn select_session(&self, _pane: &PaneId, _session: &str) -> Result<(), PaneError> {
409|         Err(PaneError::NotImplemented)
410|     }
411| 
412|     fn shown_session(
413|         &self,
414|         _pane: &PaneId,
415|         _at: &HarnessEndpoint,
416|     ) -> Result<Option<String>, PaneError> {
417|         Err(PaneError::NotImplemented)
418|     }
419| 
420|     // The four process methods answer `not-implemented` too, never their `process-unsupported`
421|     // defaults (#831): an unwired port is not a server harness.
422|     fn process_command(
423|         &self,
424|         _name: &PaneName,
425|         _spec: &ServeSpec,
426|         _session: &SessionChoice,
427|     ) -> Result<Argv, PaneError> {
428|         Err(PaneError::NotImplemented)
429|     }
430| 
431|     fn new_session_id(&self) -> Result<String, PaneError> {
432|         Err(PaneError::NotImplemented)
433|     }
434| 
435|     fn locate_process(&self, _name: &PaneName, _session: &str) -> Result<Option<u32>, PaneError> {
436|         Err(PaneError::NotImplemented)
437|     }
438| 
439|     fn login_status(&self) -> Result<LoginStatus, PaneError> {
440|         Err(PaneError::NotImplemented)
441|     }
442| }
````

**E30.** `SpecField::ALL` already has 16 fields and `OpencodeAgent` is one (#644): the `SpecField` row of the plan-check finding W-7 is done.

`sed -n '40,70p' crates/holler-pane/src/profile_diff.rs | awk -v n=40 '{printf "%d| %s\n", n+NR-1, $0}'`

````text
40|     Provider,
41|     ModelId,
42|     Effort,
43|     OpencodeAgent,
44|     Role,
45|     Env,
46|     ContextSoft,
47|     ContextHard,
48|     Command,
49|     Check,
50|     Expect,
51| }
52| 
53| impl SpecField {
54|     /// Every field, in the order `profile show` lists a spec and [`diff_spec`] compares
55|     /// it. `harness.port_policy` is compared like every other field: the live side is
56|     /// the policy of the port the pane's harness uses (ADR-0021 section 3).
57|     pub const ALL: [SpecField; 16] = [
58|         SpecField::Workspace,
59|         SpecField::Grid,
60|         SpecField::Cwd,
61|         SpecField::HarnessKind,
62|         SpecField::PortPolicy,
63|         SpecField::Provider,
64|         SpecField::ModelId,
65|         SpecField::Effort,
66|         SpecField::OpencodeAgent,
67|         SpecField::Role,
68|         SpecField::Env,
69|         SpecField::ContextSoft,
70|         SpecField::ContextHard,
````

**E31.** ADR-0021 'Decisions taken' items 16 to 18: a relaunch keeps the recorded agent, `--agent` only sets one, and `opencode_agent` is a `SpecField` (the plan-check finding W-7 is settled by #644).

`sed -n '1542,1547p' docs/adr/ADR-0021.md | awk -v n=1542 '{printf "%d| %s\n", n+NR-1, $0}'`

````text
1542| 16. **`pane relaunch` without `--agent` keeps the recorded agent** (operator, 2026-10-11, #644): a relaunch never changes
1543|     the agent silently.
1544| 17. **`--agent` only sets an agent** (operator, 2026-10-11, #644): no flag clears one; removing an agent is a separate
1545|     decision and is not planned.
1546| 18. **`opencode_agent` is a field of `profile_diff`'s table** (operator, 2026-10-11, #644, section 3): `SpecField::ALL` has
1547|     16 fields, so `profile show` and `profile apply` (#664) see an agent difference. #644 owns the field.
````

**E32.** ADR-0021 section 14 says `observe_pane` is a contract, not a default, since #835: "a wrapper port by forwarding it, and one that leaves it out does not compile".

`sed -n '1116,1122p' docs/adr/ADR-0021.md | awk -v n=1116 '{printf "%d| %s\n", n+NR-1, $0}'`

````text
1116| - **`HerdrPort::observe_pane(pane) -> PaneObservation { cwd?, last_screen_change? }`** is the one new port method. It has a
1117|   contract, not a default, since #835: every implementation answers it, a wrapper port by forwarding it, and one that leaves it
1118|   out does not compile. (#832 gave it a default, `Ok` with both `None`, which would answer silently in place of a wrapper that
1119|   did not forward it; #649's wrappers forward it.) An implementation that cannot tell answers `Ok` with both `None`;
1120|   `pane-not-found` for a pane Herdr does not have. `observe_panes(&[PaneId])`, which keeps its default, is the same
1121|   for several panes at once (an empty observation for a pane that cannot be observed), so a list of many unmanaged panes costs
1122|   one call: the Herdr adapter reads every `cwd` from the `PaneInfo` of **one** `session.snapshot` (any tab, grid cell or not).
````

**E33.** #835's ADR decision 31: `HerdrPort::observe_pane` and `HostPort::foreground_commands` are required, for the reason of the wrapper that did not forward a default.

`sed -n '1587,1590p' docs/adr/ADR-0021.md | awk -v n=1587 '{printf "%d| %s\n", n+NR-1, $0}'`

````text
1587| 31. **`HerdrPort::observe_pane` is required** (operator, 2026-10-11, #835, section 14): the operator folded this change to
1588|     #832's method into #835, which makes `HostPort::foreground_commands` required for the same reason. A wrapper port that
1589|     does not forward a defaulted method answers the default silently over the real wiring, where a required method is a
1590|     compile error.
````

**E34.** A pane's TUI is an `opencode attach` that runs inside the pane's tmux session; nothing in Holler attaches a tmux client to a Herdr pane.

`sed -n '1,8p' crates/holler-adapter-opencode/src/attach.rs | awk -v n=1 '{printf "%d| %s\n", n+NR-1, $0}'`

````text
1| //! The TUI side of `HarnessPort` (#642): `attach_tui`, `select_session` and
2| //! `shown_session`, to which the crate root delegates in one line each, as it delegates
3| //! the server's argv to `server`. A pane's TUI is an `opencode attach` that runs in the
4| //! pane's tmux session (the one `tui_session` names). The adapter reaches it only through
5| //! tmux, by the calls [`crate::tui`] builds, and reads what it shows only from its terminal
6| //! title.
7| //!
8| //! - **One deadline per method**, taken at its entry: every tmux call and every request of
````

**E35.** The guide: one Herdr pane shows one tmux session, named as the pane.

`sed -n '43,47p' docs/pane-control.md | awk -v n=43 '{printf "%d| %s\n", n+NR-1, $0}'`

````text
43| One Herdr pane shows one tmux session, named as the pane. The harness server (OpenCode) runs **inside that tmux
44| session**: the host adapter starts it from the argv the harness adapter builds, and stops it with everything else the
45| session owns, by `TERM` to every owned process group, one shared grace (2 s by default), then `KILL` to every group that
46| still has a member. A launch is a transaction: it makes the Herdr pane and the tmux session, starts and checks the
47| server, creates the session of record over the harness API, attaches the TUI to it, observes that the TUI shows it, and
````

**E36.** The docs test forces `docs/pane-control.md` section 5 to name every `FindingKind::ALL` code.

`sed -n '173,183p' crates/holler-cli/tests/docs_pane_control_test.rs | awk -v n=173 '{printf "%d| %s\n", n+NR-1, $0}'`

````text
173| #[test]
174| fn section_5_names_every_finding_kind() {
175|     let s5 = section(&guide(), 5);
176|     let missing = FindingKind::ALL
177|         .iter()
178|         .map(|k| ticked(k.code()))
179|         .filter(|t| !s5.contains(t))
180|         .collect();
181|     assert_none("section 5 (doctor) does not name these finding kinds", missing);
182| }
183| 
````

**E37.** `FakeHost` on main: sessions, `ps`, `foreground_commands` (#835), `faults.enter(HostOp::..)` on every method.

`sed -n '190,226p' crates/holler-pane-testkit/src/host.rs | awk -v n=190 '{printf "%d| %s\n", n+NR-1, $0}'`

````text
190| impl HostPort for FakeHost {
191|     fn ensure_session(&self, name: &PaneName, cwd: &str) -> Result<(), PaneError> {
192|         self.faults.enter(HostOp::EnsureSession)?;
193|         self.lock()
194|             .sessions
195|             .entry(name.clone())
196|             .or_insert_with(|| Session {
197|                 cwd: cwd.to_owned(),
198|                 processes: Vec::new(),
199|             });
200|         Ok(())
201|     }
202| 
203|     fn run(&self, name: &PaneName, cwd: &str, argv: &Argv) -> Result<(), PaneError> {
204|         self.faults.enter(HostOp::Run)?;
205|         self.lock().start(name, cwd, argv)
206|     }
207| 
208|     fn stop_owned(&self, name: &PaneName) -> Result<(), PaneError> {
209|         self.faults.enter(HostOp::StopOwned)?;
210|         if let Some(session) = self.lock().sessions.get_mut(name) {
211|             session.processes.clear();
212|         }
213|         Ok(())
214|     }
215| 
216|     fn ps(&self, name: &PaneName) -> Result<Vec<u32>, PaneError> {
217|         self.faults.enter(HostOp::Ps)?;
218|         self.lock().each_process(name, |process| process.pid)
219|     }
220| 
221|     fn foreground_commands(&self, name: &PaneName) -> Result<Vec<String>, PaneError> {
222|         self.faults.enter(HostOp::ForegroundCommands)?;
223|         self.lock()
224|             .each_process(name, |process| process.command.clone())
225|     }
226| }
````

**E38.** `HarnessOp` of the fake harness lists the ports' calls; the default-armed two are not there.

`sed -n '49,60p' crates/holler-pane-testkit/src/harness.rs | awk -v n=49 '{printf "%d| %s\n", n+NR-1, $0}'`

````text
49| pub enum HarnessOp {
50|     ServeCommand,
51|     VerifyServing,
52|     Health,
53|     CreateSession,
54|     ListSessions,
55|     Abort,
56|     AttachTui,
57|     SelectSession,
58|     ShownSession,
59| }
60| 
````

**E39.** The file-size gate: fails at 900 lines for `crates/**/*.rs`.

`sed -n '43,52p' scripts/lint.sh | awk -v n=43 '{printf "%d| %s\n", n+NR-1, $0}'`

````text
43| # 4. File-size gate: warn at 600 lines, fail at 900 (skill rule: never let a
44| #    file cross 1k unnoticed).
45| while read -r n f; do
46|   if [ "$n" -ge 900 ]; then
47|     echo "lint: $f is $n lines — decompose before merging"
48|     fail=1
49|   elif [ "$n" -ge 600 ]; then
50|     echo "warn: $f is $n lines"
51|   fi
52| done < <(find crates -name '*.rs' -not -path '*/target/*' -exec wc -l {} + | grep -v ' total$')
````

**E40.** An outside caller of `FindingKind::remedy`.

`sed -n '362,373p' crates/holler-pane/src/tx_switch.rs | awk -v n=362 '{printf "%d| %s\n", n+NR-1, $0}'`

````text
362| fn check_health(ports: Ports<'_>, record: &Pane, at: &HarnessEndpoint) -> Result<(), PaneError> {
363|     if ports.harness.health(at)? {
364|         return Ok(());
365|     }
366|     let pane = Some(&record.name);
367|     let remedy = match FindingKind::ServerDown.remedy(pane, FixState::NotFixable) {
368|         Some(remedy) => remedy,
369|         None => doctor_command(pane, false),
370|     };
371|     let state = match at {
372|         HarnessEndpoint::Port(port) => {
373|             format!(
````

**E41.** #831 keyed every `HarnessPort` method by `HarnessEndpoint` (section 15 item 3): a harness method is written with `at: &HarnessEndpoint`, as every other.

`sed -n '1199,1209p' docs/adr/ADR-0021.md | awk -v n=1199 '{printf "%d| %s\n", n+NR-1, $0}'`

````text
1199| 3. **The endpoint.** A Claude Code process has no port. `holler_pane::ports` gains
1200|    `enum HarnessEndpoint { Port(u16), Process { pane: PaneName, pid: u32 } }`, and **every `HarnessPort` method keyed by
1201|    `port: u16` is keyed by `&HarnessEndpoint` instead** (`verify_serving`, `health`, `create_session`, `list_sessions`,
1202|    `abort`, `send_prompt`, `session_activity`; `attach_tui` and `shown_session` gain it too, `shown_session(pane, endpoint)`,
1203|    since a single process is read through its endpoint and not through a TUI). This changes the port rather than adding
1204|    default-armed twins, as #802 decided (decision 2 there): a call site that forgets the endpoint is a compile error, and
1205|    every implementation is in this workspace. `HarnessInfo.port` becomes `Option<u16>` (`#[serde(default,
1206|    skip_serializing_if = "Option::is_none")]`), so every stored `opencode` record reads and writes the same bytes. A record
1207|    is valid only when `opencode` has a `port`, and `claude` has no `port` and a `pid`; anything else does not load
1208|    (`deny_unknown_fields` and the fail-closed registry, section 7). `HarnessInfo::endpoint(&self, pane: &PaneName)` is the
1209|    one place a record becomes an endpoint.
````

**E42.** (in flight) #839's brief (commit `1e1f203`) adds a `HubPort` with required methods and a field of `Ports`; this story adds methods, not a port, so it adds no `Ports` literal field.

`git show 1e1f203:docs/handoffs/839-brief.md | sed -n '36,48p' | awk -v n=36 '{printf "%d| %s\n", n+NR-1, $0}'`

````text
36| - A new port, `HubPort` (`holler-pane`, new file `crates/holler-pane/src/hub.rs`), with three **required** methods (no defaults: #835 made `observe_pane` and `foreground_commands` required for exactly the reason that a default answers silently for a wrapper that forgot to forward, E92) and `Ports` gains `pub hub: &'a dyn HubPort` (E29; `Wiring::ports` lends the real ports, E25, and `Unwired` is the not-implemented set, E26; `PaneStore`, E27, gets no new method):
37|   ```rust
38|   pub struct Advertised { pub token: String, pub name: String /* "label/session" */, pub harness_session_id: String, pub state: DriverState }
39|   pub enum DriverState { Idle, Working, InputRequired }
40|   pub enum PromptMode { Check, Send }
41|   pub trait HubPort: Send + Sync {
42|       /// The live attach-mode sessions the hub knows, with the id each advertises.
43|       fn advertised(&self) -> Result<Vec<Advertised>, PaneError>;
44|       /// Ask the body behind `driver` to drive `harness_session_id`; returns the id it acknowledged.
45|       fn rebind(&self, driver: &Advertised, harness_session_id: &str) -> Result<String, PaneError>;
46|       /// The hub's prompt path: `Check` runs the gate and sends nothing; `Send` queues the prompt through `send_prompt`.
47|       fn prompt(&self, harness_session_id: &str, text: &str, mode: PromptMode) -> Result<(), PaneError>;
48|   }
````

**E43.** (in flight) #839's merge order, from its 'Clashes' section.

`git show 1e1f203:docs/handoffs/839-brief.md | sed -n '316,316p' | awk -v n=316 '{printf "%d| %s\n", n+NR-1, $0}'`

````text
316| **Recommended merge order:** #835, then #831, then #648 (any order among these three, as each is nearly or fully built and none depends on #839), then **#839**, then #667. Reasons: #839 adds a field to `Ports` (15 literals) and rewrites the engine's tail, and it has no code yet, while #831 and #835 have code and tests written against the current `Ports` and the current `send_prompt` step; rebasing #839 onto them is smaller than rebasing both onto it. #831's ADR "Deferred" bullet edit lands first and #839 supersedes it with the text above. #667 has no code yet and should be written against `Ports` with `hub`. If #831 must land before #839 with its `reset_first_reaches_the_new_process` as it is, #839's run changes that one test and the sentence of its ADR item 18 that says the story proves delivery "through `reset --first`" (E89, line 1176) in the same change; the plan check should confirm this with the operator because it edits another story's contract text.
````

**E44.** The shared end-state helper `assert_end_state` (moved here by #667, used by #649's scenario and by `profile_apply_scenario/{scenario,grid,probe}.rs`) asserts `doctor` reports no finding; with real detached tmux sessions every scratch pane is `unattached`, so part 2 changes this one assertion, here.

`sed -n '95,104p' crates/holler-cli/tests/pane_integration/scratch/verbs.rs | awk -v n=95 '{printf "%d| %s\n", n+NR-1, $0}'`

````text
95|     /// The end state: `doctor` reports no finding, the records of `panes` equal Herdr's
96|     /// snapshot of the scratch panes in `workspaces` (pane id and cell), and none of them is in
97|     /// `pane list`'s unmanaged table.
98|     pub fn assert_end_state(&self, panes: &[&str], workspaces: &[&str]) {
99|         let report = self.json(&["pane", "doctor"], 0).data();
100|         assert_eq!(
101|             report["findings"],
102|             json!([]),
103|             "doctor reports no finding: {report}"
104|         );
````

**E45.** The CHANGELOG shape: one bullet at the top of `### Enhancements`.

`sed -n '8,14p' CHANGELOG.md | awk -v n=8 '{printf "%d| %s\n", n+NR-1, $0}'`

````text
8| ## [Unreleased]
9| 
10| ### Enhancements
11| - Pane control, a Claude Code harness (epic [#633](https://github.com/Performant-Labs/holler/issues/633)):
12|   `holler pane launch PANE --harness claude --port-policy none` launches a pane whose harness is one interactive Claude
13|   Code process, recorded by its pid and with no port, and the pane verbs drive it
14|   ([#831](https://github.com/Performant-Labs/holler/issues/831); adopting the fleet's existing Claude Code panes is its
````

**E46.** The pipeline script fixes the handoff directory as `docs/handoffs/<issueNumber>`: no argument.

`sed -n '3806,3808p' .aftersight/releases/claude/workflow/coding-pipeline.workflow.mjs | awk -v n=3806 '{printf "%d| %s\n", n+NR-1, $0}'`

````text
3806| function runHandoffDir(issueNumber) {
3807|   return `docs/handoffs/${issueNumber}`
3808| }
````

**E47.** The run context takes `handoffDir` from `runHandoffDir(args.issueNumber)`; only `briefPath` comes from the arguments.

`sed -n '5075,5080p' .aftersight/releases/claude/workflow/coding-pipeline.workflow.mjs | awk -v n=5075 '{printf "%d| %s\n", n+NR-1, $0}'`

````text
5075|   // handoff, per its own role doc's file-naming convention.
5076|   const runContext = {
5077|     issueNumber: args.issueNumber,
5078|     briefPath: args.briefPath,
5079|     handoffDir: runHandoffDir(args.issueNumber),
5080|     // Aftersight #540: phase F's test command (an argv array), resolved once, at pre-flight.
````

**E48.** The brief gate's files are named by the issue number too.

`sed -n '5165,5166p' .aftersight/releases/claude/workflow/coding-pipeline.workflow.mjs | awk -v n=5165 '{printf "%d| %s\n", n+NR-1, $0}'`

````text
5165|         const briefSidecar = `docs/handoffs/${args.issueNumber}-brief-sidecar-r${briefRound}.md`
5166|         const briefResult = `docs/handoffs/${args.issueNumber}-brief-result-r${briefRound}.md`
````

**E49.** The optional handoff cleanup would `git rm` `docs/handoffs/647` and `docs/handoffs/647-*`: never set `removeHandoffsBeforePr` for this run.

`sed -n '3929,3934p' .aftersight/releases/claude/workflow/coding-pipeline.workflow.mjs | awk -v n=3929 '{printf "%d| %s\n", n+NR-1, $0}'`

````text
3929| function buildHandoffCleanupTaskPrompt(repoPath, issueNumber) {
3930|   return (
3931|     bashTaskPrefix(repoPath) +
3932|     `Run this exact command via Bash: git rm -r --ignore-unmatch "docs/handoffs/${issueNumber}" ` +
3933|     `"docs/handoffs/${issueNumber}-*"\n` +
3934|     `Then run: git commit -m "chore: remove pipeline handoff artifacts for #${issueNumber}" --allow-empty\n` +
````

**E50.** The PR is opened from `issue-647-implementation` with the body `Closes #647.`

`sed -n '5689,5692p' .aftersight/releases/claude/workflow/coding-pipeline.workflow.mjs | awk -v n=5689 '{printf "%d| %s\n", n+NR-1, $0}'`

````text
5689|     branch: `issue-${args.issueNumber}-implementation`,
5690|     title: `Implements #${args.issueNumber}`,
5691|     body: `Closes #${args.issueNumber}.`,
5692|     model: resolvePhaseModel(args, 'PR'),
````

**E51.** ADR-0021 'Decisions taken' items 12 and 13 (#802): the operator changed the port rather than adding a default-armed method, so an omission is a compile error.

`sed -n '1528,1532p' docs/adr/ADR-0021.md | awk -v n=1528 '{printf "%d| %s\n", n+NR-1, $0}'`

````text
1528| 12. **The agent reaches `HarnessPort` by changing the port** (operator, 2026-10-11, #802): `serve` became
1529|     `serve_command(name, port, &ServeSpec)`, so an omitted agent is a compile error at every call site; no additive
1530|     default-armed method.
1531| 13. **`ServeSpec`** (operator, 2026-10-11, #802): a `#[non_exhaustive]` struct holding `agent: Option<AgentKey>` today. New
1532|     fields of the pane's spec for its server go into it, never into new parameters.
````

**E52.** The doctor test rig builds every server world in `make_live`: the place where a healthy fake world gets its tmux client (a fresh fake session has none, as a fresh real one has none).

`sed -n '374,406p' crates/holler-cli/tests/pane_verbs/doctor/rig.rs | awk -v n=374 '{printf "%d| %s\n", n+NR-1, $0}'`

````text
374| fn make_live(
375|     herdr: &FakeHerdr,
376|     host: &FakeHost,
377|     harness: &FakeHarness,
378|     seed: &Seed,
379|     port: u16,
380| ) -> Live {
381|     let name = PaneName::parse(seed.name).expect("a pane name");
382|     let spec = HerdrSpec {
383|         session: SCRATCH.to_owned(),
384|         workspace: SCRATCH.to_owned(),
385|         grid: seed.grid,
386|     };
387|     let pane = herdr.ensure_pane(&spec).expect("place the pane");
388|     host.ensure_session(&name, "/srv/demo")
389|         .expect("the tmux session");
390|     if let Some(dir) = seed.data_dir {
391|         harness.set_data_dir(port, dir);
392|     }
393|     start_server(harness, &name, port);
394|     let session = harness
395|         .create_session(&HarnessEndpoint::Port(port))
396|         .expect("a session");
397|     harness
398|         .attach_tui(&pane.pane_id, &HarnessEndpoint::Port(port), &session)
399|         .expect("attach the TUI");
400|     Live {
401|         name,
402|         herdr: pane,
403|         port,
404|         session,
405|     }
406| }
````

**E53.** The host conformance suite is one table of cases (ten on main, `foreground-commands-names-what-run-started` last, #835); part 2 adds one.

`sed -n '53,84p' crates/holler-pane-testkit/src/conformance/host.rs | awk -v n=53 '{printf "%d| %s\n", n+NR-1, $0}'`

````text
53| /// The suite, in order: the one table that the runner iterates and [`host_cases`]
54| /// lists.
55| const CASES: [(&str, Case); 10] = [
56|     (
57|         "ps-of-missing-session-is-pane-not-found",
58|         ps_of_missing_session_is_pane_not_found,
59|     ),
60|     (
61|         "run-in-missing-session-is-pane-not-found",
62|         run_in_missing_session_is_pane_not_found,
63|     ),
64|     ("run-adds-a-process", run_adds_a_process),
65|     ("ensure-session-is-idempotent", ensure_session_is_idempotent),
66|     ("run-empty-argv-is-usage", run_empty_argv_is_usage),
67|     (
68|         "stop-owned-stops-every-owned-process",
69|         stop_owned_stops_every_owned_process,
70|     ),
71|     (
72|         "stop-owned-of-missing-session-is-ok",
73|         stop_owned_of_missing_session_is_ok,
74|     ),
75|     (
76|         "stop-owned-leaves-other-sessions",
77|         stop_owned_leaves_other_sessions,
78|     ),
79|     ("ps-lists-only-its-session", ps_lists_only_its_session),
80|     (
81|         "foreground-commands-names-what-run-started",
82|         foreground_commands_names_what_run_started,
83|     ),
84| ];
````
