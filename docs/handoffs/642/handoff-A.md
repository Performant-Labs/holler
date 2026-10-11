# Handoff-A: #642 part 3 - The OpenCode adapter applies and verifies the pane's agent, and sends prompts  (up-front plan review)

**Date:** 2026-10-11
**Branch:** `issue-642-implementation` (base `origin/main` at `7284001`, which contains #802; the branch's own commits touch only `docs/handoffs/`, and the brief was re-cut at `6752b70`)
**Brief reviewed:** `docs/handoffs/642-brief.md` at `6752b70`   **Reuse map:** the brief's decisions D1 to D13 and evidence E1 to E36 (this run has no separate `survey.md`)   **Wireframe:** N/A (no UI surface)
**Verdict:** PASS

This review replaces round 1's, which is in the branch's history (`a4c9ca4`). Round 1 blocked on a base without #802 and on D4(b)'s `mode == "primary"`.

## Summary

PASS: no `block` finding and five `warn` findings.

Round 1's two blocks are closed:

- **The base has #802.** `origin/main` and the merge-base are `7284001`. All 32 file excerpts of the brief (E1 to E31 and E34 to E36) equal `sed -n A,Bp` of their files in this tree, byte for byte.
- **D4(b) is OpenCode's own rule.** It refuses only a missing agent, `mode == "subagent"` and `hidden == true`, and it passes `"all"`. Each of E33's seven cuts and E32's cuts occurs exactly once in the installed `opencode-ai` 1.19.0 binary.

Round 1's nine warns are folded in as D2, D5 to D8, D12 and D13 and acceptance 1, 7 to 9, 11 and 12. Each matches the merged code. The plan extends the right objects:

- `ProcessEnv::prefix`, through one `prefix_with`;
- `server.rs`, for the agent;
- `Call`, `known`, `json_of`, `poll` and `string_at`, for the new requests, plus a body-free sibling of `Call::unexpected`;
- one `/session/status` predicate, shared with `settled`;
- `holler-body`'s child-session scope, re-read as plain JSON;
- the rig's `Recording` and `serve`;
- one `tests/support/fixtures.rs`.

It adds no dependency, keeps the adapter on `holler-pane` alone (ADR-0021 section 5), and leaves every touched file under 800 lines.

The warns are gaps that T and F can close within the brief's rules, without an amendment:

1. acceptance 10 names a hermetic conformance run that does not exist;
2. D7(a) does not say what a failed `/question` or `/permission` answer means, or where the child-session reader lives;
3. D5's tests name a constant that F creates, against the brief's no-compile-error RED note;
4. D12's move list misses four fixtures that the new test files need;
5. D7 makes three docs false.

## Findings

| # | Severity | Plan element | Drift dimension | Finding | Suggested fix |
|---|---|---|---|---|---|
| 1 | warn | Acceptance 10: "`tests/hermetic_test.rs`' existing conformance use for the stub run" | reuse (names an object that does not exist) | `run_harness_conformance` has one caller in the crate: the opt-in, `#[ignore]` test `ac12_the_adapter_passes_the_harness_conformance_suite` (`tests/real_opencode_test.rs:56-60`). `hermetic_test.rs` has no conformance run, and the suite cannot run on the stub, because it launches servers through a launcher and attaches TUIs through tmux. CI therefore never runs the suite for `OpenCodeHarness`. | T builds no hermetic conformance run. Acceptance 10 is the opt-in run. Acceptance 1 pins case 16's property hermetically (the key is in exactly one element). S audits acceptance 10 on that basis. |
| 2 | warn | D7(a): `GET /question` and `GET /permission`, and the child sessions read from `GET /session` | cross-cutting (error handling); anti-duplication | D7(a) does not say what a list answer that is not a 200 JSON array means (an HTML 200, or a 500). By the crate's default rule (`src/lib.rs:28-32`) it would be `unavailable` before step (b) runs, so D7(c)'s "older server" fallback could never be reached on a server without those routes. `holler-body` reads both lists best-effort (`connection.rs:385-391` and `:423-429`). D7(a) also does not say where the reader of `session`'s children goes. `top_level_ids` (`src/lib.rs:528-540`) already reads `parentID` on the same route. | Read a list answer that is not a 200 JSON array as "nothing held", as D7 already does for `GET /session` and as `holler-body` does. A refused connection stays `unavailable` and a missed deadline stays `timeout`. This is safe, because a turn that waits on an answer is still running: its status entry reads `Busy`, and `pane reset` still refuses. T adds an HTML-200 case and a 500 case for each list to acceptance 8. F puts the children reader beside `top_level_ids`, reading `parentID` the same way. |
| 3 | warn | D5's "the tests compare `AGENT_UNUSABLE.as_str()`", against the RED note | plan consistency (test-first) | F creates `AGENT_UNUSABLE` and its crate-root re-export, so every test file that names it fails to compile at RED. The RED note says RED is "not a compile error", and the pipeline's Phase 5 rule counts an import error as an invalid RED. | T follows #808's precedent (`docs/handoffs/808/handoff-T-red.md:94-115`). Compile RED is allowed at this one new name only, and behaviour RED is then shown with a throwaway constant that is never staged. Both go in `handoff-T-red.md`. Name the constant only in `agent_test.rs` and `real_opencode_test.rs`, so that `hermetic_test.rs` (its new retry case and the moved fixtures) keeps a runtime RED. |
| 4 | warn | D12's move list | anti-duplication (test fixtures) | `prompt_test.rs` needs `ID` (`tests/hermetic_test.rs:30`), `known_session` (`:156-164`) and `harness()` (`:103-106`): `send_prompt`'s existence check needs the stub to answer `GET /session/:id`. `agent_test.rs` needs `agent()` (`:642-644`). D12 lists none of the four, so they would be copied. | Move all four into `tests/support/fixtures.rs` with D12's list. `known_session` takes `stub::Stub`, which all three test files include. At Phase 7, a verbatim copy of `known_session` or `harness` counts as a copy, just as one of `config()` or `serving()` does. |
| 5 | warn | D7(b) moves `settled` onto the shared predicate; D7(a) counts a permission request | one source of truth (docs) | Once `settled` waits until the session's entry is absent or `idle`, two docs are false: `settled`'s doc (`src/lib.rs:493-494`, "absent from it or not `busy`") and `SETTLE_POLL`'s doc (`:127-129`, "still busy"). `Activity::HoldingQuestion`'s contract says "Waiting for an answer to a question it asked" (`crates/holler-pane/src/ports.rs:329`), and D7(a) maps a pending permission request onto it. #684's fake, and any later harness, need the same mapping. | F rewrites both docs to say what the shared predicate means. The ADR section 8 edit (acceptance 12) says that a pending permission request, whether the session's own or a direct child's, reads as `HoldingQuestion`. D9 allows only one comment edit in `ports.rs`, so `Activity`'s doc stays as it is unless the brief widens D9. |

### Details

**1. There is no stub conformance run.**

- **The only run.** I searched the crate's tests for `run_harness_conformance` and `conformance`. The one call is `real_opencode_test.rs:60`: `run_harness_conformance(|| Rig::start().into_case(), Recording::launch)`.
- **Why the stub cannot host one.** The suite's cases start servers through the launcher (`conformance/harness.rs:499`, `launch`) and attach TUIs through tmux (`:545`, `attach`), and the stub can answer neither.
- **Round 1 missed this.** Its "the stub run needs no `/config` or `/agent`" accepted the brief's wording without checking it.
- **Nothing is lost.** Case 16 checks only that some element of the argv contains the key, and acceptance 1 pins the whole argv.

**2. What a failed list answer means.**

- **What the brief says.** D7(a) states a best-effort rule for `GET /session` only: "when `GET /session` does not answer an array of objects, only an exact `sessionID` match counts". Acceptance 8 has no case where `/question` or `/permission` answers with HTML or a 500.
- **What happens without a rule.** F follows the crate doc: "each step requires the JSON it relies on. Anything else is `unavailable`".
  - `session_activity` then fails on any server that does not serve the two lists, before it reads `/session/status`.
  - D7(c)'s fallback for an "older server, HTML page" cannot be reached on such a server.
  - OpenCode 1.19.0 serves both lists: `question.list` and `permission.list` each occur once in the binary, each described as covering all sessions.
- **Why best-effort is safe.**
  - A question or a permission request blocks a tool call inside a running turn, so the session's status entry stays `busy` while it waits.
  - For a subagent's question, the parent's entry stays `busy` too, because its task tool waits on the child.
  - So a list that cannot be read turns `HoldingQuestion` into `Busy`. `pane reset` still refuses, with a less precise message.
  - Failing closed instead would block `pane reset` on such a server altogether.
- **If the operator prefers fail-closed,** say so in D7, and drop "(older server, HTML page)" from step (c).
- **Where the children reader goes.** Two readers of `GET /session` would sit in one crate, and their rules agree:
  - `list_sessions`, through `top_level_ids`: a child is a session whose `parentID` is a string;
  - D7(a)'s reader of `session`'s children: `parentID == session`.

  Placed together in `lib.rs`, they read `parentID` one way, as D7(b) already arranges for `/session/status`.

**3. RED at a new constant.**

- **D5's rule is right for the code.** The tests compare against `AGENT_UNUSABLE.as_str()` and never the literal (the test kit's rule, `crates/holler-pane-testkit/src/herdr.rs:44-51`). It follows that acceptance 5, 6 and 11 cannot compile before F adds the re-export.
- **The precedent.** #808 met the same conflict. Its brief allowed compile RED at a new API, and its T proved behaviour RED with a throwaway stub.
- **The gap.** This brief's RED note does not allow compile RED. T should record the exception and its proof, rather than choose between D5 and the note without saying so.

