# Handoff-T-red: Phase 4 - #647 part 2: `unattached` and `agent-cannot-dispatch`

**Date:** 2026-10-12 (10:25 MDT)
**Branch:** `issue-647-implementation`. The RED tests sit on top of `3a09f7d`, which is on top of `45bff85` and A's PASS at `4973f9a`.
**Brief / wireframe reviewed:** `docs/handoffs/647-part2-brief.md` (D1 to D19, acceptance 1 to 21, W1 to W8) and `docs/handoffs/647/handoff-A.md` (PASS, W-1 to W-8). There is no wireframe because the story has no UI surface.

## A precondition

Confirmed: A returned **PASS** on the plan (`4973f9a`, 0 blocks, 8 warns). This RED follows A's notes for T:

- W-6: the two host-read assertions now share one list.
- W-4: Claude Code answers an empty view only for a `Process` endpoint, and a `Port` endpoint is `server-unsupported`.
- W-1: acceptance 9's "quoting nothing" stands.

It also pins three of A's notes for F as tests:

- W-3: a profile member's message names `--profile`.
- W-4: both Claude implementations refuse a `Port` endpoint.
- W-1: the message list is not quoted.

## Two commits made before the tests (scope 1, acceptance 20)

The driver commits everything a phase leaves with one `git add -A`. A contract commit left for the driver would land in the same commit as the tests, and acceptance 20 requires the contract commit to come before every test or code commit. So T made the two commits that must come first:

1. **`45bff85` `docs(adr): contract amendment for the attached-clients and agent-view reads (#647)`.** It is documentation only:
   - ADR-0021 gets the brief's six edits:
     - the section 2 table rows and the #835 paragraph;
     - the section 9 `pane doctor` row;
     - the section 12 "Decided (#647, part 2)" paragraph;
     - a "Deferred" bullet;
     - "Decisions taken" item 34.
   - The `ports.rs` module doc gets a sixth "Amended since" bullet, with the method names in code spans.
   - Verified at that tree, with the in-progress test edits set aside:
     - `cargo test -p holler-pane --doc`: 7 passed;
     - `docs_cli_test`: 3 passed (the new `holler pane relaunch PANE --agent=KEY` span parses);
     - `pane_verbs import`: 46 passed;
     - a full `cargo test --workspace --no-run` at that point: exit 0.
2. **`3a09f7d` `test(pane): move switch.rs's two harness doubles to switch/doubles.rs (#647)`.** The brief's Blast radius gives this pure move to F. It moves test files, and T owns test files, so T made it:
   - `WriterInSelect` and `SeamHarness` (with `SeamLog` and `QueuedPrompt`) move unchanged.
   - `WriterInSelect`'s fields become `pub(super)`.
   - `switch.rs` re-exports `SeamHarness` and `SeamLog` for `reset.rs`.
   - Verified at that tree: `pane_verbs` 347 passed, and clippy is clean.
   - `switch.rs` goes from 893 to 700 lines.

## What F must add for the suite to compile (the API the tests name)

Every name below comes from the brief or A's handoff, except the ones marked *chosen here*:

| Item | Where | Notes |
|---|---|---|
| `HostPort::attached_clients(&self, &PaneName) -> Result<u32, PaneError>`, **required** | `holler-pane/src/ports.rs` | D4 |
| `HarnessPort::agent_view(&self, &HarnessEndpoint, &str, usize) -> Result<AgentView, PaneError>`, **required** | `ports.rs` | D4 |
| `AgentView { pub turns: Vec<TurnAgent>, pub dispatchers: Vec<AgentKey> }` | `src/ports/agent_view.rs` (A's W-5), re-exported as `holler_pane::AgentView` | `#[non_exhaustive]`; derives `Debug, Clone, Default, PartialEq, Eq` |
| `AgentView::with_turns(self, Vec<TurnAgent>) -> Self` and `with_dispatchers(self, Vec<AgentKey>) -> Self` | same | names *chosen here*. The tests pass `vec![..]`, so `impl IntoIterator` also fits. `with_dispatchers` **sorts and drops duplicates**, so that rule is written once (*chosen here*) |
| `TurnAgent { pub agent: Option<AgentKey>, pub can_dispatch: Option<bool> }` and `TurnAgent::new(agent, can_dispatch)` | same, re-exported as `holler_pane::TurnAgent` | the constructor is *chosen here*. Derives `Debug, Clone, PartialEq, Eq` (`vec![turn; 3]` needs `Clone`). May be `#[non_exhaustive]` |
| `FindingKind::Unattached` and `FindingKind::AgentCannotDispatch`, appended in that order to the enum and to `ALL`; codes `unattached` and `agent-cannot-dispatch` | `findings.rs` | D17 |
| `FindingKind::remedy_for(self, Option<&PaneName>, FixState, Option<&AgentKey>) -> Option<String>`; `remedy` delegates with `None` | `findings.rs` | D8 |
| `HostOp::AttachedClients` (`"host.attached_clients"`); `FakeHost::attach_client(&self, &PaneName) -> Result<(), PaneError>` and `detach_clients(&self, &PaneName) -> Result<(), PaneError>` | testkit `host.rs` | D14. The `Result` type and the missing-session answer (`pane-not-found`) are *chosen here*, as `exit_process` and `end_session` answer |
| `HarnessOp::AgentView` (`"harness.agent_view"`); `FakeHarness::seed_turns(&self, u16, &str, Vec<TurnAgent>)` and `seed_dispatchers(&self, u16, Vec<AgentKey>)` | testkit `harness.rs` | D14. Argument types are *chosen here*. Dispatchers belong to the port. The tests read turns only on the port they were seeded on |
| `ClaudeOp::AgentView` (`"harness.agent_view"`) | testkit `claude.rs` | A's W-4 |
| The forwards or answers in `HarnessRouter`, `FakeHarnesses`, `FakeClaude`, `ClaudeHarness`, `TmuxHost`, `OpenCodeHarness`, `BindingHost`, `Wired` x2 and `Unwired` | production | the compiler lists them |

T added the forwards that live in **test files**:

- for the host: `TestHost`, the host `Mutant`, the four `LinkedHost`s, `LostChain` and the new `Counting`;
- for the harness:
  - `TestHarness`, `ServerOnly`, both `Recording`s, `Resolving`, `HealthGate`;
  - `WriterInSelect`, `SeamHarness`, both `HookedHarness`es;
  - the harness `Mutant` and the Claude `Mutant`.

Each double that wraps a fake forwards to it in one line (W6).

## Tests authored

### Contract and types (`holler-pane`)

| Test | Pins | Tier |
|---|---|---|
| `ports.rs` doctest `HostPort` "Nor does one that leaves out `attached_clients`" (`NoClientCount`, `compile_fail,E0046`) | acc 1: `attached_clients` is required | doctest (a compile-time property) |
| `ports.rs` doctest `HarnessPort` (`NoAgentView`, `compile_fail,E0046`) | acc 1: `agent_view` is required | doctest |
| `PidsOnly` gains `attached_clients`, and its lead sentence names the one method it leaves out | acc 1: it still pins `foreground_commands` alone | doctest, edited |
| `ports.rs` doctests `HarnessPort` (two `compile_fail,E0639`) | acc 2: a struct literal of `AgentView` does not compile outside the crate. **A guard, not RED** (see below) | doctest |
| `tests/agent_view_test.rs` (new): `the_default_view_is_empty`, `a_turn_holds_its_agent_and_whether_it_can_dispatch`, `the_turns_builder_keeps_the_order_given`, `the_dispatchers_builder_sorts_and_drops_duplicates`, `the_builders_compose` | acc 2 | unit |
| `findings_test.rs`: `CODES` goes from 16 to 18 (`finding_kind_codes_are_stable`) | acc 3 | unit |
| `findings_test.rs`: `agent_cannot_dispatch_names_the_relaunch_with_the_one_key`, `a_key_with_a_leading_dash_is_joined_to_the_flag`, `agent_cannot_dispatch_without_a_key_or_a_pane_has_no_remedy`, `unattached_never_has_a_remedy`, `remedy_is_the_table_with_no_agent_and_the_agent_touches_only_its_kind` | acc 4 (the table) | unit |

The clap half of acceptance 4 is in `holler-cli`: `doctor::agent::a_dispatcher_whose_key_starts_with_a_dash_still_parses_as_the_agent`. A doctor finding with the dispatcher `-x` gives a remedy that parses, through `Cli::try_parse_from`, as relaunch's agent `-x`.

### Test kit

| Test | Pins |
|---|---|
| Host conformance case 11, `attached-clients-of-fresh-session-is-zero-and-of-missing-is-pane-not-found` | acc 6. A missing session is `pane-not-found`, and a fresh session has 0 clients. The real tmux adapter runs it in `real_tmux_test` (opt-in) |
| `host_conformance_test.rs`: `DOCUMENTED_CASES` grows to 11; new mutants `AttachedAlwaysOne` and `AttachedOfMissingIsZero`, each with a test asserting the suite fails on case 11 | acc 6 |
| `fake_host_test.rs`: `attached_clients_counts_what_attach_client_added`, `attached_clients_of_a_missing_session_is_pane_not_found`, `clients_survive_stop_owned_and_a_second_ensure_session`, `a_fault_on_attached_clients_fails_that_call_only`. The wedged and op-name tests are extended | acc 5 |
| Harness conformance case 17, `agent-view-of-a-fresh-session-has-no-turns` | acc 7. An unserved port is `unavailable`, checked first. An unknown id is `session-not-found`. A fresh session has no turn |
| `harness_conformance_test.rs`: 17 documented cases; mutants `AgentViewAlwaysEmpty` and `AgentViewInventsATurn` | acc 7 |
| `fake_harness_agent_test.rs` (new): `the_op_is_named_harness_dot_agent_view`, `agent_view_answers_the_newest_seeded_turns_and_the_dispatchers`, `an_unseeded_session_on_a_running_server_answers_the_empty_view`, `agent_view_reaches_the_server_before_it_looks_at_the_session`, `a_process_endpoint_is_process_unsupported`, `agent_view_passes_the_faults_and_the_seeds_bypass_the_log` | acc 7 (exact errors, `abort`'s check order, E84) |
| Claude conformance case 13, `agent-view-of-a-process-is-empty`, with the mutant `GuessesATurn` | acc 7 (W-4). Run over `FakeHarnesses` too, which pins the `Process` route |
| `fake_claude_test.rs`: `agent_view_is_empty_for_a_process_and_refused_for_a_port`, `a_fault_on_agent_view_fails_that_call_and_is_logged` | acc 7 and W-4 |
| `fake_harnesses_test.rs`: `agent_view_reaches_the_fake_its_endpoint_names` | acc 7 (the router) |

### Adapters

| Test | Pins |
|---|---|
| `holler-adapter-host/tests/attached_test.rs` (new): `attached_clients_reads_the_sessions_count` (0, 1, 2, repeated per pane), `the_listing_is_ps_s_exact_target_argv` (`list-panes -s -t =demo-c1r1:`, no shell, a count only), `a_value_that_is_not_a_count_is_unavailable_and_quotes_nothing` (`""`, a marker word, `-1`, `1.5`), `attached_clients_of_a_missing_session_is_pane_not_found`, `another_tmux_failure_is_unavailable_with_its_first_line` | acc 8 |
| `fake_tmux_test.rs`: `METHODS` gains `HostOp::AttachedClients` (the bound and the missing binary), 822 lines | acc 8 |
| `holler-adapter-opencode/tests/agent_view_test.rs` (new), 13 tests | acc 9, over the hermetic stub and the fixtures. The list follows this table |
| `tests/fixtures/agents/agents.json` and `messages.json` (new) | Cut from a scratch OpenCode 1.19.0 server at 10:00 MDT today; see "Facts measured" below |
| `real_opencode_test.rs` (opt-in): `ac647_agent_view_reads_a_real_servers_turns_and_agents` (**proves `info.agent`**, D15) and `ac647_a_scratch_server_without_project_agents_has_no_dispatcher` (acc 18's claim). The rig gains `write_project_config` and `Recording::agent_view` | acc 9 and 18 |
| `holler-adapter-claude/tests/hermetic_test.rs`: `agent_view_is_empty_for_a_process_and_refused_for_a_port` | acc 7 and W-4 |

What the 13 tests of `agent_view_test.rs` cover:

- The real server's answers: the newest three user turns, oldest first. The assistant message is not a turn. The one dispatcher is `orchestrator`.
- A table of 14 rule lists decides `Some(false)`, `Some(true)` and `None`.
- Dispatchers: only custom (`native: false`) agents that could be a pane's default and can dispatch, sorted and without duplicates. A name that is not an `AgentKey` is left out.
- Errors:
  - an unknown session is `session-not-found`;
  - a message list that is not a list is `unavailable`, naming `/session/:id/message` and quoting nothing;
  - an agent list that is not a list is `unavailable`;
  - an agent with no `mode` is `unavailable`;
  - a frozen server is `timeout` with op `harness.agent_view`, equal to `HarnessOp::AgentView.as_str()`;
  - a refused port is `unavailable`;
  - a `Process` endpoint is `process-unsupported`.
- The call only reads: `GET` requests only.

### Reconcile and `holler pane doctor` (`holler-cli`, `pane_verbs`)

| Test | Pins |
|---|---|
| `doctor/attach.rs` (new), 8 tests: `a_session_with_no_client_raises_one_unattached_and_attached_raises_nothing`, `the_verb_reports_unattached_in_both_formats`, `a_missing_session_raises_tmux_session_missing_and_no_count_is_asked`, `a_failed_count_is_observe_failed_and_not_unattached`, `a_parked_pane_and_the_orchestrators_pane_raise_it_too`, `a_world_built_without_a_client_raises_unattached_for_every_pane`, `unattached_is_reported_once_per_pass_and_writes_nothing`, `fix_does_nothing_for_unattached` | acc 10, 11, 14 and 16's proof. The message is checked for the substrings `no tmux client is attached` and `no Holler verb attaches one` |
| `doctor/agent.rs` (new), 18 tests | acc 4 (clap), 12, 13, 14 and W-3. The list follows this table |
| `doctor/read_only.rs`: `doctor_without_fix_makes_only_read_calls` uses the shared `rig::HOST_READS` and `rig::HARNESS_READS`, and asserts that the pass makes both new reads | acc 15 and W-6 |
| `doctor.rs`: the `--fix` test's host-reads assertion uses `HOST_READS` | acc 15 and W-6 (the second assertion A named, `doctor.rs:338-342`) |
| `doctor/rig.rs`: `make_live` attaches a client, and the module doc says so | acc 16 |
| `pane_claude/rig.rs` (`seed_claude`, `seed_opencode`, `LinkedHost` forward) and `pane_claude/mixed.rs` (`applied()` attaches through `rig.host`) | acc 16 |

The 18 tests of `doctor/agent.rs`:

- The finding: `every_recent_turn_unable_to_dispatch_raises_one_finding_naming_the_relaunch`, `fix_changes_nothing_for_it`, `a_dispatcher_whose_key_starts_with_a_dash_still_parses_as_the_agent`, `several_dispatchers_give_no_remedy_and_a_message_that_lists_them` (two quoted; up to three of four) and `a_profile_members_message_names_its_profile` (W-3).
- The negatives, each its own test:
  - `a_record_that_names_build_raises_nothing_and_asks_nothing` and `a_record_that_names_the_orchestrator_raises_nothing_and_asks_nothing`;
  - `no_dispatcher_raises_nothing`, `one_recent_turn_that_could_dispatch_raises_nothing`, `one_recent_turn_the_server_says_nothing_about_raises_nothing` and `an_empty_view_raises_nothing`;
  - `a_failed_view_is_observe_failed`;
  - `a_down_server_is_never_asked`, `a_server_whose_health_check_fails_is_never_asked`, `a_healthy_server_whose_list_fails_is_never_asked`, `a_server_without_the_session_of_record_is_never_asked` and `a_record_without_a_session_is_never_asked`.
- Once per pass: `a_second_pass_reports_the_same_two_findings_once_each_and_writes_nothing`.

`RECENT_TURNS = 3` is pinned from both sides:

- The positive world seeds four turns whose oldest could dispatch. With 3 it raises the finding; with 4 or more it would not.
- `one_recent_turn_that_could_dispatch_raises_nothing` seeds exactly three. With 3 it raises nothing; with 2 it would raise one.

### Wrapper forwarding (`holler-cli`, `pane_integration`)

| Item | Test |
|---|---|
| **W1** | `forwarding.rs` (new), over the recording host `Counting` (`Ok(7)` for one name, an error for another; every call logged by method and name): `the_binding_host_forwards_attached_clients`, `the_wired_host_forwards_attached_clients`, `the_binding_host_over_the_wired_host_forwards_attached_clients` |
| **W2** | `bind.rs`'s existing `the_binding_host_passes_the_host_conformance_suite` and `the_wired_host_passes_the_host_conformance_suite` pick up case 11 with no edit |
| **W3** | `forwarding.rs::a_misconfigured_wired_host_answers_its_error_from_attached_clients`; `wired.rs` gains `host.attached_clients`; `pane_verbs/main.rs::unwired_ports_answer_not_implemented` gains both methods |
| **W4** | `bind.rs`: `Recording::agent_view` answers `distinctive_view()` (one turn as `orchestrator`, the dispatcher `reviewer`) and records its arguments. `call_every_method` gains `agent_view`, and the expected-calls list gains `"agent_view"`. The test asserts the wrapper's answer equals the inner one's and the inner call saw `(Port(48100), "ses_1", 3)`. The existing `answers[11..]` slice becomes `answers[11..15]` |
| **W5** | `a_misconfigured_wired_harness_answers_its_error_from_every_method` covers `agent_view` through `call_every_method`; `wired.rs` gains `harness.agent_view` |
| **W6** | the one-line forwards listed above. The doctor tests attach their client through the fake that each double wraps |
| **W7** | the mutation check, for Phase 6 (T verifies, F records) |
| **W8** | `bind.rs` module doc: F's |

### Docs and gates

| Test | Pins |
|---|---|
| `docs_pane_control_test.rs::section_5_rows_of_part_2_name_their_exact_commands` | acc 17. The `unattached` row ends `\| none \|`. The `agent-cannot-dispatch` row ends `` \| `holler pane relaunch PANE --agent=KEY` \| `` and names `--profile` (W-3). Section 5 states the "terminal" limit. `section_5_names_every_finding_kind` already covers the codes once they are in `ALL` |
| `scratch/verbs.rs::assert_end_state` | acc 18. It now compares sorted `(pane, kind)` lists against exactly one `unattached` per pane in `panes`. The doc comment says why |
| `scenario.rs::break_both` | acc 18. The C1 check leaves `unattached` out |

These run opt-in only (real Herdr, tmux and OpenCode); the default run compiles them.

Acceptance 19 to 21 are gates and greps for Phase 6 and S:

- 19: `cli-surface.txt`, lint, clippy and the privacy grep. T ran lint and the privacy grep on the RED: both clean.
- 20: the commit order. Satisfied by `45bff85` coming first.
- 21: the module docs.

## RED confirmation

**1. The real tree** (`cargo build --workspace --tests --keep-going`, exit 101). Every error is a missing piece of the new API, and none is a typo or a setup error:

```
crates/holler-pane-testkit/src/conformance/host.rs   E0599 no method named `attached_clients` found for reference `&dyn HostPort` (x2)
crates/holler-pane-testkit/src/conformance/harness.rs E0599 no method named `agent_view` found for reference `&dyn HarnessPort` (x3)
crates/holler-pane-testkit/src/conformance/claude.rs  E0599 no method named `agent_view` ...; E0432 unresolved import `holler_pane::AgentView`
crates/holler-pane/tests/findings_test.rs   E0599 no variant ... `AgentCannotDispatch` (x6), `Unattached` (x1); no method named `remedy_for` (x2)
crates/holler-pane/tests/agent_view_test.rs E0432 unresolved imports `holler_pane::AgentView`, `holler_pane::TurnAgent`
crates/holler-pane/tests/ports_test.rs      E0432 `AgentView`; E0407 `attached_clients` / `agent_view` is not a member of trait
crates/holler-pane/tests/claude_kind_test.rs E0432 `AgentView`; E0407 `agent_view` is not a member of trait `HarnessPort`
error: could not compile `holler-pane-testkit` (lib) due to 7 previous errors
```

Every target that depends on the test kit (the adapters' tests and `holler-cli`'s) stops at that lib error.

**2. The doc tests on the real tree** (`cargo test -p holler-pane --doc`): 9 passed, 2 failed.

```
test crates/holler-pane/src/ports.rs - ports::HostPort (line 383) - compile fail ... FAILED
test crates/holler-pane/src/ports.rs - ports::HarnessPort (line 457) - compile fail ... FAILED
Test compiled successfully, but it's marked `compile_fail`.   (x2)
test crates/holler-pane/src/ports.rs - ports::HostPort (line 357) - compile fail ... ok      (PidsOnly, now E0407 + E0046)
test crates/holler-pane/src/ports.rs - ports::HarnessPort (line 501|504) - compile fail ... ok   (the E0639 guards)
```

**3. Behavioural RED, proved with a throwaway shim.** A failing test-kit lib hides the downstream tests, so a typo in one of them would stay invisible until F's code compiles. To rule that out, T applied a temporary shim of the API above to the 13 production files:

- the types and the trait methods;
- `NotImplemented`, `Ok(0)` or an empty view from every implementation;
- no-op seed and attach methods;
- no reconcile rule;
- the kinds left out of `ALL`.

T built and ran everything against it, then restored the 13 files byte for byte from a backup. `grep -rn SHIM crates/` finds nothing, and the only production-file change is the doc tests in `ports.rs`. Against the shim:

- `cargo clippy --workspace --all-targets -- -D warnings` was clean.
- Every new or edited test failed on its assertion about the missing behaviour, for example:
  - `left: Err(NotImplemented) right: Ok(())`;
  - `exactly one unattached for demo-c1r1; findings: []`;
  - `exactly one agent-cannot-dispatch for demo-c1r1; findings: []`;
  - `#647 part 2: a healthy pane with no recorded agent is asked its agent view: [Health, ListSessions, ShownSession]`;
  - `the wired host forwards: left: Ok(0)`;
  - `host.attached_clients: want usage naming HOLLER_TMUX_SOCKET; got Ok(0)`;
  - `the unattached row must end | none |: None`;
  - the conformance suites: `CaseFailure { case: "attached-clients-of-fresh-session-is-zero-...", detail: "... expected pane-not-found, got not-implemented" }`, and the same for `agent-view-of-a-fresh-session-has-no-turns` and `agent-view-of-a-process-is-empty`.
- Failing targets:
  - `holler-pane`: `agent_view_test` (4 of 5), `findings_test` (3).
  - The test kit:
    - `fake_host_test` (4) and `host_conformance_test`;
    - `fake_harness_agent_test` (5 of 6), `harness_conformance_test`;
    - `claude_conformance_test`, `fake_claude_test` (2) and `fake_harnesses_test` (3).
  - The adapters: `attached_test` (5 of 5), `fake_tmux_test` (2), OpenCode's `agent_view_test` (13 of 13) and Claude's `hermetic_test` (1).
  - `holler-cli`:
    - `pane_verbs`: 15, which is all positives of `doctor/attach.rs` and `doctor/agent.rs` plus `read_only`;
    - `pane_integration`: 10 (W1 to W5 and `wired.rs`);
    - `docs_pane_control_test`: 1.
- `pane_claude` (43) and `profile_verbs` (118) passed. The new forwards break nothing there.

## Notes on validity

- **The negatives of `doctor/agent.rs` are guards.** There are 12, and they assert that nothing is raised and nothing is asked. Before the feature exists the rule raises nothing, so they cannot fail on an assertion. On the real tree they are RED only by not compiling (`seed_turns` and `HarnessOp::AgentView` do not exist). After F they fail if the rule fires too eagerly or asks for the view when it should not. The same holds for:
  - `unattached`'s missing-session test;
  - `a_record_that_names_*`;
  - the `remedy` equivalence test.
- **Stable rustdoc does not enforce a `compile_fail` error code.** The two `compile_fail,E0639` doc tests pass on `main`, where `AgentView` does not exist (E0422), so they are not RED. Behind any compile error, "does not exist" and "is `#[non_exhaustive]`" look the same. After F they fail if `#[non_exhaustive]` is left out. The two new `E0046` doc tests are RED for the reason the brief gives: on `main`, the implementation that leaves out the new method compiles.
- **Where the E0639 tests sit.** They are on `HarnessPort`'s doc in `ports.rs`, not in `agent_view.rs`. That file is F's to create, and a doc test only runs once its module exists. They read correctly there, because the trait's doc describes the view it answers.
- **A conflict in acceptance 9, resolved for the ADR.** The brief says "a message without an agent is skipped". ADR-0021 section 12, as amended at `45bff85`, says a turn's `agent` is "`None` when it recorded none". T pinned the ADR: a user message with no `agent` is a turn `TurnAgent { agent: None, can_dispatch: None }` (`a_turn_the_server_says_nothing_about_is_none`). It is still one of "the last `max_turns` user turns". Because `can_dispatch` is `None`, it stops the rule, which is the conservative reading. This is journalled in `decisions.md`.

## Facts measured outside the repository (this phase)

On 2026-10-12 at 09:58 to 10:01 MDT, T started `opencode serve --pure` (OpenCode 1.19.0) on port 48190:

- The setup:
  - a scratch directory laid out as `real_opencode/rig.rs` lays one out: the dead-end `CONFIG`, `isolated_vars`' environment and the five `OPENCODE_DISABLE_*` flags;
  - the project `opencode.json` of the brief;
  - the model guard checked: `/config/providers` listed only `deadend`.
- `GET /agent` listed 8 agents. Their `mode`, `native` and `hidden` values, and their `task` and `*` rules, match the brief's table.
- Four user messages were sent with `noReply`, as `build`, `build`, `orchestrator` and `build`. A fifth was sent as `build` without `noReply`: the dead-end model failed and left an empty assistant message.
- `GET /session/:id/message` answered 6 messages. Each user message's `info.agent` was the agent it was sent as. The assistant message also carries `agent`, so `agent_view` has to filter on `info.role == "user"`.
- `GET /session/ses_nosuch/message` and `GET /session/ses_nosuch` both answered 404.
- The server was stopped afterwards.

The fixtures leave out each agent's `prompt` and every `external_directory` rule, and replace the scratch paths with `/srv/demo` and `srv/demo-scratch`.

## Ready for F

T-red is complete, and the RED is valid. F may implement against the authored tests.

For F:

- Implement exactly the API in the table above.
- Do not create the doc tests' module elsewhere.
- Keep `with_dispatchers` sorting and dropping duplicates.
- The opt-in real-OpenCode test is where the message field is proved (D15). Run it with `HOLLER_TEST_OPENCODE=1 cargo test -p holler-adapter-opencode --test real_opencode_test -- --ignored --test-threads=1 ac647`.
