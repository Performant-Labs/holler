# [feat(pane): doctor finds a tmux session no Herdr pane shows (`unattached`) and a pane that runs as an agent that cannot dispatch (`agent-cannot-dispatch`): part 2 of #647](https://github.com/Performant-Labs/holler/issues/647)

Part of epic #633. This is the complete statement of **part 2** of story #647, written as one text against `main` at `bda1069` (#840 merged, 2026-10-11); every excerpt (E1 and following) was cut there, except the ones marked in flight, which are cut from a named commit of another story's branch. Part 1 (the reconcile engine and `holler pane doctor`, 13 finding kinds today) merged as PR #701 (`e612878`, 2026-10-09 18:56 MDT) and is the record in `docs/handoffs/647/`, which this story does not touch. Part 2 is the two amendments the issue parked: `agent-cannot-dispatch` (operator-confirmed 2026-10-09 17:40 MDT, epic decision 8, from pfleet#372) and `unattached` (2026-10-09 19:03 MDT, from the c4r1 incident).

It starts from the operator's decision of 2026-10-11, the last comment on #647: the contract change is **allowed**; `agent-cannot-dispatch` and `unattached` may add read-only port methods, "default-armed or required as the contract amendment decides"; #835 and #832 set the pattern; and #649's wrappers must forward every new method. It also settles every finding of the earlier plan check (`docs/handoffs/647-part2/earlier-attempt/handoff-A.md`, verdict BLOCK, B-1, B-2, W-1 to W-9); the table in "Where this brief is the decision" says where each is settled.

**Marks.** A decision marked *(operator)* was taken by the operator and is quoted from the issue or the ADR. A decision marked *(decided by the preparer, to be confirmed at the plan check)* is the preparer's judgment: the plan check (A) re-reads it against the code and either keeps it or sends it back, and the operator-only questions are at the end of the section.

## Where this brief is the decision

**D1. The scope is both amendments, in one PR that closes #647.** *(decided by the preparer, to be confirmed at the plan check)* Both findings need the same kind of change (a read-only port method, the ADR amended first, a wrapper test, a finding row in the guide), and they edit the same files. The pipeline script opens the PR with `Closes #647.` (E50), which is right only if both land. Rejected: split `unattached` into its own issue (the #665 precedent): it would amend the same ADR section and the same traits twice, in two PRs that clash, for no saving. If the plan check or the build run must drop one, the PR body is edited (`gh pr edit`) so it does not close #647.

**D2. The story starts after #835 and #831 have merged, and is built on the merged `main`, not on their commits.** *(decided by the preparer, to be confirmed at the plan check)* #835 (branch `issue-835-implementation`, at `e5e35ee`, anti-duplication PASS, S and the PR remaining; its excerpts here are cut at `b488a33`) makes the rule this story follows, required port methods (E5, E6, E33), and edits the same `ports.rs`, the same wrappers (E22, E24), the same test doubles and the same ADR lines. #831 (branch `issue-831-implementation`, at `b3f0295`, T-red PASS) changes **every `HarnessPort` method keyed by `port: u16` to be keyed by `&HarnessEndpoint`** (E41), so a harness method written now would be rewritten by it. Stacking on either branch is rejected: both still move (review rounds), and a stack on #835 alone would still be rewritten by #831. If the operator wants an earlier start, build on #835's commits only and write `agent_view` with `port: u16`; the one-line-per-site conversion then belongs to whichever of #831 and this story merges second. After #831, the method is `agent_view(at: &HarnessEndpoint, session, max_turns)`, a single-process (Claude Code) endpoint answers an empty view, and #831's `HarnessRouter` routes it like the other methods. The build run re-cuts the excerpts named *in flight* from `main` before it writes code.

**D3. Both new port methods are required, with no default.** *(operator, by the rule of 2026-10-11, ADR decisions 12 and 23, applied by the preparer)* E51 says the contract changes by changing the port, so an omission is a compile error; E33 says why a default is wrong here: E2 was default-armed, and the review of #849-era found it answering an empty observation, silently, over the real wiring; #835 made it required. E4 is the shape this story does not repeat. Cost, stated: every implementation of `HostPort` and `HarnessPort` in the workspace gets a method (the list is in "Blast radius"); the compiler finds them. Rejected: default-armed methods "that answer cannot-tell" (the operator's decision allows them; they are the exact failure the wrapper tests below exist to catch, and `session_activity` shows the cost of a default that is a plausible answer, `Idle`).