**4. The fixture list.**

- **Needed by the new files.**
  - `ID` and `known_session` are what `abort`'s hermetic cases use for the existence check that D6 reuses (`hermetic_test.rs:445-457`).
  - `harness(quick())` is the adapter that every non-argv case builds.
  - `agent()` is the `AgentKey` helper of the test that acceptance 9 rewrites.
- **Staying in `hermetic_test.rs`,** because only its own cases use them: `HEALTHY`, `Scratch`, `no_abort_sent` and the credential probe.

**5. The docs.** These are small. They are the kind of stale sentence round 1's finding 9 was about, and acceptance 12 does not list them.

### Checked and sound

**The base and the evidence.**

- After `git fetch`, `origin/main` and the merge-base are both `7284001`, and the branch's five commits change only `docs/handoffs/`.
- A script compared each E-section's fenced block with `sed -n A,Bp` of its file. All 32 file excerpts match, across:
  - `ports.rs`, `argv.rs` and `tx_switch.rs` in `holler-pane`;
  - the adapter's `lib.rs`, `server.rs` and `tui.rs`;
  - `hermetic_test.rs`, `real_opencode_test.rs` and `rig.rs`;
  - `holler-body`'s three files, the spike notes and ADR-0021.
- The line counts are those D8 states: `lib.rs` 603, `server.rs` 76, `hermetic_test.rs` 813, `real_opencode_test.rs` 373, `rig.rs` 528.

