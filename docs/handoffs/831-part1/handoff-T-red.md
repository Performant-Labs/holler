# Handoff-T-red: Phase 4 - #831 A Claude Code harness for panes

**Date:** 2026-10-11
**Branch:** `issue-831-implementation` (worktree `.claude/worktrees/0831-claude-harness`, head `f065325`)
**Brief / wireframe reviewed:** `docs/handoffs/831-brief.md`; ADR-0021 section 15 (the amendment, `008a995`); `docs/handoffs/831/handoff-A.md` (PASS, W1 to W15); `docs/research/claude-code-pane-spike.md`; #835's committed RED (`36b0de8`) and F (`9bd101b`) on its branch. No wireframe (no UI).

**VERDICT: RED is valid.** It is compile-level for the contract, as #802's round 1 was. Every compile error names a
contract item that does not exist yet: a type, variant, method, field, module or crate. None of them is a typo, a
missing `[[test]]`, or a wrong path. One test is also RED on an assertion today: the CLI surface fixture's two
`--harness` lines do not parse.

## A precondition

Confirmed: A returned PASS on the plan, with 0 blocks and 15 warns. The suite pins the corrections A asked T to pin:

- **W1:** a switch to an unknown conversation is `session-not-found`, and nothing is touched.
- **W2:** `--fix` relaunches an idle pane, refuses a busy one, and reports `fixed` with no `observe-failed`.
- **W3:** doctor is clean for a healthy, idle Claude pane, and a person's `/clear` is a fixable mismatch.
- **W4:** `reset --first` reaches the new process.
- **W11:** a launch into a session that already runs `claude` is refused, with `process-in-use`.
- **W12:** delivery is proven at `send_prompt`.

It also pins these, as A suggested:

- **W5:** both suites run over the router, and the `Wired` forwarding test is extended.
- **W9:** `login_status` fails closed.
- **W10:** `ServeSpec` gains optional fields, `process_command` runs in the plan, and its refusals write nothing.
- **W13:** the `none` policy, and a spec that round-trips.

## Files

New:

- `crates/holler-pane/tests/claude_kind_test.rs`
- `crates/holler-pane-testkit/tests/{fake_claude_test,claude_conformance_test,fake_harnesses_test}.rs`
- `crates/holler-adapter-claude/tests/{hermetic_test,real_claude_test}.rs`
- `crates/holler-cli/tests/pane_claude/{main,rig,spec_flags,launch,lifecycle,doctor,mixed}.rs`. This is a new `[[test]]`
  target, `pane_claude`, declared in `crates/holler-cli/Cargo.toml`.

**The crate skeleton.** The workspace's `crates/*` glob fails every build when a directory has no manifest. So T wrote
two files for the new crate:

- `crates/holler-adapter-claude/Cargo.toml`. It has an empty `[dependencies]` and only the dev-dependencies the tests
  use: `holler-pane`, `holler-pane-testkit`, `serde_json` and `tempfile`.
- `crates/holler-adapter-claude/src/lib.rs`, which holds a doc comment only.

`Cargo.lock` gained the crate's entry. **F owns both files.**

Changed, all of them tests:

- `crates/holler-pane/tests/rework_test.rs`: the exhaustive match gains `Claude` (the spike's item 1).
- `crates/holler-cli/tests/pane_verbs/main.rs`: the `Unwired` test uses endpoints, and the four process methods answer
  `not-implemented`.
- `crates/holler-cli/tests/pane_integration/bind.rs`:
  - `Recording` and `call_every_method` are migrated to endpoints and gain the four process methods;
  - the forwarding test pins those four;
  - the `Resolving` double is migrated to endpoints.
- `crates/holler-cli/tests/fixtures/cli-surface.txt`: two lines, `pane launch ... --harness claude --port-policy none`
  and `pane relaunch ... --harness opencode`.

Every file is under 600 lines: the largest is `hermetic_test.rs` at 558, and `bind.rs` is 621 lines in total.
`scripts/lint.sh` exits 0. The new files are rustfmt-clean.

**Not changed: the endpoint migration of the other existing tests.** A's note 3 says: "F: the endpoint step first, alone,
and green with no assertion edited". About 280 harness call sites and every `HarnessInfo { port: N }` literal move from
`u16` to `&HarnessEndpoint` and `Some(N)`, in these places:

- the rigs `launch/rig.rs`, `apply/rig.rs`, `doctor/rig.rs` and `profile_verbs/rig.rs`;
- `switch.rs`, `doctor/surface.rs` and `fake_harness_test.rs`;
- the OpenCode adapter's tests;
- `ports_test.rs`, `harness_conformance_test.rs` and `real_opencode/rig.rs`;
- every other test double of `HarnessPort`.

This is a signature change with no assertion edited, and it is F's endpoint step. T did it only in the three files above,
which T was editing anyway. Phase 6 checks that no assertion moved.

## The contract the tests pin (F implements exactly these names)

### `holler-pane`

**The kind** (`pane.rs`):

- `HarnessKind::Claude`, with serde name `claude`.
- `HarnessKind::shape() -> Shape`, where `Shape::{ServerAndTui, SingleProcess}`.
- `HarnessKind::abort_style() -> AbortStyle`, where `AbortStyle::{InPlace, Relaunch}`.
- Both enums are in `holler_pane::pane` and are `Debug + Copy + Eq`.

**`HarnessInfo`:**

- `port` becomes `Option<u16>`, with `#[serde(default, skip_serializing_if = "Option::is_none")]`.
- A record loads only as `opencode` with a port, or as `claude` with a pid and no port. On a `Pane`, anything else fails
  to load.
- An `opencode` record reads and writes the same bytes as before.
- `HarnessInfo::endpoint(&self, &PaneName) -> Result<HarnessEndpoint, PaneError>`. It returns `Err` for a value built in
  code that does not fit its kind, and never guesses.

**`HarnessEndpoint`:** `ports::HarnessEndpoint { Port(u16), Process { pane: PaneName, pid: u32 } }`, re-exported at
the root. It is `Debug + Clone + Eq`.

**The `HarnessPort` methods keyed by endpoint:**

- `verify_serving(&HarnessEndpoint, &ServeSpec)`
- `health(&HarnessEndpoint)`
- `create_session(&HarnessEndpoint)`
- `list_sessions(&HarnessEndpoint)`
- `abort(&HarnessEndpoint, &str)`
- `attach_tui(&PaneId, &HarnessEndpoint, &str)`
- `shown_session(&PaneId, &HarnessEndpoint)`
- `send_prompt(&HarnessEndpoint, &str, &str)`
- `session_activity(&HarnessEndpoint, &str)`

`serve_command(&PaneName, u16, &ServeSpec)` and `select_session(&PaneId, &str)` are unchanged.

**The four new methods.** Each is default-armed, and each default refuses `process-unsupported`, `login_status`
included (W9):

- `process_command(&PaneName, &ServeSpec, &SessionChoice) -> Result<Argv, PaneError>`
- `new_session_id() -> Result<String, PaneError>`
- `locate_process(&PaneName, &str) -> Result<Option<u32>, PaneError>`
- `login_status() -> Result<LoginStatus, PaneError>`

**The new types in `ports`:**

- `SessionChoice::{Fresh(String), Keep(String), Resume(String)}`, which is `Debug + Clone`, with `session() -> &str`.
  This is W1's choice:
  - `Fresh`: `--session-id`;
  - `Keep`: `--resume` if a transcript exists, otherwise `--session-id`;
  - `Resume`: `--resume`, or `session-not-found` when there is no transcript.
- `LoginStatus::{Ok, Missing}`, which is `Debug + Eq`. A `NotApplicable` variant is allowed but not used.
- `PROCESS_UNSUPPORTED`: a refusal, exit 3.
- `Activity::WaitingPermission`.
- `ServeSpec` gains the public fields `model: Option<ModelSpec>` and `cwd: Option<String>`, both `None` by default, and
  the builders `with_model(ModelSpec)` and `with_cwd(impl Into<String>)`.

**The snapshot:** `spec_from_pane` of a `claude` record gives `harness.port_policy == "none"` (W13).

### `holler-pane-testkit` (`src/` is F's)

**`claude::FakeClaude`:**

- `new`, and `faults() -> &FaultSwitch<ClaudeOp>`. `ClaudeOp` has at least `Health`, `NewSessionId`, `LocateProcess`
  and `LoginStatus`, logged in call order.
- `start(&Argv) -> Option<Result<u32, PaneError>>`. It answers `None` for an argv the fake did not build. It refuses
  `session-in-use` for a session that is held, or that has a conversation under `--session-id`.
- `stop(&PaneName)`. A conversation outlives its process.
- `processes() -> Vec<ProcessView>` (live processes only) and `process(pid) -> Option<ProcessView>`.
- `ProcessView { pane, pid, session_id, status: ClaudeStatus, waiting_for: Option<String>, inbox: Vec<String>,
  accept_inbound: bool }`, where `ClaudeStatus::{Idle, Busy, Waiting}`.
- `process_specs() -> Vec<ServeSpec>`.
- `seed_conversation(&str)` and `has_conversation(&str) -> bool`. The first delivered message starts a conversation.
- `run_by_hand(&PaneName, &str) -> u32`: a process with no `accept_inbound`. `send_prompt` to it is `inbound-held`:
  nothing is delivered, and the process waits on a permission.
- `clear(pid) -> Result<String, _>`: a new session on the same pid.
- `sigint(pid)`: the process ends.
- `stall(pid)`: `Busy`.
- `hold_permission(pid)`: `WaitingPermission`, with "permission prompt".
- `hold_question(pid)`: `HoldingQuestion`, with "input needed".
- `set_idle(pid)` and `set_login(LoginStatus)`. The login is `Ok` by default.
- The fake's argv carries the pairs `--session-id <id>` or `--resume <id>`.

**`harnesses::FakeHarnesses`:**

- It is `HarnessPort`, built on the one `holler-pane` router (W5). Phase 7 checks that it is not a second router.
- It has `new`, `opencode() -> &FakeHarness` and `claude() -> &FakeClaude`.
- `start(&Argv) -> Option<Result<u32, PaneError>>` runs either fake's argv, and nothing else.
- `stop(&PaneName)` ends both fakes' processes of that pane, and only that pane's.
- **Routing:**
  - `Port` goes to OpenCode.
  - `Process` goes to Claude.
  - `serve_command` goes to OpenCode.
  - The four process methods go to Claude.

**`conformance::claude`:**

- `claude_cases()`, and `run_claude_conformance(fresh, launch, stop) -> Conformance`, where:
  - `fresh: FnMut() -> (S, ClaudeRig, K)`;
  - `launch: Fn(&S, &PaneName, &Argv) -> Result<(), PaneError>`;
  - `stop: Fn(&S, &PaneName) -> Result<(), PaneError>`.
- `ClaudeRig { pub panes: [PaneName; 2], pub pane_ids: [PaneId; 2], pub cwd: String }`.
- `ClaudeRig::sample()` gives `demo-c1r1` and `demo-c2r1`, `scratch:<name>`, and `/srv/demo`.
- **The 12 case ids**, in this order (`DOCUMENTED_CASES` in `claude_conformance_test.rs`), each with the mutant that
  must fail it:

| # | Case | Mutant that must fail it |
|---|---|---|
| 1 | `process-command-starts-nothing` | `process_command` starts it |
| 2 | `launch-then-located` (a bounded wait) | `locate_process` always `None` |
| 3 | `new-session-ids-are-distinct` | a constant id |
| 4 | `located-process-is-healthy-and-idle` | `Busy` |
| 5 | `shown-is-the-held-session` | `None` |
| 6 | `resume-without-conversation-is-session-not-found` (W1) | `Resume` treated as `Keep` |
| 7 | `keep-without-conversation-starts-it` | `Keep` treated as `Resume` |
| 8 | `prompt-to-another-session-is-session-mismatch` | the named session ignored |
| 9 | `resume-after-a-prompt-keeps-the-session` (W12, and the frame's guard) | the prompt dropped |
| 10 | `stopped-process-is-not-healthy` | always healthy |
| 11 | `abort-is-unsupported` | abort acknowledged |
| 12 | `login-status-answers` | the login check errors |

### `holler-adapter-claude` (the crate is F's)

**The types:**

- `ClaudeHarness::new(ClaudeConfig)` builds the real seams. `ClaudeHarness::with_seams(ClaudeConfig, Arc<dyn
  AgentsList>, Arc<dyn InboxWrite>, Arc<dyn LoginCheck>)` takes injected ones.
- `ClaudeConfig::new(bin, config_dir, socket_dir)`, each `impl Into<PathBuf>`.

**The seams.** Each returns the raw I/O, and the adapter parses it:

- `AgentsList::agents_json() -> Result<String, PaneError>`. The fixture is a JSON array of `{cwd, kind, name, pid,
  sessionId, startedAt, status, waitingFor?}`, the spike's shape.
- `InboxWrite::write_line(&Path, &str)`. The line holds no newline.
- `LoginCheck::auth_status_json() -> Result<String, PaneError>`.

**What the hermetic tests pin:**

- **The argv:**
  - `env`, then `-u NAME` for each of the 7 login variables, before the program;
  - no variable is given a value;
  - `--session-id` or `--resume`, decided by the existence of `<config>/projects/<cwd, every non-alphanumeric character
    as "-">/<id>.jsonl`;
  - `--model model_id` and `--effort effort`, with no provider;
  - `--disallowed-tools AskUserQuestion`;
  - `--settings` JSON with `crossSessionInbound: "accept"` and a `SessionStart` hook command.
- **The plan's refusals:** `Resume` with no transcript is `session-not-found`, and an agent is `agent-unsupported`.
- **Session ids** are UUIDs.
- **The login:** `login_status` reads `authMethod` alone. `claude.ai` is `Ok`, and anything else is `Missing`. An
  unreadable answer is never `Ok`, and no error quotes it.
- **Observation:**
  - `locate_process` and `shown_session` come from `sessionId`;
  - `health` is whether an entry exists for the pid;
  - the activity mapping is item 8's table;
  - `abort` is `abort-unsupported`.
- **Delivery (W8), end to end.** The test runs the argv's own `SessionStart` hook through `sh -c`. It passes the argv's
  `NAME=value` assignments, `CLAUDE_CODE_MESSAGING_SOCKET` and the hook's JSON on stdin, and the hook's parent pid is
  the test's. Then:
  - `send_prompt` writes exactly one frame line, `{"type":"user","message":{"role":"user","content":TEXT}}`, to that
    socket;
  - the socket directory is `0700`, every directory under it is `0700`, and every file is `0600`;
  - with no hook report, `send_prompt` is `unavailable` and nothing is written. A planted
    `<config>/sessions/<pid>.json` is never read;
  - another session is `session-mismatch`;
  - after a person's `/clear`, the new session alone is delivered to.

  The file's key and location stay F's choice.

**The opt-in real test.** `real_claude_test.rs` is `#[ignore]` and runs only with `HOLLER_TEST_CLAUDE=1`. It runs
`run_claude_conformance` against a scratch Claude Code in a private tmux server. It sends one model turn (case 9).

### `holler-cli`

**The flag:** `SpecValues.harness: Option<HarnessKind>`, from `--harness`, which is declared in `SpecFlags`
(`pane/args.rs`, W13). Anything but `opencode` or `claude` is `usage`, and the value is never echoed raw.

**The prompt verbs:**

- `interrupt_cmd::interrupt_pane(Ports, pane: &str, profile: Option<&str>, text: Option<&str>) ->
  Result<PaneInterrupt, PaneError>`, where `PaneInterrupt::{Hub { session }, Relaunched { session }}` is `Debug + Eq`.
  For a Claude pane:
  - the pane gates of `resolve_pane_target` apply first (`pane-parked`);
  - with text, it is `interrupt-text-unsupported`;
  - otherwise it is a relaunch on the session of record, the session unchanged and a busy pane included.

  An OpenCode pane answers `Hub { session_of_record }` and touches nothing live.
- `answer_cmd::answer_pane_target(Ports, pane, profile) -> Result<String, PaneError>`: `answer-unsupported` for a
  Claude pane, and the session of record for an OpenCode pane.

**Engine behaviour, pinned through `pane_claude`:**

- **`launch --harness claude --port-policy none`:**
  - The record has `kind` `claude`, the process's pid, no port (no `port` key in JSON), health `healthy`, a fresh
    session of record, and `shown` equal to it.
  - The host ran the argv with `--session-id <sor>`.
  - No OpenCode harness call is made, and nothing is typed.
  - `process_command` gets the spec's model, `cwd` and no agent.
  - With `--profile`, the spec stores `claude` and `none`.
- **The plan's refusals.** Each is exit 3, makes no live act and writes nothing:
  - `port-policy-unsupported`, for `claude` with `fixed:N` and for `opencode` with `none`;
  - `agent-unsupported`;
  - `claude-login-env-refused`, for each of the 7 names, with the message naming the variable;
  - `process-in-use`, for a session whose `foreground_commands` holds `claude` (W11);
  - a bad `--harness` is `usage`, exit 2.
- **`relaunch`:** `stop_owned`, then `run`, with `--resume <sor>` when a conversation exists. The session is kept and
  the new pid recorded. `--harness opencode` on a Claude pane is `harness-change-refused`, with nothing touched.
- **`switch S2`:** a relaunch with `--resume S2`, and S2 recorded. An unknown S2 is `session-not-found` with nothing
  touched and no conversation started (W1). A busy pane is `session-busy`, and a pane waiting on a permission is
  refused with exit 3 (code F's), each with nothing touched.
- **`reset`:** a relaunch with `--session-id <fresh>`. `--first` lands in the new process's inbox (W4). A busy pane is
  `session-busy`.
- **`abort`** of a Claude endpoint is `abort-unsupported`.
- **`park`** writes the hold only, and **`close`** stops the process and deletes the record.
- **`doctor`:**
  - It is clean for a mixed grid with an idle Claude pane (W3).
  - A person's `/clear` gives exactly one `shown-driven-mismatch`, `fixable`, with the remedy `doctor PANE --fix` (W3).
  - `--fix` on an idle pane gives `fixed`: a relaunch with `--resume <sor>`, a new pid recorded, and a clean second
    pass (W2).
  - `--fix` on a busy pane gives `not-fixable` with the remedy `relaunch`, and nothing is stopped (W2).
  - `claude-login-missing` is reported once per pass. Its finding does not contain "auth status".
  - A fleet with no Claude pane never asks the Claude harness anything.
  - `pane-waiting` names its reason.
  - A gone process gives `server-down` with the remedy `relaunch`, and no `observe-failed`.
- **`profile apply`** of a mixed profile:
  - It launches both panes.
  - A second apply writes nothing.
  - A spec that changes a live pane's harness is refused for that pane, with `harness-change-refused`, and the pane is
    untouched.
  - `profile show` names `harness.kind` as drift.
- **The acceptance scenario on the fakes:** both panes are in `pane list` with `shown`, `driven` and `sync` `ok`;
  `send_prompt` delivers to the Claude pane; `interrupt` relaunches it on its session; the OpenCode pane is parked;
  and `doctor` is clean.
- **The `say` step** is `#[ignore = "blocked by #839 ... and #850 ..."]` (item 18).

**#835's API, used by the rig.** The rig's `LinkedHost` forwards `HostPort::foreground_commands`. W11 relies on
`FakeHost` naming a process by the basename of its `argv[0]`. Both are #835's (on its branch at `9bd101b`, not yet on
`main`). If #835 has not merged when F starts, F builds them to #835's committed API, as the brief's coordination
paragraph says.

## Tests authored

| File | Tests | Pins | Tier | Why this tier |
|---|---|---|---|---|
| `holler-pane/tests/claude_kind_test.rs` | 15 | items 1, 3, 5, 8; W9, W13 | unit | pure types and defaults |
| `holler-pane/tests/rework_test.rs` (arm) | 1, changed | item 1: `claude` is in `HARNESS_IDS` | unit | the existing exhaustive pin |
| `testkit/tests/fake_claude_test.rs` | 13 | item 15: the fake's own scenarios | unit (fake) | the rigs rely on them |
| `testkit/tests/claude_conformance_test.rs` | 4 (12 mutants) | item 15, W1, W12 | conformance | one suite for the fake and the real adapter |
| `testkit/tests/fake_harnesses_test.rs` | 5 | item 4, W5: routing, the one linker | conformance + unit | both suites over the router |
| `holler-adapter-claude/tests/hermetic_test.rs` | 15 | items 5, 7, 8, 9, 10; W8 | unit (seams) | no binary, no login |
| `holler-adapter-claude/tests/real_claude_test.rs` | 1, ignored | the frame's guard | real (opt-in) | it must run against the binary |
| `pane_claude/spec_flags.rs` | 2 | item 12, W13 | unit | clap and validate |
| `pane_claude/launch.rs` | 11 | items 6, 10, 11, 12; W10, W11 | integration (verb over fakes) | the plan, act and record of the engine |
| `pane_claude/lifecycle.rs` | 16 | items 6, 9; W1, W4; abort decision | integration | the engines through the verbs |
| `pane_claude/doctor.rs` | 8 | item 13; W2, W3 | integration | the reconcile chain and `--fix` through the verb |
| `pane_claude/mixed.rs` | 5, plus 1 ignored | items 4, 12, 18; acceptance | integration | a mixed grid through `profile apply` |
| `pane_integration/bind.rs` (forwarding) | 2, changed | W5: the `Wired` forwards of the four methods | unit | the existing recorder, extended (no new one) |
| `pane_verbs/main.rs` (`Unwired`) | 1, changed | `Unwired` answers `not-implemented` | unit | the existing pin |
| `fixtures/cli-surface.txt` | 2 lines | item 12: the flag parses on launch and relaunch | surface | the existing fixture |

## RED confirmation

**Assertion-level** (the target compiles today):

```text
$ cargo test -p holler-cli --test cli_surface_test
2 of 148 surface lines failed to parse:
["holler", "pane", "launch", "demo-c1r1", "--harness", "claude", "--port-policy", "none"]: error: unexpected argument '--harness' found
["holler", "pane", "relaunch", "demo-c1r1", "--harness", "opencode"]: error: unexpected argument '--harness' found
test result: FAILED. 2 passed; 1 failed
```

**Compile-level** (`cargo test -p <crate> --test <target> --no-run`): the unique errors per target. Every one is a
contract item above.

**`holler-pane` `claude_kind_test`:**

- no variant `HarnessKind::Claude` (8);
- no `Activity::WaitingPermission` (3);
- no method `HarnessInfo::endpoint` (3);
- `port` is `u16`, not `Option` (the mismatched types, `can't compare u16 with Option`, `expect` on `u16`);
- no `ServeSpec.model` or `.cwd`, no `with_model`;
- no `shape` or `abort_style`;
- no `process_command`, `new_session_id`, `locate_process` or `login_status` on a `HarnessPort`;
- unresolved `ports::{SessionChoice, PROCESS_UNSUPPORTED}`, `pane::{AbortStyle, Shape}` and `holler_pane::HarnessEndpoint`;
- `shown_session` has 2 parameters, not 3.

**`holler-pane` `rework_test`:** no variant `HarnessKind::Claude` (2).

**`holler-pane-testkit` `fake_claude_test`, `claude_conformance_test` and `fake_harnesses_test`:**

- unresolved `holler_pane_testkit::{claude, harnesses, conformance::claude}`;
- unresolved `ports::{LoginStatus, SessionChoice}` and `HarnessEndpoint`;
- no `ServeSpec::with_cwd`;
- no `WaitingPermission`;
- the four process methods are not members of `HarnessPort` (E0407);
- the `shown_session` arity.

**`holler-adapter-claude` `hermetic_test` and `real_claude_test`:** unresolved `holler_adapter_claude::{AgentsList,
ClaudeConfig, ClaudeHarness, InboxWrite, LoginCheck}`, plus the `holler-pane` and `testkit` items above.

**`holler-cli` `pane_claude`:**

- no `HarnessKind::Claude` (11);
- no field `SpecValues.harness` (3);
- unresolved `interrupt_cmd::{interrupt_pane, PaneInterrupt}` and `answer_cmd::answer_pane_target`;
- unresolved `holler_pane_testkit::{claude, harnesses}`, `ports::{SessionChoice, LoginStatus}` and `HarnessEndpoint`;
- `HarnessInfo.port` is `u16`;
- no `HarnessInfo::endpoint`;
- no `ServeSpec::with_cwd`;
- `HostPort::foreground_commands` and `FakeHost::foreground_commands` do not exist yet (#835).

**`holler-cli` `pane_verbs`:** only `main.rs`'s `Unwired` test (7). It references `HarnessEndpoint`, `SessionChoice`
and the four methods.

**`holler-cli` `pane_integration`:** `bind.rs` only (14). It references `HarnessEndpoint`, `LoginStatus` and
`SessionChoice`, the four methods, and the endpoint arities.

**What the RED does not prove.** No assertion beyond the surface fixture is proven to fail for the right reason before
F's code exists. A trial implementation of this contract would be most of the story, so it was not done, unlike #802's
round 1. T-green therefore spot-checks the pins that matter most: W1, W2, W4 and W11, the frame line, and the
`process-unsupported` defaults. It removes the behaviour each one pins and confirms that the test then fails.

## Not authored, and why

- **Item 17: adopting the fleet's existing Claude Code panes** (`pane import --adopt-claude-code` on an existing
  profile). It needs #835 on `main`: its refused flag, its classification and its `profile-exists` path. #835 is F-done
  on its branch, but not merged, and section 17 makes this the story's last item, which stops the story if #835 has
  not landed. **T authors its RED when F reaches item 17 with #835 on `main`** (a T re-entry). Otherwise the PR says
  "Part of #831" and lists it.
- **`harness-dialog-open`** (a launch that waits on `dialog open`). The fake's faults (item 15) have no dialog state.
  It is left as an advisory for F, either as a `FakeClaude` scenario method or covered by the adapter's mapping. No
  test pins it.
- **The adapter's `inbound-held` window** (a delivered line followed by `waiting`). It is timing-dependent at the
  seam, so it is pinned only on `FakeClaude`. The real test (case 9) catches a held frame indirectly: no transcript
  appears, so the resume fails.
- **The roster and the watchdog.** Neither reads the registry on `main`: the roster's pane view is #648, and no
  watchdog code exists. `pane list` and `doctor` are pinned. **S judges this acceptance line.**
- **#803's wizard** and **`pane-stalled`** are not in this story (items 16 and 13).
- **The `say` step** is the ignored test above (item 18).

## Ready for F

RED is valid at compile level, plus one assertion. F may implement against these tests. The order is A's:

1. Move code out of `tx_launch.rs` first (W14).
2. The endpoint step alone, with the migration of the existing tests and no assertion edited.
3. The variant, with W7's `not_opencode` fix (`import/join.rs:408-419` stays green, unchanged).
4. The process methods and the router.
5. The fakes and the suite.
6. The adapter crate.
7. The engines and the verbs.
8. The docs.

Risks to watch:

- **#828:** if #828 (moving `reset --first` to the hub's prompt path) lands first, `reset_first_reaches_the_new_process`
  must be re-pointed at whatever #828 leaves for a Claude pane. Route it back to T, not F.