**D4. The two methods, and what already answers.** *(decided by the preparer, to be confirmed at the plan check)*
- `HostPort::attached_clients(&self, name: &PaneName) -> Result<u32, PaneError>`: the number of tmux clients attached to the session, as tmux counts them (`#{session_attached}`). A fresh session has none; a missing session is `pane-not-found`, as `ps` answers. Modelled on #835's `foreground_commands` (E6).
- `HarnessPort::agent_view(&self, port: u16, session: &str, max_turns: usize) -> Result<AgentView, PaneError>` (D5).
- **Reuse, nothing copied.** `foreground_commands` (#835) reads process names, not clients: it does not answer `unattached`, though the tmux adapter can take both values from the same `list-panes` listing `ps` already makes. `observe_pane` (#832) is Herdr's working directory and screen time: Herdr reports no tmux session. `session_activity` and `verify_serving` do not give turns or permissions, but `verify_serving` already calls `GET /agent` and owns the "agent usable as a default" rule (E18): `agent_view` calls that one function, it does not restate the rule. `from_messages` already reads `GET /session/:id/message` (E19): the reader is shared. Both routes exist (E20). Rejected: two harness methods (one for turns, one for agents): two wrapper arms, two fakes, two calls per pane, and the "can dispatch" of a turn needs the agent list anyway; raw JSON in the port (adapter text in `holler-pane`, I6, I7); a method that returns the finished `cannot-dispatch` boolean (the opt-out and the "ships an orchestrator" judgments belong to reconcile, so a second harness reports observations and never findings).

**D5. `AgentView`.** *(decided by the preparer, to be confirmed at the plan check)* A `#[non_exhaustive]` struct with builders and a `Default` that is empty, as `PaneObservation` is in `ports.rs` (E3 shows the host trait; E4 the defaults this story does not repeat): `turns: Vec<TurnAgent>` (the last `max_turns` user turns of the session, oldest first) and `dispatchers: Vec<AgentKey>` (sorted, no duplicates). `TurnAgent { agent: Option<AgentKey>, can_dispatch: Option<bool> }`. The exact meaning of each field is in the ADR text below. "Cannot tell" is an empty view or a `None`, never a guess; a server whose reply has the wrong shape is `unavailable` (an `observe-failed` finding), because a rule that is silently skipped on a version change is the failure this story is about. A name the server uses that is not an `AgentKey` is left out of `dispatchers` and is `agent: None` in a turn. Rejected: `turns` as raw message ids or times (not needed by any rule; the hub's feed is not this port's business).

**D6. The `agent-cannot-dispatch` rule.** *(operator for the finding's existence and its remedy text; the rule's details are decided by the preparer, to be confirmed at the plan check)* The finding is raised for a pane when all of these hold, each from an observation: (1) **the record names no agent** (`opencode_agent` is `None`). That is the only "opted out deliberately" the data has: a pane whose record names an agent chose one, `build` included, which is the pfleet#372 acceptance ("does not fire for a pane running `build` deliberately"); a pane whose record names a dispatching agent is a different problem (a server that lost its `default_agent`) and raises nothing here. No new `Pane` field. (2) The pane's harness is OpenCode (`HarnessKind` has one variant on `main`; the `match` is written exhaustively so #831's second kind must choose). (3) Its server is healthy and lists the session of record (the trust rule of E12). (4) `agent_view(port, session_of_record, RECENT_TURNS)` answers at least one turn and **every** turn has `can_dispatch == Some(false)`, with `RECENT_TURNS = 3`, one constant in `observe.rs`. (5) `dispatchers` is not empty: the project "ships an orchestrator". "Recent hub-delivered turns" can only mean the recent turns of the session of record, the stand-in for DRIVEN that #645 chose and E14 records: OpenCode records the agent of a turn, not who sent it. Rejected: reading `.opencode/agents/` or `opencode.json` (I6; and on an Aftersight pane the orchestrator lives elsewhere, epic decision 8); `Pane.role` as the gate (the epic says the role is the fleet role, not the agent); a majority of turns instead of all (a pane that mixes agents is partly working); a stored "was reported" flag (part 1 keeps no memory between runs).

**D7. "Ships an orchestrator", and how an agent "cannot dispatch".** *(decided by the preparer, to be confirmed at the plan check)* The adapter reports what the server says; the rule is one definition. An agent **cannot dispatch** when the server's permission rules for it deny the `task` permission for every subagent: in OpenCode 1.19.0 the rules are a list of `{permission, action, pattern}`, and the last rule whose permission is `task` or `*` and whose pattern is `*` is `deny`, with no later `task` rule that allows another pattern (measured: "Facts measured outside the repository"). The project **ships an orchestrator** when `dispatchers` is not empty: an agent the server lists that is usable as a default (the `verify_serving` rule: not a subagent, not hidden), is not one of OpenCode's own (`native` is false) and can dispatch. Anything else the adapter cannot read (a rule list of another shape, an agent missing from the list) is `None`, which stops the rule. Rejected: counting every dispatching primary agent (OpenCode's built-in `plan` is one: a false positive on every pane); a list of agent names in the ADR (a name is the project's).

**D8. The remedy of `agent-cannot-dispatch`.** *(operator for "naming the cure (`relaunch --agent KEY`)"; the form is decided by the preparer, to be confirmed at the plan check)* `holler pane relaunch PANE --agent=KEY`, with KEY the one dispatcher when `dispatchers` has exactly one; with several, no remedy, and the message lists up to three (quoted) and says to pass one. The form `--agent=KEY`, not `--agent KEY`, because `AgentKey::parse` accepts a leading `-` and capitals (E17), so `-x` is a valid key and `--agent -x` would read as a flag. KEY reaches a message and a remedy only as a parsed `AgentKey`. The one table (E8) stays one table: `FindingKind::remedy_for(self, pane, fix, agent: Option<&AgentKey>)` is the table, and `remedy(self, pane, fix)` (the signature `tx_switch.rs` calls, E40) delegates with `None`; the `Finding` is built with its key through `Finding::with_agent`. `--fix` is not offered: the finding is `not-fixable`, and `--fix` never changes a pane's agent, silently or not (the issue; ADR decision 16 says a relaunch never changes the agent silently, and this one names it on the command line). Rejected: `remedy` gains a parameter (breaks an outside caller for one kind); a remedy with a `KEY` placeholder (not a command); `holler pane relaunch PANE` alone (does not change the agent: decision 16).

**D9. The `unattached` rule.** *(operator for the finding and its acceptance: "a running, registered session with no attached client raises it; the same pane attached raises nothing"; the rule's edges are decided by the preparer, to be confirmed at the plan check)* Raised when `host.ps` answered for the pane (the session exists) and `attached_clients` is `0`. It is not gated on Herdr, on the pane's hold or on its role: the operator sees nothing in every case. It is independent of `herdr-pane-missing` and `tmux-session-missing`: a missing session raises only the latter (no `attached_clients` call is made); a failed `attached_clients` is `observe-failed` naming `host.attached_clients` and no `unattached`.

**D10. The `HerdrSnapshot` amendment the ADR defers: none is needed, and the deferral is already closed.** *(decided by the preparer, to be confirmed at the plan check)* The ADR's old sentence ("`HerdrSnapshot`, which waits for a contract amendment before #647 and #650 need it") was replaced when #808 added the label (the "Deferred" bullet now reads "the label is added by #808"); a search of `docs/adr/` for `HerdrSnapshot` finds nothing, and the earlier plan check's line 741 no longer exists. The only fact that would let `unattached` say "attached in the Herdr pane the record names" is what process that Herdr pane runs, and Herdr answers that with whole command lines (`pane.process_info`, argv included), which I7 and ADR section 13 refuse. So the rule is "no client at all", and its limit is written in the ADR, the guide and the message: a client attached in some other terminal hides the finding. This is the issue's own acceptance ("no attached client"). Rejected: adding `foreground_processes` to `HerdrPane` (a Herdr read of command lines, a wider `HerdrSnapshot`, a change #650's join does not need); a `HerdrPort` method that returns the pane's tty (no source says Herdr reports one; the spike lists `tty` only as an optional field).

**D11. The cure of `unattached` that is a Holler verb: there is none today, so the remedy is none, and the finding says why.** *(decided by the preparer, to be confirmed at the plan check; the operator's answer to Q1 below may add a verb)* The earlier plan check's W-3 asked for "a remedy that exists today, or `null` with the reason in the message". Evidence: the TUI is an `opencode attach` that runs inside the pane's tmux session (E34); `launch` and `relaunch` ensure the Herdr pane, the tmux session, the server and the TUI, and nothing in `tx_launch`, the Herdr adapter or the host adapter attaches a tmux client to a Herdr pane (E35 states the intent: one Herdr pane shows one tmux session, without saying what attaches it). `pane launch` refuses a registered pane (`pane-exists`), and `pane relaunch` does not attach a client, so naming either would be a false remedy. The issue's other cure is the operator's own tmux-attach tool, which is not a Holler verb and is not named in code, tests or output (public repository). So `unattached` has `remedy: null`, and its message says that no tmux client is attached to the session, so no Herdr pane shows it, and that no Holler verb attaches one. Rejected: `holler pane relaunch PANE` as the remedy (a claim the code does not back, and a relaunch stops the pane's server for nothing); a new verb `holler pane attach` now (new write methods on `HerdrPort` or `HostPort`, a keystroke into a Herdr pane against I4's spirit, an ADR 0003 surface and a flag set: its own story; the ADR "Deferred" list names it with no owner).

**D12. `SpecField` row for `opencode_agent`: already done, nothing to build.** *(operator, ADR decision 18, #644)* E30 shows `SpecField::OpencodeAgent` and `ALL: [SpecField; 16]` on `main`; E31 item 18 records it. The earlier W-7 is settled; this story edits no line of `profile_diff.rs`.

**D13. Reconcile's chain and the read-only guarantee.** *(decided by the preparer, to be confirmed at the plan check)* `check_tmux` (E11) gains the `attached_clients` call after a successful `ps`; the agent check is a new function called after `check_session_of_record` in `check_pane` (E10), so it can use the same `Server` value. The module doc's bound (E9) becomes "at most nine port calls" and the read-only allowlist (E15) adds exactly `HostOp::AttachedClients` and `HarnessOp::AgentView`. Neither rule writes the record. The agent check calls `agent_view` only when rules (1) to (3) of D6 hold, so a pane with a recorded agent costs no extra call. I6 stays true: E13.

**D14. The test kit gets the support; no wrapper doubles are needed for the engine tests.** *(decided by the preparer, to be confirmed at the plan check)* The earlier W-5 wanted test-local doubles because the kit lacked support. #684 (the kit) is closed, and the support is reusable, so it goes in the kit: `FakeHost` gains `attach_client(name)` and `detach_clients(name)` and the op `HostOp::AttachedClients`, and a fresh fake session has **no** client, as a fresh real one has none (a conformance case pins it, so the fake cannot drift); `FakeHarness` gains `seed_turns(port, session, turns)`, `seed_dispatchers(port, keys)` and `HarnessOp::AgentView`. A "healthy world" of the doctor rig attaches one client per pane in `make_live` (E52), so every existing doctor test keeps its meaning. The kit files are listed in "Blast radius" (E37, E38, E53 show where).

**D15. The real implementations.** *(decided by the preparer, to be confirmed at the plan check)* `holler-adapter-host`: `attached_clients` reads `#{session_attached}` through the exact-target listing `ps` uses, with a parser as strict as `ps`'s (a value that is not an unsigned integer is `unavailable`; a missing session is `pane-not-found`). `holler-adapter-opencode`: a new module `agents.rs` (so `lib.rs`, 741 lines, grows by the method and its `mod` line only) implements `agent_view` from `GET /session/:id/message` and `GET /agent`, existing routes (E20); the "usable as a default" rule is `server::usable`'s, shared. Unknown session is `session-not-found` (the check `abort` makes). A reply that is not the JSON a step needs is `unavailable` and quotes no reply (the adapter's rule). The message field that names the agent of a user turn is **not verified on 1.19.0** (the SDK types of 1.14.39 give `UserMessage.agent`; see "Facts measured outside the repository"): the build run proves it with the opt-in real-OpenCode test and cuts the message fixture from the answer before the adapter relies on it. `Unwired` answers `not-implemented` for both.

**D16. What `--fix` does: nothing new.** *(operator, the issue)* `--fix` repairs `shown-driven-mismatch` only. For both new findings `fix` is `not-fixable`. A second `doctor` run reports each standing finding again exactly once and writes nothing for them ("once per pass", the 2026-10-09 split).

**D17. Finding order and the `ALL` pin.** *(decided by the preparer, to be confirmed at the plan check)* The two kinds are appended to `FindingKind` and `ALL` in this order, `Unattached` then `AgentCannotDispatch`, **after the last kind present on `main` at build time** (E7: `unmanaged-stalled` today; #831 adds `pane-waiting` and `claude-login-missing`, #665 adds `profile-drift`). `findings_test.rs` pins every code in order (E16); the build run edits that list, not a copy. Codes: `unattached`, `agent-cannot-dispatch`; neither equals a closed error code or an import finding code (`import_test.rs` checks the import side).

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

1. **Contract first, its own commit, before any test:** the ADR-0021 amendment in the next section, and the module doc of `crates/holler-pane/src/ports.rs` (the amendment list, "Four amendments since", becomes five).
2. **`holler-pane`:** the two required methods and the types `AgentView` and `TurnAgent` (in `ports.rs` if it stays under 800 lines after #835 and #831, else in a new `crates/holler-pane/src/agent_view.rs` re-exported from `lib.rs`); two `FindingKind`s with `code()`, `remedy_for`, `Finding::with_agent`; the reads in `reconcile/observe.rs`.
3. **Test kit:** `FakeHost`, `FakeHarness`, their ops and conformance cases (D14).
4. **Adapters:** `holler-adapter-host` (`attached_clients`), `holler-adapter-opencode` (`agent_view`, new `agents.rs`) (D15).
5. **Wiring:** `BindingHost`, `Wired<dyn HostPort>`, `Wired<dyn HarnessPort>`, `Unwired` forward or refuse both (the "Wrapper forwarding" section).
6. **Docs:** `docs/pane-control.md` section 5 (two rows, the "command it names" cells, the limit of `unattached`), the CHANGELOG bullet, the module docs that name the bound or the amendment list (the `bind.rs` doc E21, `reconcile.rs` and `observe.rs` module docs).
7. **Out of scope:** a verb that attaches a client (Q1); `--fix` for either finding; a stored "already reported" state; DRIVEN (#839); the Claude Code harness's answers beyond "empty view" (#831); any `Pane` field; `profile-drift` (#665); the roster (#648).

## The ADR-0021 amendment that lands first

One commit, `docs(adr): contract amendment for the attached-clients and agent-view reads (#647)`, before any test or code. Locate each edit by its words (line numbers move when #835 and #831 merge). Text in quotation marks is verbatim.

**1. Section 2, the port table.** In the `HostPort` row, after the last method listed, add: "`attached_clients(name)` (#647 part 2, section 12)". In the `HarnessPort` row, after the last method listed, add: "and the required `agent_view(port, session, max_turns)` (#647 part 2, section 12)". Each cell's last column is unchanged.

**2. Section 2, after the sentence that ends "…where a default would answer silently in place of a port the wrapper did not forward"** (#835's), add this sentence: "#647 part 2 adds two more required reads, `HostPort::attached_clients` and `HarnessPort::agent_view` (section 12), for the same reason; #649's wrappers forward both."

**3. Section 9, the `pane doctor` row.** Replace "the reported `herdr-version-unsupported` and," with "the reported `herdr-version-unsupported`, `unattached` and `agent-cannot-dispatch` (#647 part 2, section 12), and,", keeping the rest of the sentence.

**4. Section 12**, directly after the paragraph that begins "**Decided (#647): no hub timer.**", add:

> **Decided (#647, part 2): `unattached` and `agent-cannot-dispatch`, each from one new read-only port method.** Reconcile reads the world only through the ports (I6), and no port could observe either finding. The operator allowed the contract change on 2026-10-11 (decisions taken, below). Both methods are **required**, with no default, for the reason in section 2 (#835): a default answers silently in place of a port a wrapper did not forward, and the compiler finds a wrapper that left one out. #649's wrappers forward both, and a test per wrapper pins it.
>
> - **`HostPort::attached_clients(name) -> Result<u32, PaneError>`**: the number of tmux clients attached to the session `name`, as tmux counts them (`#{session_attached}`). A fresh session made by `ensure_session` has none. A missing session is `pane-not-found`, as `ps` answers. A count only: never a client's name, tty or process (I7).
> - **`HarnessPort::agent_view(port, session, max_turns) -> Result<AgentView, PaneError>`** (keyed by `&HarnessEndpoint` instead of `port` once #831 has merged): what the harness server says about the agents of `session`, in one call. `AgentView` is `#[non_exhaustive]`, with builders and an empty default, as `PaneObservation` is.
>   - `turns`: for each of the last `max_turns` user turns of `session`, oldest first, a `TurnAgent { agent, can_dispatch }`. `agent` is the key the server recorded for the turn, `None` when it recorded none or a name that is not an `AgentKey`. `can_dispatch` is `Some(false)` only when the server's permission rules for that agent deny the `task` permission for every subagent (OpenCode: the last rule whose permission is `task` or `*` and whose pattern is `*` is `deny`, and no later `task` rule allows another pattern), `Some(true)` when they do not, and `None` when the server does not say, the agent is not in its list, or the rules have a shape the adapter does not know.
>   - `dispatchers`: the agents the server lists that a pane could run as its default (not a subagent and not hidden: the rule `verify_serving` applies), that are not the harness's own (OpenCode: `native` is false), and whose `can_dispatch` is `Some(true)`. Sorted by key, without duplicates; a name that is not an `AgentKey` is left out.
>   - An implementation that cannot tell answers an empty view, never a guess. A session the server does not have is `session-not-found`; a reply of the wrong shape is `unavailable`. The OpenCode adapter reads `GET /session/:id/message` and `GET /agent`, and never a file.
> - **`unattached`** is raised when `host.ps` answered for the pane (the session exists) and `attached_clients` is `0`. The rule is "no tmux client at all", not "a client in the Herdr pane the record names": telling which Herdr pane displays a tmux session needs Herdr's `pane.process_info`, whose answer is whole command lines (I7; section 13 refuses it), so a client attached in some other terminal hides the finding. `HerdrSnapshot` and `HerdrPane` do not change. The finding names no command: no Holler verb attaches a tmux client to a Herdr pane (`launch` and `relaunch` attach the TUI inside the tmux session; `launch` refuses a registered pane), and its message says so.
> - **`agent-cannot-dispatch`** is raised when all of these hold: the record names no agent (`opencode_agent` is absent; a pane whose record names one chose it, `build` included); the pane's harness is OpenCode; its server is healthy and lists the session of record; `agent_view(port, session_of_record, 3)` answers at least one turn and every turn has `can_dispatch` `Some(false)`; and `dispatchers` is not empty, which is what "the project ships an orchestrator" means. "Recent hub-delivered turns" are the recent turns of the session of record, which I2 makes the session the hub drives: OpenCode records a turn's agent, not who sent it. The remedy is `holler pane relaunch PANE --agent=KEY` when `dispatchers` holds exactly one KEY; with several there is no remedy and the message lists up to three. `--fix` does nothing for either finding.
> - **Reads and writes.** A chain makes at most nine port calls, each bounded by I5 (`attached_clients` and `agent_view` are the two new ones). Neither finding writes the record, and each is raised once per pass.

**5. "Deferred to named stories"**, a new bullet: "A Holler verb that attaches a pane's tmux session to its Herdr pane, the cure for `unattached`: no story owns it; until one does, the finding names no command (section 12)."

**6. "Decisions taken"**, one new item with the next free number at merge time: "**Part 2 of #647 may add read-only port methods** (operator, 2026-10-11, #647, section 12): `HostPort::attached_clients` and `HarnessPort::agent_view`, required like #835's, so a wrapper that does not forward one is a compile error."

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
7. `FakeHarness`: `agent_view` of a seeded session returns the newest `max_turns` seeded turns, oldest first, and the seeded dispatchers; an unseeded session on a live server returns an empty view; an unknown session is `session-not-found`; a killed or frozen server fails as the other reads do. The harness conformance suite has the matching case, and its `Mutant` fails it.

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
16. The existing doctor tests all pass unchanged in meaning: the rig attaches a client to each pane it builds (E52), and a test proves that a world built without that call would raise `unattached` for every pane (so the attach is what keeps them clean, not the rule being off).

**Wrapper forwarding** is its own section below (items W1 to W6 are acceptance items too).

**Docs and gates.**
17. `docs_pane_control_test`: section 5 names both codes (E36); the table rows say "none" for `unattached`'s command and `holler pane relaunch PANE --agent=KEY` for the other; the guide states the limit of `unattached` (a client in any terminal counts).
18. The opt-in scenario's end-state assertion (E44) is changed to "no finding except `unattached` for each scratch pane", with a comment saying why (a detached scratch tmux session has no client by design); #667 moves that helper, so whichever of the two merges second applies the change to the helper's current home. The default run still compiles and passes.
19. `cli-surface.txt`, ADR 0003 and the clap tree are unchanged (no verb or flag is added); `scripts/lint.sh` passes (no file reaches 900 lines, E39); `cargo fmt` is run per file, never workspace-wide; clippy is clean; no `unsafe`; the privacy grep of the added lines finds no personal host, user, path or tool name.
20. The ADR amendment is its own, first commit, and the code commits cite the section.

## Wrapper forwarding

#849-era review found `observe_pane` answering an empty observation over the real wiring because a wrapper had not forwarded a default-armed method; the pin that followed is E26 (the conformance suite over the wrapper, one recording check per wrapper, the failed-build case), and ADR section 14 says "A wrapper port must delegate it" (E32). #835 repeats it for the host (E27, E28). Here both new methods are **required**, so a wrapper that omits one does not compile; the tests below catch the other failure, a wrapper that compiles and forwards wrongly (answers a constant, drops an argument, calls the wrong inner method).

| New method | Implementations that must forward or answer | The test that fails if one does not forward |
|---|---|---|
| `HostPort::attached_clients` | `BindingHost` (E22); `Wired<dyn HostPort>` (E24) | **W1** `pane_integration/forwarding.rs` (new): over a recording inner host that answers `Ok(7)` for one name and `Err(Unavailable)` for another, each wrapper returns 7 and the same error, and the inner host recorded exactly one `attached_clients` call per ask, with the name given. **W2** the host conformance suite (its new case, D14) runs over `BindingHost` and over `WiredHost` (E27 is the model): the missing-session case fails for a wrapper that answers a constant. |
| `HostPort::attached_clients` | `Wired` built from a misconfiguration; `Unwired` | **W3** a misconfigured `WiredHost` answers its configuration error from `attached_clients`, and `wired.rs`'s socket list gains `host.attached_clients` (E28); `Unwired` (E29) answers `not-implemented` (the existing `Unwired` test is extended). |
| `HarnessPort::agent_view` | `Wired<dyn HarnessPort>` (E23); after #831, `HarnessRouter` | **W4** `bind.rs`: `Recording` answers a distinctive `AgentView` (one turn as `orchestrator`, one dispatcher `reviewer`) and records its arguments; `call_every_method` gains `agent_view`; the expected-calls list gains `"agent_view"`; the test asserts the wrapper's answer equals the inner one's and the inner call saw `(48100, "ses_1", 3)` (E25). **W5** `a_misconfigured_wired_harness_answers_its_error_from_every_method` loops over `call_every_method`, so it covers the new method with the same edit; `wired.rs` gains `harness.agent_view`. |
| both | the test doubles `LinkedHost` (three copies), `Recording`, `Resolving`, `HealthGate`, `WriterInSelect`, `SeamHarness`, `HookedHarness` (two), `LostChain` (#835), `TestHost`, `TestHarness`, the two `Mutant`s | compile-enforced. **W6** a double that wraps a fake forwards to it in one line; the doctor tests of items 10 and 16 attach their client through the fake that a wrapping double wraps, so a double that answered a constant would make them fail. |

Also required: **W7** (T verifies, F records in its handoff) a **mutation check**: replace each wrapper's body in turn with a constant (`Ok(0)`, `Ok(AgentView::default())`) and show that W1, W2, W4 fail for it; revert. **W8** the module doc of `bind.rs` (E21) is edited so that it names the new methods, and says that no method of these wrappers has a default to fall back on.

## Blast radius

Re-measured with `wc -l`: "main" is this worktree after the merge of `bda1069`; "#835" and "#831" are the files in their branches at `e5e35ee` and `b3f0295` (read only; a dash is a file that branch does not have). `scripts/lint.sh` fails a `crates/**/*.rs` file at 900 lines and warns at 600 (E39); docs are not under the gate.

| File | main | #835 | #831 | Change | Fits |
|---|---|---|---|---|---|
| `crates/holler-pane/src/ports.rs` | 423 | 497 | 551 | two methods; `AgentView`/`TurnAgent` (in this file, or a new `agent_view.rs` re-exported from `lib.rs` if the file would pass 700); the module doc; two doc tests | yes |
| `crates/holler-pane/src/findings.rs` | 357 | 357 | 373 | two kinds, `remedy_for`, `Finding::with_agent` | yes |
| `crates/holler-pane/src/reconcile.rs`, `reconcile/observe.rs` | 450, 391 | same | 532, 449 | module docs; the two reads and the rule; `RECENT_TURNS` | yes |
| `crates/holler-pane/src/lib.rs` | 80 | 80 | 81 | re-exports | yes |
| `crates/holler-pane/tests/findings_test.rs`, `ports_test.rs` | 45, 596 | 45, 603 | 45, 596 | the code list; `TestHost`/`TestHarness` methods | `ports_test.rs` stays under 650 |
| `crates/holler-pane-testkit/src/host.rs` | 234 | 278 | 278 | `attach_client`, `detach_clients`, `HostOp::AttachedClients` | yes |
| `crates/holler-pane-testkit/src/harness.rs` | 599 | 599 | 631 | `seed_turns`, `seed_dispatchers`, `HarnessOp::AgentView`, the impl | about 700 after #831: warn, under 900 |
| `crates/holler-pane-testkit/src/conformance/host.rs`, `conformance/harness.rs` | 271, 588 | 319, 588 | 271, 606 | one case each | harness.rs near 660: warn |
| `crates/holler-pane-testkit/tests/host_conformance_test.rs`, `harness_conformance_test.rs` | 225, 337 | 254, 337 | 225, 337 | the `Mutant`s | yes |
| `crates/holler-pane-testkit/tests/fake_host_test.rs`, `fake_harness_test.rs` | 300, 764 | 372, 764 | 300, 764 | the fake's cases; **the harness cases go in a new `fake_harness_agent_test.rs`**, not in the 764-line file | yes |
| `crates/holler-adapter-host/src/lib.rs`, `src/tmux.rs` | 499, 299 | 521, 336 | 521, 336 | `attached_clients`, its listing and parser | yes |
| `crates/holler-adapter-host/tests/attached_test.rs` (new) | | | | the cases of item 8 (**not** in `fake_tmux_test.rs`, 818 lines) | yes |
| `crates/holler-adapter-opencode/src/lib.rs`, new `src/agents.rs` | 741 | 741 | 741 | `mod agents;` and a one-line method in `lib.rs`; the logic in `agents.rs` | yes |
| `crates/holler-adapter-opencode/tests/agent_view_test.rs` (new), `tests/fixtures/agents/*.json` (new), `tests/real_opencode/**` | | | | the hermetic cases; the opt-in real case | yes |
| `crates/holler-cli/src/pane/wiring.rs`, `wiring/bind.rs` | 396, 439 | 400, 448 | 396, 439 | `Unwired` arms; the forwarding arms; the module doc | yes |
| `crates/holler-cli/tests/pane_integration/bind.rs`, `wired.rs`, `scenario.rs`, `main.rs`, new `forwarding.rs` | 580, 141, 394, 163 | 592, 142, 394, 163 | 681, 141, 394, 163 | W1 to W6; `mod forwarding;`; the scenario end state | yes |
| `crates/holler-cli/tests/pane_verbs/doctor.rs`, `doctor/rig.rs`, `doctor/read_only.rs`, new `doctor/attach.rs` and `doctor/agent.rs` | 560, 434, 147 | same | same | two `mod` lines; the client in `make_live`; the allowlist; the two rule suites | yes |
| test doubles that gain a one-line forward | | | | `pane_verbs/launch/rig.rs` (835 on main, 839 after #835), `profile_verbs/apply/rig.rs` (759, 763), **`pane_verbs/switch.rs` (868 on main and after #835; 833 after #831)**, `pane_integration/bind.rs`, `pane_integration/observe.rs`, `pane_verbs/doctor/surface.rs` (591), `pane_verbs/import/{harness,seams}.rs` (340, 120; `harness.rs` is #835's) | **`switch.rs` has two doubles: each new method is a one-line forward, 876 at most if #831 has not shrunk it; if a double would carry the file past 890, F moves the two doubles to a sibling file first. `launch/rig.rs` is at 839 and takes three lines.** |
| `crates/holler-cli/tests/docs_pane_control_test.rs` | 523 | | | none expected (it reads the code's tables) | yes |
| `docs/adr/ADR-0021.md`, `docs/pane-control.md`, `CHANGELOG.md` | 1121, 794, 982 | 1176, | 1307, | the amendment; section 5; one bullet at the top of `### Enhancements` (E45), linking /issues/647 | docs |

Not edited: `crates/holler-pane/src/profile_diff.rs` (D12), `crates/holler-cli/src/pane/doctor.rs` (generic over kinds), `crates/holler-cli/tests/fixtures/cli-surface.txt`, the hub and proto crates, and the `Ports` struct (this story adds methods, not a port, so it adds no field to the `Ports` literals #839 changes).

## Depends on

Verified with `gh issue view` on 2026-10-11: closed and on `main`: #637, #638, #642 (the OpenCode adapter and the agent), #644 (launch, relaunch, `--agent`, the `SpecField` row), #645, #649 (the wiring), #684 (the kit), #700 (`Pane.opencode_agent`), #802, #808, #832, and part 1 of #647 (PR #701). **Open and this story starts after them (D2): #835** (import leaves panes alone; makes the required-method rule; at `e5e35ee`, anti-duplication PASS) **and #831** (the Claude Code harness; `HarnessEndpoint`; at T-red PASS, `b3f0295`). #654 (the live cutover, operator-run) is open: see Q2.

## Clashes with stories in flight

Read-only look on 2026-10-11 at the branches and briefs named. "Shared" files are those both stories edit.

| Story | Shared files and hunks | What it means here |
|---|---|---|
| **#835** import leaves panes alone (`e5e35ee`; excerpts at `b488a33`) | `ports.rs` (the module doc, `observe_pane`'s doc, `HostPort` gains `foreground_commands`), the same wrappers and doubles (`bind.rs`, `wiring.rs`, `LinkedHost` x3, `host.rs` kit, conformance), ADR-0021 sections 2, 9, 13, 14 and the decisions list, `CHANGELOG.md`, `docs/pane-control.md` | Mechanical, and this story is written to follow it (D2, D3). Merge #835 first. |
| **#831** Claude Code harness (`b3f0295`) | `ports.rs` (every `HarnessPort` method keyed by `HarnessEndpoint`), `findings.rs` (adds `pane-waiting`, `claude-login-missing`), `reconcile.rs`, `reconcile/observe.rs` (110 lines changed), `tx_switch.rs`, the kit (`harness.rs`, conformance), `bind.rs` tests (+195), ADR-0021 (a new section 15, decisions 22 to 25), `CHANGELOG.md` | The widest clash. Merge #831 first (D2): `agent_view` is then written with `&HarnessEndpoint`, `ALL` appends after #831's kinds (D17), and rule (2) of D6 is a real `match` on two harness kinds. |
| **#839** DRIVEN follows the session of record (brief at `1e1f203`, no code) | `ports.rs` only for a new port `HubPort` (this story adds methods to existing traits, so it adds no field to the 15 `Ports` literals #839 changes); `reconcile.rs`, `reconcile/observe.rs` (reads `hub.advertised()` once per pass, records `driven`, adds a finding); `findings.rs`; `pane_integration/bind.rs`; ADR-0021 sections 2, 5, 8, 11 and the "Deferred" DRIVEN bullet; `CHANGELOG.md`; `docs/pane-control.md` sections 3, 4, 10, 12 (E42, E43) | Mechanical. No dependency either way: D6 uses the session of record, which stays right when DRIVEN is observed (I2). The chain's call bound (D13) is edited by both: whoever merges second sums them. Its end-state helper edit and item 18 touch the same scenario assertion as #667. |
| **#648** roster SHOWN and DRIVEN | `docs/pane-control.md` (the roster paragraphs of section 4), `CHANGELOG.md`; its brief does not touch `ports.rs`, `findings.rs` or `reconcile*` | CHANGELOG and guide only; no order needed. |
| **#667** `profile apply` scenario (brief at `bc965ae`, A PASS at `bcb6031`; the 4-file crate diff in its worktree is uncommitted and was not read) | moves `assert_end_state` ("`doctor` reports no finding") from `pane_integration/scenario.rs` into a shared helper; builds a `Ports` literal from the real adapters | **Real clash with item 18**: every scratch pane is `unattached`, so a clean `doctor` no longer means no finding. Land #667's helper with the `unattached` allowance, or this story edits the helper where it then lives. Recommend #667 merges after this story (below). |
| **#665** `holler profile rename/export/import`, `profile-drift` (no code; another preparer is writing its brief) | `findings.rs` and `reconcile.rs`: "the `ProfileDrift` kind and the one call that raises it only"; `findings_test.rs`; the guide's section 5 | Appends one more kind after ours (D17), and a call in the same `reconcile.rs`. Mechanical; it should merge after this story, and its `CODES` edit takes the list as it is then. |

**Recommended merge order:** #835, then #831, then #839, then **this story (#647 part 2)**, then #667, then #665; #648 anywhere. Reasons: this story edits the traits and wrappers that #835 and #831 rewrite, so it is smaller to write after them than to rebase them onto it. #839 has no code yet and its engine changes in `reconcile*` are a smaller rebase either way; putting #839 before this story means this story's chain-bound and finding sort run against the final engine, but nothing in the code depends on it, so the order of #839 and this story is the operator's to swap. #667 and #665 have no merged code yet and are written against the merged findings and clean-doctor meaning. Shared ADR-0021 and CHANGELOG hunks resolve mechanically: each story adds a bullet at the top of `### Enhancements`, a row in the section 2 table, a "Decisions taken" item (renumber to the next free number), and the "Deferred" list is appended to by this story and #839.

## Pipeline

`rigor: second-opinion` (D18). Phase order from CLAUDE.md: O, A (this brief), T (author, RED), F, T (verify, GREEN), A (anti-duplication), S. `uiSurface: false`. The GitHub issue is the source of truth; this brief is the plan the run is given (`briefPath`). The brief gate is run on this brief; the earlier PASS (r1 of 2026-10-11 02:22 MDT, which passed part 1's brief) is not carried forward.

**The pipeline script cannot write a different handoff directory.** The release `claude-pipeline-v0.1.7` (`.aftersight/releases/claude/workflow/coding-pipeline.workflow.mjs`) fixes the run's handoff directory as `docs/handoffs/<issueNumber>` (E46, E47); there is no `slug`, `handoffDir` or `handoffRoot` argument. The brief gate's files are named `docs/handoffs/<issueNumber>-brief-sidecar-r<N>.md` and `...-brief-result-r<N>.md`, and the diff gate's the same with `diff` (E48). Only `briefPath` is an argument. The branch is `issue-<N>-implementation` and the PR is `Implements #<N>` with body `Closes #<N>.` (E50). `issueNumber` must be an integer. So a run with `issueNumber: 647` writes into **part 1's record** (`docs/handoffs/647/`, and the loose `docs/handoffs/647-brief*` files) and its role phases read every file there as a prior phase's handoff. The earlier attempt did exactly that.

**Launch arguments** (the rest as CLAUDE.md gives them): `issueNumber: 647`, `briefPath: 'docs/handoffs/647-part2-brief.md'`, `uiSurface: false`, `rigor: 'second-opinion'`, `models: { F: 'claude-opus-5-5' }`, `removeHandoffsBeforePr` **not set** (its cleanup does `git rm -r docs/handoffs/647 docs/handoffs/647-*`, which would delete this story's record and part 1's: E49). Run from a fresh worktree on `origin/main` after #835 and #831 have merged, with the brief and the slug directory brought in by cherry-pick.

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

## Evidence (verbatim, from `main` at `bda1069`)

Each excerpt is cut by the command shown; the line numbers on the left come from `awk`, not from typing. An excerpt marked in flight is cut from a named commit of another story's branch (`git show <sha>:<file>`), so it is stable but is not `main`'s. Run the commands from the root of a worktree of this repository (the `.aftersight/releases/claude` link must exist for the pipeline excerpts) with those commits available.

**E1.** The module doc of `ports.rs` on main lists two amendments (#645, #802); #832's is in the trait below it, and #835's branch rewrites this list to four.

`sed -n '12,24p' crates/holler-pane/src/ports.rs | awk -v n=12 '{printf "%d| %s\n", n+NR-1, $0}'`

````text
12| //! **Frozen when #637 merges**, after which a change goes through the epic's
13| //! amend-first rule. [`HerdrPort`], and the minimal data types it takes and returns,
14| //! stay provisional until the spike #636 (Herdr) reports. [`HarnessPort`] is final:
15| //! the spike #635 (OpenCode) confirmed it, and #642 built it. Two amendments since:
16| //!
17| //! - #645's two default-armed seam methods, [`HarnessPort::send_prompt`] and
18| //!   [`HarnessPort::session_activity`], whose defaults keep every impl that does not
19| //!   override them as it was;
20| //! - #802's: `serve` became the pure [`HarnessPort::serve_command`], which takes a
21| //!   [`ServeSpec`] and returns the server's argv, beside the required
22| //!   [`HarnessPort::verify_serving`]; and [`HostPort::run`] takes the directory it starts
23| //!   its program in. The host starts and stops a pane's processes, its harness server
24| //!   among them.
````

**E2.** #832's `observe_pane` is default-armed on main: a wrapper that forgets it answers an empty observation, silently.

`sed -n '196,208p' crates/holler-pane/src/ports.rs | awk -v n=196 '{printf "%d| %s\n", n+NR-1, $0}'`

````text
196|     /// What Herdr says about the pane `pane` that a record does not hold: its working
197|     /// directory and when its screen last changed (#832). It is an observation (I6), read
198|     /// fresh and never stored; `pane list` and `pane get` show it for a pane no record
199|     /// names.
200|     ///
201|     /// The default answers an empty [`PaneObservation`]: an implementation that cannot
202|     /// tell says so with `None` fields and never guesses, so one that does not override
203|     /// this keeps compiling and reports nothing. `pane-not-found` when Herdr has no such
204|     /// pane.
205|     fn observe_pane(&self, pane: &PaneId) -> Result<PaneObservation, PaneError> {
206|         let _ = pane;
207|         Ok(PaneObservation::default())
208|     }
````

**E3.** `HostPort` on main has four methods and no default; `foreground_commands` is #835's.

`sed -n '253,275p' crates/holler-pane/src/ports.rs | awk -v n=253 '{printf "%d| %s\n", n+NR-1, $0}'`

````text
253| /// The machine a pane lives on: tmux sessions and the processes in them (the adapter
254| /// is `holler-adapter-host`, #641).
255| ///
256| /// **Blocking.** Every method is synchronous. Call from `spawn_blocking` (or a
257| /// thread) in async code. Every method returns within I5's bound (default 10 s) or
258| /// with [`PaneError::Timeout`]. An implementation is `Send + Sync`.
259| pub trait HostPort: Send + Sync {
260|     /// Make the tmux session `name` exist, working in `cwd`.
261|     fn ensure_session(&self, name: &PaneName, cwd: &str) -> Result<(), PaneError>;
262| 
263|     /// Run `argv` (never through a shell) in the session `name`, starting it in `cwd`,
264|     /// which need not be the directory the session was made in (#802: the launch engine
265|     /// passes the pane spec's directory, for the spec's command and for the harness
266|     /// server alike).
267|     fn run(&self, name: &PaneName, cwd: &str, argv: &Argv) -> Result<(), PaneError>;
268| 
269|     /// Stop the processes the session `name` owns: what `run` started there, the pane's
270|     /// harness server among them (#802).
271|     fn stop_owned(&self, name: &PaneName) -> Result<(), PaneError>;
272| 
273|     /// The process ids running in the session `name`.
274|     fn ps(&self, name: &PaneName) -> Result<Vec<u32>, PaneError>;
275| }
````

**E4.** `HarnessPort`'s two default-armed methods (#645), the shape this story does not repeat.

`sed -n '344,365p' crates/holler-pane/src/ports.rs | awk -v n=344 '{printf "%d| %s\n", n+NR-1, $0}'`

````text
344|     /// Queue `text` as a prompt to `session` on the server at `port`, through the
345|     /// harness API, never by typing into a TUI (I4). `pane reset --first` calls it (#645).
346|     ///
347|     /// The default refuses with [`PROMPT_UNSUPPORTED`]: an impl that does not send
348|     /// prompts says so as a stable refusal, which the real adapter replaces (#642).
349|     fn send_prompt(&self, port: u16, session: &str, text: &str) -> Result<(), PaneError> {
350|         let _ = (port, session, text);
351|         Err(PaneError::Refused {
352|             code: PROMPT_UNSUPPORTED,
353|             message: "this harness does not queue a prompt".to_owned(),
354|         })
355|     }
356| 
357|     /// What `session` on the server at `port` is doing now (#645: `pane reset` refuses a
358|     /// conversation that is not [`Activity::Idle`]).
359|     ///
360|     /// The default answers [`Activity::Idle`], so an impl that does not report activity
361|     /// refuses nothing.
362|     fn session_activity(&self, port: u16, session: &str) -> Result<Activity, PaneError> {
363|         let _ = (port, session);
364|         Ok(Activity::Idle)
365|     }
````

**E5.** (in flight) #835 (branch `issue-835-implementation`, `b488a33`) makes the rule: required methods, so the compiler finds every implementation, a wrapper included.

`git show b488a33:crates/holler-pane/src/ports.rs | sed -n '12,31p' | awk -v n=12 '{printf "%d| %s\n", n+NR-1, $0}'`

````text
12| //! **Frozen when #637 merges**, after which a change goes through the epic's
13| //! amend-first rule. [`HerdrPort`], and the minimal data types it takes and returns,
14| //! stay provisional until the spike #636 (Herdr) reports. [`HarnessPort`] is final:
15| //! the spike #635 (OpenCode) confirmed it, and #642 built it. Four amendments since:
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
27| //! - #835's: the required [`HostPort::foreground_commands`], the command name of each pane
28| //!   process of a tmux session (how the import tells a pane that runs Claude Code), and
29| //!   [`HerdrPort::observe_pane`] lost its default. Both are required so that the compiler
30| //!   finds every implementation, a wrapper port included: a default answers silently in place
31| //!   of a port a wrapper forgot to forward.
````

**E6.** (in flight) #835's `HostPort::foreground_commands`: names only, one list per session, `pane-not-found` for a missing session. The model for `attached_clients`.

`git show b488a33:crates/holler-pane/src/ports.rs | sed -n '339,349p' | awk -v n=339 '{printf "%d| %s\n", n+NR-1, $0}'`

````text
339| 
340|     /// The command name of the foreground process of every live pane in the session `name`,
341|     /// one per pane, in the order tmux lists them (#835): what tmux reports as
342|     /// `#{pane_current_command}`, the basename of that process's `argv[0]`. Names only, never
343|     /// an argv, a command line or a program's configuration (I7). A session with no pane
344|     /// process answers an empty list; a missing session is `pane-not-found`, as `ps` answers.
345|     ///
346|     /// The import reads it to leave alone a pane that runs Claude Code
347|     /// ([`crate::pane::is_claude_code_command`]).
348|     fn foreground_commands(&self, name: &PaneName) -> Result<Vec<String>, PaneError>;
349| }
````

**E7.** The end of `FindingKind` and `ALL` on main (13 kinds). Part 2 appends two after the last kind present at build time.

`sed -n '70,94p' crates/holler-pane/src/findings.rs | awk -v n=70 '{printf "%d| %s\n", n+NR-1, $0}'`

````text
70|     ShownDrivenMismatch,
71|     /// A session on a pane's server that is the session of record of no pane.
72|     StraySession,
73|     /// A Herdr pane no record names has shown the same screen for longer than the
74|     /// operator's threshold (#832). Only a whole-fleet pass with a threshold looks.
75|     UnmanagedStalled,
76| }
77| 
78| impl FindingKind {
79|     /// Every kind, in declaration order.
80|     pub const ALL: &'static [FindingKind] = &[
81|         FindingKind::HerdrVersionUnsupported,
82|         FindingKind::ObserveFailed,
83|         FindingKind::UnregisteredHerdrPane,
84|         FindingKind::HerdrPaneMissing,
85|         FindingKind::TmuxSessionMissing,
86|         FindingKind::ServerWedged,
87|         FindingKind::ServerDown,
88|         FindingKind::NoSessionOfRecord,
89|         FindingKind::SessionOfRecordMissing,
90|         FindingKind::TuiForeignSession,
91|         FindingKind::ShownDrivenMismatch,
92|         FindingKind::StraySession,
93|         FindingKind::UnmanagedStalled,
94|     ];
````

**E8.** The one remedy table. `remedy(self, pane, fix)` has an outside caller (`tx_switch.rs`), so part 2 adds `remedy_for` and leaves this signature.

`sed -n '116,149p' crates/holler-pane/src/findings.rs | awk -v n=116 '{printf "%d| %s\n", n+NR-1, $0}'`

````text
116|     /// The command a finding of this kind tells the operator to run, for the pane `pane`
117|     /// (`None` when it is about no pane) once `--fix` left it at `fix`; `None` when no Holler
118|     /// verb repairs it. The one table of remedies:
119|     ///
120|     /// - a read that failed: run the pass again, `holler pane doctor [<pane>]`;
121|     /// - the pane's process or its harness is gone or wrong: `holler pane relaunch <pane>`;
122|     /// - the pane has no usable session of record: `holler pane reset <pane>`, which creates
123|     ///   one (choosing an existing session is the operator's, never a guess);
124|     /// - a mismatch: `holler pane doctor <pane> --fix` while a fix can repair it, relaunch when
125|     ///   it cannot or failed, nothing once it is fixed.
126|     pub fn remedy(self, pane: Option<&PaneName>, fix: FixState) -> Option<String> {
127|         match self {
128|             FindingKind::HerdrVersionUnsupported
129|             | FindingKind::UnregisteredHerdrPane
130|             | FindingKind::UnmanagedStalled
131|             | FindingKind::StraySession => None,
132|             FindingKind::ObserveFailed => Some(doctor_command(pane, false)),
133|             FindingKind::HerdrPaneMissing
134|             | FindingKind::TmuxSessionMissing
135|             | FindingKind::ServerWedged
136|             | FindingKind::ServerDown
137|             | FindingKind::TuiForeignSession => pane.map(relaunch_command),
138|             FindingKind::NoSessionOfRecord | FindingKind::SessionOfRecordMissing => {
139|                 pane.map(reset_command)
140|             }
141|             FindingKind::ShownDrivenMismatch => match fix {
142|                 FixState::Fixable | FixState::Skipped => {
143|                     pane.map(|pane| doctor_command(Some(pane), true))
144|                 }
145|                 FixState::NotFixable | FixState::Failed => pane.map(relaunch_command),
146|                 FixState::Fixed => None,
147|             },
148|         }
149|     }
````

**E9.** The chain's port-call bound is written in the module doc: seven calls. Part 2 makes it nine and says so.

`sed -n '1,9p' crates/holler-pane/src/reconcile/observe.rs | awk -v n=1 '{printf "%d| %s\n", n+NR-1, $0}'`

````text
1| //! One pane's chain in a reconcile pass (#647): observe the pane's tmux session, harness
2| //! server and TUI, compare them with its record, repair a mismatch with `--fix`, and record
3| //! what was observed. A pass runs one chain per pane, each on a thread of its own; the
4| //! parent module has the rules and what the record write means.
5| //!
6| //! A chain makes at most seven port calls, each bounded by I5: `ps`, `health`,
7| //! `list_sessions`, `shown_session`, and with `--fix` `select_session` and `shown_session`
8| //! again, then one `cas_put`.
9| 
````

**E10.** `check_pane` is the chain: `check_tmux`, `observe_server`, `observe_screen`, `check_session_of_record`, `check_screen`, `record`.

`sed -n '95,109p' crates/holler-pane/src/reconcile/observe.rs | awk -v n=95 '{printf "%d| %s\n", n+NR-1, $0}'`

````text
95| /// Run the chain of `pane`: observe, compare, repair, record.
96| pub(super) fn check_pane(ports: Ports<'_>, request: &ReconcileRequest<'_>, pane: &Pane) -> Checked {
97|     let mut findings = Vec::new();
98|     check_tmux(ports, pane, &mut findings);
99|     let server = observe_server(ports, pane, &mut findings);
100|     let screen = observe_screen(ports, pane, &mut findings);
101|     check_session_of_record(pane, &server, &mut findings);
102|     let screen = check_screen(ports, request, pane, &server, screen, &mut findings);
103|     findings.extend(record(ports, request, pane, &server, &screen));
104|     Checked {
105|         findings,
106|         summary: summary(pane, screen.shown(), server.health()),
107|         listed: server.into_sessions(),
108|     }
109| }
````

**E11.** `check_tmux` reads `host.ps` only; `unattached` extends it.

`sed -n '132,146p' crates/holler-pane/src/reconcile/observe.rs | awk -v n=132 '{printf "%d| %s\n", n+NR-1, $0}'`

````text
132| /// `tmux-session-missing` when the pane's tmux session (named as the pane) is gone.
133| fn check_tmux(ports: Ports<'_>, pane: &Pane, findings: &mut Vec<Finding>) {
134|     match ports.host.ps(&pane.name) {
135|         Ok(_) => {}
136|         Err(PaneError::PaneNotFound { .. }) => {
137|             let message = format!("the tmux session {} does not exist", pane.name);
138|             findings.push(Finding::about(
139|                 FindingKind::TmuxSessionMissing,
140|                 pane,
141|                 message,
142|             ));
143|         }
144|         Err(error) => findings.push(observe_failed(pane, HOST_PS, &error)),
145|     }
146| }
````

**E12.** `check_session_of_record` is the trust rule for the session of record: the agent rule runs only where the healthy server listed it.

`sed -n '197,218p' crates/holler-pane/src/reconcile/observe.rs | awk -v n=197 '{printf "%d| %s\n", n+NR-1, $0}'`

````text
197| /// `no-session-of-record`, or `session-of-record-missing` when the pane's healthy server
198| /// listed its sessions without it.
199| fn check_session_of_record(pane: &Pane, server: &Server, findings: &mut Vec<Finding>) {
200|     let Some(record) = pane.session_of_record.as_deref() else {
201|         let message = "the record names no session of record, and doctor never picks one";
202|         findings.push(Finding::about(
203|             FindingKind::NoSessionOfRecord,
204|             pane,
205|             message.to_owned(),
206|         ));
207|         return;
208|     };
209|     if server.sessions().is_some() && !server.has(record) {
210|         let port = pane.harness.port;
211|         let message = format!(
212|             "the harness server on port {port} does not have the session of record {}",
213|             quoted(record)
214|         );
215|         let finding = Finding::about(FindingKind::SessionOfRecordMissing, pane, message);
216|         findings.push(finding.with_session(Some(record)).with_ports(vec![port]));
217|     }
218| }
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

`sed -n '32,36p' crates/holler-pane/src/reconcile.rs | awk -v n=32 '{printf "%d| %s\n", n+NR-1, $0}'`

````text
32| //!   then). `None` with `at > 0` is an observed home screen, or a pane with no TUI.
33| //! - `last_observed.driven` is left as stored. No port observes DRIVEN before #649, so the
34| //!   mismatch compares SHOWN with `session_of_record` ([`shown_differs`]), which I2 makes
35| //!   the session the hub drives.
36| //! - A record is written only when one of those values changed, or on its first
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

**E16.** `findings_test.rs` pins every kind code in order (13 on main).

`sed -n '1,25p' crates/holler-pane/tests/findings_test.rs | awk -v n=1 '{printf "%d| %s\n", n+NR-1, $0}'`

````text
1| #![allow(clippy::unwrap_used, clippy::expect_used, clippy::panic)] // #647
2| //! The finding vocabulary of reconcile and `holler pane doctor` (#647 AC 25): twelve kinds
3| //! with stable kebab-case codes, each equal to the kind's JSON form. A code is stable once
4| //! merged (ADR-0021 section 9), so the list is pinned in order; #665 appends
5| //! `profile-drift` and edits this list.
6| 
7| use std::collections::BTreeSet;
8| 
9| use holler_pane::error::is_valid_code;
10| use holler_pane::findings::FindingKind;
11| 
12| const CODES: [&str; 13] = [
13|     "herdr-version-unsupported",
14|     "observe-failed",
15|     "unregistered-herdr-pane",
16|     "herdr-pane-missing",
17|     "tmux-session-missing",
18|     "server-wedged",
19|     "server-down",
20|     "no-session-of-record",
21|     "session-of-record-missing",
22|     "tui-foreign-session",
23|     "shown-driven-mismatch",
24|     "stray-session",
25|     "unmanaged-stalled",
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

`sed -n '449,475p' crates/holler-adapter-opencode/src/lib.rs | awk -v n=449 '{printf "%d| %s\n", n+NR-1, $0}'`

````text
449| const GET_AGENT: Route = Route {
450|     method: "GET",
451|     label: "/agent",
452| };
453| const PROMPT_ASYNC: Route = Route {
454|     method: "POST",
455|     label: "/session/:id/prompt_async",
456| };
457| const GET_QUESTION: Route = Route {
458|     method: "GET",
459|     label: "/question",
460| };
461| const GET_PERMISSION: Route = Route {
462|     method: "GET",
463|     label: "/permission",
464| };
465| const GET_MESSAGES: Route = Route {
466|     method: "GET",
467|     label: "/session/:id/message",
468| };
469| 
470| /// One port call in flight: the server's port, the method's `op` and its deadline.
471| struct Call {
472|     port: u16,
473|     op: &'static str,
474|     /// [`Timeouts::call`] from the method's entry (I5).
475|     deadline: Instant,
````

**E21.** The wiring's rule: every wrapper forwards every method, defaults included (D31 W-4, D33). The sentence names the default-armed methods; part 2 edits it.

`sed -n '15,20p' crates/holler-cli/src/pane/wiring/bind.rs | awk -v n=15 '{printf "%d| %s\n", n+NR-1, $0}'`

````text
15| //! **Every method is forwarded (D31 W-4, D33).** Each wrapper here overrides every method of its
16| //! port, the ones the port gives a default included: `HarnessPort`'s `send_prompt` and
17| //! `session_activity`, and `HerdrPort`'s `observe_pane` and `observe_panes` (#832). A default
18| //! never answers in place of the inner port, and `observe_panes` goes to the inner port's own
19| //! `observe_panes`, so a list of panes stays one read of Herdr (ADR-0021 section 14), never one
20| //! `observe_pane` per pane.
````

**E22.** `BindingHost` on main: four methods forwarded one by one.

`sed -n '162,183p' crates/holler-cli/src/pane/wiring/bind.rs | awk -v n=162 '{printf "%d| %s\n", n+NR-1, $0}'`

````text
162| impl HostPort for BindingHost {
163|     fn ensure_session(&self, name: &PaneName, cwd: &str) -> Result<(), PaneError> {
164|         let offered = self.bindings.claim();
165|         self.inner.ensure_session(name, cwd)?;
166|         if let Some(pane) = offered {
167|             self.bindings.bind(pane, name.clone());
168|         }
169|         Ok(())
170|     }
171| 
172|     fn run(&self, name: &PaneName, cwd: &str, argv: &Argv) -> Result<(), PaneError> {
173|         self.inner.run(name, cwd, argv)
174|     }
175| 
176|     fn stop_owned(&self, name: &PaneName) -> Result<(), PaneError> {
177|         self.inner.stop_owned(name)
178|     }
179| 
180|     fn ps(&self, name: &PaneName) -> Result<Vec<u32>, PaneError> {
181|         self.inner.ps(name)
182|     }
183| }
````

**E23.** `Wired<dyn HarnessPort>` forwards eleven methods explicitly.

`sed -n '284,338p' crates/holler-cli/src/pane/wiring/bind.rs | awk -v n=284 '{printf "%d| %s\n", n+NR-1, $0}'`

````text
284| impl HarnessPort for Wired<dyn HarnessPort> {
285|     fn serve_command(
286|         &self,
287|         name: &PaneName,
288|         port: u16,
289|         spec: &ServeSpec,
290|     ) -> Result<Argv, PaneError> {
291|         self.port()?.serve_command(name, port, spec)
292|     }
293| 
294|     fn verify_serving(&self, port: u16, spec: &ServeSpec) -> Result<(), PaneError> {
295|         self.port()?.verify_serving(port, spec)
296|     }
297| 
298|     fn health(&self, port: u16) -> Result<bool, PaneError> {
299|         self.port()?.health(port)
300|     }
301| 
302|     fn create_session(&self, port: u16) -> Result<String, PaneError> {
303|         self.port()?.create_session(port)
304|     }
305| 
306|     fn list_sessions(&self, port: u16) -> Result<Vec<String>, PaneError> {
307|         self.port()?.list_sessions(port)
308|     }
309| 
310|     fn abort(&self, port: u16, session: &str) -> Result<(), PaneError> {
311|         self.port()?.abort(port, session)
312|     }
313| 
314|     fn attach_tui(&self, pane: &PaneId, port: u16, session: &str) -> Result<(), PaneError> {
315|         self.port()?.attach_tui(pane, port, session)
316|     }
317| 
318|     fn select_session(&self, pane: &PaneId, session: &str) -> Result<(), PaneError> {
319|         self.port()?.select_session(pane, session)
320|     }
321| 
322|     fn shown_session(&self, pane: &PaneId) -> Result<Option<String>, PaneError> {
323|         self.port()?.shown_session(pane)
324|     }
325| 
326|     fn send_prompt(&self, port: u16, session: &str, text: &str) -> Result<(), PaneError> {
327|         self.port()?.send_prompt(port, session, text)
328|     }
329| 
330|     fn session_activity(&self, port: u16, session: &str) -> Result<Activity, PaneError> {
331|         self.port()?.session_activity(port, session)
332|     }
333| }
334| 
335| impl HostPort for Wired<dyn HostPort> {
336|     fn ensure_session(&self, name: &PaneName, cwd: &str) -> Result<(), PaneError> {
337|         self.port()?.ensure_session(name, cwd)
338|     }
````

**E24.** `Wired<dyn HostPort>` forwards four methods.

`sed -n '335,351p' crates/holler-cli/src/pane/wiring/bind.rs | awk -v n=335 '{printf "%d| %s\n", n+NR-1, $0}'`

````text
335| impl HostPort for Wired<dyn HostPort> {
336|     fn ensure_session(&self, name: &PaneName, cwd: &str) -> Result<(), PaneError> {
337|         self.port()?.ensure_session(name, cwd)
338|     }
339| 
340|     fn run(&self, name: &PaneName, cwd: &str, argv: &Argv) -> Result<(), PaneError> {
341|         self.port()?.run(name, cwd, argv)
342|     }
343| 
344|     fn stop_owned(&self, name: &PaneName) -> Result<(), PaneError> {
345|         self.port()?.stop_owned(name)
346|     }
347| 
348|     fn ps(&self, name: &PaneName) -> Result<Vec<u32>, PaneError> {
349|         self.port()?.ps(name)
350|     }
351| }
````

**E25.** The existing pin for `WiredHarness`: a recording harness that answers what no default would, and a list of the calls. Part 2 adds `agent_view` to `Recording`, `call_every_method` and the expected calls.

`sed -n '228,281p' crates/holler-cli/tests/pane_integration/bind.rs | awk -v n=228 '{printf "%d| %s\n", n+NR-1, $0}'`

````text
228| fn call_every_method(harness: &dyn HarnessPort) -> Vec<Result<String, String>> {
229|     let (pane, port, spec) = (PaneId::new("w1:p1"), 48100, ServeSpec::default());
230|     let code = |e: PaneError| e.code().to_owned();
231|     vec![
232|         harness.serve_command(&name("demo-c1r1"), port, &spec).map(|_| "serve_command".into()).map_err(code),
233|         harness.verify_serving(port, &spec).map(|()| "verify_serving".into()).map_err(code),
234|         harness.health(port).map(|_| "health".into()).map_err(code),
235|         harness.create_session(port).map(|_| "create_session".into()).map_err(code),
236|         harness.list_sessions(port).map(|_| "list_sessions".into()).map_err(code),
237|         harness.abort(port, "ses_1").map(|()| "abort".into()).map_err(code),
238|         harness.attach_tui(&pane, port, "ses_1").map(|()| "attach_tui".into()).map_err(code),
239|         harness.select_session(&pane, "ses_1").map(|()| "select_session".into()).map_err(code),
240|         harness.shown_session(&pane).map(|_| "shown_session".into()).map_err(code),
241|         harness.send_prompt(port, "ses_1", "hello").map(|()| "send_prompt".into()).map_err(code),
242|         harness.session_activity(port, "ses_1").map(|a| format!("{a:?}")).map_err(code),
243|     ]
244| }
245| 
246| #[test]
247| fn the_wired_harness_forwards_every_method_the_default_armed_ones_included() {
248|     let inner = Arc::new(Recording::default());
249|     let port: Arc<dyn HarnessPort> = inner.clone();
250|     let wired = WiredHarness::new(Ok(port));
251|     let answers = call_every_method(&wired);
252|     assert_eq!(
253|         answers.last(),
254|         Some(&Ok("Busy".to_owned())),
255|         "session_activity is the inner one's, not the default Idle"
256|     );
257|     assert_eq!(
258|         answers[9],
259|         Ok("send_prompt".to_owned()),
260|         "send_prompt is the inner one's, not the default prompt-unsupported"
261|     );
262|     assert_eq!(
263|         *inner.calls.lock().unwrap(),
264|         vec![
265|             "serve_command",
266|             "verify_serving",
267|             "health",
268|             "create_session",
269|             "list_sessions",
270|             "abort",
271|             "attach_tui",
272|             "select_session",
273|             "shown_session",
274|             "send_prompt",
275|             "session_activity",
276|         ],
277|         "each method reached the inner harness once"
278|     );
279| }
280| 
281| #[test]
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

**E27.** (in flight) #835 adds the same pin for the host: the host conformance suite over `WiredHost`.

`git show b488a33:crates/holler-cli/tests/pane_integration/bind.rs | sed -n '326,335p' | awk -v n=326 '{printf "%d| %s\n", n+NR-1, $0}'`

````text
326| /// #835 (A's W6): the production host is `BindingHost` over `WiredHost` over the adapter, so
327| /// `WiredHost` forwards every `HostPort` method too; the suite's `foreground_commands` case
328| /// pins the method #835 adds.
329| #[test]
330| fn the_wired_host_passes_the_host_conformance_suite() {
331|     let result = run_host_conformance(|| (WiredHost::new(Ok(Arc::new(FakeHost::new()))), ()));
332|     assert_eq!(result, Ok(()));
333| }
334| 
335| // --- the resolver (D8, W-5) ---------------------------------------------------------------
````

**E28.** (in flight) #835 adds its method to the misconfigured-socket list in `wired.rs`; part 2 adds its two.

`git show b488a33:crates/holler-cli/tests/pane_integration/wired.rs | sed -n '104,116p' | awk -v n=104 '{printf "%d| %s\n", n+NR-1, $0}'`

````text
104|         let wiring = Wiring::connect().expect("connect needs only the hub (D2, D26)");
105|         let ports = wiring.ports();
106|         let name = PaneName::parse("hlr649-c1r1").unwrap();
107|         let pane = PaneId::new("w1:p1");
108|         [
109|             usage_naming_the_socket("host.ensure_session", ports.host.ensure_session(&name, "/tmp")),
110|             usage_naming_the_socket("host.ps", ports.host.ps(&name)),
111|             usage_naming_the_socket("host.foreground_commands", ports.host.foreground_commands(&name)),
112|             usage_naming_the_socket("harness.health", ports.harness.health(48199)),
113|             usage_naming_the_socket("harness.shown_session", ports.harness.shown_session(&pane)),
114|         ]
115|         .into_iter()
116|         .flatten()
````

**E29.** `Unwired` is the not-implemented port set; every new required method needs an arm here.

`sed -n '311,328p' crates/holler-cli/src/pane/wiring.rs | awk -v n=311 '{printf "%d| %s\n", n+NR-1, $0}'`

````text
311| impl HostPort for Unwired {
312|     fn ensure_session(&self, _name: &PaneName, _cwd: &str) -> Result<(), PaneError> {
313|         Err(PaneError::NotImplemented)
314|     }
315| 
316|     fn run(&self, _name: &PaneName, _cwd: &str, _argv: &Argv) -> Result<(), PaneError> {
317|         Err(PaneError::NotImplemented)
318|     }
319| 
320|     fn stop_owned(&self, _name: &PaneName) -> Result<(), PaneError> {
321|         Err(PaneError::NotImplemented)
322|     }
323| 
324|     fn ps(&self, _name: &PaneName) -> Result<Vec<u32>, PaneError> {
325|         Err(PaneError::NotImplemented)
326|     }
327| }
328| 
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

`sed -n '1104,1109p' docs/adr/ADR-0021.md | awk -v n=1104 '{printf "%d| %s\n", n+NR-1, $0}'`

````text
1104| 16. **`pane relaunch` without `--agent` keeps the recorded agent** (operator, 2026-10-11, #644): a relaunch never changes
1105|     the agent silently.
1106| 17. **`--agent` only sets an agent** (operator, 2026-10-11, #644): no flag clears one; removing an agent is a separate
1107|     decision and is not planned.
1108| 18. **`opencode_agent` is a field of `profile_diff`'s table** (operator, 2026-10-11, #644, section 3): `SpecField::ALL` has
1109|     16 fields, so `profile show` and `profile apply` (#664) see an agent difference. #644 owns the field.
````

**E32.** ADR-0021 section 14 says 'A wrapper port must delegate it' for `observe_pane`.

`sed -n '974,978p' docs/adr/ADR-0021.md | awk -v n=974 '{printf "%d| %s\n", n+NR-1, $0}'`

````text
974| - **`HerdrPort::observe_pane(pane) -> PaneObservation { cwd?, last_screen_change? }`** is the one new port method. It has a
975|   default (`Ok` with both `None`), so an implementation that cannot tell says so and a port that does not override it keeps
976|   compiling; `pane-not-found` for a pane Herdr does not have. A wrapper port must delegate it. `observe_panes(&[PaneId])` is the same
977|   for several panes at once (an empty observation for a pane that cannot be observed), so a list of many unmanaged panes costs
978|   one call: the Herdr adapter reads every `cwd` from the `PaneInfo` of **one** `session.snapshot` (any tab, grid cell or not).
````

**E33.** (in flight) #835's ADR decision 23: required, for the reason of the wrapper that did not forward a default.

`git show b488a33:docs/adr/ADR-0021.md | sed -n '1168,1175p' | awk -v n=1168 '{printf "%d| %s\n", n+NR-1, $0}'`

````text
1168| 22. **`pane import` leaves a pane whose tmux session runs Claude Code alone, and `--adopt-claude-code` will adopt it**
1169|     (operator, 2026-10-11, #835, section 13): #654's option (b), changed to a default plus a switch. The switch is refused
1170|     until the Claude Code harness (#831) lands.
1171| 23. **`HerdrPort::observe_pane` is required** (operator, 2026-10-11, #835, section 14): the operator folded this change to
1172|     #832's method into #835, which makes `HostPort::foreground_commands` required for the same reason. A wrapper port that
1173|     does not forward a defaulted method answers the default silently over the real wiring, where a required method is a
1174|     compile error.
1175| 
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

`sed -n '38,42p' docs/pane-control.md | awk -v n=38 '{printf "%d| %s\n", n+NR-1, $0}'`

````text
38| One Herdr pane shows one tmux session, named as the pane. The harness server (OpenCode) runs **inside that tmux
39| session**: the host adapter starts it from the argv the harness adapter builds, and stops it with everything else the
40| session owns, by `TERM` to every owned process group, one shared grace (2 s by default), then `KILL` to every group that
41| still has a member. A launch is a transaction: it makes the Herdr pane and the tmux session, starts and checks the
42| server, creates the session of record over the harness API, attaches the TUI to it, observes that the TUI shows it, and
````

**E36.** The docs test forces `docs/pane-control.md` section 5 to name every `FindingKind::ALL` code.

`sed -n '188,198p' crates/holler-cli/tests/docs_pane_control_test.rs | awk -v n=188 '{printf "%d| %s\n", n+NR-1, $0}'`

````text
188| #[test]
189| fn section_5_names_every_finding_kind() {
190|     let s5 = section(&guide(), 5);
191|     let missing = FindingKind::ALL
192|         .iter()
193|         .map(|k| ticked(k.code()))
194|         .filter(|t| !s5.contains(t))
195|         .collect();
196|     assert_none("section 5 (doctor) does not name these finding kinds", missing);
197| }
198| 
````

**E37.** `FakeHost` on main: sessions, `ps`, `faults.enter(HostOp::..)` on every method.

`sed -n '176,232p' crates/holler-pane-testkit/src/host.rs | awk -v n=176 '{printf "%d| %s\n", n+NR-1, $0}'`

````text
176| impl HostPort for FakeHost {
177|     fn ensure_session(&self, name: &PaneName, cwd: &str) -> Result<(), PaneError> {
178|         self.faults.enter(HostOp::EnsureSession)?;
179|         self.lock()
180|             .sessions
181|             .entry(name.clone())
182|             .or_insert_with(|| Session {
183|                 cwd: cwd.to_owned(),
184|                 pids: Vec::new(),
185|             });
186|         Ok(())
187|     }
188| 
189|     fn run(&self, name: &PaneName, cwd: &str, argv: &Argv) -> Result<(), PaneError> {
190|         self.faults.enter(HostOp::Run)?;
191|         self.lock().start(name, cwd, argv)
192|     }
193| 
194|     fn stop_owned(&self, name: &PaneName) -> Result<(), PaneError> {
195|         self.faults.enter(HostOp::StopOwned)?;
196|         if let Some(session) = self.lock().sessions.get_mut(name) {
197|             session.pids.clear();
198|         }
199|         Ok(())
200|     }
201| 
202|     fn ps(&self, name: &PaneName) -> Result<Vec<u32>, PaneError> {
203|         self.faults.enter(HostOp::Ps)?;
204|         self.lock()
205|             .sessions
206|             .get(name)
207|             .map(|session| session.pids.clone())
208|             .ok_or_else(|| not_found(name))
209|     }
210| }
211| 
212| impl HostState {
213|     /// `run`'s work: refuse a missing session, then an empty argv; otherwise give the
214|     /// session a new pid and log the argv as given, with its `cwd`.
215|     fn start(&mut self, name: &PaneName, cwd: &str, argv: &Argv) -> Result<(), PaneError> {
216|         let session = self.sessions.get_mut(name).ok_or_else(|| not_found(name))?;
217|         if argv.as_slice().is_empty() {
218|             return Err(PaneError::Usage {
219|                 message: "an empty argv has no program to run".to_owned(),
220|             });
221|         }
222|         session.pids.push(FIRST_PID + self.pids_minted);
223|         self.pids_minted += 1;
224|         self.runs.push((name.clone(), cwd.to_owned(), argv.clone()));
225|         Ok(())
226|     }
227| }
228| 
229| /// The error for a session `name` that does not exist (or a pid it does not hold).
230| fn not_found(name: &PaneName) -> PaneError {
231|     PaneError::PaneNotFound {
232|         what: name.to_string(),
````

**E38.** `HarnessOp` of the fake harness lists the ports' calls; the default-armed two are not there.

`sed -n '47,58p' crates/holler-pane-testkit/src/harness.rs | awk -v n=47 '{printf "%d| %s\n", n+NR-1, $0}'`

````text
47| pub enum HarnessOp {
48|     ServeCommand,
49|     VerifyServing,
50|     Health,
51|     CreateSession,
52|     ListSessions,
53|     Abort,
54|     AttachTui,
55|     SelectSession,
56|     ShownSession,
57| }
58| 
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

`sed -n '290,300p' crates/holler-pane/src/tx_switch.rs | awk -v n=290 '{printf "%d| %s\n", n+NR-1, $0}'`

````text
290|     let port = record.harness.port;
291|     if ports.harness.health(port)? {
292|         return Ok(());
293|     }
294|     let pane = Some(&record.name);
295|     let remedy = match FindingKind::ServerDown.remedy(pane, FixState::NotFixable) {
296|         Some(remedy) => remedy,
297|         None => doctor_command(pane, false),
298|     };
299|     Err(PaneError::Refused {
300|         code: SERVER_UNHEALTHY,
````

**E41.** (in flight) #831 (branch `issue-831-implementation`, `b3f0295`) keys every `HarnessPort` method by `HarnessEndpoint`: a harness method added now would be rewritten by it.

`git show b3f0295:docs/adr/ADR-0021.md | sed -n '1040,1046p' | awk -v n=1040 '{printf "%d| %s\n", n+NR-1, $0}'`

````text
1040| 3. **The endpoint.** A Claude Code process has no port. `holler_pane::ports` gains
1041|    `enum HarnessEndpoint { Port(u16), Process { pane: PaneName, pid: u32 } }`, and **every `HarnessPort` method keyed by
1042|    `port: u16` is keyed by `&HarnessEndpoint` instead** (`verify_serving`, `health`, `create_session`, `list_sessions`,
1043|    `abort`, `send_prompt`, `session_activity`; `attach_tui` and `shown_session` gain it too, `shown_session(pane, endpoint)`,
1044|    since a single process is read through its endpoint and not through a TUI). This changes the port rather than adding
1045|    default-armed twins, as #802 decided (decision 2 there): a call site that forgets the endpoint is a compile error, and
1046|    every implementation is in this workspace. `HarnessInfo.port` becomes `Option<u16>` (`#[serde(default,
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

**E44.** The opt-in scenario ends with 'doctor reports no finding'; with real detached tmux sessions every scratch pane is `unattached`, so part 2 changes this assertion (and #667's moved copy).

`sed -n '232,240p' crates/holler-cli/tests/pane_integration/scenario.rs | awk -v n=232 '{printf "%d| %s\n", n+NR-1, $0}'`

````text
232| }
233| 
234| /// The end state after the repairs: `doctor` reports no finding, `pane list` equals Herdr's
235| /// snapshot for the scratch panes of the scenario's workspace, and none of them is in the
236| /// unmanaged table.
237| fn assert_end_state(scratch: &Scratch) {
238|     let report = scratch.json(&["pane", "doctor"], 0).data();
239|     assert_eq!(report["findings"], json!([]), "doctor reports no finding: {report}");
240|     let observed: BTreeSet<(String, String)> = scratch
````

**E45.** The CHANGELOG shape: one bullet at the top of `### Enhancements`.

`sed -n '8,11p' CHANGELOG.md | awk -v n=8 '{printf "%d| %s\n", n+NR-1, $0}'`

````text
8| ## [Unreleased]
9| 
10| ### Enhancements
11| - Pane control, the operator guide (epic [#633](https://github.com/Performant-Labs/holler/issues/633)): [`docs/pane-control.md`](docs/pane-control.md) documents every `holler pane` and `holler profile` verb ([#652](https://github.com/Performant-Labs/holler/issues/652)).
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

`sed -n '1090,1094p' docs/adr/ADR-0021.md | awk -v n=1090 '{printf "%d| %s\n", n+NR-1, $0}'`

````text
1090| 12. **The agent reaches `HarnessPort` by changing the port** (operator, 2026-10-11, #802): `serve` became
1091|     `serve_command(name, port, &ServeSpec)`, so an omitted agent is a compile error at every call site; no additive
1092|     default-armed method.
1093| 13. **`ServeSpec`** (operator, 2026-10-11, #802): a `#[non_exhaustive]` struct holding `agent: Option<AgentKey>` today. New
1094|     fields of the pane's spec for its server go into it, never into new parameters.
````

**E52.** The doctor test rig builds every world in `make_live`: the place where a healthy fake world gets its tmux client (a fresh fake session has none, as a fresh real one has none).

`sed -n '367,397p' crates/holler-cli/tests/pane_verbs/doctor/rig.rs | awk -v n=367 '{printf "%d| %s\n", n+NR-1, $0}'`

````text
367| fn make_live(
368|     herdr: &FakeHerdr,
369|     host: &FakeHost,
370|     harness: &FakeHarness,
371|     seed: &Seed,
372|     port: u16,
373| ) -> Live {
374|     let name = PaneName::parse(seed.name).expect("a pane name");
375|     let spec = HerdrSpec {
376|         session: SCRATCH.to_owned(),
377|         workspace: SCRATCH.to_owned(),
378|         grid: seed.grid,
379|     };
380|     let pane = herdr.ensure_pane(&spec).expect("place the pane");
381|     host.ensure_session(&name, "/srv/demo")
382|         .expect("the tmux session");
383|     if let Some(dir) = seed.data_dir {
384|         harness.set_data_dir(port, dir);
385|     }
386|     start_server(harness, &name, port);
387|     let session = harness.create_session(port).expect("a session");
388|     harness
389|         .attach_tui(&pane.pane_id, port, &session)
390|         .expect("attach the TUI");
391|     Live {
392|         name,
393|         herdr: pane,
394|         port,
395|         session,
396|     }
397| }
````

**E53.** The host conformance suite is one table of cases; part 2 adds one.

`sed -n '43,57p' crates/holler-pane-testkit/src/conformance/host.rs | awk -v n=43 '{printf "%d| %s\n", n+NR-1, $0}'`

````text
43| const CASES: [(&str, Case); 9] = [
44|     (
45|         "ps-of-missing-session-is-pane-not-found",
46|         ps_of_missing_session_is_pane_not_found,
47|     ),
48|     (
49|         "run-in-missing-session-is-pane-not-found",
50|         run_in_missing_session_is_pane_not_found,
51|     ),
52|     ("run-adds-a-process", run_adds_a_process),
53|     ("ensure-session-is-idempotent", ensure_session_is_idempotent),
54|     ("run-empty-argv-is-usage", run_empty_argv_is_usage),
55|     (
56|         "stop-owned-stops-every-owned-process",
57|         stop_owned_stops_every_owned_process,
````