**D2, the variable and its one prefix builder.**

- `prefix()` delegates to `prefix_with`, so `tui_argv` (`tui.rs:173`) is unchanged, and `serve_command` is the one caller that passes a pair.
- The JSON reaches the server unchanged. The host adapter's `new_window` passes the argv after `new-window ... -- env --`, and `escape` changes only a trailing `;` (`crates/holler-adapter-host/src/tmux.rs:69-87` and `:147-157`). An `AgentKey` cannot contain `;`.
- OpenCode 1.19.0 merges `OPENCODE_CONFIG_CONTENT` last. Its docs call it "inject inline JSON as a final local-scope merge", and its loader applies it after the project's directories. So the variable wins over a project's own `default_agent`, and D4(a) remains the backstop.
- The rig's dead-end provider comes from a file (`$XDG_CONFIG_HOME/opencode/opencode.json`, `rig.rs:363`), not from a `ProcessEnv` pair. The appended pair merges with it and does not replace it.

**D4 and D5, and the launch engine.**

- `verify_serving` runs right after the health wait (`crates/holler-pane/src/tx_launch/server.rs:103-107`).
- Its error passes through `roll_back` unchanged (`tx_launch.rs:726-741`). So a `Refused` stays exit 3 after acting, as the act's second `port-in-use` check does.
- No conformance case calls `verify_serving`, `send_prompt` or `session_activity`.
- Case 16 (`conformance/harness.rs:459-493`) now takes its "argv carries the key" branch through the new pair.
- `HarnessOp` has `VerifyServing` and has no `SendPrompt` or `SessionActivity` (`crates/holler-pane-testkit/src/harness.rs:47-73`), as D8 says. `FakeHarness` owns its own `agent-mismatch` (`:40-42`).
- Nothing outside the crate builds `OpenCodeHarness` yet; that is #649's wiring.

