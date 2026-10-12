# [feat(pane): a Claude Code harness for panes: launch, register, drive, observe](https://github.com/Performant-Labs/holler/issues/831)

Part of epic #633 (amended 2026-10-11, claude). **Blocked by the spike story and by the contract amendment it recommends (amend-first rule).**

## Goal
`holler pane launch` and `relaunch` accept a harness choice of OpenCode or Claude Code (`--harness`). A Claude Code pane is a registry pane like any other: a record in the hub's pane registry with the session of record, a harness health, `SHOWN` and `DRIVEN`, `hold`, a `profile` membership, and the same verbs (`list`, `get`, `watch`, `switch`, `reset`, `park`, `say`, `interrupt`, `close`, `doctor`). A profile spec names the harness, so `holler profile apply` can build a grid that mixes OpenCode and Claude Code panes, with the orchestrator role on any of them.

## Work (shaped by the spike's answers; the spike decides the mechanism)
1. The contract amendment, first: `HarnessKind` gains a variant (and `holler_proto::vocab::HARNESS_IDS` the id; a test pins both lists together), the `harness` field of `Pane` and `ProfileSpec`, ADR-0021 and the epic's contract block edited in one PR.
2. The `HarnessPort` impl for Claude Code in a new adapter crate or module, behind the same seam as the OpenCode adapter; the host adapter starts and stops the process by the recorded pid.
3. The test kit: a fake Claude Code harness and its conformance suite; the real-tool test is opt-in and `#[ignore]`.
4. `--harness` in the spec flags (declared once in `cli.rs`), `profile show` and `apply` compare it, doctor codes for a harness that cannot be reached.
5. The wizard (story #803) uses it for the Claude panes of a mixed grid.

## Acceptance
A scratch scenario launches one OpenCode pane and one Claude Code pane from one profile, shows both in `holler pane list` with SHOWN and DRIVEN, delivers a message to each through `holler say`, interrupts one, parks one, and `doctor` is clean; the subscription login is never read or printed; the roster, the watchdog and `doctor` see the Claude pane.

## Pipeline
`rigor: in-session`. Wave 2 of this amendment: after the spike and the amendment.

## Operator decision on abort (2026-10-11)

The spike (#830, merged as #833) found no in-place, no-typing interrupt for a Claude Code pane, and asked whether one Esc keypress through `HerdrPort::send_keys` may be used as the interrupt. **The operator's answer is no.** Abort for a Claude Code pane is **stop and relaunch with `--resume <session of record>`**: it loses only the turn in flight and keeps the session. `AbortStyle` therefore needs the relaunch style first; an `InPlace` style for Claude Code is not built. The optional `PreToolUse` hook gate ("stop after this step") stays a later, separate option. Record this decision in the contract amendment (ADR-0021) and in the research note's "Decisions" section, which still lists it as the operator's to confirm.

## Rules
See the epic (#633): public repository (no personal host, tailnet or account names), the amend-first rule for the contract, `in-session` rigor, no story touches a live fleet, a real Herdr session, a running pane or a real Claude or OpenCode session: verification is the test kit and scratch instances. A subscription login is never read, copied or logged; env var NAMES only (invariant I7).


Filed by an AI agent (Claude Code) at the operator's direction, 2026-10-11, from the setup wizard's mixed-grid design (wizard change 10) and the epic's own open scoping question of 2026-10-11.


## Adopting the fleet's existing Claude Code panes (added 2026-10-11, operator decision)
This story owns the step that adopts Claude Code panes that already exist. #835 makes `holler pane import` leave a pane Holler does not drive alone, and `--adopt-claude-code` adopts it once this harness exists. But once the main import has created the `fleet` profile, a later import is refused with `profile-exists`, so nothing else covers adding those panes afterwards. This story decides and builds how existing Claude Code panes are added to an existing `fleet` profile (for example through `--adopt-claude-code` on a re-run, or a separate verb), and records the choice in the contract amendment. Raised by the architecture review of #835.



---

## Contract amendment (amend-first)

Everything above this line is the issue, verbatim, and stays the source of truth where it is explicit. The contract amendment its Work item 1 puts first is **ADR-0021 section 15, "A second harness: Claude Code"**, with the edits to sections 1 and 2, "Deferred to named stories" and "Decisions taken" (items 22 to 25) in the same commit, and the operator's abort decision recorded in `docs/research/claude-code-pane-spike.md` ("Decisions"). Section 15 is the normative text: this brief does not repeat it, it says what it changes in the issue's list and what the pipeline must do with it. The epic's contract block is a GitHub issue body, which the pipeline does not edit: an amendment comment on #633 carries the same changes.

### What the amendment decides, against the issue's list

| Issue item | Decided in section 15 |
|---|---|
| 1. `HarnessKind` variant, `HARNESS_IDS`, `Pane`/`ProfileSpec` | item 1 (`Claude`, serde `claude`; no new id; `shape()` and `abort_style()` are methods of the kind), item 3 (`HarnessInfo.port` becomes `Option<u16>`; `claude` has a `pid` and no port), item 12 (`--harness`, `port_policy` word `none`) |
| 2. the `HarnessPort` impl | items 3 to 9: `HarnessEndpoint`, a router so `Ports` keeps one `harness`, `process_command`, `new_session_id`, `locate_process`, `login_status`, relaunch-based switch and reset, the inbox-socket delivery, `Activity::WaitingPermission`. A new crate `holler-adapter-claude` (item 15) |
| 3. the test kit | item 15: `FakeClaude`, three seams, `FakeHarnesses`, `claude_cases()` with a mutant per case; the real-binary test is opt-in and `#[ignore]` |
| 4. `--harness`, `profile show`/`apply`, doctor codes | items 12 and 13 (`claude-login-missing`, `pane-waiting`; `server-down` reused; `pane-stalled` is not built) |
| 5. the wizard | item 16: **not in this story** (#803 is open and unbuilt) |
| Adopting existing panes | item 17 (a re-run of `pane import --adopt-claude-code` extends the existing profile; it acts on nothing live; the report ends with `holler pane relaunch NAME`) |
| Abort | the operator's decision (no keypress; stop and relaunch with `--resume`), items 6 and 9 |

### The acceptance, as it can be met (item 18)

The issue's scratch scenario step "delivers a message to each through `holler say`" **needs #839 and, for the Claude pane, a body (#850)**: the hub reaches a harness only through a body, and nothing joins a body's `harness_session_id` to `Pane.session_of_record` until #839. This story proves delivery at `HarnessPort::send_prompt` and `reset --first` on the fakes and in the opt-in real test, writes the `say` step into the scenario as a step marked blocked by #839 and #850 (an `#[ignore]` test that names both, not a silent omission), and **does not close #831**: the PR says "Part of #831" and lists what remains. Every other acceptance line is a test: one OpenCode and one Claude pane from one profile, both in `pane list` with SHOWN and DRIVEN as the records give them, one interrupted (a relaunch on the session of record, the session unchanged), one parked, `doctor` clean, the login never read or printed, the roster and the watchdog seeing the Claude pane where they read the registry.

### Coordination with #835 (c1r2)

One constant and one signal (item 2): `CLAUDE_COMMAND` in `holler-pane/src/pane.rs` beside `HarnessKind`, and #835's required `HostPort::foreground_commands`. At the start of phase F, and again before the adoption step, check `origin/main`: if #835 has merged, adopt its constant and method and add nothing; if not, build them exactly to its brief (`docs/handoffs/835-brief.md` on its branch is the source) and #835 adopts them. Do not touch `wiring.rs` decorators beyond forwarding the methods added here (and #835's, if built here), each forward pinned by a test. The import's adoption step is the last item of the story.

### Blast radius and hot files

- `crates/holler-pane/src/tx_launch.rs` is 831 lines: `scripts/lint.sh` fails at 900. The SingleProcess flow goes in a sibling module (`tx_launch/process.rs` or similar), not into `tx_launch.rs`. `cli.rs` is 780: the one `--harness` flag adds a few lines, no more. `ports.rs` (423) takes the endpoint, the new methods and `Activity::WaitingPermission`. `crates/holler-pane-testkit/src/harness.rs` (599) and `tests/pane_verbs/switch.rs`, `park.rs`, `relaunch.rs` are large: check `wc -l` before extending any file and put new code in a sibling.
- The endpoint change is wide and mechanical: `tx_launch` (and `tx_launch/server.rs`), `tx_switch`, `tx_apply` (and its `observe.rs`), `reconcile` (and `observe.rs`), `import/observe.rs`, `profile_snapshot.rs`, `holler-adapter-opencode`, `FakeHarness`, the conformance suite, `wiring/bind.rs`. Land it first and alone in the test order (the OpenCode behaviour must not change in that step: the whole existing suite stays green with no assertion edited).
- `holler-cli` is the only crate that depends on the adapter crates. The hub depends on none (decision 1 of section 5).
- Docs in the same change: ADR-0021 (the sections the amendment already edited, plus "as built" facts), `docs/pane-control.md` if present, `CHANGELOG.md` (`## [Unreleased]`, linking #831), ADR 0003's rows for the new flag and finding codes, `cli-surface.txt`, `docs/protocol/v2.md` is not touched (the wire does not change).
- No real Claude Code or login is used by any test in the default run. The opt-in real test starts a scratch Claude Code in a scratch directory and a scratch tmux socket only; it never reads, copies or prints the login.
- Coexistence: #835 edits `import*`, `ports.rs` (`HostPort`) and the finding table of section 13; #828 edits section 8 and `CHANGELOG.md`; #649 and #648 are merged or in flight. Rebase onto `origin/main` before the PR; where an ADR sentence was rewritten by another story, edit the rewritten words.