**D6, `send_prompt`.**

- It reuses `known` as `abort` does (`src/lib.rs:318-327`).
- `HttpError::Garbled`'s texts carry no reply bytes (`src/http.rs:114-355`). So the new body-free sibling of `Call::unexpected` closes the one path by which a prompt could reach an error message.

**D7(a) and D7(b), `session_activity`.**

- The scope rule is `holler-body`'s, as E34 shows: direct children only. It is re-read as plain JSON, because ADR-0021 section 5 forbids importing `holler-body`'s types.
- The shared status predicate makes `settled` stricter only for an entry in `retry` and for an entry without a `type`. OpenCode's abort sets the session `idle` (E32), so the abort stays correct.

**D8, where the code goes.**

- The submodules reach the crate root's private `Call`, `known`, `json_of`, `poll`, `string_at` and `OP_*` items, as `attach.rs` does (`attach.rs:25-29`). So `prompt.rs` follows an existing seam.
- The new `Route`s join the one route table, from which `attach.rs` already takes `GET_SESSION` and `SELECT`.

**D12, the shared fixtures.** `#[path = "support/..."]` with `#[allow(dead_code)]` is the crate's existing way to share test code (`hermetic_test.rs:10-11`, `attach_test.rs:9-13`, `stub.rs:138`).

**D13 and the choke-point rule.**

- The adapter's `send_prompt` stays pure transport, with no hold, limit or park logic.
- `pane reset --first` bypassing #646's gate is ADR-0021 section 8's recorded carve-out (E23), and the plan has the ADR say that the adapter now sends for real. That is not drift.

**File sizes.** My estimates after the change, all under the 800-line flag:

- `lib.rs` about 700 lines;
- `server.rs` about 170;
- the new `prompt.rs` about 200;
- `hermetic_test.rs` under 800, once its fixtures move out.

**Settled since round 1: which agent an attached TUI starts on.**

- The server lists `/agent` with the configured `default_agent` first, then by name: the bundle sorts on `s.name===n.default_agent`, and E21's probes show the configured agent first.
- The TUI's local state takes the server's list, filters out subagents and hidden agents, and falls back to the first agent left.
- So an operator's TUI starts on the pane's agent, and D2's "on the server's argv only" is enough.
- Not traced: a selection the TUI may remember for a session it has shown before.

## Notes for O

None are required, since the verdict is PASS. There is no O in this automated path, and a PASS goes straight on to T, so the warns are written for T and F to act on directly:

- **T:** findings 1, 3 and 4, plus the acceptance-8 cases of finding 2.
- **F:** findings 2 and 5.

If the brief is touched again before T starts, fix the wording of acceptance 10 (finding 1) and state D7(a)'s rule for a failed list answer (finding 2) there.

## Patterns referenced

1. `crates/holler-adapter-opencode/src/lib.rs` at `7284001`: `ProcessEnv::prefix`, the `OP_*` and `Route` tables, `Call::send_by` and `Call::unexpected`, `known`, `settled`, `poll`, `top_level_ids` and `json_of`. Also `src/attach.rs:1-29`, the submodule seam.
2. `crates/holler-adapter-opencode/tests/hermetic_test.rs:10-164` and `:528-669`, `tests/real_opencode/rig.rs:195-330` and `tests/real_opencode_test.rs:52-60`: the fixtures, the agent pins, the rig's wrapper and the crate's only conformance run.
3. `crates/holler-pane/src/tx_launch/server.rs:85-107` and `tx_launch.rs:722-741`, with `crates/holler-pane-testkit/src/harness.rs:40-73` and `conformance/harness.rs:459-493`: where `verify_serving` runs, how its refusal leaves the engine, `HarnessOp`, and case 16.
4. `crates/holler-body/src/http_attach_driver/connection.rs:305-327` and `:375-432`: the child-session scope, and the best-effort reading of the two lists.
5. `crates/holler-adapter-host/src/tmux.rs:64-87` and `:147-157`, and `docs/handoffs/808/handoff-T-red.md:94-115`: what reaches tmux, and the compile-RED precedent.
